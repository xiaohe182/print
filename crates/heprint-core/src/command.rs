//! HE_xxx 命令枚举与解析

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 与文档第 3 章保持一致的 25 个命令
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HeCommand {
    /// 初始化任务
    HeInit { task_name: String },

    HeAddText { top: i32, left: i32, width: i32, height: i32, text: String },
    HeAddHtml { top: i32, left: i32, width: i32, height: i32, html: String },
    HeAddTable { top: i32, left: i32, width: i32, height: i32, table_html: String },
    HeAddImage { top: i32, left: i32, width: i32, height: i32, src: String },
    HeAddBarcode { top: i32, left: i32, width: i32, height: i32, btype: String, value: String },
    HeAddPdf { top: i32, left: i32, width: i32, height: i32, content: String },
    HeAddLine { top1: i32, left1: i32, top2: i32, left2: i32, line_style: Option<String>, line_width: Option<f64> },
    HeAddRect { top: i32, left: i32, width: i32, height: i32, line_style: Option<String>, line_width: Option<f64> },

    HeSetStyle { name: String, value: Value },
    HeSetPage { orient: i32, width: f64, height: f64, name: Option<String> },
    HeSetPrinter { printer: Value },     // string | number
    HeSetCopies { count: u32 },
    HeSetOption { key: String, value: Value },

    HePrint,
    HePrintSilent,
    HePreview,
    HeNewPage,

    HeGetPrinters,
    HeGetDefaultPrinter,
    HeHasPrinter { name: String },
    HeGetInfo { key: String },

    HeOnResult,             // 仅注册回调，无具体参数
    HeSendRaw { printer_name: String, data: String, encoding: Option<String> },

    /// 心跳/版本探测
    HeVersion,
}

impl HeCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::HeInit { .. } => "HE_INIT",
            Self::HeAddText { .. } => "HE_ADD_TEXT",
            Self::HeAddHtml { .. } => "HE_ADD_HTML",
            Self::HeAddTable { .. } => "HE_ADD_TABLE",
            Self::HeAddImage { .. } => "HE_ADD_IMAGE",
            Self::HeAddBarcode { .. } => "HE_ADD_BARCODE",
            Self::HeAddPdf { .. } => "HE_ADD_PDF",
            Self::HeAddLine { .. } => "HE_ADD_LINE",
            Self::HeAddRect { .. } => "HE_ADD_RECT",
            Self::HeSetStyle { .. } => "HE_SET_STYLE",
            Self::HeSetPage { .. } => "HE_SET_PAGE",
            Self::HeSetPrinter { .. } => "HE_SET_PRINTER",
            Self::HeSetCopies { .. } => "HE_SET_COPIES",
            Self::HeSetOption { .. } => "HE_SET_OPTION",
            Self::HePrint => "HE_PRINT",
            Self::HePrintSilent => "HE_PRINT_SILENT",
            Self::HePreview => "HE_PREVIEW",
            Self::HeNewPage => "HE_NEW_PAGE",
            Self::HeGetPrinters => "HE_GET_PRINTERS",
            Self::HeGetDefaultPrinter => "HE_GET_DEFAULT_PRINTER",
            Self::HeHasPrinter { .. } => "HE_HAS_PRINTER",
            Self::HeGetInfo { .. } => "HE_GET_INFO",
            Self::HeOnResult => "HE_ON_RESULT",
            Self::HeSendRaw { .. } => "HE_SEND_RAW",
            Self::HeVersion => "HE_VERSION",
        }
    }
}
