//! HePrint 桌面端应用 v1.1
//!
//! 三件套：
//! 1. 系统托盘（常驻右下角）—— Shell_NotifyIconW
//! 2. 主窗口（Win32 窗口 + 嵌入 HTML UI）
//! 3. 打印服务（HTTP + WebSocket）
//!
//! 双击 heprint.exe → 弹出主窗口 + 托盘图标 + 后台服务
//! 关闭主窗口 → 最小化到托盘
//! 右键托盘 → 菜单（显示/退出）
//! 再次点击托盘 → 恢复主窗口

#![cfg_attr(all(not(debug_assertions)), windows_subsystem = "windows")]

use anyhow::Result;
use heprint_server::{run, ServerConfig};
use parking_lot::Mutex;
use std::net::TcpListener;
use std::time::Instant;
use tracing_subscriber::EnvFilter;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, SetBkMode, TextOutW, UpdateWindow, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos,
    GetMessageW, LoadCursorW, LoadIconW, PostQuitMessage, RegisterClassExW, SetForegroundWindow,
    ShowWindow, TrackPopupMenu, TranslateMessage, CW_USEDEFAULT, IDC_ARROW, IDI_INFORMATION,
    MF_SEPARATOR, MF_STRING, MSG, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD, WM_APP,
    WM_COMMAND, WM_DESTROY, WM_LBUTTONUP, WM_PAINT, WM_RBUTTONUP, WNDCLASSEXW, WS_EX_APPWINDOW,
    WS_OVERLAPPEDWINDOW,
};

const WM_TRAYICON: u32 = WM_APP + 1;
const ID_TRAY: u32 = 1001;

const IDM_SHOW: usize = 2001;
const IDM_HIDE: usize = 2002;
const IDM_OPEN_TEST: usize = 2003;
const IDM_OPEN_FOLDER: usize = 2004;
const IDM_QUIT: usize = 2999;

// 全局状态（HWND 用 isize 存储，因为 *mut 不是 Send）
static SERVICE_STARTED: Mutex<Option<Instant>> = Mutex::new(None);
static MAIN_HWND: Mutex<isize> = Mutex::new(0);
static VISIBLE: Mutex<bool> = Mutex::new(true);

fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,heprint=debug")),
        )
        .with_target(false)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let config = parse_args(&args);

    // Prevent a second tray process from surviving without an HTTP service.
    if !service_port_available(&config) {
        tracing::info!(
            "HePrint is already running on {}:{}",
            config.host,
            config.http_port
        );
        open_test_page(config.http_port);
        return Ok(());
    }

    // 启动打印服务（独立线程）
    let server_config = config.clone();
    std::thread::Builder::new()
        .name("heprint-server".to_string())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(2)
                .build()
                .unwrap();
            *SERVICE_STARTED.lock() = Some(Instant::now());
            tracing::info!(
                "✅ HePrint 服务已启动: {}:{}",
                server_config.host,
                server_config.http_port
            );
            runtime.block_on(async move {
                if let Err(e) = run(server_config).await {
                    tracing::error!("HePrint service stopped unexpectedly: {e}");
                    std::process::exit(1);
                }
            });
        })
        .map_err(|e| anyhow::anyhow!("failed to start HePrint service thread: {e}"))?;

    // 启动桌面端
    run_desktop()
}

// 消息循环线程（必须有，且不能退出）
fn run_desktop() -> Result<()> {
    unsafe {
        let module = GetModuleHandleW(None).unwrap();
        let instance: windows::Win32::Foundation::HINSTANCE = module.into();
        let class_name = w!("HePrintMainClass");

        // 注册窗口类
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            lpszClassName: class_name,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            hIcon: LoadIconW(
                windows::Win32::Foundation::HINSTANCE::default(),
                IDI_INFORMATION,
            )
            .unwrap_or_default(),
            ..Default::default()
        };
        let atom = RegisterClassExW(&wc);
        if atom == 0 {
            tracing::error!("RegisterClassExW 失败");
            return Err(anyhow::anyhow!("RegisterClassExW 失败"));
        }

        // 创建主窗口
        let title = w!("HePrint 桌面端 - 打印服务 v1.1");
        tracing::info!("正在创建主窗口...");
        let hwnd = CreateWindowExW(
            WS_EX_APPWINDOW,
            class_name,
            title,
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            440,
            320,
            None,
            None,
            instance,
            None,
        )
        .map_err(|e| {
            tracing::error!("CreateWindowExW 失败: {e:?}, atom={atom}");
            e
        })?;

        *MAIN_HWND.lock() = hwnd.0 as isize;
        // C-Lodop 模式：启动即隐藏到托盘（不显示主窗口）
        *VISIBLE.lock() = false;

        // 不主动显示窗口（用户双击托盘才显示）
        let _ = ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_HIDE);
        let _ = UpdateWindow(hwnd);

        // 创建托盘图标（在主窗口创建后）
        create_tray_icon(hwnd);

        // 设置托盘菜单 ID
        setup_tray_menu(hwnd);

        // 消息循环
        let mut msg = MSG::default();
        loop {
            let ret = GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0);
            if ret.0 == 0 || ret.0 == -1 {
                break;
            }
            if msg.message == WM_QUIT_MSG {
                break;
            }
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }
    }
    Ok(())
}

const WM_QUIT_MSG: u32 = 0x0012;

unsafe fn create_tray_icon(hwnd: HWND) {
    // 使用系统信息图标（可替换为自定义 .ico 资源）
    let icon = LoadIconW(
        windows::Win32::Foundation::HINSTANCE::default(),
        IDI_INFORMATION,
    )
    .unwrap_or_default();

    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: ID_TRAY,
        uFlags: NIF_ICON | NIF_TIP | NIF_MESSAGE,
        uCallbackMessage: WM_TRAYICON,
        hIcon: icon,
        szTip: [0; 128],
        ..Default::default()
    };
    // 写 "HePrint 打印服务" 字符到 szTip
    let tip_text: Vec<u16> = "HePrint 打印服务 v1.1\0".encode_utf16().collect();
    for i in 0..tip_text.len().min(127) {
        nid.szTip[i] = tip_text[i];
    }

    let _ = Shell_NotifyIconW(NIM_ADD, &nid);
}

unsafe fn setup_tray_menu(_hwnd: HWND) {
    // 菜单 ID 保留，这里只需要在右键时动态创建
}

unsafe fn show_context_menu(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS;
    let menu = CreatePopupMenu().unwrap();
    let _ = AppendMenuW(
        menu,
        MENU_ITEM_FLAGS(MF_STRING.0),
        IDM_SHOW,
        w!("📋 显示主窗口"),
    );
    let _ = AppendMenuW(
        menu,
        MENU_ITEM_FLAGS(MF_STRING.0),
        IDM_HIDE,
        w!("👁 隐藏主窗口"),
    );
    let _ = AppendMenuW(menu, MENU_ITEM_FLAGS(MF_SEPARATOR.0), 0, w!(""));
    let _ = AppendMenuW(
        menu,
        MENU_ITEM_FLAGS(MF_STRING.0),
        IDM_OPEN_TEST,
        w!("🌐 打开测试页"),
    );
    let _ = AppendMenuW(
        menu,
        MENU_ITEM_FLAGS(MF_STRING.0),
        IDM_OPEN_FOLDER,
        w!("📂 打开安装目录"),
    );
    let _ = AppendMenuW(menu, MENU_ITEM_FLAGS(MF_SEPARATOR.0), 0, w!(""));
    let _ = AppendMenuW(
        menu,
        MENU_ITEM_FLAGS(MF_STRING.0),
        IDM_QUIT,
        w!("❌ 退出 HePrint"),
    );

    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let _ = SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenu(
        menu,
        TPM_BOTTOMALIGN | TPM_LEFTALIGN | TPM_RETURNCMD,
        pt.x,
        pt.y,
        0,
        hwnd,
        None,
    );

    if cmd.0 > 0 {
        execute_menu_command(cmd.0 as u32);
    }
}

fn execute_menu_command(cmd: u32) {
    unsafe {
        let hwnd_raw = *MAIN_HWND.lock();
        let hwnd = HWND(hwnd_raw as *mut _);
        match cmd {
            x if x == IDM_SHOW as u32 => {
                if !hwnd.is_invalid() {
                    ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_SHOW);
                    SetForegroundWindow(hwnd);
                    *VISIBLE.lock() = true;
                }
            }
            x if x == IDM_HIDE as u32 => {
                if !hwnd.is_invalid() {
                    ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_HIDE);
                    *VISIBLE.lock() = false;
                }
            }
            x if x == IDM_OPEN_TEST as u32 => {
                open_test_page(18000);
            }
            x if x == IDM_OPEN_FOLDER as u32 => {
                if let Some(dir) = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                {
                    let _ = std::process::Command::new("explorer").arg(&dir).spawn();
                }
            }
            x if x == IDM_QUIT as u32 => {
                PostQuitMessage(0);
            }
            _ => {}
        }
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            // 清理托盘
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: ID_TRAY,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                szTip: [0; 128],
                ..Default::default()
            };
            Shell_NotifyIconW(NIM_DELETE, &nid).ok();
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_PAINT => {
            // 绘制窗口内容：显示服务状态信息
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            if !hdc.is_invalid() {
                SetBkMode(hdc, TRANSPARENT);

                // 使用默认字体绘制信息
                let lines = [
                    "HePrint 打印服务",
                    "",
                    &format!("版本 v{} | 正在运行", heprint_server::HE_VERSION),
                    "",
                    "状态: 运行中",
                    "端口: 18000",
                    "地址: 127.0.0.1",
                    "协议: HTTP + WebSocket (JSON-RPC 2.0)",
                    "",
                    "单击托盘图标切换此窗口 | 右键托盘查看更多选项",
                    "测试页面: http://127.0.0.1:18000/",
                ];
                for (i, line) in lines.iter().enumerate() {
                    if !line.is_empty() {
                        let text: Vec<u16> =
                            line.encode_utf16().chain(std::iter::once(0)).collect();
                        let _ = TextOutW(hdc, 20, 20 + i as i32 * 22, &text[..text.len() - 1]);
                    }
                }
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_TRAYICON => {
            match lparam.0 as u32 {
                WM_LBUTTONUP => {
                    // 单击 = 切换窗口
                    let hwnd_main = HWND(*MAIN_HWND.lock() as *mut _);
                    if !hwnd_main.is_invalid() {
                        let visible = *VISIBLE.lock();
                        if visible {
                            ShowWindow(hwnd_main, windows::Win32::UI::WindowsAndMessaging::SW_HIDE);
                            *VISIBLE.lock() = false;
                        } else {
                            ShowWindow(hwnd_main, windows::Win32::UI::WindowsAndMessaging::SW_SHOW);
                            SetForegroundWindow(hwnd_main);
                            *VISIBLE.lock() = true;
                        }
                    }
                }
                WM_RBUTTONUP => {
                    show_context_menu(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            // 托盘菜单命令（wparam 低字节是菜单 ID）
            let cmd = (wparam.0 & 0xFFFF) as u32;
            if cmd > 0 {
                execute_menu_command(cmd);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn service_port_available(config: &ServerConfig) -> bool {
    TcpListener::bind((config.host.as_str(), config.http_port)).is_ok()
}

fn open_test_page(port: u16) {
    let local_page = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.join("index.html")))
        .filter(|path| path.is_file());

    let target = local_page
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("http://127.0.0.1:{port}/"));

    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", "", target.as_str()])
        .spawn();
}

fn parse_args(args: &[String]) -> ServerConfig {
    let mut config = ServerConfig::default();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                if let Some(p) = args.get(i + 1).and_then(|s| s.parse().ok()) {
                    config.http_port = p;
                    i += 1;
                }
            }
            "--host" => {
                if let Some(h) = args.get(i + 1) {
                    config.host = h.clone();
                    i += 1;
                }
            }
            "--workers" | "--max-concurrent" => {
                if let Some(n) = args.get(i + 1).and_then(|s| s.parse().ok()) {
                    config.max_concurrent = n;
                    i += 1;
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("HePrint v{}", heprint_server::HE_VERSION);
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    config
}

fn print_help() {
    println!("HePrint v{} 桌面端打印服务", heprint_server::HE_VERSION);
    println!();
    println!("用法:");
    println!("  heprint.exe [选项]");
    println!();
    println!("选项:");
    println!("  --port <端口>      HTTP 端口 (默认 18000)");
    println!("  --host <地址>      绑定地址 (默认 127.0.0.1)");
    println!("  --workers <N>      并发打印 worker (默认 4)");
    println!("  --help, -h         显示帮助");
    println!("  --version, -V      显示版本");
    println!();
    println!("交互:");
    println!("  双击 heprint.exe → 弹出主窗口");
    println!("  关闭窗口          → 最小化到托盘");
    println!("  单击托盘          → 切换主窗口显示");
    println!("  右键托盘          → 菜单（显示/隐藏/测试页/退出）");
}
