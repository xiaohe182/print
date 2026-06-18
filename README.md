# HePrint —— 桌面端打印服务 v1.0

> 一个用于替代 C-Lodop 的**桌面端**打印服务，单文件 ~1.5 MB，双击启动自动打开测试页。

[![status](https://img.shields.io/badge/status-v1.0-blue)]()
[![platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6)]()
[![lang](https://img.shields.io/badge/lang-Rust-orange)]()
[![size](https://img.shields.io/badge/installer-1.5MB-brightgreen)]()

---

## ✨ 项目目标

| 维度 | 目标 |
|---|---|
| 🎯 功能 | 网页 → 本地打印机的零摩擦打印链路 |
| 📦 体积 | 安装包 ≤ 5 MB（核心 .exe ≤ 3 MB） |
| 🚀 性能 | 启动 < 200ms，单任务延迟 < 100ms |
| 🪟 平台 | Windows 10/11 (64-bit) |
| 🔌 兼容 | 兼容 C-Lodop 同时存在（端口错开） |
| 🌐 协议 | HTTP + WebSocket（127.0.0.1:18000 / 18443） |

---

## 🆚 与 C-Lodop 的关键差异

| 对比项 | C-Lodop | **HePrint v1** |
|---|---|---|
| 安装包 | ~10 MB | **~3-5 MB** |
| HTML 渲染 | IE TWebBrowser（已停更） | **WebView2（系统 Edge）** |
| API 数量 | 80+ 命令（庞杂） | **26 个精选** |
| API 风格 | 全大写过程式 | **HE_ 前缀，链式 + Promise** |
| 类型支持 | 无 | **TypeScript 一等公民** |
| 端口冲突 | 8000/8443 | **18000/18443**（可与 C-Lodop 共存） |
| 商业授权 | 企业版收费 | **MIT / 自有** |
| 设计器 | 内置可视化 | v2 规划，v1 不做 |

---

## 🏗️ 整体架构

```
┌──────────────────────────────────────────────────┐
│   浏览器（业务页面）                              │
│   └── heprint.min.js (前端 SDK ~10KB)            │
└────────────────────┬─────────────────────────────┘
                     │ HTTP / WebSocket
                     ▼
┌──────────────────────────────────────────────────┐
│   HePrint 本地服务（Rust 单 .exe ~3MB）           │
│   ┌────────────────────────────────────────┐    │
│   │  heprint-server  (Axum + WebSocket)    │    │
│   │       │                                │    │
│   │       ▼                                │    │
│   │  heprint-core   (命令模型 + 任务)       │    │
│   │   ┌──────┴──────┐                      │    │
│   │   ▼             ▼                      │    │
│   │ heprint-render  heprint-print          │    │
│   │ (WebView2 +     (winspool +            │    │
│   │  条码 + 图像)    GDI + ESC/POS)         │    │
│   └────────────────────────────────────────┘    │
└──────────────────────┬───────────────────────────┘
                       ▼
                 物理打印机
```

详见 → [`设计文档.md`](./设计文档.md)

---

## 🎯 核心 API 一览

```js
// 初始化
HE.init('订单小票');

// 设置纸张 + 打印机
HE.setPage(3, 80, 0, '');             // 80mm 卷筒
HE.setPrinter('XP-80C');

// 添加内容
HE.addText(20, 10, 280, 30, '订单 #2026001').setStyle('FontSize', 14).setStyle('Bold', 1);
HE.addBarcode(60, 50, 200, 200, 'QRCode', 'https://example.com/order/001');
HE.addTable(280, 10, 280, 300, document.getElementById('items').outerHTML);

// 静默打印（返回 Promise）
const result = await HE.printSilent();
console.log(result.taskId, result.success);
```

完整 API 参考 → [`docs/API参考.md`](./docs/API参考.md)（待生成）

---

## 📂 项目结构

```
D:\打印插件v1\
├── README.md                    # 本文件
├── 设计文档.md                  # ★ 完整架构与实现设计
├── Cargo.toml                   # Rust workspace 根
│
├── crates\                      # 5 个 Rust crate
│   ├── heprint-core\            # 命令模型 + 任务
│   ├── heprint-render\          # WebView2 + 条码 + 图像
│   ├── heprint-print\           # Win GDI + ESC/POS
│   ├── heprint-server\          # Axum + WS + 证书
│   └── heprint-cli\             # 主程序入口（输出 .exe）
│
├── web-sdk\                     # TypeScript 前端 SDK
│   ├── src\                     # SDK 源码
│   ├── dist\                    # 打包产物（heprint.min.js）
│   └── examples\                # 调用示例
│
├── installer\                   # Inno Setup 安装脚本
├── docs\                        # 详细文档
└── tests\                       # 集成测试
```

---

## 📅 开发路线图

| 阶段 | 周期 | 状态 | 交付物 |
|---|---|---|---|
| **P0** 骨架 | 第 1 周 | ⏳ 等待启动 | 项目结构 + Axum 服务 + `/version` |
| **P1** 核心打印 | 第 2-3 周 | 📋 规划中 | 文本/图片打印 + 打印机管理 |
| **P2** HTML+条码 | 第 4-5 周 | 📋 规划中 | WebView2 + 表格 + 二维码 |
| **P3** 完善 | 第 6-7 周 | 📋 规划中 | PDF/RAW/回调/选项 |
| **P4** 发布 | 第 8-10 周 | 📋 规划中 | 安装包 + 证书 + 托盘 |

---

## 🚀 快速开始（P0 完成后可用）

```bash
# 1. 安装依赖
rustup install stable

# 2. 编译 release
cd D:\打印插件v1
cargo build --release

# 3. 启动服务
.\target\release\heprint.exe

# 4. 浏览器打开示例
start web-sdk\examples\01-hello.html
```

---

## 🛠️ 技术栈

- **后端**：Rust 1.75+ / Axum 0.7 / Tokio 1.x
- **GUI 桥接**：WebView2 (系统 Edge) / windows-rs 0.58
- **渲染**：qrcode / barcoders / image (image-rs)
- **TLS**：rustls + rcgen（自签 CA）
- **前端**：TypeScript 5 / Vite 打包
- **打包**：Inno Setup 6 + UPX

---

## 📜 许可

MIT（计划） / 你（个人/团队）保留全部权利

---

## 📞 文档与支持

- 详细设计 → [`设计文档.md`](./设计文档.md)
- 命令对照 → [`docs/命令对照表.md`](./docs/命令对照表.md)（待生成）
- 开发指南 → [`docs/开发指南.md`](./docs/开发指南.md)（待生成）

---

> **当前版本：v0.0.0 — 设计阶段**
> **下一步：编写《设计文档.md》→ 用户审阅 → 进入 P0 编码**
