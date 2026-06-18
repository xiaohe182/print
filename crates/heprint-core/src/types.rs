//! 通用类型

use serde::{Deserialize, Serialize};

/// 矩形（单位：0.1 mm）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub top: i32,
    pub left: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(top: i32, left: i32, width: i32, height: i32) -> Self {
        Self { top, left, width, height }
    }

    /// 转换为 mm（浮点）
    pub fn to_mm(&self) -> (f64, f64, f64, f64) {
        (
            self.top as f64 / 10.0,
            self.left as f64 / 10.0,
            self.width as f64 / 10.0,
            self.height as f64 / 10.0,
        )
    }
}

/// 纸张方向
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Orient {
    Portrait = 1,
    Landscape = 2,
    Roll = 3,
}

impl From<i32> for Orient {
    fn from(n: i32) -> Self {
        match n {
            2 => Self::Landscape,
            3 => Self::Roll,
            _ => Self::Portrait,
        }
    }
}

/// 条码类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BarcodeType {
    QRCode,
    Code128,
    Code39,
    EAN13,
    EAN8,
    #[serde(rename = "UPC-A")]
    UpcA,
    #[serde(rename = "UPC-E")]
    UpcE,
    PDF417,
    DataMatrix,
}

/// 线型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self::Solid
    }
}

impl LineStyle {
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "dashed" => Self::Dashed,
            "dotted" => Self::Dotted,
            _ => Self::Solid,
        }
    }
}
