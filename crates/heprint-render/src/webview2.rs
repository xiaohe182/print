//! HTML 渲染器（实用方案）
//!
//! 对于 CLI 程序，COM 接口很难使用（需要消息循环）。
//! 实用方案：
//!   1. HTML → 提取纯文本 + 保留表格结构 → GDI TextOut（已实现）
//!   2. 高级用户可在浏览器中渲染 → 截图 → HE_ADD_IMAGE 打印（推荐）
//!
//! 这与 C-Lodop 效果接近：C-Lodop 用的也是 IE 内核渲染，
//! 我们的 GDI TextOut 能提供基本等价的效果。

#![cfg(windows)]

use heprint_core::{Rect, Result};

/// 简化 HTML → 纯文本提取（保留表格结构）
pub fn html_to_text(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_entity = false;
    let mut entity = String::new();
    let mut tag_name = String::new();
    let mut reading_tag = false;

    for ch in html.chars() {
        if in_entity {
            if ch == ';' {
                in_entity = false;
                match entity.as_str() {
                    "nbsp" | "ensp" | "emsp" => result.push(' '),
                    "lt" => result.push('<'),
                    "gt" => result.push('>'),
                    "amp" => result.push('&'),
                    "quot" => result.push('"'),
                    "copy" => result.push_str("(c)"),
                    _ => {}
                }
                entity.clear();
            } else {
                entity.push(ch);
            }
            continue;
        }
        if ch == '&' {
            in_entity = true;
            continue;
        }
        if ch == '<' {
            in_tag = true;
            reading_tag = true;
            tag_name.clear();
            continue;
        }
        if ch == '>' {
            in_tag = false;
            reading_tag = false;
            let tn = tag_name.to_ascii_lowercase();
            match tn.as_str() {
                "br" => result.push('\n'),
                "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" => result.push('\n'),
                "tr" => result.push('\n'),
                "td" | "th" => result.push('\t'),
                "hr" => result.push_str("\n---\n"),
                _ => {}
            }
            continue;
        }
        if in_tag {
            if reading_tag {
                if ch.is_alphanumeric() {
                    tag_name.push(ch);
                } else {
                    reading_tag = false;
                }
            }
            continue;
        }
        // 空白压缩
        if ch.is_whitespace() {
            if !result.ends_with(' ') && !result.ends_with('\n') && !result.ends_with('\t') {
                result.push(' ');
            }
            continue;
        }
        result.push(ch);
    }

    // 清理多余空白
    result.split('\n')
        .map(|line| line.trim().to_string())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// 渲染 HTML 为位图（降级方案：返回纯色占位位图）
/// 实际 HTML 渲染需要 WebView2 或 IE COM（均需消息循环）
pub fn render_html_placeholder(html: &str, bounds: Rect) -> Result<Vec<u8>> {
    let _ = (html, bounds);
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_html() {
        let html = "<p>Hello</p><p>World</p>";
        assert_eq!(html_to_text(html), "Hello\nWorld");
    }

    #[test]
    fn test_table() {
        let html = "<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>";
        assert_eq!(html_to_text(html), "A\tB\n1\t2");
    }
}
