//! PDF rasterization through the Windows Runtime PDF engine.

#![cfg(windows)]

use std::io::Cursor;

use base64::Engine;
use heprint_core::{ErrorCode, HeError, Result};
use image::ImageReader;
use windows::Data::Pdf::{PdfDocument, PdfPageRenderOptions};
use windows::Storage::Streams::{DataReader, DataWriter, InMemoryRandomAccessStream};
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

use crate::ImageBuffer;

const MAX_PDF_BYTES: usize = 100 * 1024 * 1024;
const MAX_RENDER_EDGE: u32 = 6000;

pub struct PdfRenderer {
    document: PdfDocument,
    apartment_initialized: bool,
}

impl PdfRenderer {
    pub fn new(content: &str) -> Result<Self> {
        let bytes = decode_pdf_content(content)?;
        let apartment_initialized = unsafe { RoInitialize(RO_INIT_MULTITHREADED).is_ok() };

        let stream = InMemoryRandomAccessStream::new().map_err(pdf_error)?;
        let output = stream.GetOutputStreamAt(0).map_err(pdf_error)?;
        let writer = DataWriter::CreateDataWriter(&output).map_err(pdf_error)?;
        writer.WriteBytes(&bytes).map_err(pdf_error)?;
        writer
            .StoreAsync()
            .and_then(|op| op.get())
            .map_err(pdf_error)?;
        writer.DetachStream().map_err(pdf_error)?;
        stream.Seek(0).map_err(pdf_error)?;

        let document = PdfDocument::LoadFromStreamAsync(&stream)
            .and_then(|op| op.get())
            .map_err(pdf_error)?;

        Ok(Self {
            document,
            apartment_initialized,
        })
    }

    pub fn page_count(&self) -> Result<u32> {
        self.document.PageCount().map_err(pdf_error)
    }

    pub fn render_page(&self, index: u32, width: u32, height: u32) -> Result<ImageBuffer> {
        let page = self.document.GetPage(index).map_err(pdf_error)?;
        let page_size = page.Size().map_err(pdf_error)?;
        let (render_width, render_height) = fit_size(
            page_size.Width.max(1.0) as u32,
            page_size.Height.max(1.0) as u32,
            width,
            height,
        );

        let options = PdfPageRenderOptions::new().map_err(pdf_error)?;
        options
            .SetDestinationWidth(render_width)
            .map_err(pdf_error)?;
        options
            .SetDestinationHeight(render_height)
            .map_err(pdf_error)?;

        let stream = InMemoryRandomAccessStream::new().map_err(pdf_error)?;
        page.RenderWithOptionsToStreamAsync(&stream, &options)
            .and_then(|op| op.get())
            .map_err(pdf_error)?;

        let size = stream.Size().map_err(pdf_error)?;
        if size == 0 || size > u32::MAX as u64 {
            return Err(HeError::coded(
                ErrorCode::PdfDecodeFailed,
                "PDF page produced an invalid bitmap",
            ));
        }

        let input = stream.GetInputStreamAt(0).map_err(pdf_error)?;
        let reader = DataReader::CreateDataReader(&input).map_err(pdf_error)?;
        reader
            .LoadAsync(size as u32)
            .and_then(|op| op.get())
            .map_err(pdf_error)?;
        let mut png = vec![0u8; size as usize];
        reader.ReadBytes(&mut png).map_err(pdf_error)?;

        ImageReader::new(Cursor::new(png))
            .with_guessed_format()
            .map_err(|e| HeError::coded(ErrorCode::PdfDecodeFailed, e.to_string()))?
            .decode()
            .map_err(|e| HeError::coded(ErrorCode::PdfDecodeFailed, e.to_string()))
    }
}

impl Drop for PdfRenderer {
    fn drop(&mut self) {
        if self.apartment_initialized {
            unsafe { RoUninitialize() };
        }
    }
}

fn decode_pdf_content(content: &str) -> Result<Vec<u8>> {
    let bytes = if let Some(rest) = content.strip_prefix("data:") {
        let marker = ";base64,";
        let start = rest.find(marker).ok_or_else(|| {
            HeError::coded(
                ErrorCode::PdfDecodeFailed,
                "PDF data URL is not base64 encoded",
            )
        })?;
        base64::engine::general_purpose::STANDARD
            .decode(&rest[start + marker.len()..])
            .map_err(|e| {
                HeError::coded(
                    ErrorCode::PdfDecodeFailed,
                    format!("PDF base64 decode failed: {e}"),
                )
            })?
    } else if content.starts_with("http://") || content.starts_with("https://") {
        return Err(HeError::coded(
            ErrorCode::PdfDecodeFailed,
            "Remote PDF URLs are not supported; upload the PDF as a data URL",
        ));
    } else if std::path::Path::new(content).is_file() {
        std::fs::read(content).map_err(|e| {
            HeError::coded(
                ErrorCode::PdfDecodeFailed,
                format!("Cannot read PDF file: {e}"),
            )
        })?
    } else {
        base64::engine::general_purpose::STANDARD
            .decode(content)
            .map_err(|e| {
                HeError::coded(
                    ErrorCode::PdfDecodeFailed,
                    format!("Invalid PDF content: {e}"),
                )
            })?
    };

    if bytes.len() > MAX_PDF_BYTES {
        return Err(HeError::code(ErrorCode::DataTooLarge));
    }
    if !bytes.starts_with(b"%PDF-") {
        return Err(HeError::coded(
            ErrorCode::PdfDecodeFailed,
            "Content is not a PDF document",
        ));
    }
    Ok(bytes)
}

fn fit_size(
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
) -> (u32, u32) {
    let max_width = target_width.max(1).min(MAX_RENDER_EDGE);
    let max_height = target_height.max(1).min(MAX_RENDER_EDGE);
    let scale =
        (max_width as f64 / source_width as f64).min(max_height as f64 / source_height as f64);
    (
        (source_width as f64 * scale).round().max(1.0) as u32,
        (source_height as f64 * scale).round().max(1.0) as u32,
    )
}

fn pdf_error(error: windows::core::Error) -> HeError {
    HeError::coded(
        ErrorCode::PdfDecodeFailed,
        format!("Windows PDF renderer: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    use super::{fit_size, PdfRenderer};

    #[test]
    fn fit_size_preserves_aspect_ratio() {
        assert_eq!(fit_size(595, 842, 2480, 3508), (2479, 3508));
        assert_eq!(fit_size(842, 595, 2480, 3508), (2480, 1752));
    }

    #[test]
    fn renders_every_page_of_a_pdf() {
        let pdf = two_page_pdf();
        let data_url = format!(
            "data:application/pdf;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(pdf)
        );
        let renderer = PdfRenderer::new(&data_url).expect("load PDF");
        assert_eq!(renderer.page_count().expect("page count"), 2);

        for index in 0..2 {
            let image = renderer.render_page(index, 600, 800).expect("render page");
            assert!(image.width() > 0);
            assert!(image.height() > 0);
        }
    }

    fn two_page_pdf() -> Vec<u8> {
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] /Resources << /Font << /F1 7 0 R >> >> /Contents 4 0 R >>",
            "<< /Length 38 >>\nstream\nBT /F1 24 Tf 40 300 Td (Page 1) Tj ET\nendstream",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] /Resources << /Font << /F1 7 0 R >> >> /Contents 6 0 R >>",
            "<< /Length 38 >>\nstream\nBT /F1 24 Tf 40 300 Td (Page 2) Tj ET\nendstream",
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        ];

        let mut pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets = Vec::with_capacity(objects.len());
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
        }

        let xref = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );
        pdf
    }
}
