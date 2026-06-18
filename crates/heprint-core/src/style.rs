//! 打印样式系统

use serde::{Deserialize, Serialize};
use crate::types::LineStyle;

/// 文字对齐
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Alignment {
    Left = 1,
    Center = 2,
    Right = 3,
}

impl Default for Alignment {
    fn default() -> Self {
        Self::Left
    }
}

impl From<i32> for Alignment {
    fn from(n: i32) -> Self {
        match n {
            2 => Self::Center,
            3 => Self::Right,
            _ => Self::Left,
        }
    }
}

/// 打印项类型（特殊用途）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ItemType {
    Normal = 0,
    HeaderFooter = 1,
    PageNum = 2,
    PageTotal = 3,
    Sequence = 4,
}

impl Default for ItemType {
    fn default() -> Self {
        Self::Normal
    }
}

impl From<i32> for ItemType {
    fn from(n: i32) -> Self {
        match n {
            1 => Self::HeaderFooter,
            2 => Self::PageNum,
            3 => Self::PageTotal,
            4 => Self::Sequence,
            _ => Self::Normal,
        }
    }
}

/// 打印项样式（设计文档 3.3 节）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrintStyle {
    pub font_name: Option<String>,
    pub font_size: Option<f64>,
    pub font_color: Option<[u8; 3]>,

    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,

    pub alignment: Option<Alignment>,
    pub angle: Option<i32>,
    pub item_type: Option<ItemType>,

    pub as_image: Option<bool>,
    pub keep_color: Option<bool>,
    pub back_color: Option<[u8; 3]>,

    pub line_style: Option<LineStyle>,
    pub line_width: Option<f64>,
}

impl PrintStyle {
    pub fn new() -> Self {
        Self::default()
    }

    /// 通过名称设置样式
    pub fn set(&mut self, name: &str, value: &serde_json::Value) -> Result<(), String> {
        match name {
            "FontName" => {
                self.font_name = value.as_str().map(String::from);
            }
            "FontSize" => {
                self.font_size = value.as_f64();
            }
            "FontColor" => {
                if let Some(s) = value.as_str() {
                    self.font_color = parse_color(s);
                }
            }
            "Bold" => {
                self.bold = Some(parse_bool(value));
            }
            "Italic" => {
                self.italic = Some(parse_bool(value));
            }
            "Underline" => {
                self.underline = Some(parse_bool(value));
            }
            "Alignment" => {
                self.alignment = value.as_i64().map(|n| Alignment::from(n as i32));
            }
            "Angle" => {
                self.angle = value.as_i64().map(|n| n as i32);
            }
            "ItemType" => {
                self.item_type = value.as_i64().map(|n| ItemType::from(n as i32));
            }
            "AsImage" => {
                self.as_image = Some(parse_bool(value));
            }
            "KeepColor" => {
                self.keep_color = Some(parse_bool(value));
            }
            "BackColor" => {
                if let Some(s) = value.as_str() {
                    self.back_color = parse_color(s);
                }
            }
            "LineStyle" => {
                if let Some(s) = value.as_str() {
                    self.line_style = Some(LineStyle::parse(s));
                }
            }
            "LineWidth" => {
                self.line_width = value.as_f64();
            }
            other => return Err(format!("未知样式名: {other}")),
        }
        Ok(())
    }
}

fn parse_bool(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
        _ => false,
    }
}

/// 解析 #RRGGBB 或 named color → RGB
fn parse_color(s: &str) -> Option<[u8; 3]> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some([r, g, b]);
        }
    }
    match s.to_ascii_lowercase().as_str() {
        "black" => Some([0, 0, 0]),
        "white" => Some([255, 255, 255]),
        "red" => Some([255, 0, 0]),
        "green" => Some([0, 128, 0]),
        "blue" => Some([0, 0, 255]),
        "yellow" => Some([255, 255, 0]),
        "gray" | "grey" => Some([128, 128, 128]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_color("#FF0000"), Some([255, 0, 0]));
        assert_eq!(parse_color("#00ff00"), Some([0, 255, 0]));
    }

    #[test]
    fn test_parse_named_color() {
        assert_eq!(parse_color("red"), Some([255, 0, 0]));
        assert_eq!(parse_color("BLUE"), Some([0, 0, 255]));
    }

    #[test]
    fn test_set_font_size() {
        let mut s = PrintStyle::new();
        s.set("FontSize", &serde_json::json!(14.0)).unwrap();
        assert_eq!(s.font_size, Some(14.0));
    }
}
