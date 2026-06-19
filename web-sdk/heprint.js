/**
 * HePrint Web SDK v1.1.0
 *
 * 纯 JS 单文件实现，无依赖。
 * 使用方式：<script src="heprint.js"></script>
 *
 * v1.1 新增能力：
 *   1. 多任务并行（HE.openTask 返回 taskId，HE.addText(taskId, ...)）
 *   2. 多打印机并发（一键多打印机派发）
 *   3. 任务状态查询（HE.listTasks()）
 *   4. 队列状态查询（HE.getInfo('queueLength')）
 *   5. 自动重连 + 心跳保活
 *   6. 智能降级到浏览器原生打印
 *
 * 浏览器兼容性：Chrome 90+ / Edge 90+ / Firefox 88+ / Safari 14+
 */

(function (root) {
    'use strict';

    // ============ 常量 ============
    const DEFAULT_HOST = '127.0.0.1';
    const DEFAULT_PORT = 18000;
    const VERSION = '1.1.0';
    const RECONNECT_DELAY_MS = 2000;
    const REQUEST_TIMEOUT_MS = 30000;

    // ============ 错误码 ============
    const ERROR_CODES = {
        Success: 0, Unknown: -1, Timeout: -2,
        ConnectionTimeout: 1001, ConnectionRefused: 1002,
        InvalidJsonRpc: 1003, MethodNotFound: 1004, InvalidParam: 1005,
        TaskNotFound: 2001, TaskEmpty: 2002,
        PrinterNotFound: 2004, PrinterOffline: 2005, PaperNotLoaded: 2006,
        PrintFailed: 2007, DuplexNotSupported: 2008,
        WebView2NotInstalled: 3001, HtmlRenderTimeout: 3002,
        ImageDecodeFailed: 4001, FileNotFound: 4002,
        InvalidBarcodeType: 4003, PdfDecodeFailed: 4004, DataTooLarge: 4005
    };

    // ============ Transport ============
    class Transport {
        constructor(host, port, useHttps) {
            this.host = host;
            this.port = port;
            this.useHttps = useHttps;
            this.baseUrl = `http${useHttps ? 's' : ''}://${host}:${port}`;
            this.wsUrl = `ws${useHttps ? 's' : ''}://${host}:${port}/ws`;
            this.ws = null;
            this.reqId = 0;
            this.pending = new Map();
            this.connected = false;
            this.connecting = false;
            this.connectPromise = null;
            this.eventListeners = { connect: [], disconnect: [], error: [], push: [] };
            this.lastError = null;
        }

        on(event, callback) {
            if (this.eventListeners[event]) this.eventListeners[event].push(callback);
        }
        emit(event, ...args) {
            if (this.eventListeners[event]) {
                this.eventListeners[event].forEach(cb => { try { cb(...args); } catch (e) {} });
            }
        }

        connect(timeoutMs = 5000) {
            if (this.connected) return Promise.resolve();
            if (this.connecting && this.connectPromise) return this.connectPromise;

            this.connecting = true;
            this.connectPromise = new Promise((resolve, reject) => {
                let done = false;
                const timer = setTimeout(() => {
                    if (done) return;
                    done = true;
                    this.connecting = false;
                    this.connectPromise = null;
                    this.lastError = new Error('连接 HePrint 服务超时');
                    reject(this.lastError);
                }, timeoutMs);

                try {
                    this.ws = new WebSocket(this.wsUrl);
                } catch (e) {
                    clearTimeout(timer);
                    this.connecting = false;
                    this.connectPromise = null;
                    this.lastError = e;
                    reject(e);
                    return;
                }

                this.ws.onopen = () => {
                    if (done) return;
                    done = true;
                    clearTimeout(timer);
                    this.connected = true;
                    this.connecting = false;
                    this.connectPromise = null;
                    this.lastError = null;
                    this.emit('connect');
                    resolve();
                };

                this.ws.onmessage = (e) => {
                    let msg;
                    try { msg = JSON.parse(e.data); } catch (err) { return; }
                    this.handleMessage(msg);
                };

                this.ws.onclose = () => {
                    const wasConnected = this.connected;
                    this.connected = false;
                    this.pending.forEach(({ reject }) => reject(new Error('连接已断开')));
                    this.pending.clear();
                    if (wasConnected) this.emit('disconnect');
                };

                this.ws.onerror = () => {
                    if (done) return;
                    done = true;
                    clearTimeout(timer);
                    this.connecting = false;
                    this.connectPromise = null;
                    this.lastError = new Error('WebSocket 连接失败');
                    this.emit('error', this.lastError);
                    reject(this.lastError);
                };
            });

            return this.connectPromise;
        }

        handleMessage(msg) {
            if (msg.id != null && this.pending.has(msg.id)) {
                const { resolve, reject, timer } = this.pending.get(msg.id);
                clearTimeout(timer);
                this.pending.delete(msg.id);
                if (msg.error) {
                    const err = new Error(msg.error.message || 'Unknown error');
                    err.code = msg.error.code;
                    err.data = msg.error.data;
                    reject(err);
                } else {
                    resolve(msg.result);
                }
                return;
            }
            if (msg.method === 'HE_PUSH' && msg.params) {
                this.emit('push', msg.params);
            }
        }

        async call(method, params = {}, timeoutMs = REQUEST_TIMEOUT_MS) {
            await this.connect();
            const id = ++this.reqId;
            const req = { jsonrpc: '2.0', id, method, params };
            return new Promise((resolve, reject) => {
                const timer = setTimeout(() => {
                    if (this.pending.has(id)) {
                        this.pending.delete(id);
                        reject(new Error(`请求超时: ${method}`));
                    }
                }, timeoutMs);
                this.pending.set(id, { resolve, reject, timer });
                try { this.ws.send(JSON.stringify(req)); }
                catch (e) {
                    clearTimeout(timer);
                    this.pending.delete(id);
                    reject(e);
                }
            });
        }

        async fetchHttp(path) {
            const r = await fetch(this.baseUrl + path, { headers: { 'Accept': 'application/json' } });
            if (!r.ok) throw new Error(`HTTP ${r.status}`);
            return r.json();
        }

        disconnect() {
            if (this.ws) { try { this.ws.close(); } catch (e) {} this.ws = null; }
            this.connected = false;
        }
    }

    // ============ Task（多任务核心）============
    class PrintTask {
        constructor(he, taskId, fullId, name) {
            this.he = he;
            this.id = taskId;     // 短 ID（T_001）
            this.fullId = fullId; // UUID
            this.name = name;
            this.commands = [];   // 待发送的命令队列
            this.started = false;
        }

        // === 链式 API ===
        page(orient, width, height, name) {
            this.commands.push({ method: 'HE_SET_PAGE', params: { taskId: this.id, orient, width, height, name: name || '' } });
            return this;
        }
        printer(name) {
            this.commands.push({ method: 'HE_SET_PRINTER', params: { taskId: this.id, printer: name } });
            return this;
        }
        copies(n) {
            this.commands.push({ method: 'HE_SET_COPIES', params: { taskId: this.id, count: n } });
            return this;
        }
        option(key, value) {
            this.commands.push({ method: 'HE_SET_OPTION', params: { taskId: this.id, key, value } });
            return this;
        }
        text(t, l, w, h, text) {
            this.commands.push({ method: 'HE_ADD_TEXT', params: { taskId: this.id, top: t, left: l, width: w, height: h, text } });
            return this;
        }
        html(t, l, w, h, html) {
            this.commands.push({ method: 'HE_ADD_HTML', params: { taskId: this.id, top: t, left: l, width: w, height: h, html } });
            return this;
        }
        table(t, l, w, h, tableHtml) {
            this.commands.push({ method: 'HE_ADD_TABLE', params: { taskId: this.id, top: t, left: l, width: w, height: h, tableHtml } });
            return this;
        }
        image(t, l, w, h, src) {
            this.commands.push({ method: 'HE_ADD_IMAGE', params: { taskId: this.id, top: t, left: l, width: w, height: h, src } });
            return this;
        }
        barcode(t, l, w, h, type, value) {
            this.commands.push({ method: 'HE_ADD_BARCODE', params: { taskId: this.id, top: t, left: l, width: w, height: h, btype: type, value } });
            return this;
        }
        pdf(t, l, w, h, content) {
            this.commands.push({ method: 'HE_ADD_PDF', params: { taskId: this.id, top: t, left: l, width: w, height: h, content } });
            return this;
        }
        line(t1, l1, t2, l2, style = 'solid', width = 1) {
            this.commands.push({ method: 'HE_ADD_LINE', params: { taskId: this.id, top1: t1, left1: l1, top2: t2, left2: l2, lineStyle: style, lineWidth: width } });
            return this;
        }
        rect(t, l, w, h, style = 'solid', width = 1) {
            this.commands.push({ method: 'HE_ADD_RECT', params: { taskId: this.id, top: t, left: l, width: w, height: h, lineStyle: style, lineWidth: width } });
            return this;
        }
        newPage() {
            this.commands.push({ method: 'HE_NEW_PAGE', params: { taskId: this.id } });
            return this;
        }
        style(name, value) {
            this.commands.push({ method: 'HE_SET_STYLE', params: { taskId: this.id, name, value } });
            return this;
        }
        styleA(index, name, value) {
            this.commands.push({ method: 'HE_SET_STYLEA', params: { taskId: this.id, index, name, value } });
            return this;
        }

        /**
         * 发送累积的所有命令到服务端
         */
        async send() {
            for (const cmd of this.commands) {
                await this.he.transport.call(cmd.method, cmd.params);
            }
            this.started = true;
            this.commands = [];
            return this;
        }

        /**
         * 立即打印（先 send + 然后 print）
         */
        async print(silent = true) {
            if (!this.started) await this.send();
            return await this.he.printTask(this.id, silent);
        }

        /**
         * 关闭任务
         */
        async close() {
            return await this.he.closeTask(this.id);
        }
    }

    // ============ HePrint 主类 ============
    class HePrint {
        constructor(options = {}) {
            this.VERSION = VERSION;
            this.options = {
                host: options.host || DEFAULT_HOST,
                port: options.port || DEFAULT_PORT,
                useHttps: options.useHttps || false,
                autoFallback: options.autoFallback !== false,
                autoReconnect: options.autoReconnect !== false,
                ...options
            };
            this.transport = new Transport(this.options.host, this.options.port, this.options.useHttps);
            this.callbacks = { onResult: [] };
            this._available = null;

            this.transport.on('push', (data) => {
                if (data.type === 'taskResult' && this.callbacks.onResult.length) {
                    this.callbacks.onResult.forEach(cb => { try { cb(data.result); } catch (e) {} });
                }
            });

            if (this.options.autoReconnect) {
                this.transport.on('disconnect', () => {
                    setTimeout(() => { this.transport.connect().catch(() => {}); }, RECONNECT_DELAY_MS);
                });
            }
        }

        // ========== 检测 ==========
        async isAvailable(forceCheck = false) {
            if (forceCheck) this._available = null;
            if (this._available !== null) return this._available;
            try {
                const r = await this.transport.fetchHttp('/version');
                this._available = !!(r && r.version);
                return this._available;
            } catch (e) {
                this._available = false;
                return false;
            }
        }

        // ========== 连接管理 ==========
        async connect() { await this.transport.connect(); return true; }
        disconnect() { this.transport.disconnect(); }
        on(event, callback) { this.transport.on(event, callback); }

        // ========== v1.1: 任务管理 ==========
        /**
         * 旧 API 兼容：使用 current task
         */
        async init(taskName) {
            const r = await this.transport.call('HE_INIT', { taskName: taskName || 'untitled' });
            return r;
        }

        /**
         * v1.1 新 API：打开独立任务（不共享 current）
         * 返回 PrintTask 对象，可链式调用
         */
        async openTask(taskName) {
            const r = await this.transport.call('HE_OPEN_TASK', { taskName: taskName || 'untitled' });
            return new PrintTask(this, r.taskId, r.fullId, r.taskName);
        }

        /**
         * 通过 taskId 获取已有任务引用（用于已打开的任务）
         * 注意：服务端需要先打开，否则 taskId 无效
         */
        getTask(taskId, fullId, name) {
            return new PrintTask(this, taskId, fullId, name || 'existing');
        }

        /**
         * 关闭任务
         */
        async closeTask(taskId) {
            return await this.transport.call('HE_CLOSE_TASK', { taskId });
        }

        async clear() {
            return await this.transport.call('HE_CLEAR');
        }

        /**
         * 列出所有活跃任务
         */
        async listTasks() {
            const r = await this.transport.call('HE_LIST_TASKS');
            return (r && r.tasks) || [];
        }

        // ========== 通用命令（支持 taskId）============
        async _call(method, params) {
            return await this.transport.call(method, params);
        }

        async addText(t, l, w, h, text, taskId) { return await this._call('HE_ADD_TEXT', { taskId, top: t, left: l, width: w, height: h, text }); }
        async addHtml(t, l, w, h, html, taskId) { return await this._call('HE_ADD_HTML', { taskId, top: t, left: l, width: w, height: h, html }); }
        async addTable(t, l, w, h, tableHtml, taskId) { return await this._call('HE_ADD_TABLE', { taskId, top: t, left: l, width: w, height: h, tableHtml }); }
        async addImage(t, l, w, h, src, taskId) { return await this._call('HE_ADD_IMAGE', { taskId, top: t, left: l, width: w, height: h, src }); }
        async addBarcode(t, l, w, h, type, value, taskId) { return await this._call('HE_ADD_BARCODE', { taskId, top: t, left: l, width: w, height: h, btype: type, value }); }
        async addPdf(t, l, w, h, content, taskId) { return await this._call('HE_ADD_PDF', { taskId, top: t, left: l, width: w, height: h, content }); }
        async addLine(t1, l1, t2, l2, lineStyle, lineWidth, taskId) { return await this._call('HE_ADD_LINE', { taskId, top1: t1, left1: l1, top2: t2, left2: l2, lineStyle: lineStyle || 'solid', lineWidth: lineWidth || 1 }); }
        async addRect(t, l, w, h, lineStyle, lineWidth, taskId) { return await this._call('HE_ADD_RECT', { taskId, top: t, left: l, width: w, height: h, lineStyle: lineStyle || 'solid', lineWidth: lineWidth || 1 }); }
        async newPage(taskId) { return await this._call('HE_NEW_PAGE', { taskId }); }

        async setStyle(name, value, taskId) { return await this._call('HE_SET_STYLE', { taskId, name, value }); }
        async setStyleA(index, name, value, taskId) { return await this._call('HE_SET_STYLEA', { taskId, index, name, value }); }
        async setPage(orient, width, height, name, taskId) { return await this._call('HE_SET_PAGE', { taskId, orient, width, height, name: name || '' }); }
        async setPrinter(printer, taskId) { return await this._call('HE_SET_PRINTER', { taskId, printer }); }
        async setCopies(count, taskId) { return await this._call('HE_SET_COPIES', { taskId, count }); }
        async setOption(key, value, taskId) { return await this._call('HE_SET_OPTION', { taskId, key, value }); }

        // ========== 执行 ==========
        async print() {
            const r = await this.transport.call('HE_PRINT');
            this._notifyResult(r);
            return r;
        }
        async printSilent() {
            const r = await this.transport.call('HE_PRINT_SILENT');
            this._notifyResult(r);
            return r;
        }
        /**
         * v1.1：按 taskId 打印
         */
        async printTask(taskId, silent = true) {
            const r = await this.transport.call('HE_PRINT_TASK', { taskId, silent });
            this._notifyResult(r);
            return r;
        }
        async preview() {
            return await this.transport.call('HE_PREVIEW');
        }
        async sendRaw(printerName, data, encoding) {
            return await this.transport.call('HE_SEND_RAW', { printerName, data, encoding: encoding || 'base64' });
        }

        _notifyResult(r) {
            this.callbacks.onResult.forEach(cb => { try { cb(r); } catch (e) {} });
        }

        /**
         * v1.1：一键多打印机并发打印
         * @param {Array<{task: PrintTask, silent?: boolean}>} jobs
         * @returns {Promise<Array<{taskId, printer, result}>>}
         */
        async printParallel(jobs) {
            const promises = jobs.map(async (job) => {
                try {
                    if (!job.task.started) await job.task.send();
                    const r = await this.printTask(job.task.id, job.silent !== false);
                    return { taskId: job.task.id, name: job.task.name, ok: true, result: r };
                } catch (e) {
                    return { taskId: job.task.id, name: job.task.name, ok: false, error: e.message };
                }
            });
            return await Promise.all(promises);
        }

        // ========== 查询 ==========
        async getPrinters() {
            const r = await this.transport.call('HE_GET_PRINTERS');
            return (r && r.printers) || [];
        }
        async getPrinterCount() {
            const r = await this.transport.call('HE_GET_PRINTER_COUNT');
            return (r && r.count) || 0;
        }
        async getPrinterName(index) {
            const r = await this.transport.call('HE_GET_PRINTER_NAME', { index });
            return (r && r.name) || '';
        }
        async getDefaultPrinter() {
            const r = await this.transport.call('HE_GET_DEFAULT_PRINTER');
            return (r && r.name) || '';
        }
        async hasPrinter(name) {
            const r = await this.transport.call('HE_HAS_PRINTER', { name });
            return !!(r && r.exists);
        }
        async getInfo(key) {
            const r = await this.transport.call('HE_GET_INFO', { key });
            return r && r.value;
        }
        async getQueueStatus() {
            const running = await this.getInfo('runningJobs');
            const queue = await this.getInfo('queueLength');
            return { runningJobs: running || 0, queueLength: queue || 0 };
        }

        // ========== 回调 ==========
        onResult(callback) {
            if (typeof callback === 'function') this.callbacks.onResult.push(callback);
        }

        // ========== C-Lodop 风格兼容别名 ==========
        async PRINT_INIT(taskName) { return await this.init(taskName); }
        async PRINT_INITA(_top, _left, _width, _height, taskName) { return await this.init(taskName || 'untitled'); }
        async ADD_PRINT_TEXT(t, l, w, h, text, taskId) { return await this.addText(t, l, w, h, text, taskId); }
        async ADD_PRINT_HTM(t, l, w, h, html, taskId) { return await this.addHtml(t, l, w, h, html, taskId); }
        async ADD_PRINT_HTML(t, l, w, h, html, taskId) { return await this.addHtml(t, l, w, h, html, taskId); }
        async ADD_PRINT_TABLE(t, l, w, h, tableHtml, taskId) { return await this.addTable(t, l, w, h, tableHtml, taskId); }
        async ADD_PRINT_IMAGE(t, l, w, h, src, taskId) { return await this.addImage(t, l, w, h, src, taskId); }
        async ADD_PRINT_BARCODE(t, l, w, h, type, value, taskId) { return await this.addBarcode(t, l, w, h, type, value, taskId); }
        async ADD_PRINT_PDF(t, l, w, h, content, taskId) { return await this.addPdf(t, l, w, h, content, taskId); }
        async ADD_PRINT_LINE(t1, l1, t2, l2, lineStyle, lineWidth, taskId) { return await this.addLine(t1, l1, t2, l2, lineStyle, lineWidth, taskId); }
        async ADD_PRINT_RECT(t, l, w, h, lineStyle, lineWidth, taskId) { return await this.addRect(t, l, w, h, lineStyle, lineWidth, taskId); }
        async SET_PRINT_STYLE(name, value, taskId) { return await this.setStyle(name, value, taskId); }
        async SET_PRINT_STYLEA(index, name, value, taskId) { return await this.setStyleA(index, name, value, taskId); }
        async SET_PRINT_PAGESIZE(orient, width, height, name, taskId) { return await this.setPage(orient, width, height, name, taskId); }
        async SET_PRINTER_INDEX(printer, taskId) { return await this.setPrinter(printer, taskId); }
        async SET_PRINT_COPIES(count, taskId) { return await this.setCopies(count, taskId); }
        async SET_PRINT_MODE(key, value, taskId) { return await this.setOption(key, value, taskId); }
        async NEWPAGE(taskId) { return await this.newPage(taskId); }
        async PRINT() { return await this.print(); }
        async PRINTA() { return await this.printSilent(); }
        async PREVIEW() { return await this.preview(); }
        async GET_PRINTER_NAMES() { return await this.getPrinters(); }
        async GET_PRINTER_COUNT() { return await this.getPrinterCount(); }
        async GET_PRINTER_NAME(index) { return await this.getPrinterName(index); }
        async GET_DEFAULTPRINTER() { return await this.getDefaultPrinter(); }
        async IS_PRINTER_EXIST(name) { return await this.hasPrinter(name); }
        async GET_VALUE(key) { return await this.getInfo(key); }
        async SEND_PRINT_RAWDATA(printerName, data, encoding) { return await this.sendRaw(printerName, data, encoding); }

        // ========== Builder（兼容旧 API）============
        build(taskName) {
            // v1.1 改为异步
            const builder = {
                _he: this,
                _commands: [],
                _name: taskName,
                init(name) { this._name = name; return this; },
                page(...args) { this._commands.push(['HE_SET_PAGE', { orient: args[0], width: args[1], height: args[2], name: args[3] || '' }]); return this; },
                printer(p) { this._commands.push(['HE_SET_PRINTER', { printer: p }]); return this; },
                copies(n) { this._commands.push(['HE_SET_COPIES', { count: n }]); return this; },
                text(t, l, w, h, txt, style) {
                    this._commands.push(['HE_ADD_TEXT', { top: t, left: l, width: w, height: h, text: txt }]);
                    if (style) for (const k in style) this._commands.push(['HE_SET_STYLE', { name: k, value: style[k] }]);
                    return this;
                },
                async print(silent = true) {
                    // 用 INIT 创建 current
                    await this._he.init(this._name);
                    for (const [m, p] of this._commands) await this._he._call(m, p);
                    return silent ? await this._he.printSilent() : await this._he.print();
                }
            };
            // 加上缺失方法
            ['html', 'table', 'image', 'barcode', 'pdf', 'line', 'rect', 'newPage', 'option', 'style', 'styleA'].forEach(m => {
                builder[m] = function(...args) {
                    const map = { html: 'HE_ADD_HTML', table: 'HE_ADD_TABLE', image: 'HE_ADD_IMAGE',
                                 barcode: 'HE_ADD_BARCODE', pdf: 'HE_ADD_PDF', line: 'HE_ADD_LINE',
                                 rect: 'HE_ADD_RECT', newPage: 'HE_NEW_PAGE' };
                    if (m === 'style') {
                        this._commands.push(['HE_SET_STYLE', { name: args[0], value: args[1] }]);
                    } else if (m === 'styleA') {
                        this._commands.push(['HE_SET_STYLEA', { index: args[0], name: args[1], value: args[2] }]);
                    } else if (m === 'option') {
                        this._commands.push(['HE_SET_OPTION', { key: args[0], value: args[1] }]);
                    } else {
                        const method = map[m];
                        if (method === 'HE_ADD_BARCODE') {
                            this._commands.push([method, { top: args[0], left: args[1], width: args[2], height: args[3], btype: args[4], value: args[5] }]);
                        } else if (method === 'HE_ADD_LINE' || method === 'HE_ADD_RECT') {
                            const ps = { line: ['top1', 'left1', 'top2', 'left2'], rect: ['top', 'left', 'width', 'height'] }[m];
                            const params = { [ps[0]]: args[0], [ps[1]]: args[1], [ps[2]]: args[2], [ps[3]]: args[3], lineStyle: args[4] || 'solid', lineWidth: args[5] || 1 };
                            this._commands.push([method, params]);
                        } else if (method === 'HE_ADD_HTML' || method === 'HE_ADD_TABLE') {
                            this._commands.push([method, { top: args[0], left: args[1], width: args[2], height: args[3], [m === 'table' ? 'tableHtml' : 'html']: args[4] }]);
                        } else if (method === 'HE_ADD_IMAGE' || method === 'HE_ADD_PDF') {
                            this._commands.push([method, { top: args[0], left: args[1], width: args[2], height: args[3], [m === 'image' ? 'src' : 'content']: args[4] }]);
                        } else if (method === 'HE_NEW_PAGE') {
                            this._commands.push([method, {}]);
                        }
                    }
                    return this;
                };
            });
            return builder;
        }

        // ========== 浏览器原生降级 ==========
        nativePrint(html, options) {
            options = options || {};
            const win = window.open('', '_blank', 'width=900,height=700');
            if (!win) throw new Error('浏览器阻止了弹窗');
            win.document.open();
            win.document.write(`<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>${escapeHtml(options.title || 'HePrint')}</title>
    <style>
        @page { size: ${options.pageSize || 'A4'}; margin: ${options.margin || '1cm'}; }
        body { font-family: ${options.fontFamily || "'Microsoft YaHei', Arial, sans-serif"}; color: #000; }
        table { border-collapse: collapse; }
        th, td { border: 1px solid #888; padding: 4px 8px; }
        ${options.styles || ''}
    </style>
</head>
<body>${html}</body>
</html>`);
            win.document.close();
            win.focus();
            setTimeout(() => {
                try { win.print(); } catch (e) {}
                if (options.autoClose !== false) setTimeout(() => win.close(), 600);
            }, 300);
        }

        async smartPrint(buildFn, nativeHtml) {
            const available = await this.isAvailable();
            if (available) {
                try { return await buildFn(); }
                catch (e) {
                    if (this.options.autoFallback && nativeHtml) {
                        this.nativePrint(nativeHtml);
                        return { ok: true, fallback: true, error: e.message };
                    }
                    throw e;
                }
            } else if (this.options.autoFallback && nativeHtml) {
                this.nativePrint(nativeHtml);
                return { ok: true, fallback: true };
            } else {
                throw new Error('HePrint 服务不可用');
            }
        }
    }

    // ============ 工具 ============
    function escapeHtml(s) {
        return String(s).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
    }

    // ============ 暴露 ============
    const defaultInstance = new HePrint();
    defaultInstance.ERROR_CODES = ERROR_CODES;
    defaultInstance.COMMANDS = [
        'HE_INIT', 'HE_OPEN_TASK', 'HE_CLOSE_TASK', 'HE_LIST_TASKS', 'HE_CLEAR',
        'HE_ADD_TEXT', 'HE_ADD_HTML', 'HE_ADD_TABLE', 'HE_ADD_IMAGE', 'HE_ADD_BARCODE',
        'HE_ADD_PDF', 'HE_ADD_LINE', 'HE_ADD_RECT', 'HE_SET_STYLE', 'HE_SET_STYLEA',
        'HE_SET_PAGE', 'HE_SET_PRINTER', 'HE_SET_COPIES', 'HE_SET_OPTION',
        'HE_PRINT', 'HE_PRINT_SILENT', 'HE_PRINT_TASK', 'HE_PREVIEW', 'HE_NEW_PAGE',
        'HE_GET_PRINTERS', 'HE_GET_PRINTER_COUNT', 'HE_GET_PRINTER_NAME',
        'HE_GET_DEFAULT_PRINTER', 'HE_HAS_PRINTER', 'HE_GET_INFO', 'HE_ON_RESULT',
        'HE_SEND_RAW', 'HE_VERSION'
    ];
    defaultInstance.HePrint = HePrint;
    defaultInstance.PrintTask = PrintTask;

    if (typeof module !== 'undefined' && module.exports) {
        module.exports = defaultInstance;
        module.exports.HePrint = HePrint;
        module.exports.PrintTask = PrintTask;
    } else {
        root.HE = defaultInstance;
    }
})(typeof window !== 'undefined' ? window : this);
