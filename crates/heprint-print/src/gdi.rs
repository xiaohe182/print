//! GDI 绘制：把 PrintItem 画到 HDC
//!
//! P1 实现：Text / Image / Barcode(QR) / Line / Rect
//! P2 补齐：Html / Table / Pdf

#![cfg(windows)]

use heprint_core::{
    Alignment, ErrorCode, HeError, ItemType, LineStyle, PrintItem, PrintStyle, Rect, Result,
};
use heprint_render::{decode_image, render_barcode, html_to_text};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, GetDeviceCaps,
    LineTo, MoveToEx, Rectangle, SelectObject, SetBkMode, SetTextColor,
    StretchDIBits, TextOutW, DrawTextW, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    DT_CENTER, DT_LEFT, DT_RIGHT, DT_VCENTER, DT_WORDBREAK,
    HDC, LOGPIXELSX, LOGPIXELSY, PS_DASH, PS_DOT, PS_SOLID,
    SRCCOPY, TRANSPARENT,
};
/// 0.1 mm → 像素（基于设备 DPI）
struct DeviceMetrics {
    dpi_x: i32,
    dpi_y: i32,
}

impl DeviceMetrics {
    fn from(hdc: HDC) -> Self {
        unsafe {
            Self {
                dpi_x: GetDeviceCaps(hdc, LOGPIXELSX),
                dpi_y: GetDeviceCaps(hdc, LOGPIXELSY),
            }
        }
    }

    /// 0.1mm → 像素 (X 方向)
    fn x(&self, deci_mm: i32) -> i32 {
        ((deci_mm as f64) / 254.0 * self.dpi_x as f64) as i32
    }

    fn y(&self, deci_mm: i32) -> i32 {
        ((deci_mm as f64) / 254.0 * self.dpi_y as f64) as i32
    }
}

/// 渲染一个打印项
pub fn render_item(hdc: HDC, item: &PrintItem) -> Result<()> {
    let m = DeviceMetrics::from(hdc);

    match item {
        PrintItem::Text { bounds, style, text } => render_text(hdc, &m, *bounds, style, text),
        PrintItem::Image { bounds, src, .. } => render_image(hdc, &m, *bounds, src),
        PrintItem::Barcode { bounds, btype, value, .. } => {
            render_barcode_item(hdc, &m, *bounds, *btype, value)
        }
        PrintItem::Line { x1, y1, x2, y2, line_style, line_width, .. } => {
            render_line(hdc, &m, *x1, *y1, *x2, *y2, *line_style, *line_width)
        }
        PrintItem::Rect { bounds, line_style, line_width, .. } => {
            render_rect(hdc, &m, *bounds, *line_style, *line_width)
        }
        PrintItem::Html { bounds, html, style } | PrintItem::Table { bounds, html, style } => {
            render_html_item(hdc, &m, *bounds, style, html)
        }
        PrintItem::Pdf { bounds, content, .. } => {
            render_pdf_item(hdc, &m, *bounds, content)
        }
        PrintItem::PageBreak => Ok(()),
    }
}

/// 文本绘制
fn render_text(
    hdc: HDC,
    m: &DeviceMetrics,
    bounds: Rect,
    style: &PrintStyle,
    text: &str,
) -> Result<()> {
    unsafe {
        let _x = m.x(bounds.left);
        let _y = m.y(bounds.top);
        let _w = m.x(bounds.width);
        let _h = m.y(bounds.height);

        // 1. 字体
        let font_name = style
            .font_name
            .clone()
            .unwrap_or_else(|| "Microsoft YaHei".to_string());
        let mut font_name_w: Vec<u16> = font_name.encode_utf16().collect();
        font_name_w.push(0);

        let font_size_pt = style.font_size.unwrap_or(12.0);
        // pt → 像素：1pt = 1/72 inch
        let font_height_px = -(font_size_pt * m.dpi_y as f64 / 72.0) as i32;

        let weight = if style.bold.unwrap_or(false) { 700 } else { 400 };
        let italic = if style.italic.unwrap_or(false) { 1 } else { 0 };
        let underline = if style.underline.unwrap_or(false) { 1 } else { 0 };

        let hfont = CreateFontW(
            font_height_px,
            0, 0, 0,
            weight,
            italic,
            underline,
            0,
            1, // DEFAULT_CHARSET (会兼容中文)
            0, 0, 0, 0,
            PCWSTR(font_name_w.as_ptr()),
        );
        let old_font = SelectObject(hdc, hfont);

        // 2. 颜色
        let color = style.font_color.unwrap_or([0, 0, 0]);
        SetTextColor(hdc, COLORREF(rgb_to_colorref(color)));
        SetBkMode(hdc, TRANSPARENT);

        // 3. 绘制（使用 DrawTextW 支持对齐 + 自动换行 + 宽高约束）
        let mut text_w: Vec<u16> = text.encode_utf16().collect();

        let alignment = style.alignment.unwrap_or(Alignment::Left);
        let dt_align = match alignment {
            Alignment::Center => DT_CENTER,
            Alignment::Right => DT_RIGHT,
            _ => DT_LEFT,
        };

        // width/height > 0 时用 DrawTextW + DT_WORDBREAK 实现区域约束和对齐
        if bounds.width > 0 && bounds.height > 0 {
            let mut rect = RECT {
                left: _x,
                top: _y,
                right: _x + _w,
                bottom: _y + _h,
            };
            let _ = DrawTextW(hdc, &mut text_w, &mut rect, DT_WORDBREAK | dt_align);
        } else {
            // 无宽高约束时退回 TextOutW 单行左对齐
            let _ = TextOutW(hdc, _x, _y, &text_w);
        }

        // 4. 清理
        SelectObject(hdc, old_font);
        let _ = DeleteObject(hfont);

        // 处理 ItemType 提示
        let _ = style.item_type.unwrap_or(ItemType::Normal);
    }
    Ok(())
}

/// 图片绘制
fn render_image(hdc: HDC, m: &DeviceMetrics, bounds: Rect, src: &str) -> Result<()> {
    let img = decode_image(src)?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    let pixels: &[u8] = rgba.as_raw();

    // BITMAPINFO（top-down，每行无 padding，假定 32-bit BGRA）
    // 注意：Windows DIB 是 BGRA + bottom-up（默认）。这里我们用负高度变 top-down。
    let mut bi = BITMAPINFO::default();
    bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bi.bmiHeader.biWidth = w as i32;
    bi.bmiHeader.biHeight = -(h as i32); // top-down
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 32;
    bi.bmiHeader.biCompression = BI_RGB.0;

    // 把 RGBA → BGRA（GDI 要 BGRA）
    let mut bgra = Vec::with_capacity(pixels.len());
    for chunk in pixels.chunks_exact(4) {
        bgra.push(chunk[2]); // B
        bgra.push(chunk[1]); // G
        bgra.push(chunk[0]); // R
        bgra.push(chunk[3]); // A
    }

    unsafe {
        let dest_x = m.x(bounds.left);
        let dest_y = m.y(bounds.top);
        let dest_w = if bounds.width > 0 { m.x(bounds.width) } else { w as i32 };
        let dest_h = if bounds.height > 0 { m.y(bounds.height) } else { h as i32 };

        let r = StretchDIBits(
            hdc,
            dest_x, dest_y, dest_w, dest_h,
            0, 0, w as i32, h as i32,
            Some(bgra.as_ptr() as _),
            &bi,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
        if r == 0 {
            return Err(HeError::coded(
                ErrorCode::ImageDecodeFailed,
                "StretchDIBits 失败",
            ));
        }
    }
    Ok(())
}

/// 条码绘制：通过 render_barcode 生成位图后走图片管道
fn render_barcode_item(
    hdc: HDC,
    m: &DeviceMetrics,
    bounds: Rect,
    btype: heprint_core::BarcodeType,
    value: &str,
) -> Result<()> {
    let bmp = render_barcode(btype, value)?;

    // 转 BGRA
    let mut bgra = Vec::with_capacity(bmp.pixels.len());
    for chunk in bmp.pixels.chunks_exact(4) {
        bgra.push(chunk[2]);
        bgra.push(chunk[1]);
        bgra.push(chunk[0]);
        bgra.push(chunk[3]);
    }

    let mut bi = BITMAPINFO::default();
    bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bi.bmiHeader.biWidth = bmp.width as i32;
    bi.bmiHeader.biHeight = -(bmp.height as i32);
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 32;
    bi.bmiHeader.biCompression = BI_RGB.0;

    unsafe {
        let r = StretchDIBits(
            hdc,
            m.x(bounds.left), m.y(bounds.top),
            m.x(bounds.width), m.y(bounds.height),
            0, 0, bmp.width as i32, bmp.height as i32,
            Some(bgra.as_ptr() as _),
            &bi,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
        if r == 0 {
            return Err(HeError::coded(
                ErrorCode::Unknown,
                "条码绘制失败",
            ));
        }
    }
    Ok(())
}

fn render_line(
    hdc: HDC,
    m: &DeviceMetrics,
    x1: i32, y1: i32, x2: i32, y2: i32,
    line_style: LineStyle,
    line_width: f64,
) -> Result<()> {
    unsafe {
        let pen_style = match line_style {
            LineStyle::Solid => PS_SOLID,
            LineStyle::Dashed => PS_DASH,
            LineStyle::Dotted => PS_DOT,
        };
        let pen_width = (line_width * m.dpi_x as f64 / 254.0).max(1.0) as i32;
        let hpen = CreatePen(pen_style, pen_width, COLORREF(0));
        let old = SelectObject(hdc, hpen);

        let mut prev = Default::default();
        let _ = MoveToEx(hdc, m.x(x1), m.y(y1), Some(&mut prev));
        let _ = LineTo(hdc, m.x(x2), m.y(y2));

        SelectObject(hdc, old);
        let _ = DeleteObject(hpen);
    }
    Ok(())
}

fn render_rect(
    hdc: HDC,
    m: &DeviceMetrics,
    bounds: Rect,
    line_style: LineStyle,
    line_width: f64,
) -> Result<()> {
    unsafe {
        let pen_style = match line_style {
            LineStyle::Solid => PS_SOLID,
            LineStyle::Dashed => PS_DASH,
            LineStyle::Dotted => PS_DOT,
        };
        let pen_width = (line_width * m.dpi_x as f64 / 254.0).max(1.0) as i32;
        let hpen = CreatePen(pen_style, pen_width, COLORREF(0));
        let old_pen = SelectObject(hdc, hpen);

        // 透明填充
        let hbrush = CreateSolidBrush(COLORREF(0xFFFFFF));
        let old_brush = SelectObject(hdc, hbrush);

        let _ = Rectangle(
            hdc,
            m.x(bounds.left),
            m.y(bounds.top),
            m.x(bounds.left + bounds.width),
            m.y(bounds.top + bounds.height),
        );

        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        let _ = DeleteObject(hbrush);
        let _ = DeleteObject(hpen);
    }
    Ok(())
}

#[inline]
fn rgb_to_colorref(rgb: [u8; 3]) -> u32 {
    // COLORREF = 0x00BBGGRR
    (rgb[0] as u32) | ((rgb[1] as u32) << 8) | ((rgb[2] as u32) << 16)
}

// 静音 unused 警告
#[allow(dead_code)]
fn _unused_dt() {
    let _ = (DT_VCENTER,);
}

// ========== HTML / Table 渲染 ==========

/// 渲染 HTML 项：提取文本 → GDI TextOut 绘制
/// 与 C-Lodop 的 ADD_PRINT_HTM 效果一致
fn render_html_item(
    hdc: HDC,
    m: &DeviceMetrics,
    bounds: Rect,
    style: &PrintStyle,
    html: &str,
) -> Result<()> {
    // 提取纯文本（保留表格结构、换行等）
    let text = html_to_text(html);
    render_text(hdc, m, bounds, style, &text)
}

/// 渲染 PDF 项
/// v1: 提取 PDF 为位图后 GDI 绘制
fn render_pdf_item(
    hdc: HDC,
    m: &DeviceMetrics,
    bounds: Rect,
    content: &str,
) -> Result<()> {
    // PDF 内容可以是 base64 编码或文件路径
    // v1: 尝试将 PDF 内容作为 base64 图像处理（用户可能传入的是扫描件）
    // 或者尝试作为图片 URL 处理

    // 先尝试作为 base64 图片解码
    if content.starts_with("data:image/") || content.starts_with("data:application/pdf") {
        // 尝试作为图片处理（PDF 第一页截图的常见做法）
        if let Ok(img) = decode_image(content) {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let pixels: &[u8] = rgba.as_raw();

            let mut bi = BITMAPINFO::default();
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = w as i32;
            bi.bmiHeader.biHeight = -(h as i32);
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB.0;

            let mut bgra = Vec::with_capacity(pixels.len());
            for chunk in pixels.chunks_exact(4) {
                bgra.push(chunk[2]);
                bgra.push(chunk[1]);
                bgra.push(chunk[0]);
                bgra.push(chunk[3]);
            }

            unsafe {
                let dest_w = if bounds.width > 0 { m.x(bounds.width) } else { w as i32 };
                let dest_h = if bounds.height > 0 { m.y(bounds.height) } else { h as i32 };
                let r = StretchDIBits(
                    hdc, m.x(bounds.left), m.y(bounds.top), dest_w, dest_h,
                    0, 0, w as i32, h as i32,
                    Some(bgra.as_ptr() as _),
                    &bi, DIB_RGB_COLORS, SRCCOPY,
                );
                if r == 0 {
                    return Err(HeError::coded(ErrorCode::PdfDecodeFailed, "PDF 图片绘制失败"));
                }
            }
            return Ok(());
        }
    }

    // 降级：显示 PDF 占位文字
    let placeholder = "[PDF 文档 - 请用浏览器原生打印查看完整内容]";
    render_text(hdc, m, bounds, &PrintStyle::default(), placeholder)
}

/// 渲染位图到 HDC（公共函数，HTML/PDF/条码 共用）
#[allow(dead_code)]
fn render_bitmap(hdc: HDC, m: &DeviceMetrics, bounds: Rect, bmp: &heprint_render::Bitmap) -> Result<()> {
    let mut bgra = Vec::with_capacity(bmp.pixels.len());
    for chunk in bmp.pixels.chunks_exact(4) {
        bgra.push(chunk[2]);
        bgra.push(chunk[1]);
        bgra.push(chunk[0]);
        bgra.push(chunk[3]);
    }

    let mut bi = BITMAPINFO::default();
    bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bi.bmiHeader.biWidth = bmp.width as i32;
    bi.bmiHeader.biHeight = -(bmp.height as i32);
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 32;
    bi.bmiHeader.biCompression = BI_RGB.0;

    unsafe {
        let r = StretchDIBits(
            hdc,
            m.x(bounds.left), m.y(bounds.top),
            m.x(bounds.width), m.y(bounds.height),
            0, 0, bmp.width as i32, bmp.height as i32,
            Some(bgra.as_ptr() as _),
            &bi, DIB_RGB_COLORS, SRCCOPY,
        );
        if r == 0 {
            return Err(HeError::coded(ErrorCode::Unknown, "位图绘制失败"));
        }
    }
    Ok(())
}
