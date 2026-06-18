//! 图片解码与处理

use heprint_core::{ErrorCode, HeError, Result};
use base64::Engine;
use image::ImageReader;
use std::io::Cursor;

pub use image::DynamicImage as ImageBuffer;

/// 解码图片：支持 base64 / 本地路径 / URL（仅本地，URL 暂不实现）
pub fn decode_image(src: &str) -> Result<ImageBuffer> {
    if let Some(b64) = extract_base64(src) {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| HeError::coded(ErrorCode::ImageDecodeFailed, format!("base64 解码失败: {e}")))?;

        let img = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|e| HeError::coded(ErrorCode::ImageDecodeFailed, e.to_string()))?
            .decode()
            .map_err(|e| HeError::coded(ErrorCode::ImageDecodeFailed, e.to_string()))?;

        return Ok(img);
    }

    // 本地路径
    if !src.starts_with("http") {
        let img = ImageReader::open(src)
            .map_err(|_| HeError::code(ErrorCode::FileNotFound))?
            .decode()
            .map_err(|e| HeError::coded(ErrorCode::ImageDecodeFailed, e.to_string()))?;
        return Ok(img);
    }

    Err(HeError::coded(
        ErrorCode::Unknown,
        "URL 图片下载未实现（v1 仅支持 base64 / 本地路径）",
    ))
}

fn extract_base64(src: &str) -> Option<&str> {
    if let Some(rest) = src.strip_prefix("data:") {
        if let Some(idx) = rest.find(";base64,") {
            return Some(&rest[idx + 8..]);
        }
    }
    None
}
