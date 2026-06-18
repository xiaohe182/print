//! 打印机信息

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterInfo {
    pub name: String,
    pub driver: Option<String>,
    pub port: Option<String>,
    pub is_default: bool,
}

/// 打印机注册表（运行时缓存）
#[derive(Debug, Default)]
pub struct PrinterRegistry {
    pub printers: Vec<PrinterInfo>,
    pub default_printer: Option<String>,
}

impl PrinterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn names(&self) -> Vec<String> {
        self.printers.iter().map(|p| p.name.clone()).collect()
    }

    pub fn has(&self, name: &str) -> bool {
        self.printers.iter().any(|p| p.name.eq_ignore_ascii_case(name))
    }
}
