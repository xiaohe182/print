//! 错误码与错误类型

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 错误码（与文档第 3.4 节一致）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum ErrorCode {
    Success = 0,
    Unknown = -1,
    Timeout = -2,

    ConnectionTimeout = 1001,
    ConnectionRefused = 1002,
    InvalidJsonRpc = 1003,
    MethodNotFound = 1004,
    InvalidParam = 1005,

    TaskNotFound = 2001,
    TaskEmpty = 2002,
    PrinterNotFound = 2004,
    PrinterOffline = 2005,
    PaperNotLoaded = 2006,
    PrintFailed = 2007,
    DuplexNotSupported = 2008,

    WebView2NotInstalled = 3001,
    HtmlRenderTimeout = 3002,

    ImageDecodeFailed = 4001,
    FileNotFound = 4002,
    InvalidBarcodeType = 4003,
    PdfDecodeFailed = 4004,
    DataTooLarge = 4005,
}

impl ErrorCode {
    pub fn message(&self) -> &'static str {
        match self {
            Self::Success => "OK",
            Self::Unknown => "未知错误",
            Self::Timeout => "操作超时",
            Self::ConnectionTimeout => "连接超时",
            Self::ConnectionRefused => "连接被拒绝",
            Self::InvalidJsonRpc => "JSON-RPC 格式错误",
            Self::MethodNotFound => "方法不存在",
            Self::InvalidParam => "参数无效",
            Self::TaskNotFound => "任务不存在",
            Self::TaskEmpty => "任务为空，无内容可打印",
            Self::PrinterNotFound => "打印机不存在",
            Self::PrinterOffline => "打印机脱机或未就绪",
            Self::PaperNotLoaded => "缺纸",
            Self::PrintFailed => "打印失败",
            Self::DuplexNotSupported => "不支持双面打印",
            Self::WebView2NotInstalled => "WebView2 运行时未安装",
            Self::HtmlRenderTimeout => "HTML 渲染超时",
            Self::ImageDecodeFailed => "图片解码失败",
            Self::FileNotFound => "文件不存在",
            Self::InvalidBarcodeType => "不支持的条码类型",
            Self::PdfDecodeFailed => "PDF 解码失败",
            Self::DataTooLarge => "数据过大",
        }
    }
}

/// 顶层错误类型
#[derive(Debug, Error)]
pub enum HeError {
    #[error("[{code:?}] {message}")]
    Coded {
        code: ErrorCode,
        message: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl HeError {
    pub fn code(code: ErrorCode) -> Self {
        Self::Coded {
            code,
            message: code.message().to_string(),
        }
    }

    pub fn coded<S: Into<String>>(code: ErrorCode, msg: S) -> Self {
        Self::Coded {
            code,
            message: msg.into(),
        }
    }

    pub fn error_code(&self) -> ErrorCode {
        match self {
            Self::Coded { code, .. } => *code,
            _ => ErrorCode::Unknown,
        }
    }
}

pub type Result<T> = std::result::Result<T, HeError>;
