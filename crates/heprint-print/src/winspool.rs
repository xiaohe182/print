//! Winspool API：打印机枚举与任务提交
//!
//! windows-rs 0.58 的关键路径：
//! - `StartDocW` / `EndDoc` / `StartPage` / `EndPage` / `DOCINFOW`
//!   位于 `Win32::Storage::Xps`（不是 `Win32::Graphics::Gdi`）
//! - `PRINTER_INFO_2W` 需要 `Win32_Security` feature
//! - `GetDefaultPrinterW` 签名直接接受 `PWSTR`（非 Option）

#![cfg(windows)]

use heprint_core::{
    ErrorCode, HeError, PrinterInfo, PrintItem, PrintTask, Result, TaskResult,
};

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Graphics::Gdi::{CreateDCW, DeleteDC, HDC};
use windows::Win32::Graphics::Printing::{
    ClosePrinter, EnumPrintersW, GetDefaultPrinterW, OpenPrinterW,
    PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL, PRINTER_INFO_2W,
};
use windows::Win32::Storage::Xps::{DOCINFOW, EndDoc, EndPage, StartDocW, StartPage};

use crate::gdi;

/// 把 Rust &str 转成 wide null-terminated PWSTR 的容器
struct Wide(Vec<u16>);

impl Wide {
    fn new(s: &str) -> Self {
        let mut v: Vec<u16> = s.encode_utf16().collect();
        v.push(0);
        Self(v)
    }
    fn pwstr(&mut self) -> PWSTR {
        PWSTR(self.0.as_mut_ptr())
    }
    fn pcwstr(&self) -> PCWSTR {
        PCWSTR(self.0.as_ptr())
    }
}

/// 列出本机所有打印机
pub fn enum_printers() -> Result<Vec<PrinterInfo>> {
    unsafe {
        // 第一次调用：获取所需缓冲区大小
        let mut needed: u32 = 0;
        let mut count: u32 = 0;
        // windows-rs 0.58 中这些是 const u32，直接 OR
        let flags: u32 = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;

        // 第一次调用通常会返回错误（缓冲区太小），我们忽略它
        let _ = EnumPrintersW(flags, PCWSTR::null(), 2, None, &mut needed, &mut count);

        if needed == 0 {
            return Ok(Vec::new());
        }

        // 第二次调用：获取数据
        let mut buf = vec![0u8; needed as usize];
        let result = EnumPrintersW(
            flags,
            PCWSTR::null(),
            2,
            Some(&mut buf),
            &mut needed,
            &mut count,
        );

        if result.is_err() {
            return Err(HeError::coded(
                ErrorCode::Unknown,
                format!("EnumPrinters 失败: {:?}", result.err()),
            ));
        }

        let default_name = get_default_printer().unwrap_or_default();

        let infos: &[PRINTER_INFO_2W] =
            std::slice::from_raw_parts(buf.as_ptr() as *const PRINTER_INFO_2W, count as usize);

        let mut printers = Vec::with_capacity(count as usize);
        for info in infos {
            let name = pwstr_to_string(info.pPrinterName);
            let driver = if info.pDriverName.0.is_null() {
                None
            } else {
                Some(pwstr_to_string(info.pDriverName))
            };
            let port = if info.pPortName.0.is_null() {
                None
            } else {
                Some(pwstr_to_string(info.pPortName))
            };
            let is_default = name == default_name;
            printers.push(PrinterInfo {
                name,
                driver,
                port,
                is_default,
            });
        }

        Ok(printers)
    }
}

/// 获取默认打印机名
pub fn get_default_printer() -> Result<String> {
    unsafe {
        let mut size: u32 = 0;
        // 第一次调用获取所需大小（PWSTR::null() 表示传 null 缓冲区）
        let _ = GetDefaultPrinterW(PWSTR::null(), &mut size);
        if size == 0 {
            return Ok(String::new());
        }
        let mut buf = vec![0u16; size as usize];
        // GetDefaultPrinterW 在 0.58 返回 BOOL（非 Result）
        // 成功=非零；返回 0 表示无默认打印机
        let result = GetDefaultPrinterW(PWSTR(buf.as_mut_ptr()), &mut size);
        if !result.as_bool() {
            return Ok(String::new());
        }
        // 去掉末尾的 null 终结符
        let s = String::from_utf16_lossy(&buf[..size.saturating_sub(1) as usize]);
        Ok(s)
    }
}

/// 判断打印机是否存在
pub fn has_printer(name: &str) -> Result<bool> {
    let mut wide = Wide::new(name);
    unsafe {
        let mut handle: HANDLE = HANDLE::default();
        // OpenPrinterW 在 0.58 返回 Result<()>，成功即表示存在
        let result = OpenPrinterW(wide.pwstr(), &mut handle, None);
        if result.is_ok() && !handle.is_invalid() {
            let _ = ClosePrinter(handle);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// 提交打印任务
///
/// silent: true=静默打印，false=允许弹对话框（v1 暂未实现弹框，行为一致）
pub fn print_task(task: PrintTask, _silent: bool) -> Result<TaskResult> {
    let printer_name = task
        .printer_name
        .clone()
        .or_else(|| get_default_printer().ok())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| HeError::code(ErrorCode::PrinterNotFound))?;

    if task.is_empty() {
        return Err(HeError::code(ErrorCode::TaskEmpty));
    }

    let task_id = task.task_id.clone();
    let wide_printer = Wide::new(&printer_name);
    let wide_doc_name = Wide::new(&task.name);

    unsafe {
        // 1. 创建打印 DC
        let driver = windows::core::w!("WINSPOOL");
        let hdc = CreateDCW(driver, wide_printer.pcwstr(), PCWSTR::null(), None);
        if hdc.is_invalid() {
            return Err(HeError::coded(
                ErrorCode::PrinterOffline,
                format!("无法创建打印机 DC: {printer_name}"),
            ));
        }

        // 2. StartDoc
        let docinfo = DOCINFOW {
            cbSize: std::mem::size_of::<DOCINFOW>() as i32,
            lpszDocName: wide_doc_name.pcwstr(),
            lpszOutput: PCWSTR::null(),
            lpszDatatype: PCWSTR::null(),
            fwType: 0,
        };

        let job_id = StartDocW(hdc, &docinfo);
        if job_id <= 0 {
            let _ = DeleteDC(hdc);
            return Err(HeError::coded(
                ErrorCode::PrintFailed,
                format!("StartDoc 失败 (job_id={job_id})"),
            ));
        }

        // 3. 渲染打印项
        let copies = task.copies.max(1);
        let mut total_pages = 0u32;

        for _copy in 0..copies {
            let pages = render_task_pages(hdc, &task)?;
            total_pages += pages;
        }

        // 4. EndDoc
        let _ = EndDoc(hdc);
        let _ = DeleteDC(hdc);

        Ok(TaskResult::success(task_id, total_pages))
    }
}

/// 把任务的所有 PrintItem 渲染到 HDC，按 PageBreak 分页
unsafe fn render_task_pages(hdc: HDC, task: &PrintTask) -> Result<u32> {
    let mut page_count = 0u32;
    let mut page_started = false;
    let mut has_renderable = false;

    for item in &task.items {
        if matches!(item, PrintItem::PageBreak) {
            if page_started {
                let _ = EndPage(hdc);
                page_started = false;
            }
            continue;
        }

        if !page_started {
            let r = StartPage(hdc);
            if r <= 0 {
                return Err(HeError::coded(
                    ErrorCode::PrintFailed,
                    format!("StartPage 失败 (r={r})"),
                ));
            }
            page_started = true;
            page_count += 1;
        }

        if let Err(e) = gdi::render_item(hdc, item) {
            tracing::warn!("渲染项失败: {e}");
        }
        has_renderable = true;
    }

    if page_started {
        let _ = EndPage(hdc);
    }

    if page_count == 0 && has_renderable {
        page_count = 1;
    }

    Ok(page_count)
}

/// PWSTR → String
unsafe fn pwstr_to_string(p: PWSTR) -> String {
    if p.0.is_null() {
        return String::new();
    }
    let mut len = 0;
    while *p.0.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(p.0, len);
    String::from_utf16_lossy(slice)
}
