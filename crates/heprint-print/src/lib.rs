//! Windows 打印后端
//!
//! 通过 Win32 GDI / Winspool API 调用本机打印机。

#[cfg(windows)]
pub mod winspool;
#[cfg(windows)]
pub mod gdi;
#[cfg(windows)]
pub mod escpos;

#[cfg(windows)]
pub use winspool::{enum_printers, get_default_printer, has_printer, print_task};
#[cfg(windows)]
pub use escpos::send_raw_to_printer;

// 非 Windows 平台的桩实现
#[cfg(not(windows))]
use heprint_core::{HeError, ErrorCode, Result, PrintTask, TaskResult, PrinterInfo};

#[cfg(not(windows))]
pub fn enum_printers() -> Result<Vec<PrinterInfo>> {
    Err(HeError::coded(ErrorCode::Unknown, "本平台暂不支持打印（v1 仅支持 Windows）"))
}

#[cfg(not(windows))]
pub fn get_default_printer() -> Result<String> {
    Err(HeError::coded(ErrorCode::Unknown, "本平台暂不支持打印"))
}

#[cfg(not(windows))]
pub fn has_printer(_name: &str) -> Result<bool> {
    Ok(false)
}

#[cfg(not(windows))]
pub fn print_task(_task: PrintTask, _silent: bool) -> Result<TaskResult> {
    Err(HeError::coded(ErrorCode::Unknown, "本平台暂不支持打印"))
}

#[cfg(not(windows))]
pub fn send_raw_to_printer(_printer_name: &str, _data: &[u8]) -> Result<()> {
    Err(HeError::coded(ErrorCode::Unknown, "本平台暂不支持打印"))
}
