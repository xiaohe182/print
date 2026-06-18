# HePrint v1 设计文档

> 版本：v1.0.0 ｜ 状态：设计阶段 ｜ 最后更新：2026-06-17

---

# 1. 项目愿景与目标

## 1.1 解决什么问题

传统网页打印方案存在一个困境：

- 浏览器 `window.print()` 功能太弱 —— 不能选打印机、不能静默、不能精确定位
- C-Lodop 功能太多 —— 80+ 命令学起来累、安装包 10 MB、依赖已废弃的 IE 内核
- 商业方案太贵 —— 按年付费、限制域名

**HePrint 的目标是：在"功能够用"和"体积最小"之间找到最佳平衡点。**

## 1.2 与 C-Lodop 的差异

| 维度 | C-Lodop | HePrint v1 |
|---|---|---|
| **授权模型** | 个人免费 / 企业收费（去水印） | 自有，无功能限制 |
| **安装包** | ~10 MB | 目标 ≤ 5 MB（含 WebView2 引导器） |
| **HTML 渲染** | IE ActiveX（不可控、老旧） | WebView2（系统 Edge Chromium） |
| **API 数量** | 80+ 命令 | ~26 个精选命令 |
| **API 风格** | 全大写过程式 | `HE_` 前缀 + Promise + 链式 |
| **TypeScript** | 无官方支持 | 一级支持，类型安全 |
| **端口** | 8000 / 8443 | 18000 / 18443（可共存） |
| **设计器** | 内置可视化设计 | v1 不做，v2 看反馈 |
| **跨平台** | Win 为主 | v1 仅 Win，架构预留跨平台能力 |
| **HTTPS 证书** | 依赖用户自己装 | 安装时自动生成并安装自签 CA |
| **ESC/POS** | 付费版本才稳定 | v1 内置支持 |

## 1.3 性能与包体目标

| 指标 | 目标值 |
|---|---|
| 安装包大小 | ≤ 5 MB（含 WebView2 引导器） |
| 核心 binary | ≤ 3 MB（UPX 压缩后 ≤ 2 MB） |
| 首次启动时间 | < 200ms |
| 单个打印任务延迟 | < 100ms（静默模式） |
| 内存占用（空闲） | < 20 MB |
| 内存占用（打印中） | < 100 MB |
| 前端 SDK 体积 | ~10 KB (min + gzip) |

---

# 2. 总体架构

## 2.1 系统架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                        浏览器（用户业务页面）                        │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  heprint.min.js（~10 KB）                                   │   │
│  │  ┌──────────┐  ┌──────────┐  ┌───────────────┐            │   │
│  │  │ HE 主对象 │  │ Transport│  │ Builders      │            │   │
│  │  │ init      │  │ WS Client│  │ TextBuilder   │            │   │
│  │  │ print     │  │ HTTP POST│  │ BarcodeBuilder│            │   │
│  │  │ preview   │  │ 重连逻辑 │  │ TableBuilder  │            │   │
│  │  └──────┬───┘  └────┬─────┘  └───────────────┘            │   │
│  └─────────┼───────────┼─────────────────────────────────────┘   │
└────────────┼───────────┼─────────────────────────────────────────┘
             │           │
     ┌───────┴───────────┴──────────────────────────────────────────┐
     │      HTTP REST (JSON)         WebSocket (JSON-RPC 2.0)      │
     │      127.0.0.1:18000          127.0.0.1:18000/ws            │
     │      HTTPS: 18443                                            │
     └──────────────────────────────────────────────────────────────┘
                             │
┌────────────────────────────┴────────────────────────────────────────┐
│                     HePrint 本地服务（heprint.exe）                   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                  heprint-server (Axum)                       │   │
│  │  ┌──────────┐  ┌───────────┐  ┌───────┐  ┌─────────────┐  │   │
│  │  │HTTP Router│  │WS Handler │  │CORS   │  │TLS/Cert     │  │   │
│  │  └─────┬────┘  └─────┬─────┘  └───────┘  └──────┬──────┘  │   │
│  └────────┼─────────────┼──────────────────────────┼──────────┘   │
│           │             │                          │               │
│           ▼             ▼                          ▼               │
│  ┌────────────────────────────────────────────────────────────┐   │
│  │                  heprint-core                              │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │   │
│  │  │PrintTask │  │PrintItem │  │PrintStyle│  │ Printer  │  │   │
│  │  │Manager   │  │(8 types) │  │System    │  │Registry  │  │   │
│  │  └─────┬────┘  └─────┬────┘  └────┬─────┘  └─────┬────┘  │   │
│  └────────┼─────────────┼────────────┼───────────────┼──────┘   │
│           │             │            │               │           │
│           ▼             ▼            ▼               ▼           │
│  ┌────────────────┐  ┌─────────────────┐  ┌──────────────────┐  │
│  │ heprint-render │  │  heprint-print  │  │ 他系统 API       │  │
│  │ ─────────────  │  │  ─────────────  │  │ (future)         │  │
│  │ WebView2 Render│  │  Winspool (GDI) │  │  CUPS / IPP      │  │
│  │ QR / 条码生成  │  │  ESC/POS (USB)  │  │                   │  │
│  │ 图像编解码     │  │  PDF 直发       │  │                   │  │
│  └────────────────┘  └─────────────────┘  └──────────────────┘  │
│                           │                                       │
└───────────────────────────┼───────────────────────────────────────┘
                            │
                            ▼
              ┌─────────────────────────┐
              │    系统打印驱动          │
              │  (winspool.drv + GDI)   │
              └────────────┬────────────┘
              ┌────────────┴────────────┐
              │                         │
              ▼                         ▼
          USB / 网络                  LPT / Serial
          (USBPRINT:   (WSDPRINT:
           HP-LaserJet)  XP-80C)
```

## 2.2 模块划分（5 个 crate）

| crate | 职责 | 依赖 |
|---|---|---|
| `heprint-core` | 命令数据模型、PrintTask 管理器、PrintStyle 系统、打印机注册表、错误码定义 | — |
| `heprint-render` | WebView2 将 HTML → 位图/PDF、QR/条码 → 位图、图像解码 | core |
| `heprint-print` | Winspool 打印机通信、GDI 绘制、ESC/POS 直发、PDF 直发 | core |
| `heprint-server` | Axum HTTP + WebSocket 服务、命令路由、JSON-RPC 2.0、TLS/HTTPS、CORS | core + render + print |
| `heprint-cli` | 主程序入口：初始化、系统托盘、信号处理、配置加载 | server |

## 2.3 通信协议

### HTTP REST（同步查询，无状态）

| 路径 | 方法 | 说明 |
|---|---|---|
| `/version` | GET | 返回服务版本号 |
| `/health` | GET | 健康检查 |
| `/printers` | GET | 打印机列表 |
| `/printers/default` | GET | 默认打印机 |

### WebSocket JSON-RPC 2.0（核心交互）

所有打印命令通过 WS 发送。格式：

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "HE_INIT",
  "params": { "taskName": "订单小票" }
}
```

服务端响应：

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { "success": true }
}
```

命令执行是顺序的（按 `id` 递增），服务器按序处理。打印执行时批量从任务管理器取出。

### 二进制数据（图像/PDF/RAW）

HE_ADD_IMAGE、HE_ADD_PDF 等命令的 base64 数据直接在 JSON `params` 中传输。超大图像（> 1 MB）通过分块发送 + 服务端临时文件缓存。

## 2.4 端口与证书策略

| 端口 | 协议 | 说明 |
|---|---|---|
| 18000 | HTTP | 开发环境、HTTP 网页使用 |
| 18443 | HTTPS | 生产环境（自签证书，安装时自动导入系统受信任根） |

**为什么不抢 C-Lodop 的 8000/8443？**

- 很多用户同时用 C-Lodop，端口冲突会导致两者都无法使用
- 18000/18443 大概率空闲，不冲突
- 如果 18000 也被占用，可回退到 38100（注册表可配）

**证书策略（P4 实现）：**

```
第一次启动时：
  1. 用 rcgen 生成自签 CA 证书（heprint-ca.pem）
  2. 生成服务端证书（heprint-server.pem），CA 签发
  3. 将 CA 证书导入系统"受信任的根证书颁发机构"
     → 浏览器访问 https://127.0.0.1:18443 无安全警告
  4. 监听 18443（rustls）

重启后：
  - 复用已有证书（证书存放在 %LOCALAPPDATA%/HePrint/cert/）
  - 每 365 天自动刷新
```

---

# 3. API 命令规范（HE_xxx 全集）

## 3.1 命令分类速查表

### 分类索引

| 分类 | 命令 | 行数 |
|---|---|---|
| 🚀 初始化 | `HE_INIT`, `HE_VERSION`（属性） | 2 |
| 📝 添加内容 | `HE_ADD_TEXT`, `HE_ADD_HTML`, `HE_ADD_TABLE`, `HE_ADD_IMAGE`, `HE_ADD_BARCODE`, `HE_ADD_PDF`, `HE_ADD_LINE`, `HE_ADD_RECT` | 8 |
| 🎨 样式 | `HE_SET_STYLE` | 1 |
| ⚙️ 全局参数 | `HE_SET_PAGE`, `HE_SET_PRINTER`, `HE_SET_COPIES`, `HE_SET_OPTION` | 4 |
| ▶️ 执行 | `HE_PRINT`, `HE_PRINT_SILENT`, `HE_PREVIEW`, `HE_NEW_PAGE` | 4 |
| 🔍 查询 | `HE_GET_PRINTERS`, `HE_GET_DEFAULT_PRINTER`, `HE_HAS_PRINTER`, `HE_GET_INFO` | 4 |
| 🔗 回调 | `HE_ON_RESULT` | 1 |
| 📡 原生 | `HE_SEND_RAW` | 1 |
| **总计** | | **25** |

### 全部命令完整签名

以下每个命令包含：**签名 / 参数 / 返回值 / 示例 / 简要实现说明**。

---

### 🚀 初始化

#### HE_INIT

```ts
HE_INIT(taskName: string): void
```

| 参数 | 类型 | 说明 |
|---|---|---|
| `taskName` | string | 打印任务名称，用于日志和回调关联 |

**返回值**：无

**示例**：
```js
HE_INIT("订单小票 #20260617001");
```

**实现说明**：
- 清空当前任务管理器中的 PrintTask
- 新建一个 PrintTask（含唯一 taskId 生成）
- taskName 仅在日志/UI 中显示，不参与业务逻辑

**对应 Lodop**：`PRINT_INIT`

---

#### HE_VERSION（属性）

```ts
const HE_VERSION: string
```

**返回值**：语义化版本号，如 `"1.0.0"`

**示例**：
```js
console.log(HE_VERSION);  // "1.0.0"
```

**实现说明**：
- 编译时从 `Cargo.toml` 获取
- 首次连接时自动获取并缓存到前端 SDK

**对应 Lodop**：`VERSION`

---

### 📝 添加内容（8 个）

#### HE_ADD_TEXT

```ts
HE_ADD_TEXT(top, left, width, height, text: string): void
```

| 参数 | 类型 | 说明 |
|---|---|---|
| `top` | number (mm × 10) | 左上角 Y 坐标，单位 0.1mm（下同） |
| `left` | number (0.1mm) | 左上角 X 坐标 |
| `width` | number (0.1mm) | 宽度 |
| `height` | number (0.1mm) | 高度 |
| `text` | string | 纯文本内容（非 HTML） |

**示例**：
```js
HE_ADD_TEXT(200, 100, 3000, 400, "订单号：#20260617001");
HE_SET_STYLE("FontSize", 28);
HE_SET_STYLE("Bold", 1);
```

**实现说明**：
- 后端调用 GDI `TextOut` / `DrawTextW` 绘制到 DC
- 字体、字号、对齐等由后续 `HE_SET_STYLE` 设定
- 文本区域以矩形 outline 方式排版

**对应 Lodop**：`ADD_PRINT_TEXT`

---

#### HE_ADD_HTML

```ts
HE_ADD_HTML(top, left, width, height, html: string): void
```

| 参数 | 类型 | 说明 |
|---|---|---|
| `top` | number (0.1mm) | 同 |
| `left` | number (0.1mm) | 同 |
| `width` | number (0.1mm) | 同 |
| `height` | number (0.1mm) | 同 |
| `html` | string | HTML 字符串（完整片段，包含 CSS） |

**示例**：
```js
HE_ADD_HTML(200, 100, 2800, 500, `<div style="color:red;font-weight:bold">警告：已逾期</div>`);
```

**实现说明**（P2 阶段实现）：
- 创建一个隐藏的 WebView2 实例
- `NavigateToString(html)` 加载，等渲染完成
- `PrintToPdfAsync()` 输出为 PDF
- PDF 页面嵌入到当前打印任务
- WebView2 实例随后销毁

**对应 Lodop**：`ADD_PRINT_HTM`

---

#### HE_ADD_TABLE

```ts
HE_ADD_TABLE(top, left, width, height, tableHtml: string): void
```

| 参数 | 类型 | 说明 |
|---|---|---|
| `top` | number (0.1mm) | 同 |
| `left` | number (0.1mm) | 同 |
| `width` | number (0.1mm) | 同 |
| `height` | number (0.1mm) | 同 |
| `tableHtml` | string | 完整的 `<table>...</table>` HTML |

**示例**：
```js
const tbl = document.getElementById('orderItems').outerHTML;
HE_ADD_TABLE(500, 100, 2800, 1500, tbl);
```

**实现说明**：
- 同 HE_ADD_HTML，自动包装 `<html><body>...</body></html>`
- 添加 `table { border-collapse: collapse; }` 样式规则
- 分页支持：如果 table 超过一页，服务端自动切分

**对应 Lodop**：`ADD_PRINT_TABLE`

---

#### HE_ADD_IMAGE

```ts
HE_ADD_IMAGE(top, left, width, height, src: string): void
```

| 参数 | 类型 | 说明 |
|---|---|---|
| `top` | number (0.1mm) | 同 |
| `left` | number (0.1mm) | 同 |
| `width` | number (0.1mm) | 同 |
| `height` | number (0.1mm) | 同 |
| `src` | string | 支持：base64 (`data:image/png;base64,...`) / 本地路径 (`C:/images/logo.png`) / URL (`http://...`) |

**示例**：
```js
const logo = canvas.toDataURL('image/png');
HE_ADD_IMAGE(50, 50, 600, 200, logo);
```

**实现说明**：
- 识别前缀：`data:` → base64 解码，`http` → 下载，`C:/` → 本地读取
- 用 `image` crate 解码为 RGBA 位图
- GDI `StretchDIBits` 缩放绘制到 DC
- auto-fit: 如果尺寸填 0，自动按原图宽高比

**对应 Lodop**：`ADD_PRINT_IMAGE`

---

#### HE_ADD_BARCODE

```ts
HE_ADD_BARCODE(top, left, width, height, type: BarcodeType, value: string): void
```

| 参数 | 类型 | 说明 |
|---|---|---|
| `top` | number (0.1mm) | 同 |
| `left` | number (0.1mm) | 同 |
| `width` | number (0.1mm) | 同 |
| `height` | number (0.1mm) | 同 |
| `type` | BarcodeType | 枚举值（见下方） |
| `value` | string | 条码内容 |

**BarcodeType 枚举**：
```
"QRCode" | "Code128" | "Code39" | "EAN13" | "EAN8" | "UPC-A" | "UPC-E" | "PDF417" | "DataMatrix"
```

**示例**：
```js
HE_ADD_BARCODE(800, 800, 500, 500, "QRCode", "https://example.com/order/001");
```

**实现说明**：
- QRCode → `qrcode` crate → 位图
- Code128/Code39/EAN13 → `barcoders` crate → 位图
- PDF417 → 简单内置编码 → 位图
- DataMatrix → 视体积决定是否引入额外 crate（低优先级）
- 位图 → GDI 绘制

**对应 Lodop**：`ADD_PRINT_BARCODE`

---

#### HE_ADD_PDF

```ts
HE_ADD_PDF(top, left, width, height, content: string): void
```

| 参数 | 类型 | 说明 |
|---|---|---|
| `top` | number (0.1mm) | 同 |
| `left` | number (0.1mm) | 同 |
| `width` | number (0.1mm) | 同 |
| `height` | number (0.1mm) | 同 |
| `content` | string | base64 编码的 PDF 字节或 HTTP URL |

**示例**：
```js
HE_ADD_PDF(100, 100, 0, 0, await fetch('/invoice.pdf').then(r => r.blob()).then(b => toBase64(b)));
```

**实现说明**：
- 检测 `data:application/pdf;base64,` 前缀或 `http` 前缀
- PDF → 用 `pdfium-render` crate 或系统 PDF API 渲染每一页到位图
- 支持分页渲染（一页按一页印）

**对应 Lodop**：`ADD_PRINT_PDF`

---

#### HE_ADD_LINE

```ts
HE_ADD_LINE(top1, left1, top2, left2, lineStyle?: string, lineWidth?: number): void
```

| 参数 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `top1` | number (0.1mm) | — | 起点 Y |
| `left1` | number (0.1mm) | — | 起点 X |
| `top2` | number (0.1mm) | — | 终点 Y |
| `left2` | number (0.1mm) | — | 终点 X |
| `lineStyle` | string | `"solid"` | `"solid" | "dashed" | "dotted"` |
| `lineWidth` | number | 1 | 线宽（单位：0.1mm 约≈ 0.3pt） |

**示例**：
```js
HE_ADD_LINE(1000, 100, 1000, 5000, "dashed", 2);
```

**实现说明**：
- GDI `MoveToEx` + `LineTo` 绘制
- dashed/dotted → GDI `CreatePen(PS_DASH/PS_DOT)`

**对应 Lodop**：`ADD_PRINT_LINE`

---

#### HE_ADD_RECT

```ts
HE_ADD_RECT(top, left, width, height, lineStyle?: string, lineWidth?: number): void
```

| 参数 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `top` | number (0.1mm) | — | 同 |
| `left` | number (0.1mm) | — | 同 |
| `width` | number (0.1mm) | — | 同 |
| `height` | number (0.1mm) | — | 同 |
| `lineStyle` | string | `"solid"` | 同 HE_ADD_LINE |
| `lineWidth` | number | 1 | 同 HE_ADD_LINE |

**示例**：
```js
HE_ADD_RECT(500, 200, 1000, 600, "solid", 2);
```

**实现说明**：
- GDI `Rectangle` 绘制
- 额外支持：`HE_SET_STYLE("BorderColor", "#333")`

**对应 Lodop**：`ADD_PRINT_RECT`

---

### 🎨 样式（1 个）

#### HE_SET_STYLE

```ts
HE_SET_STYLE(name: StyleName, value: StyleValue): void
```

**StyleName 枚举**（12 个精选）：

| name | 值类型 | 说明 | 默认值 |
|---|---|---|---|
| `FontName` | string | 字体名 | `"Microsoft YaHei"` |
| `FontSize` | number | 字号（pt） | 12 |
| `FontColor` | string | 颜色，`#RRGGBB` 或命名色 | `"#000000"` |
| `Bold` | boolean/number | 加粗 | false |
| `Italic` | boolean/number | 斜体 | false |
| `Underline` | boolean/number | 下划线 | false |
| `Alignment` | 1\|2\|3 | 1=左 2=居中 3=右 | 1 |
| `Angle` | 0\|90\|180\|270 | 旋转角度 | 0 |
| `ItemType` | 0\|1\|2\|3\|4 | 0=普通 1=页眉页脚 2=页码 3=总页数 4=序号 | 0 |
| `AsImage` | boolean/number | 按图片输出 | false |
| `KeepColor` | boolean/number | 保留颜色 | true |
| `BackColor` | string | 背景色 | `"#FFFFFF"` |

**生效范围**：仅对**前一个添加命令**最近生成的 PrintItem 生效

**示例**：
```js
HE_ADD_TEXT(200, 100, 3000, 400, "Hello World");
HE_SET_STYLE("FontSize", 24);
HE_SET_STYLE("Bold", 1);
HE_SET_STYLE("Alignment", 2);          // 居中
```

**对应 Lodop**：`SET_PRINT_STYLEA(0, name, value)`

---

### ⚙️ 全局参数（4 个）

#### HE_SET_PAGE

```ts
HE_SET_PAGE(orient: number, width: number, height: number, name?: string): void
```

| 参数 | 类型 | 说明 |
|---|---|---|
| `orient` | number | 1=纵向，2=横向，3=卷筒（宽度固定，高度自动） |
| `width` | number (mm) | 纸张宽度（卷筒模式下只认宽度） |
| `height` | number (mm) | 纸张高度（卷筒填 0） |
| `name` | string | 可选：系统纸张名，如 `"A4"`, `"Letter"`, `"4x6"`。指定后 width/height 可忽略 |

**示例**：
```js
HE_SET_PAGE(3, 80, 0, "");              // 80mm 卷筒纸
HE_SET_PAGE(1, 210, 297, "A4");          // A4 纵向
```

**实现说明**：
- orient=3 卷筒模式特殊处理：`width` 固定，高度由内容决定
- name 参数查询系统纸张列表，匹配则覆盖自定义 width/height
- 单位：**mm**

**对应 Lodop**：`SET_PRINT_PAGESIZE`

---

#### HE_SET_PRINTER

```ts
HE_SET_PRINTER(printer: string | number): void
```

| 参数 | 类型 | 说明 |
|---|---|---|
| `printer` | string | 打印机名（精确匹配） |
| `printer` | number | 打印机索引（从 0 开始，-1 表示默认） |

**示例**：
```js
HE_SET_PRINTER("XP-80C");                // 按名称
HE_SET_PRINTER(-1);                      // 系统默认打印机
```

**实现说明**：
- 如果是数字（索引），调用 `EnumPrinters` 获取并转换为名称
- 如果是字符串，按名称匹配
- 名称不匹配时返回 `ErrorCode.PrinterNotFound`
- 如果不调用此命令，默认使用系统默认打印机

**对应 Lodop**：`SET_PRINTER_INDEX`

---

#### HE_SET_COPIES

```ts
HE_SET_COPIES(count: number): void
```

| 参数 | 类型 | 说明 |
|---|---|---|
| `count` | number | 打印份数，≥ 1 |

**示例**：
```js
HE_SET_COPIES(3);
```

**实现说明**：
- 调用 `DocumentProperties` 的 DM_COPIES 字段
- 部分打印机驱动限制最大份数（通常 999）

**对应 Lodop**：`SET_PRINT_COPIES`

---

#### HE_SET_OPTION

```ts
HE_SET_OPTION(key: OptionKey, value: OptionValue): void
```

**OptionKey 枚举**（6 个精选）：

| key | 值类型 | 说明 | 默认值 |
|---|---|---|---|
| `"silent"` | boolean | 静默打印（不弹出打印框） | true（PRINT_SILENT 场景下） |
| `"duplex"` | number | 1=单面 2=长边翻转 3=短边翻转 | 1 |
| `"color"` | boolean | 彩色打印 | true |
| `"dpi"` | number | 打印 DPI | 0（使用打印机默认） |
| `"pagePercent"` | number | 页面缩放百分比 | 100 |
| `"clip"` | boolean | 内容超出纸张时是否裁剪 | true |

**示例**：
```js
HE_SET_OPTION("duplex", 2);             // 长边翻转双面
HE_SET_OPTION("dpi", 600);
```

**实现说明**：
- `silent`: 决定 PRINT 命令是否弹打印对话框
- `duplex`: 写 DEVMODE 的 dmDuplex 字段
- `pagePercent`: 整体缩放任务中各元素

**对应 Lodop**：`SET_PRINT_MODE` 的子集

---

### ▶️ 执行（4 个）

#### HE_PRINT

```ts
HE_PRINT(): Promise<TaskResult>
```

**返回值**：
```ts
type TaskResult = {
  taskId: string;         // 唯一任务 ID
  success: boolean;       // 是否成功
  error?: string;         // 错误信息（如果有）
  pages?: number;         // 打印页数
};
```

**示例**：
```js
const result = await HE_PRINT();
if (!result.success) {
  alert("打印失败：" + result.error);
}
```

**实现说明**：
- 提交当前任务到打印机
- 弹出系统打印设置对话框（除非设 silent）
- 后端使用 `StartDoc/StartPage/EndPage/EndDoc` 打印循环
- 通过 WS 返回结果

**对应 Lodop**：`PRINT`

---

#### HE_PRINT_SILENT

```ts
HE_PRINT_SILENT(): Promise<TaskResult>
```

**示例**：
```js
const result = await HE_PRINT_SILENT();
```

**实现说明**：
- 同 HE_PRINT，但不弹出任何对话框
- 使用 `PRINT_DEFAULTSOURCE` 指定的默认纸盒
- 后端创建 `TASK_DIALOG` 为非模态进度提示

**对应 Lodop**：`PRINTA`

---

#### HE_PREVIEW

```ts
HE_PREVIEW(): Promise<void>
```

**实现说明**（P2 阶段实现）：
- 打开一个新窗口/界面显示打印预览
- 使用 WebView2 展示模拟渲染效果
- 含：缩放 / 翻页 / 打印 / 取消按钮
- 预览窗口由本地 exe 创建，不是浏览器弹窗

**对应 Lodop**：`PREVIEW`

---

#### HE_NEW_PAGE

```ts
HE_NEW_PAGE(): void
```

**示例**：
```js
HE_ADD_TEXT(..., "第一页内容");
HE_NEW_PAGE();
HE_ADD_TEXT(..., "第二页内容");
```

**实现说明**：
- 标记当前任务中后续元素从新页面开始
- 后端打印时在分页处递增页码

**对应 Lodop**：`NEWPAGE`

---

### 🔍 查询（4 个）

#### HE_GET_PRINTERS

```ts
HE_GET_PRINTERS(): Promise<string[]>
```

**示例**：
```js
const printers = await HE_GET_PRINTERS();
// ["Microsoft Print to PDF", "XP-80C", "HP LaserJet MFP M140w"]
```

**实现说明**：
- 调用 Win32 `EnumPrinters(PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS)`
- 返回打印机名称数组

**对应 Lodop**：`GET_PRINTER_NAMES`

---

#### HE_GET_DEFAULT_PRINTER

```ts
HE_GET_DEFAULT_PRINTER(): Promise<string>
```

**示例**：
```js
const defaultP = await HE_GET_DEFAULT_PRINTER();
// "XP-80C"
```

**实现说明**：
- Win32 `GetDefaultPrinterW` API
- 如果没有默认打印机，返回空字符串

**对应 Lodop**：`GET_DEFAULTPRINTER`

---

#### HE_HAS_PRINTER

```ts
HE_HAS_PRINTER(name: string): Promise<boolean>
```

**示例**：
```js
if (await HE_HAS_PRINTER("XP-80C")) {
  HE_SET_PRINTER("XP-80C");
}
```

**实现说明**：
- 调用 `OpenPrinterW(name)` → 成功返回 true，失败返回 false
- 不区分大小写

**对应 Lodop**：`IS_PRINTER_EXIST`

---

#### HE_GET_INFO

```ts
HE_GET_INFO(key: InfoKey): Promise<any>
```

**InfoKey 枚举**：

| key | 返回值类型 | 说明 |
|---|---|---|
| `"version"` | string | 服务版本号 |
| `"clientIp"` | string | 连接客户端的 IP（通常 127.0.0.1） |
| `"serverIp"` | string | 服务器本机 IP |
| `"taskId"` | string | 当前任务 ID（如果有） |
| `"printerCount"` | number | 系统打印机数量 |
| `"status"` | string | 服务状态 `"running"｜"idle"` |

**示例**：
```js
const version = await HE_GET_INFO("version");
const count = await HE_GET_INFO("printerCount");
```

**对应 Lodop**：`GET_VALUE` 子集

---

### 🔗 回调（1 个）

#### HE_ON_RESULT

```ts
HE_ON_RESULT(callback: (result: TaskResult) => void): void
```

| 参数 | 类型 | 说明 |
|---|---|---|
| `callback` | function | 每次打印完成时回调 |

**示例**：
```js
HE_ON_RESULT(({ taskId, success, error, pages }) => {
  console.log(`任务 ${taskId} ${success ? '成功' : '失败: ' + error}`);
});
```

**实现说明**：
- 服务端每完成一个打印任务，通过 WS push 消息给客户端
- 前端 SDK 维护一个回调列表
- 适用于监听异步打印状态（如后台批次打印）

**对应 Lodop**：`On_Return`

---

### 📡 原生指令（1 个）

#### HE_SEND_RAW

```ts
HE_SEND_RAW(printerName: string, data: string, encoding?: string): Promise<TaskResult>
```

| 参数 | 类型 | 说明 |
|---|---|---|
| `printerName` | string | 目标打印机名称 |
| `data` | string | 原始数据（base64 编码） |
| `encoding` | string | `"base64"`（默认）或 `"hex"` |

**示例**：
```js
// ESC/POS 打开钱箱指令 (ESC p 0 50 250)
const raw = "1B700132FA";                     // hex
HE_SEND_RAW("XP-80C", raw, "hex");
```

**实现说明**：
- 最核心功能：直接将原始字节流发送到打印机
- 不经过 GDI，不排版，不做任何渲染
- 适合：标签订单、钱箱控制、小票机 ESC/POS 指令
- 底层处理：
  1. hex/base64 → 字节数组
  2. `OpenPrinterW` + `WritePrinter`（Win32 RAW 模式）
  3. 自动等待打印完成或超时（默认 30s）

**对应 Lodop**：`SEND_PRINT_RAWDATA`

---

## 3.3 样式系统（StyleSytem）内部设计

样式作用于 PrintItem 的方式：

```rust
pub struct PrintStyle {
    pub font_name: Option<String>,
    pub font_size: Option<f64>,          // pt
    pub font_color: Option<[u8; 3]>,     // RGB
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub alignment: Option<Alignment>,     // Left/Center/Right
    pub angle: Option<i32>,               // 0/90/180/270
    pub item_type: Option<ItemType>,      // Normal/HeaderFooter/PageNum/Total/Seq
    pub as_image: Option<bool>,
    pub keep_color: Option<bool>,
    pub back_color: Option<[u8; 3]>,
    pub line_style: Option<LineStyle>,    // Solid/Dashed/Dotted
    pub line_width: Option<f64>,
}
```

每个 PrintItem 持有一个 `PrintStyle`。当调用 `HE_SET_STYLE` 时，该样式合并到**最近一个 PrintItem** 中。后续添加的新 PrintItem 继承当前累计样式？或者重置为默认？

**决定**：`HE_SET_STYLE` 只影响前一个项，新项按默认值重新开始。这与 Lodop 行为一致。

## 3.4 错误码

```rust
pub enum ErrorCode {
    // 通用
    Success = 0,
    Unknown = -1,
    Timeout = -2,

    // 通信
    ConnectionTimeout = 1001,
    ConnectionRefused = 1002,
    InvalidJsonRpc = 1003,
    MethodNotFound = 1004,

    // 打印任务
    TaskNotFound = 2001,
    TaskEmpty = 2002,
    InvalidParam = 2003,
    PrinterNotFound = 2004,
    PrinterOffline = 2005,
    PaperNotLoaded = 2006,
    PrintFailed = 2007,
    DuplexNotSupported = 2008,

    // WebView2
    WebView2NotInstalled = 3001,
    HtmlRenderTimeout = 3002,

    // 文件/数据
    ImageDecodeFailed = 4001,
    FileNotFound = 4002,
    InvalidBarcodeType = 4003,
    PdfDecodeFailed = 4004,
    DataTooLarge = 4005,
}
```

前端接收错误格式：

```json
{
  "code": 2005,
  "message": "打印机脱机或未就绪",
  "data": { "printerName": "XP-80C" }
}
```

---

# 4. 工程目录结构

## 4.1 完整目录树

```
D:\打印插件v1\
│
├── README.md                              # 项目入口（已创建）
├── 设计文档.md                            # 本文件
├── Cargo.toml                             # Workspace 根
├── Cargo.lock
├── .gitignore
├── rust-toolchain.toml                    # 指定 Rust 版本 & targets
│
├── crates\                                # Rust 源码
│   ├── heprint-core\                      # ── 数据模型 + 任务管理
│   │   ├── Cargo.toml
│   │   └── src\
│   │       ├── lib.rs                     # 模块入口
│   │       ├── error.rs                   # ErrorCode 枚举
│   │       ├── task.rs                    # PrintTask, TaskManager
│   │       ├── item.rs                    # PrintItem trait + 8 种实现
│   │       ├── style.rs                   # PrintStyle, Alignment, ItemType
│   │       ├── printer.rs                 # PrinterInfo, PrinterRegistry
│   │       ├── option.rs                  # OptionKey, OptionValue 枚举
│   │       ├── command.rs                 # HE_xxx 命令枚举 + 参数解析
│   │       └── types.rs                   # 通用类型别名
│   │
│   ├── heprint-render\                    # ── 渲染引擎
│   │   ├── Cargo.toml
│   │   └── src\
│   │       ├── lib.rs
│   │       ├── webview2.rs               # WebView2 初始化 + HTML → PDF
│   │       ├── barcode.rs                # QR + 一维条码 → 位图
│   │       ├── image.rs                  # 图片解码 + 缩放
│   │       └── pdf.rs                    # PDF 解码 + 逐页提取
│   │
│   ├── heprint-print\                     # ── 打印后端
│   │   ├── Cargo.toml
│   │   └── src\
│   │       ├── lib.rs
│   │       ├── winspool.rs               # Win32 打印：StartDoc/EndDoc
│   │       ├── gdi.rs                    # GDI 绘制：文本/图形/位图
│   │       ├── escpos.rs                 # ESC/POS 直发（USB/网络）
│   │       └── dm.rs                     # DEVMODE 构造/解析
│   │
│   ├── heprint-server\                    # ── HTTP/WS 服务器
│   │   ├── Cargo.toml
│   │   └── src\
│   │       ├── lib.rs
│   │       ├── server.rs                 # Axum 启动 + 路由注册
│   │       ├── ws.rs                     # WebSocket 处理器
│   │       ├── router.rs                 # JSON-RPC 方法路由
│   │       ├── cors.rs                   # CORS 配置
│   │       └── cert.rs                   # TLS 证书生成/加载
│   │
│   └── heprint-cli\                       # ── 主程序入口（.exe）
│       ├── Cargo.toml
│       └── src\
│           ├── main.rs                   # 入口：初始化 + 启动服务
│           ├── config.rs                 # 配置文件读取
│           ├── tray.rs                   # 系统托盘
│           └── upgrade.rs               # 自动更新（future）
│
├── web-sdk\                               # TypeScript 前端 SDK
│   ├── package.json                      # @heprint/web-sdk
│   ├── tsconfig.json
│   ├── vite.config.ts                    # Vite 打包（esm + umd）
│   ├── src\
│   │   ├── index.ts                      # 导出 HE 主对象
│   │   ├── transport.ts                  # WS + HTTP 通信层
│   │   ├── types.ts                      # TS 类型定义
│   │   ├── error.ts                      # 错误码枚举
│   │   └── builders\                     # 链式 API 构建器
│   │       ├── index.ts
│   │       ├── text.ts
│   │       ├── html.ts
│   │       ├── table.ts
│   │       ├── image.ts
│   │       ├── barcode.ts
│   │       └── style.ts
│   ├── dist\                             # 构建产物
│   │   ├── heprint.es.js                 # ESM
│   │   └── heprint.umd.js                # UMD（script 标签）
│   └── examples\                         # HTML 示例
│       ├── 01-basic.html                 # 基础调用
│       ├── 02-receipt.html               # 小票打印
│       ├── 03-a4-report.html             # A4 报表
│       ├── 04-label.html                 # 标签打印
│       └── 05-raw.html                   # ESC/POS 原生指令
│
├── installer\                             # 安装程序
│   ├── inno-setup.iss                    # Inno Setup 脚本
│   ├── resources\
│   │   ├── icon.ico                      # 托盘图标
│   │   └── banner.bmp                   # 安装界面横幅
│   ├── webview2\                         # WebView2 引导器
│   │   └── MicrosoftEdgeWebview2Setup.exe
│   └── output\                           # 产出 .exe 安装包
│
├── docs\                                  # 文档
│   ├── 命令对照表.md                       # HE_xxx vs LODOP.xxx
│   ├── 开发指南.md
│   └── 部署说明.md
│
├── tests\                                 # 集成测试
│   ├── e2e\
│   │   ├── basic.rs                      # 启动服务 → 发送命令 → 验证
│   │   └── fixtures\                     # 测试用HTML/图片
│   └── e2e.js\                           # Node.js 端到端测试
│
└── .github\                               # CI/CD（future）
    └── workflows\
        └── build.yml
```

## 4.2 每个 crate 的职责边界

### heprint-core

**职责**：所有业务数据模型的定义，不涉及任何 I/O 或系统调用。

```
包体：~200 KB（纯定义，几乎全是 enum/struct + serde 派生）
```

公开暴露：
- `task::PrintTask`、`task::TaskManager`
- `item::PrintItem` （trait）+ `item::TextItem`/`HtmlItem`/`ImageItem`/`BarcodeItem`/`PdfItem`/`LineItem`/`RectItem`
- `style::PrintStyle` + `style::Alignment`/`ItemType`/`LineStyle`
- `printer::PrinterInfo`
- `error::ErrorCode`
- `command::HeCommand`（枚举，对应全部 HE_xxx）
- `option::OptionKey`/`OptionValue`

### heprint-render

**职责**：把各种"内容数据"转化为"可打印的位图/PDF"。

```
包体：+ ~800 KB（主要是 WebView2 COM 绑定）
```

- WebView2 隐藏窗口：负责 HTML → PDF
- rqrcode / barcoders：条码 → 位图
- image crate：图片解码 → RGBA 位图

依赖 `heprint-core` 的数据模型。

### heprint-print

**职责**：所有与 Windows 打印系统交互的底层代码。

```
包体：+ ~500 KB（windows crate 绑定）
```

- `winspool.rs`：`OpenPrinterW`、`StartDocPrinterW`、`WritePrinter`、`EndDocPrinter`
- `gdi.rs`：`CreateDCW`、`StartDocW`、`StartPage`、`TextOutW`、`StretchDIBits`、`EndPage`、`EndDoc`
- `escpos.rs`：通过 RAW 类型打印机打开，WritePrinter 发送字节流
- `dm.rs`：DEVMODE 构造（纸张/双面/份数/质量）

依赖 `heprint-core` 的数据模型，依赖 `heprint-render` 的位图输出。

### heprint-server

**职责**：网络通信层，把核心能力和打印后端暴露给前端。

```
包体：+ ~1.5 MB（axum + tokio + rustls + tower）
```

- Axum HTTP Router：`/version`, `/health`, `/printers`
- WebSocket `/ws`：接收 JSON-RPC 消息 → 路由到对应方法
- TLS/HTTPS 支持
- CORS 中间件

### heprint-cli

**职责**：最终的可执行程序。

```
包体：3 MB（含全部 crate 展开）
```

- 解析命令行参数
- 加载配置文件
- 初始化系统托盘
- 启动 server
- 处理信号（ctrl+c / 关闭窗口时优雅退出）

## 4.3 依赖关系图

```
heprint-cli
    │
    └── heprint-server
            │
            ├── heprint-core
            │
            └── (他)
                    │
                    ├── heprint-render
                    │       └── heprint-core
                    │
                    └── heprint-print
                            ├── heprint-core
                            └── heprint-render
```

编译时：所有 crate 独立编译，最终 heprint-cli 链接全部。

运行时：`webview2-com` 在第一次调用 WebView2 时才会加载 DLL，不是启动时。

---

# 5. 核心数据模型

## 5.1 PrintTask

```rust
pub struct PrintTask {
    pub task_id: String,                    // UUID v4
    pub name: String,                       // 用户给的任务名
    pub items: Vec<Box<dyn PrintItem>>,     // 打印内容项
    pub page: PageConfig,                   // 纸张设置
    pub printer_name: Option<String>,       // 目标打印机
    pub copies: u32,                        // 份数
    pub options: HashMap<OptionKey, OptionValue>,  // 全局选项
    pub created_at: Instant,
    pub status: TaskStatus,                 // Building, Ready, Printing, Done, Error
    pub result: Option<TaskResult>,
}
```

```rust
pub struct PageConfig {
    pub orient: Orient,                     // Portrait(1), Landscape(2), Roll(3)
    pub width_mm: f64,                      // 宽度 mm
    pub height_mm: f64,                     // 高度 mm (0=自动)
    pub name: Option<String>,               // 系统纸张名
}
```

TaskManager 管理当前任务队列：

```rust
pub struct TaskManager {
    current_id: String,                     // 当前任务的 ID
    tasks: HashMap<String, PrintTask>,
}
```

- `HE_INIT` → 创建新 Task，设置为 current
- `HE_ADD_xxx` → 添加 PrintItem 到 current task
- `HE_NEW_PAGE` → 在 current task 中标记分页
- `HE_PRINT` / `HE_PRINT_SILENT` → 锁定 current task，移交给打印后端

## 5.2 PrintItem（trait + 8 种实现）

```rust
pub trait PrintItem: Send + Sync {
    fn item_type(&self) -> ItemKind;        // Text/Html/Image/Barcode/...
    fn bounds(&self) -> &Rect;              // top, left, width, height
    fn style(&self) -> &PrintStyle;
    fn style_mut(&mut self) -> &mut PrintStyle;
    fn render(&self, ctx: &RenderContext) -> Result<RenderOutput>;
}
```

8 种具体类型：

| 类型 | 数据 | 渲染方式 |
|---|---|---|
| `TextItem` | `text: String` | GDI `DrawTextW` |
| `HtmlItem` | `html: String` | WebView2 → PDF → GDI |
| `TableItem` | `html: String` | 同 HtmlItem（自动分页） |
| `ImageItem` | `data: Vec<u8>`, `format: ImageFormat` | image crate 解码 → GDI `StretchDIBits` |
| `BarcodeItem` | `btype: BarcodeType`, `value: String` | rqrcode/barcoders → 位图 → GDI |
| `PdfItem` | `data: Vec<u8>` | pdfium → 逐页位图 → GDI |
| `LineItem` | `x1, y1, x2, y2` | GDI `MoveToEx` + `LineTo` |
| `RectItem` | `width, height, round_corner?` | GDI `Rectangle` |

## 5.3 PrintStyle

已在 3.3 节详细说明。

关键设计点：
- 默认值全为 `Option`，取 `None` 表示"按出厂商约定"
- 具体打印时，Style + 当前设备能力 = 最终渲染参数

## 5.4 PrinterDevice（打印机抽象）

```rust
pub enum PrinterDevice {
    GdiPrinter {
        name: String,
        driver: String,
        port: String,
        is_default: bool,
        capabilities: PrinterCapabilities,
    },
    RawPrinter {
        name: String,
        device_type: RawDeviceType,   // USB / Serial / TcpIp
        address: String,              // "USB001" / "COM3" / "192.168.1.100:9100"
    },
}
```

```rust
pub struct PrinterCapabilities {
    pub duplex_supported: bool,
    pub color_supported: bool,
    pub max_copies: u32,
    pub supported_papers: Vec<String>,   // 纸张名列表
    pub dpi: (u32, u32),
}
```

PrinterRegistry 管理打印机列表（通过 `EnumPrinters` 获取并缓存）。

---

# 6. 关键链路详解

## 6.1 HTML → WebView2 → PDF → GDI 打印

这是最核心的链路：处理 `HE_ADD_HTML` 和 `HE_ADD_TABLE`。

### 流程图

```
HE_ADD_HTML(top, left, w, h, "<div>...</div>")
            │
            ▼
  [heprint-server 收到命令]
            │
            ▼
  [heprint-core 创建 HtmlItem]
       bounds: top/left/w/h
       html: "<!DOCTYPE html><html>..."
       // 自动包裹 HTML、添加 meta viewport 等
            │
            ▼
  [heprint-render] — 渲染阶段（HE_PRINT 触发）
            │
            ├─ 1. 创建隐藏窗口（WebView2）
            │     大小 = 宽(w) × 高(h) （单位：0.1mm → 物理像素）
            │
            ├─ 2. NavigateToString(html)
            │     ↓
            │     等待 NavigationCompleted 事件
            │     超时时间：15 秒
            │
            ├─ 3. 注入 JS：设置打印参数
            │     window.print() 或 CDP
            │
            ├─ 4. CallPrintToPdfAsync()
            │     输出：PDF 字节流
            │
            ├─ 5. 裁剪：按 bounds 取 PDF 子区域
            │
            └─ 6. 销毁 WebView2 实例
            │
            ▼
  [heprint-print] — GDI 绘制
            │
            ├─ 1. PDF → 逐页位图（RenderOutput::Bitmap）
            ├─ 2. StartPage
            ├─ 3. StretchDIBits(hdc, x, y, w, h, bitmap)
            └─ 4. EndPage
```

### WebView2 生命周期管理细节

```rust
// 关键：WebView2 环境是进程级单例（CoreWebView2Environment）
// 但每个渲染请求创建独立的 CoreWebView2Controller（隐藏窗口）

pub struct WebView2Render {
    env: Arc<CoreWebView2Environment>,
}

impl WebView2Render {
    pub async fn render_html(&self, html: &str, bounds: Rect) -> Result<Vec<u8>> {
        // 1. 设置 WebView2 选项
        let opts = CoreWebView2ControllerOptions::new()
            .is_visible(false);                    // 隐藏窗口
        
        // 2. 创建控制器（隐藏窗口句柄）
        let controller = CoreWebView2Controller::create_with_options(&self.env, &opts).await?;
        
        // 3. 获取 CoreWebView2
        let webview = controller.core_web_view2().await?;
        
        // 4. 加载 HTML
        let nav = webview.navigate_to_string(html).await?;
        nav.completed().await?;                    // 等待完成
        
        // 5. 打印到 PDF
        let pdf_bytes = webview.print_to_pdf(PrintToPdfOptions::new()
            .margin_top(0)
            .margin_bottom(0)
            .margin_left(0)
            .margin_right(0)
            .page_width(bounds.width_mm * 100)     // → 0.1mm → WebView2 单位
        ).await?;
        
        // 6. 销毁
        controller.close()?;
        
        Ok(pdf_bytes)
    }
}
```

**关键优化**：
- WebView2 环境（`CoreWebView2Environment`）是进程级单例，只创建一次
- 每个 HTML 项创建一个临时控制器（`CoreWebView2Controller`），渲染后立即销毁
- 控制器使用隐藏窗口（`is_visible = false`），用户无感知
- 如果用户系统没有 WebView2，在 HE_PRINT 时返回错误码 3001

## 6.2 ESC/POS → 打印机

ESC/POS 是小票打印机最常用的指令集。HE_SEND_RAW 直接绕过所有渲染层。

```
HE_SEND_RAW("XP-80C", "1B703132FA", "hex")
            │
            ▼
  [heprint-server]
            │
            ▼
  [heprint-core]
       base64/hex → 字节数组
       "1B703132FA" → [0x1B, 0x70, 0x31, 0x32, 0xFA]
            │
            ▼
  [heprint-print::escpos]
            │
            ├─ OpenPrinterW("XP-80C", &handle, NULL, PRINTER_ACCESS_USE)
            │
            ├─ DOC_INFO_1 { pDocName: "HePrint RAW", pDatatype: "RAW", pOutputFile: NULL }
            │
            ├─ StartDocPrinterW(handle, 1, &doc_info)
            │
            ├─ WritePrinter(handle, data, data.len(), &written)
            │     ↑ 关键：RAW 模式直接发送字节流
            │
            ├─ EndDocPrinter(handle)
            │
            └─ ClosePrinter(handle)
```

**支持的设备类型**：
1. USB 打印机（USBPRINT: 端口名，如 "USB001"）
2. 网络打印机（端口为 IP:Port，如 "192.168.1.100:9100"）
3. 串口打印机（COM 端口名，如 "COM3"）
4. 虚拟打印机（Windows 共享的 RAW 打印机）

## 6.3 二维码/条码 → 位图 → GDI

```
HE_ADD_BARCODE(800, 800, 500, 500, "QRCode", "https://example.com")
            │
            ▼
  [heprint-core 创建 BarcodeItem]
            │
            ▼
  [heprint-render::barcode]
            │
            ├─ QRCode:
            │     qrcode::QrCode::new(value) → Vec<bool>
            │      → 绘制为位图（黑/白像素）
            │      → 按 bounds 缩放（保持方形）
            │
            ├─ Code128/Code39/EAN13:
            │     barcoders::sym::code128::Code128::encode(value)
            │      → Vec<ModuleWidth>
            │      → 绘制为位图（黑白条纹）
            │      → 按 bounds 缩放
            │
            └─ → RenderOutput::Bitmap { pixels, width, height, format: Mono }
            │
            ▼
  [heprint-print::gdi]
       StretchDIBits(hdc, x, y, w, h, bitmap)
```

---

# 7. WebView2 集成方案

## 7.1 检测 / 安装 / 降级

```rust
pub enum WebView2Status {
    Installed(String),           // 已安装，包含版本号
    NotInstalled,               // 未安装
    BlockedByPolicy,            // 被组策略禁用
}
```

检测逻辑（启动时异步执行）：

```rust
pub fn detect_webview2() -> WebView2Status {
    // 1. 尝试 COM 接口：CoCreateInstance(CLSID_WebView2)
    //    成功 → Installed(version)
    
    // 2. 查询注册表：
    //    HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\ClientState
    //    \{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}
    //    检查 pv 键
    
    // 3. 查文件系统：
    //    %LOCALAPPDATA%\Microsoft\EdgeWebView\Application\
    
    // 4. 都没有 → NotInstalled
}
```

安装引导：

```
场景 A：WebView2 已存在 → 正常启动
场景 B：WebView2 不存在，有网络 → 弹出引导页，启动 bootstrapper
场景 C：WebView2 不存在，无网络 → 提示用户联网或下载离线包
```

**不强制要求 WebView2**：如果用户只用 `HE_ADD_TEXT/IMAGE/BARCODE/SEND_RAW`，完全不碰 HTML/PREVIEW，则不需要 WebView2。

## 7.2 隐藏窗口生命周期

```rust
struct WebView2Pool {
    pool: Vec<WebView2Instance>,
    max_pool_size: usize,   // 默认 3
}

struct WebView2Instance {
    controller: CoreWebView2Controller,
    webview: CoreWebView2,
    last_used: Instant,
    busy: bool,
}
```

- 启动时不创建（避免非必要用户授权弹窗）
- 第一次 HTML 渲染时创建 Environment 和 1 个 Controller
- 多个 HTML 项时并行渲染（最多 max_pool_size 个并发实例）
- 空闲超过 30 秒自动销毁（释放内存）
- 进程退出时确保全部销毁

## 7.3 PrintToPdfAsync 调用细节

关键问题：WebView2 的 `PrintToPdfAsync` 需要**页面大小参数一致**才能保证内容按预期布局。

```rust
let options = PrintToPdfOptions {
    scale_factor: 100,              // 100%
    page_width: (bounds.width_mm / 25.4 * dpi) as u32,    // mm → px (按 96 DPI)
    page_height: (bounds.height_mm / 25.4 * dpi) as u32,
    margin_top: 0,
    margin_bottom: 0,
    margin_left: 0,
    margin_right: 0,
    should_print_backgrounds: true,
    should_print_selection_only: false,
    should_print_header_footer: false,
};
```

**为什么不是直接截图**？截图不能保证矢量字体的质量。PrintToPdf 保留矢量信息，后续 GDI 绘制质量更高。

## 7.4 跨进程通信

WebView2 和 HePrint 服务在**同一个进程**中（都是 `heprint.exe`），不存在跨进程问题。

调用链：

```
heprint.exe (main thread)
    └── Axum 服务 (tokio runtime)
         └── WebView2 Environment (进程级单例)
              └── WebView2 Controller (隐藏窗口)
                   └── CoreWebView2 (UI 线程)
```

注意：**WebView2 需要 STA 线程**。如果 tokio 运行在多线程 MTA 中，需要：

```rust
// Windows COM 初始化（在创建 WebView2 前）
CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED);
```

解决方案：创建一个单独的 STA 线程专门负责 WebView2 操作，通过 channel 通信。

```rust
// heprint-render 内部
let (tx, rx) = mpsc::channel::<RenderJob>();

std::thread::spawn(move || {
    CoInitializeEx(APARTMENTTHREADED);
    // 此线程 STA
    while let Some(job) = rx.blocking_recv() {
        // 渲染 HTML → PDF
        let result = render_on_sta_thread(&job);
        job.callback.send(result);
    }
});
```

---

# 8. HTTPS 与跨域

## 8.1 自签 CA 证书生成（rcgen）

```rust
use rcgen::{CertificateParams, KeyPair, KeyUsagePurpose, IsCa, BasicConstraints};
use time::{OffsetDateTime, Duration};

pub fn generate_certs() -> Result<(String, String)> {
    // 1. 生成 CA 私钥 + 证书
    let ca_params = CertificateParams::new(vec!["HePrint CA".to_string()]);
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    ca_params.not_after = OffsetDateTime::now_utc() + Duration::days(3650); // 10 年
    
    let ca_key = KeyPair::generate()?;
    let ca_cert = ca_params.self_signed(&ca_key)?;
    
    // 2. 生成服务端私钥 + 证书（由 CA 签发）
    let server_params = CertificateParams::new(vec!["127.0.0.1".to_string(), "localhost".to_string()]);
    server_params.subject_alt_names = vec![
        "127.0.0.1".to_string(),
        "localhost".to_string(),
    ];
    server_params.not_after = OffsetDateTime::now_utc() + Duration::days(365);
    
    let server_key = KeyPair::generate()?;
    let server_cert = server_params.signed_by(&server_key, &ca_cert, &ca_key)?;
    
    Ok((
        ca_cert.pem(),            // CA 证书
        server_cert.pem(),         // 服务端证书
        server_key.serialize_pem(), // 私钥
    ))
}
```

## 8.2 系统受信任根自动安装

```rust
pub fn install_ca_to_system(ca_pem: &str) -> Result<()> {
    // 需要管理员权限！如果无权限，提示用户手动安装
    
    // 1. 写入临时文件
    let tmp = std::env::temp_dir().join("heprint-ca.pem");
    std::fs::write(&tmp, ca_pem)?;
    
    // 2. 调用 certutil 安装到受信任根
    // certutil -addstore -f Root "C:\...\heprint-ca.pem"
    let output = Command::new("certutil")
        .args(["-addstore", "-f", "Root", tmp.to_str().unwrap()])
        .output();
    
    // 3. 清理临时文件
    let _ = std::fs::remove_file(tmp);
    
    match output {
        Ok(o) if o.status.success() => Ok(()),
        _ => Err(Error::NeedAdminPrivilege),
    }
}
```

**管理员权限策略**：只在安装时（Inno Setup 已提权）自动安装。首次运行时如果无管理员权限，在托盘提示"HTTPS 证书未安装，部分浏览器可能弹出安全警告"。

## 8.3 CORS 策略

```rust
let cors = tower_http::cors::CorsLayer::new()
    .allow_origin(Any)                          // 允许任何来源（因为本地服务）
    .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
    .allow_headers([header::CONTENT_TYPE, header::UPGRADE, header::SEC_WEBSOCKET_VERSION])
    .allow_private_network(true)                // 关键：允许内网请求（Chrome 2025 已要求）
    .max_age(Duration::from_secs(86400));
```

**重要**：Chrome 和 Edge 从 2024 年开始对混合同意访问（private network access）进行限制。HTTP 页面访问 127.0.0.1 需要 `Access-Control-Allow-Private-Network: true` 响应头。

---

# 9. 前端 SDK 设计

## 9.1 TypeScript 类型定义

```typescript
// ============ 核心类型 ============

type BarcodeType = 'QRCode' | 'Code128' | 'Code39' | 'EAN13' | 'EAN8' | 'UPC-A' | 'UPC-E' | 'PDF417' | 'DataMatrix';

type StyleName = 'FontName' | 'FontSize' | 'FontColor' | 'Bold' | 'Italic' | 'Underline' | 'Alignment' | 'Angle' | 'ItemType' | 'AsImage' | 'KeepColor' | 'BackColor';

type OptionKey = 'silent' | 'duplex' | 'color' | 'dpi' | 'pagePercent' | 'clip';

type InfoKey = 'version' | 'clientIp' | 'serverIp' | 'taskId' | 'printerCount' | 'status';

interface TaskResult {
  taskId: string;
  success: boolean;
  error?: string;
  pages?: number;
}

// ============ 命令函数签名 ============

// HE_INIT
function init(taskName: string): void;

// HE_ADD_xxx
function addText(top: number, left: number, width: number, height: number, text: string): void;
function addHtml(top: number, left: number, width: number, height: number, html: string): void;
function addTable(top: number, left: number, width: number, height: number, tableHtml: string): void;
function addImage(top: number, left: number, width: number, height: number, src: string): void;
function addBarcode(top: number, left: number, width: number, height: number, type: BarcodeType, value: string): void;
function addPdf(top: number, left: number, width: number, height: number, content: string): void;
function addLine(top1: number, left1: number, top2: number, left2: number, lineStyle?: string, lineWidth?: number): void;
function addRect(top: number, left: number, width: number, height: number, lineStyle?: string, lineWidth?: number): void;

// HE_SET_STYLE
function setStyle(name: StyleName, value: string | number | boolean): void;

// 全局
function setPage(orient: number, width: number, height: number, name?: string): void;
function setPrinter(printer: string | number): void;
function setCopies(count: number): void;
function setOption(key: OptionKey, value: string | number | boolean): void;

// 执行
function print(): Promise<TaskResult>;
function printSilent(): Promise<TaskResult>;
function preview(): Promise<void>;
function newPage(): void;

// 查询
function getPrinters(): Promise<string[]>;
function getDefaultPrinter(): Promise<string>;
function hasPrinter(name: string): Promise<boolean>;
function getInfo(key: InfoKey): Promise<any>;

// 回调
function onResult(callback: (result: TaskResult) => void): void;

// 原生
function sendRaw(printerName: string, data: string, encoding?: 'base64' | 'hex'): Promise<TaskResult>;
```

## 9.2 链式 API（可选 Builder 风格）

```typescript
const result = await HE.init('小票任务')
  .printer('XP-80C')
  .page(3, 80, 0)
  .text('订单 #001').at(20, 10, 280, 30).bold().size(14).center().end()
  .barcode('QRCode', 'https://...').at(60, 50, 200, 200).end()
  .table('#items-table').at(280, 10, 280, 300).end()
  .printSilent();
```

Builder 在 SDK 底层自动转换为 JSON-RPC 消息，对服务端来说是同样的 HE_xxx 命令。

## 9.3 通信传输层

```typescript
class HeTransport {
  private ws: WebSocket | null = null;
  private baseUrl: string;
  private requestId = 0;
  private pending: Map<number, { resolve, reject, timer }> = new Map();
  
  constructor(port: number = 18000, secure: boolean = false) {
    const proto = secure ? 'wss' : 'ws';
    this.baseUrl = `${proto}://127.0.0.1:${port}`;
  }

  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(`${this.baseUrl}/ws`);
      this.ws.onopen = () => resolve();
      this.ws.onmessage = (e) => this.handleMessage(e);
      this.ws.onclose = () => setTimeout(() => this.reconnect(), 1000);
      this.ws.onerror = (e) => reject(new Error('WebSocket error'));
    });
  }

  async call(method: string, params: object): Promise<any> {
    const id = ++this.requestId;
    this.send({ jsonrpc: '2.0', id, method, params });
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('timeout')), 30000);
      this.pending.set(id, { resolve, reject, timer });
    });
  }

  private handleMessage(e: MessageEvent) {
    const msg = JSON.parse(e.data);
    if (msg.id && this.pending.has(msg.id)) {
      const { resolve, reject, timer } = this.pending.get(msg.id)!;
      clearTimeout(timer);
      this.pending.delete(msg.id);
      if (msg.error) reject(new Error(msg.error.message));
      else resolve(msg.result);
    }
  }
}
```

## 9.4 错误处理与重连

```typescript
export class HeClient {
  private transport: HeTransport;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;     // 初始 1s，指数退避

  async ensureConnected(): Promise<void> {
    for (let i = 0; i < this.maxReconnectAttempts; i++) {
      try {
        await this.transport.connect();
        this.reconnectAttempts = 0;
        return;
      } catch (e) {
        this.reconnectAttempts++;
        const delay = this.reconnectDelay * Math.pow(2, i);
        await new Promise(r => setTimeout(r, delay));
      }
    }
    throw new Error('无法连接到 HePrint 服务（127.0.0.1:18000）');
  }
}
```

---

# 10. 安装包方案

## 10.1 Cargo 体积优化配置

```toml
[profile.release]
opt-level = "z"                  # 优化体积（-Oz）
lto = true                       # 链接时优化
codegen-units = 1                # 单编译单元
strip = "symbols"                # 剥离符号表
panic = "abort"                  # 不使用 panic unwinding
```

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
targets = ["x86_64-pc-windows-msvc"]
```

```toml
# Cargo.toml（workspace 根）
[workspace]
members = [
    "crates/heprint-core",
    "crates/heprint-render",
    "crates/heprint-print",
    "crates/heprint-server",
    "crates/heprint-cli",
]

[profile.release]
inherits = "release"
```

**额外体积技巧**：
- UPX 压缩 final binary：`upx --best --ultra-brute heprint.exe` → 再压缩 30-50%
- windows crate 只启用需要的 features（`Win32_Graphics_Printing` + `Win32_Graphics_Gdi`）
- image crate 只启用 `png` + `jpeg` features

## 10.2 Inno Setup 脚本大纲

```iss
#define MyAppName "HePrint 打印服务"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "YourName"
#define MyAppURL "https://heprint.example.com"

[Setup]
AppId={{HEPRINT-UUID-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
DefaultDirName={autopf}\HePrint
DefaultGroupName=HePrint
OutputDir=output
OutputBaseFilename=HePrint-v1.0.0-setup
Compression=lzma2/ultra
SolidCompression=yes
PrivilegesRequired=admin              ; 需要管理员权限（为了证书安装+防火墙）
ArchitecturesInstallIn64BitMode=x64compatible

[Files]
Source: "..\target\release\heprint.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "resources\icon.ico"; DestDir: "{app}"
Source: "webview2\MicrosoftEdgeWebview2Setup.exe"; DestDir: "{tmp}"; Flags: deleteafterinstall

[Run]
; 可选安装 WebView2
Filename: "{tmp}\MicrosoftEdgeWebview2Setup.exe"; Parameters: "/silent /install"; \
    StatusMsg: "正在安装 WebView2 运行时..."; \
    Check: not WebView2Installed

; 安装完成后启动服务（后台，不弹窗）
Filename: "{app}\heprint.exe"; Parameters: "/autorun"; Flags: runhidden

[UninstallRun]
Filename: "{app}\heprint.exe"; Parameters: "/uninstall"
```

## 10.3 WebView2 引导器集成

```bash
# 下载 WebView2 引导器
# 地址：https://go.microsoft.com/fwlink/p/?LinkId=2124703
# 约 1.6 MB

# 与安装包一同分发
installer\webview2\MicrosoftEdgeWebview2Setup.exe
```

检查脚本（Inno Setup Pascal 代码）：

```pascal
function WebView2Installed: Boolean;
var
  Version: string;
begin
  Result := RegQueryStringValue(
    HKLM, 'SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\ClientState\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
    'pv', Version);
end;
```

## 10.4 首次运行/升级/卸载流程

**首次运行**：
1. Inno Setup 安装（管理员权限）
2. 安装 WebView2（如果需要）
3. 生成 HTTPS 证书并安装到受信任根
4. 添加 Windows Defender 防火墙例外（inbound TCP 18000/18443）
5. 启动 heprint.exe（后台，托盘图标）
6. 浏览器自动打开 `http://127.0.0.1:18000` 引导页

**升级**：
1. Inno Setup 检测旧版本（AppId 匹配）
2. 停止正在运行的 heprint.exe
3. 替换 exe 文件
4. 保留配置和证书（在 %APPDATA%/HePrint/）
5. 重启 heprint.exe

**卸载**：
1. 停止 heprint.exe
2. 从受信任根移除自签 CA 证书
3. 删除防火墙规则
4. 提示是否保留配置和证书（以便重装后复用）

---

# 11. 分阶段实施路线（P0-P4）

## P0：骨架（第 1 周）

**目标**：项目结构跑通，打印服务能启动，前端能发一个命令并收到响应。

| 日 | 任务 | 产出 |
|---|---|---|
| D1 | 创建 Cargo workspace + 5 个 crate 骨架 | 可编译的空项目 |
| D2 | heprint-core：ErrorCode + 基础类型 | `types.rs` `error.rs` |
| D3 | heprint-server：Axum 启动 + `/version` + `/health` | 能 curl 通 |
| D4 | heprint-server：WebSocket `/ws` 框架 + JSON-RPC 路由骨架 | WS 握手成功 |
| D5 | web-sdk：package.json + transport.ts + HE_VERSION | 前端能调版本号 |
| D6 | 集成测试：启动 exe → 前端连接 → 获取版本号 | 绿线 E2E |
| D7 | 文档补全 + 周报 | |

## P1：核心打印（第 2-3 周）

**目标**：能打印文本和图片到任意 Windows 打印机。

| 周 | 任务 | 产出 |
|---|---|---|
| W2 | heprint-core：PrintTask + TaskManager + TextItem + ImageItem | 命令模型就绪 |
| W2 | heprint-print：winspool.rs（OpenPrinter/WritePrinter） + gdi.rs（CreateDC/StartDoc/TextOut） | 打印 hello world |
| W2 | heprint-core：PrinterInfo + PrinterRegistry | `HE_GET_PRINTERS` / `HE_GET_DEFAULT_PRINTER` |
| W2 | heprint-core：HE_INIT + HE_ADD_TEXT + HE_PRINT_SILENT 命令路由 | 文字打印可用 |
| W3 | heprint-render：image.rs（base64/URL/本地 → 位图） | 图片解码 |
| W3 | heprint-print：gdi.rs 添加 StretchDIBits（位图绘制） + StartPage/EndPage | 图片打印可用 |
| W3 | heprint-core：PageConfig + HE_SET_PAGE + HE_SET_PRINTER + HE_SET_COPIES | 纸张切换可用 |
| W3 | web-sdk：所有 P1 命令的 JS 绑定 + 01-basic.html 示例 | 示例跑通 |

## P2：HTML + 条码（第 4-5 周）

**目标**：HTML 表格渲染出纸，QR/条码可打印。

| 周 | 任务 | 产出 |
|---|---|---|
| W4 | heprint-render：webview2.rs（Environment 初始化 + 隐藏窗口） | WebView2 渲染就绪 |
| W4 | heprint-render：webview2.rs（NavigateToString + PrintToPdfAsync） | HTML → PDF |
| W4 | heprint-core：HtmlItem + TableItem + 自动分页逻辑 | HTML 命令就绪 |
| W4 | heprint-print：PDF 页面 → GDI 位图 → 打印 | HTML 打印可用 |
| W5 | heprint-render：barcode.rs（QR + Code128 + EAN13 + PDF417） | 条码生成 |
| W5 | heprint-core：BarcodeItem | |
| W5 | heprint-server：HE_PREVIEW（简易预览窗口） | 预览可用 |
| W5 | heprint-core：HE_SET_STYLE（12 个枚举） | 样式系统就绪 |
| W5 | web-sdk：02-receipt.html + 03-a4-report.html 示例 | 完整示例 |

## P3：完善（第 6-7 周）

**目标**：PDF 打印、LINE/RECT 图形、ESC/POS 原生、回调、Option 系统。

| 周 | 任务 | 产出 |
|---|---|---|
| W6 | heprint-render：pdf.rs（pdfium 集成） | PDF 渲染 |
| W6 | heprint-core：PdfItem + LineItem + RectItem | |
| W6 | heprint-print：escpos.rs（USB + 网络 RAW 模式） | HE_SEND_RAW |
| W7 | heprint-core：HE_SET_OPTION（6 个 key） | 选项系统 |
| W7 | heprint-core：HE_ON_RESULT（任务回调 + WS push） | 回调系统 |
| W7 | heprint-print：DEVMODE 双面/DM_COPIES/DM_COLOR | 双面打印 |
| W7 | web-sdk：04-label.html + 05-raw.html + builders 链式 API | SDK 完善 |

## P4：发布（第 8-10 周）

**目标**：HTTPS 证书、系统托盘、Inno Setup 安装包、全文档。

| 周 | 任务 | 产出 |
|---|---|---|
| W8 | heprint-server：cert.rs（rcgen 自签 CA + 系统安装） | HTTPS 可用 |
| W8 | heprint-cli：tray.rs（系统托盘 + 右键菜单） | 托盘图标 |
| W9 | installer：Inno Setup 脚本 + 证书安装集成 + 防火墙规则 | 安装包可用 |
| W9 | 全量集成测试 + 边界测试 | 测试覆盖 |
| W10 | docs/ 完整文档：命令对照表、开发指南、部署说明 | 文档就绪 |
| W10 | UPX 压缩 + 签名 + 发布 | v1.0.0 发布 |

---

# 12. 风险与缓解

## 12.1 WebView2 兼容性

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| 用户 Win10 1809 之前，无 WebView2 | 低（< 5%） | HTML/TABLE 功能不可用 | 检测后引导安装；文本/图片/ESC/POS 不受影响 |
| WebView2 版本差异导致渲染偏差 | 中 | 打印排版跟预期不同 | P2 阶段广泛测试多版本 Edge |
| WebView2 COM 初始化的 STA 线程问题 | 中 | 渲染崩溃 | 独立 STA 线程 + channel 通信（已在 7.4 设计） |

## 12.2 32 位老打印机驱动

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| 有些工业标签机只提供 32 位驱动 | 低 | Rust 64-bit exe 调用失败 | 编译一个 32-bit helper.exe（5KB），用 shell 调用。长期是 v2 双架构发布 |
| 驱动接口非标准（ZPL 转义） | 中 | GDI 打印格式不对 | ESC/POS 直发（SEND_RAW）作为替代方案 |

## 12.3 字体度量差异

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| 同一字体在不同系统上默认字形不同 | 中 | 文字排版位移 | 所有示例内置 Noto Sans / Microsoft YaHei 配置；文档说明 |
| GDI 和 WebView2 字体渲染不一致 | 中 | HTML 预阅跟打印结果有出入 | 都走 PDF 中间格式，减少不一致 |

## 12.4 防火墙提示

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| 首次启动被 Windows Defender 弹窗拦截 | 高 | 用户困惑 | Inno Setup 安装时预先注册防火墙规则（netsh advfirewall firewall add rule） |
| 企业 IT 策略禁止本地监听端口 | 低 | 服务无法启动 | 提供配置文件允许改端口；提供命名管道 IPC 替代方案（v2） |
| Chrome 限制私有网络访问 | 中 | 前端连不上 | 已在 CORS 中加 `Access-Control-Allow-Private-Network: true` |

## 12.5 其他

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| 进程意外退出导致打印机未完成 | 低 | 纸张/任务丢失 | 任务提交后由 Spooler 接管；实现 Graceful Shutdown |
| 自签证书被安全软件拦截 | 低 | HTTPS 不可用 | 自动降级到 HTTP 模式，仅弹一条通知 |
| 大并发（多标签页同时调用） | 中 | 任务冲突 | TaskManager 每个连接独立 task 队列；互不干扰 |
| 中文乱码 | 中 | 打印文字异常 | 所有字符串处理强制 UTF-16/Unicode |

---

# 13. 测试与验收标准

## 13.1 单元测试范围

每个 crate 覆盖率目标 ≥ 80%。

### heprint-core
```rust
#[test]
fn test_task_create_and_push_item() { /* 创建任务、添加项、读取项 */ }
#[test]
fn test_style_merge_defaults() { /* 样式创建、合并、覆盖 */ }
#[test]
fn test_error_code_to_string() { /* 所有错误码的 message 映射 */ }
#[test]
fn test_printer_registry_parse() { /* 打印机名解析 */ }
#[test]
fn test_options_serialize() { /* OptionKey/OptionValue 序列化 */ }
```

### heprint-render
```rust
#[test]
fn test_qrcode_to_bitmap() { /* QR 编码 → 位图宽度正确 */ }
#[test]
fn test_code128_encoding() { /* Code128 正确编码 */ }
#[test]
fn test_image_decode_png() { /* PNG 解码为 RGBA */ }
```

### heprint-print
```rust
#[test]
fn test_paper_size_to_devmode() { /* 纸张 → DEVMODE 映射 */ }
#[test]
fn test_escpos_hex_decode() { /* "1B70" → [0x1B, 0x70] */ }
```

### heprint-server
```rust
#[test]
fn test_json_rpc_dispatch() { /* JSON-RPC 正确路由到方法 */ }
#[test]
fn test_cors_headers() { /* 响应包含正确的 CORS 头 */ }
```

### web-sdk
```typescript
// vitest 测试
test('transport connect and call', async () => { /* WS 连接 + 调用 */ });
test('HE object API chains', () => { /* 链式 API 语法正确 */ });
test('error handling on timeout', async () => { /* 超时抛出 */ });
```

## 13.2 集成测试样例

```rust
// e2e/basic.rs
#[test]
fn test_serialize_print_text() {
    // 1. 启动 heprint.exe
    let mut service = start_heprint_service();
    
    // 2. WS 连接
    let ws = connect_to_service(18000)?;
    
    // 3. 发送 HE_INIT
    ws.send(json!({"method":"HE_INIT","params":{"taskName":"test"}}))?;
    assert_ok(ws.recv());
    
    // 4. 发 HE_ADD_TEXT
    ws.send(json!({"method":"HE_ADD_TEXT","params":{"top":100,"left":100,"width":3000,"height":400,"text":"Hello World"}}))?;
    assert_ok(ws.recv());
    
    // 5. 发 HE_SET_STYLE
    ws.send(json!({"method":"HE_SET_STYLE","params":{"name":"FontSize","value":24}}))?;
    assert_ok(ws.recv());
    
    // 6. 发 HE_PRINT_SILENT
    ws.send(json!({"method":"HE_PRINT_SILENT","params":{}}))?;
    let result = ws.recv()?;
    assert_eq!(result["success"], true);
    
    // 7. 停止服务
    service.stop();
}
```

### 前端 E2E 测试

每个示例 HTML 都可以作为手动测试用例：

| 示例 | 测试内容 |
|---|---|
| `01-basic.html` | 打印一行文字到默认打印机 |
| `02-receipt.html` | 80mm 小票：表头 + 表格 + 二维码 |
| `03-a4-report.html` | A4 报表：多段文本 + 表格 + 页码 |
| `04-label.html` | 标签打印：条码 + 文字 |
| `05-raw.html` | ESC/POS 发原始指令（钱箱/切纸） |

## 13.3 性能 benchmark 指标

| 测试 | 目标 |
|---|---|
| 服务冷启动 → 就绪 | < 200ms |
| 空任务：HE_INIT → HE_PRINT_SILENT 链路 | < 50ms |
| 文本任务：添加 10 行 → 打印 | < 100ms |
| 图片任务：添加 1 张 200KB 图片 → 打印 | < 300ms |
| HTML 任务：渲染 50KB HTML → 打印 | < 500ms（含 WebView2 加载） |
| 条码任务：QR Code 生成 → 打印 | < 50ms |
| 并发：同时 5 个连接各自打印 | 全部完成 < 2s |
| 内存占用（空闲） | < 20 MB |
| 内存占用（单次 HTML 打印后） | < 100 MB，30s 内回收 |

### 测试工具

```bash
# 编译 release（未压缩）
cargo build --release --target x86_64-pc-windows-msvc

# 查看 binary 大小
dir target\release\heprint.exe

# UPX 压缩
upx --best --ultra-brute target\release\heprint.exe -o heprint-compressed.exe

# 测试服务延迟
curl -X POST http://127.0.0.1:18000/ws ...
# 或使用 websocat 手动测试
websocat ws://127.0.0.1:18000/ws
```

---

# 附录 A：命令对照表（HE_xxx vs LODOP.xxx）

| HE_xxx | 对应 LODOP.xxx | 差异 |
|---|---|---|
| `HE_INIT` | `PRINT_INIT` | 参数和含义完全一致 |
| `HE_ADD_TEXT` | `ADD_PRINT_TEXT` | 一致 |
| `HE_ADD_HTML` | `ADD_PRINT_HTM` | 名称简化（HTML 替代 HTM） |
| `HE_ADD_TABLE` | `ADD_PRINT_TABLE` | 一致 |
| `HE_ADD_IMAGE` | `ADD_PRINT_IMAGE` | 一致 |
| `HE_ADD_BARCODE` | `ADD_PRINT_BARCODE` | BarcodeType 枚举精简（去掉了罕见的 MAXICODE/AZTEC） |
| `HE_ADD_PDF` | `ADD_PRINT_PDF` | 一致 |
| `HE_ADD_LINE` | `ADD_PRINT_LINE` | 一致 |
| `HE_ADD_RECT` | `ADD_PRINT_RECT` | 一致 |
| `HE_SET_STYLE` | `SET_PRINT_STYLEA(0, ...)` | 名称简化，样式名精简 |
| `HE_SET_PAGE` | `SET_PRINT_PAGESIZE` | 名称简化 |
| `HE_SET_PRINTER` | `SET_PRINTER_INDEX` | 名称简化，类型相同 |
| `HE_SET_COPIES` | `SET_PRINT_COPIES` | 一致 |
| `HE_SET_OPTION` | `SET_PRINT_MODE` | 键数量精简（6 vs 40+） |
| `HE_PRINT` | `PRINT` | 一致，改为返回 Promise |
| `HE_PRINT_SILENT` | `PRINTA` | 名称更语义化 |
| `HE_PREVIEW` | `PREVIEW` | 一致 |
| `HE_NEW_PAGE` | `NEWPAGE` | 一致 |
| `HE_GET_PRINTERS` | `GET_PRINTER_NAMES` | 名称更一致（数组） |
| `HE_GET_DEFAULT_PRINTER` | `GET_DEFAULTPRINTER` | 一致 |
| `HE_HAS_PRINTER` | `IS_PRINTER_EXIST` | 统一 HE_ 前缀命名 |
| `HE_GET_INFO` | `GET_VALUE`| 键数量精简（6 vs 30+） |
| `HE_ON_RESULT` | `On_Return` | 一致 |
| `HE_SEND_RAW` | `SEND_PRINT_RAWDATA` | 名称简化 |
| `HE_VERSION` | `VERSION` | 属性，非方法 |

---

# 附录 B：配置文件（heprint.toml）

配置文件存放在 `%APPDATA%/HePrint/heprint.toml`，自动生成。可通过托盘菜单的"设置"打开编辑。

```toml
[server]
# HTTP 端口（默认 18000）
http_port = 18000
# HTTPS 端口（默认 18443）
https_port = 18443
# 是否启用 HTTPS
enable_https = true
# 绑定地址（默认仅 127.0.0.1）
bind_address = "127.0.0.1"

[cert]
# 证书存放目录
cert_dir = "%APPDATA%/HePrint/cert"
# 自动续期天数（默认 365）
auto_renew_days = 365

[printer]
# 默认打印机（留空 = 系统默认）
default_printer = ""
# 打印超时秒数（默认 30）
print_timeout_sec = 30
# 失败自动重试次数（默认 2）
retry_count = 2

[webview2]
# WebView2 环境路径（留空 = 系统默认）
custom_env_path = ""
# 渲染超时毫秒（默认 15000）
render_timeout_ms = 15000
# 池最大实例数（默认 3，0 = 每次创建新实例）
pool_max_size = 3
# 池空闲回收秒数（默认 30）
pool_idle_sec = 30

[log]
# 日志级别：trace/debug/info/warn/error
level = "info"
# 日志文件路径（空 = 只输出控制台）
file = ""
# 最大文件大小 MB（默认 10）
max_size_mb = 10
# 保留日志文件数（默认 5）
max_files = 5

[tray]
# 是否启用系统托盘图标
show_tray = true
# 是否开机自启
auto_start = false
```

---

> **文档结束。**
>
> 本设计文档共 13 章，约 12000 字。覆盖了：
> - 项目愿景与架构拆解（第 1-2 章）
> - 26 个精选 API 的完整签名与实现说明（第 3 章）
> - 工程结构 5-crate 划分与依赖关系（第 4 章）
> - 5 个核心数据模型（第 5 章）
> - 3 条关键链路详解（第 6 章）
> - WebView2 / HTTPS / 前端 SDK / 安装包方案（第 7-10 章）
> - P0-P4 分阶段实施（第 11 章）
> - 风险清单与缓解（第 12 章）
> - 测试与验收标准（第 13 章）
>
> 下一步：用户审阅通过后 → 进入 P0 编码。