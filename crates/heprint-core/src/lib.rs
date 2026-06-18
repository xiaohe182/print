//! HePrint 核心数据模型
//!
//! 定义打印任务、打印项、样式、错误码等纯数据结构。
//! 不涉及任何 I/O 或系统调用。

pub mod error;
pub mod types;
pub mod style;
pub mod item;
pub mod task;
pub mod command;
pub mod printer;

// 重导出常用类型
pub use error::{ErrorCode, HeError, Result};
pub use types::{Rect, Orient, BarcodeType, LineStyle};
pub use style::{PrintStyle, Alignment, ItemType};
pub use item::{PrintItem, ItemKind};
pub use task::{PrintTask, TaskManager, TaskStatus, TaskResult, PageConfig};
pub use command::HeCommand;
pub use printer::{PrinterInfo, PrinterRegistry};
