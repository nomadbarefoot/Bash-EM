use std::path::Path;
use crate::adapter::{Adapter, FileCategory};

const PDF_EXTENSIONS: &[&str] = &["pdf"];

pub struct PdfAdapter;

impl PdfAdapter {
    pub fn extract_text(path: &Path) -> Result<String, String> {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("read pdf: {}", e))?;

        if bytes.len() < 5 || &bytes[..5] != b"%PDF-" {
            return Err("not a PDF file".into());
        }

        let text = pdf_extract::extract_text_from_mem(&bytes)
            .map_err(|e| format!("pdf extract: {}", e))?;

        if text.trim().is_empty() {
            return Err("pdf has no extractable text (image-only or encrypted)".into());
        }

        Ok(text)
    }
}

impl Adapter for PdfAdapter {
    fn name(&self) -> &'static str { "pdf" }
    fn extensions(&self) -> &'static [&'static str] { PDF_EXTENSIONS }
    fn category(&self) -> FileCategory { FileCategory::Pdf }
    fn can_write(&self) -> bool { false }

    fn probe(&self, _path: &Path, first_bytes: &[u8]) -> bool {
        first_bytes.len() >= 5 && &first_bytes[..5] == b"%PDF-"
    }

    fn read_content(&self, path: &Path) -> Result<String, String> {
        Self::extract_text(path)
    }

    fn write_back(&self, _path: &Path, _content: &str) -> Result<(), String> {
        Err("PDF rewrite not supported — report-only".into())
    }
}
