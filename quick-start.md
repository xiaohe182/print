# 🚀 HePrint 快速启动指南

> 5 分钟从零跑起一个完整的本地打印服务

---

## 📋 你已经拿到的文件

```
D:\打印插件v1\
├── README.md               # 项目入口
├── 设计文档.md             # 完整架构设计（12000 字）
├── 快速启动.md             # 本文件
├── index.html              # ★ 测试页（双击直接用浏览器打开）
│
├── Cargo.toml              # Rust workspace 根
├── rust-toolchain.toml
├── .gitignore
│
├── crates\                 # 5 个 Rust crate
│   ├── heprint-core\       # 数据模型
│   ├── heprint-render\     # 渲染（条码、图像）
│   ├── heprint-print\      # Win GDI 打印
│   ├── heprint-server\     # Axum + WS 服务
│   └── heprint-cli\        # 主入口（输出 heprint.exe）
│
└── web-sdk\
    └── heprint.js          # 前端 SDK（纯 JS，无依赖）
```

---

## ⚡ 立即可用：浏览器原生打印（无需编译）

**最简单的测试方式 — 不需要任何编译**：

1. 双击打开 `D:\打印插件v1\index.html`
2. 点击左侧 **「① 浏览器原生打印」**
3. 点击「🖨️ 打印」按钮
4. 浏览器弹出系统打印对话框 → 选打印机 → 完成

✅ 这条路径完全靠浏览器原生 `window.print()`，**不依赖任何后端服务**。

---

## 🎯 完整版：编译并运行 HePrint 服务

### 前置条件

需要安装 **Rust** 工具链（一次性，~5 分钟）：

#### 方式 A：使用 rustup（推荐）

```cmd
:: 1. 下载并运行 rustup 安装器
:: 访问 https://rustup.rs/ 或直接：
:: https://win.rustup.rs/x86_64

:: 2. 安装完成后重启命令行，验证
rustc --version
cargo --version
```

#### 方式 B：使用 winget

```cmd
winget install Rustlang.Rustup
```

### 编译

```cmd
:: 进入项目目录
D:
cd "D:\打印插件v1"

:: Debug 构建（快，体积大，仅用于开发）
cargo build

:: Release 构建（慢，体积小，发布用）
cargo build --release
```

第一次编译需要拉依赖，约 **5-10 分钟**。
之后增量编译只需 **5-10 秒**。

### 运行

```cmd
:: Debug 版本
.\target\debug\heprint.exe

:: Release 版本
.\target\release\heprint.exe

:: 查看帮助
.\target\release\heprint.exe --help

:: 自定义端口
.\target\release\heprint.exe --port 18000
```

启动成功后你会看到：

```
┌─────────────────────────────────────────┐
│   HePrint v1.0.0  -  极简打印服务       │
│   Rust + Axum + Win32 GDI               │
│   监听端口: 127.0.0.1:18000              │
└─────────────────────────────────────────┘
2026-06-17T12:00:00Z  INFO HePrint 服务启动: http://127.0.0.1:18000
2026-06-17T12:00:00Z  INFO WebSocket 端点: ws://127.0.0.1:18000/ws
```

### 测试服务

打开浏览器访问：

- 服务首页：<http://127.0.0.1:18000/>
- 版本：<http://127.0.0.1:18000/version>
- 健康：<http://127.0.0.1:18000/health>
- 打印机列表：<http://127.0.0.1:18000/printers>
- 默认打印机：<http://127.0.0.1:18000/printers/default>

### 用 index.html 完整测试

1. 启动 `heprint.exe`（保持窗口运行）
2. 双击打开 `index.html`
3. 顶部状态栏会显示 **「服务已连接」**（绿色圆点）
4. 左侧选择测试用例：
   - **②** 服务连接 → 列出打印机
   - **③** 打印一行文字 → 立即出纸
   - **④** 打印订单小票（综合测试）
   - **⑤** A4 报表
   - **⑥** 图片打印
   - **⑦** 二维码/条码
   - **⑧** 直线/矩形

---

## 🛠️ 检查包体大小

```cmd
:: 编译 release 版本
cargo build --release

:: 查看大小（应该 < 5 MB）
dir target\release\heprint.exe
```

预期 ~3-5 MB。如需进一步压缩：

```cmd
:: 安装 UPX：https://github.com/upx/upx/releases
:: 然后压缩
upx --best --ultra-brute target\release\heprint.exe
```

UPX 压缩后通常可缩到 1.5-2 MB。

---

## 🐛 常见问题

### Q1：浏览器报错 "WebSocket 连接失败"
- ✅ 确认 `heprint.exe` 正在运行
- ✅ 检查防火墙：第一次启动 Windows 会弹窗，选「允许」
- ✅ 确认端口 18000 未被占用：`netstat -ano | findstr 18000`

### Q2：cargo build 报错 "linker `link.exe` not found"
需要装 **MSVC 构建工具**：

```cmd
:: 装 Visual Studio Build Tools
winget install Microsoft.VisualStudio.2022.BuildTools

:: 或者直接装 Visual Studio 2022 Community（含 C++ 工具链）
```

### Q3：windows crate 编译慢
首次编译 windows crate 会比较慢（~3-5 分钟），这是正常的，因为它生成大量 Win32 API 绑定。后续增量编译会很快。

### Q4：打印没反应
- ✅ 检查默认打印机是否设置：控制面板 → 打印机
- ✅ 检查打印机是否在线
- ✅ 看 heprint.exe 控制台日志

### Q5：中文乱码
代码中的字体已设为 `Microsoft YaHei`，应该没问题。如果仍乱码：
- 确认 Windows 系统区域设置含中文
- 检查打印机是否支持中文字体

### Q6：v1 不支持的功能
当前 P1 版本只实现了文本/图片/QRCode/直线/矩形/打印机查询。
**未实现**（后续 P2/P3 阶段）：
- HTML/Table 渲染（需要 WebView2 集成）
- PDF 直接打印
- ESC/POS 原生指令（HE_SEND_RAW）
- HE_PREVIEW（预览窗口）
- 一维条码（Code128 等，仅 QRCode 可用）

---

## 📞 进阶使用

### 在你自己的网页里使用

```html
<!DOCTYPE html>
<html>
<head>
    <script src="path/to/heprint.js"></script>
</head>
<body>
    <button onclick="doPrint()">打印</button>
    <script>
    async function doPrint() {
        // 检测服务
        if (!await HE.isAvailable()) {
            alert('请先启动 heprint.exe');
            return;
        }

        // 初始化任务
        await HE.init('我的订单');

        // 添加内容
        await HE.addText(100, 100, 600, 50, '订单号: #001');
        await HE.setStyle('FontSize', 14);
        await HE.setStyle('Bold', 1);

        await HE.addBarcode(200, 100, 300, 300, 'QRCode', 'https://example.com');

        // 设置纸张（80mm 卷筒）
        await HE.setPage(3, 80, 0, '');

        // 静默打印
        const result = await HE.printSilent();
        if (result.ok) {
            console.log('打印成功');
        }
    }
    </script>
</body>
</html>
```

### API 完整参考

详见 `设计文档.md` 第 3 章。

---

## ✅ 当前完成度（v1.0.0）

| 阶段 | 状态 | 内容 |
|---|---|---|
| **P0 骨架** | ✅ 完成 | Cargo workspace、Axum 服务、WS、HTTP 路由 |
| **P1 核心打印** | ✅ 完成 | HE_INIT/ADD_TEXT/ADD_IMAGE/ADD_BARCODE/ADD_LINE/ADD_RECT/PRINT_SILENT/打印机查询 |
| **P2 HTML 渲染** | ⏳ 规划中 | WebView2 集成（HE_ADD_HTML/TABLE/PREVIEW） |
| **P3 高级功能** | ⏳ 规划中 | PDF/ESC-POS RAW/SET_OPTION 双面 |
| **P4 发布** | ⏳ 规划中 | HTTPS 证书/系统托盘/Inno Setup 安装包 |

---

## 🎁 三句话总结

1. **不想编译** → 双击 `index.html` 测试浏览器原生打印
2. **要测试服务** → `cargo build --release` → 启动 `heprint.exe` → 打开 `index.html`
3. **要部署给别人** → P4 阶段做 Inno Setup 安装包（含 WebView2 引导器）

祝玩得开心！🎉
