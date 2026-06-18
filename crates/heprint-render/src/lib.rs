//! 渲染层：把 HTML/条码/图片/PDF 转成"可打印的位图"

pub mod barcode;
pub mod image;
pub mod webview2;

pub use barcode::render_barcode;
pub use image::{decode_image, ImageBuffer};
pub use webview2::html_to_text;

/// 单色 / 灰度 / RGBA 位图
#[derive(Debug, Clone)]
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    /// 像素：每像素 4 字节（RGBA）
    pub pixels: Vec<u8>,
}

impl Bitmap {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![255; (width * height * 4) as usize],
        }
    }

    pub fn set(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        self.pixels[idx..idx + 4].copy_from_slice(&rgba);
    }
}
