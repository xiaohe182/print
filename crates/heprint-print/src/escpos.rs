//! ESC/POS 原生指令直发（小票机、标签机）
//!
//! 通过 Win32 WritePrinter RAW 模式发送字节流到打印机

#![cfg(windows)]

use heprint_core::{ErrorCode, HeError, Result};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Graphics::Printing::{OpenPrinterW, ClosePrinter, StartDocPrinterW, EndDocPrinter, WritePrinter, DOC_INFO_1W};
use windows::core::PWSTR;

/// 向打印机发送原始字节数据（ESC/POS 指令）
pub fn send_raw_to_printer(printer_name: &str, data: &[u8]) -> Result<()> {
    let mut wide_name: Vec<u16> = printer_name.encode_utf16().collect();
    wide_name.push(0);

    // 预编码 docInfo 字符串（避免借用冲突）
    let doc_name_w: Vec<u16> = "HePrint RAW\0".encode_utf16().collect();
    let data_type_w: Vec<u16> = "RAW\0".encode_utf16().collect();

    unsafe {
        let mut handle: HANDLE = HANDLE::default();
        let result = OpenPrinterW(PWSTR(wide_name.as_mut_ptr()), &mut handle, None);
        if result.is_err() || handle.is_invalid() {
            return Err(HeError::coded(ErrorCode::PrinterOffline, format!("无法打开打印机: {printer_name}")));
        }

        let doc_info = DOC_INFO_1W {
            pDocName: PWSTR(doc_name_w.as_ptr() as *mut _),
            pOutputFile: PWSTR::null(),
            pDatatype: PWSTR(data_type_w.as_ptr() as *mut _),
        };

        // StartDocPrinterW 需要 level 参数（固定为 1）
        let job_id = StartDocPrinterW(handle, 1, &doc_info);
        if job_id == 0 {
            let _ = ClosePrinter(handle);
            return Err(HeError::coded(ErrorCode::PrintFailed, "StartDocPrinter 失败"));
        }

        // WritePrinter 需要 *const c_void（用 bytes.as_ptr() as *const _）
        let mut written: u32 = 0;
        let result = WritePrinter(
            handle,
            data.as_ptr() as *const _,
            data.len() as u32,
            &mut written,
        );
        if !result.as_bool() || written as usize != data.len() {
            let _ = EndDocPrinter(handle);
            let _ = ClosePrinter(handle);
            return Err(HeError::coded(ErrorCode::PrintFailed, format!("WritePrinter 失败, written={written}")));
        }

        let _ = EndDocPrinter(handle);
        let _ = ClosePrinter(handle);
    }

    Ok(())
}
