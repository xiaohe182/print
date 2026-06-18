//! 打印项（PrintItem）

use serde::{Deserialize, Serialize};
use crate::types::{Rect, BarcodeType, LineStyle};
use crate::style::PrintStyle;

/// 打印项类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Text,
    Html,
    Table,
    Image,
    Barcode,
    Pdf,
    Line,
    Rect,
}

/// 打印项数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PrintItem {
    Text {
        bounds: Rect,
        style: PrintStyle,
        text: String,
    },
    Html {
        bounds: Rect,
        style: PrintStyle,
        html: String,
    },
    Table {
        bounds: Rect,
        style: PrintStyle,
        html: String,
    },
    Image {
        bounds: Rect,
        style: PrintStyle,
        /// 可以是：
        /// - `data:image/png;base64,...`
        /// - 本地路径 `C:/path/to.png`
        /// - URL `http://...`
        src: String,
    },
    Barcode {
        bounds: Rect,
        style: PrintStyle,
        btype: BarcodeType,
        value: String,
    },
    Pdf {
        bounds: Rect,
        style: PrintStyle,
        /// base64 PDF 数据 或 URL
        content: String,
    },
    Line {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        style: PrintStyle,
        line_style: LineStyle,
        line_width: f64,
    },
    Rect {
        bounds: Rect,
        style: PrintStyle,
        line_style: LineStyle,
        line_width: f64,
    },
    /// 分页标记（不实际打印，仅控制流）
    PageBreak,
}

impl PrintItem {
    pub fn kind(&self) -> ItemKind {
        match self {
            Self::Text { .. } => ItemKind::Text,
            Self::Html { .. } => ItemKind::Html,
            Self::Table { .. } => ItemKind::Table,
            Self::Image { .. } => ItemKind::Image,
            Self::Barcode { .. } => ItemKind::Barcode,
            Self::Pdf { .. } => ItemKind::Pdf,
            Self::Line { .. } => ItemKind::Line,
            Self::Rect { .. } => ItemKind::Rect,
            Self::PageBreak => ItemKind::Text, // 占位
        }
    }

    /// 获取样式可变引用（用于 HE_SET_STYLE）
    pub fn style_mut(&mut self) -> Option<&mut PrintStyle> {
        match self {
            Self::Text { style, .. }
            | Self::Html { style, .. }
            | Self::Table { style, .. }
            | Self::Image { style, .. }
            | Self::Barcode { style, .. }
            | Self::Pdf { style, .. }
            | Self::Line { style, .. }
            | Self::Rect { style, .. } => Some(style),
            Self::PageBreak => None,
        }
    }

    pub fn bounds(&self) -> Option<Rect> {
        match self {
            Self::Text { bounds, .. }
            | Self::Html { bounds, .. }
            | Self::Table { bounds, .. }
            | Self::Image { bounds, .. }
            | Self::Barcode { bounds, .. }
            | Self::Pdf { bounds, .. }
            | Self::Rect { bounds, .. } => Some(*bounds),
            Self::Line { x1, y1, x2, y2, .. } => Some(Rect {
                top: (*y1).min(*y2),
                left: (*x1).min(*x2),
                width: (x1 - x2).abs(),
                height: (y1 - y2).abs(),
            }),
            Self::PageBreak => None,
        }
    }
}
