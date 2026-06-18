//! 条码与二维码生成
//!
//! P1 仅支持 QRCode（最常用）。
//! 其他类型在 P3 阶段补齐。

use heprint_core::{BarcodeType, ErrorCode, HeError, Result};
use crate::Bitmap;

/// 生成条码位图
pub fn render_barcode(btype: BarcodeType, value: &str) -> Result<Bitmap> {
    match btype {
        BarcodeType::QRCode => render_qrcode(value),
        // P3 阶段补齐
        _ => Err(HeError::coded(
            ErrorCode::InvalidBarcodeType,
            format!("条码类型 {btype:?} 暂未实现，目前仅支持 QRCode"),
        )),
    }
}

fn render_qrcode(value: &str) -> Result<Bitmap> {
    use qrcode::{QrCode, EcLevel};

    let code = QrCode::with_error_correction_level(value.as_bytes(), EcLevel::M)
        .map_err(|e| HeError::coded(ErrorCode::Unknown, format!("QR 生成失败: {e}")))?;

    // 每个模块画 8 x 8 像素
    let modules = code.to_colors();
    let size = (modules.len() as f64).sqrt() as u32;
    let scale = 8u32;
    let img_size = size * scale;

    let mut bmp = Bitmap::new(img_size, img_size);

    for y in 0..size {
        for x in 0..size {
            let dark = modules[(y * size + x) as usize] == qrcode::Color::Dark;
            let color = if dark { [0, 0, 0, 255] } else { [255, 255, 255, 255] };

            for dy in 0..scale {
                for dx in 0..scale {
                    bmp.set(x * scale + dx, y * scale + dy, color);
                }
            }
        }
    }

    Ok(bmp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qrcode_basic() {
        let bmp = render_qrcode("hello").unwrap();
        assert!(bmp.width > 0);
        assert_eq!(bmp.width, bmp.height);
    }
}
