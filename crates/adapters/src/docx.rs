use crate::adapter::{Adapter, FileCategory};
use std::io::{Read as IoRead, Write as IoWrite};
use std::path::Path;

const DOCX_EXTENSIONS: &[&str] = &["docx"];

pub struct DocxAdapter;

impl DocxAdapter {
    pub fn read_document_xml(path: &Path) -> Result<Vec<(String, String)>, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("open docx: {}", e))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("read docx zip: {}", e))?;

        let mut results = Vec::new();
        let target_files = [
            "word/document.xml",
            "word/header1.xml",
            "word/header2.xml",
            "word/footer1.xml",
            "word/footer2.xml",
        ];

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| format!("docx entry {}: {}", i, e))?;

            let name = entry.name().to_string();
            if !target_files.iter().any(|&t| name == t) {
                continue;
            }

            let mut xml = String::new();
            entry
                .read_to_string(&mut xml)
                .map_err(|e| format!("read docx xml: {}", e))?;
            results.push((name, xml));
        }

        Ok(results)
    }

    pub fn extract_text_nodes(xml: &str) -> Vec<String> {
        let mut texts = Vec::new();
        let mut reader = quick_xml::Reader::from_str(xml);
        let mut in_wt = false;
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Start(e)) => {
                    if e.name().as_ref() == b"w:t" {
                        in_wt = true;
                    }
                }
                Ok(quick_xml::events::Event::Text(e)) if in_wt => {
                    if let Ok(text) = e.unescape() {
                        texts.push(text.to_string());
                    }
                }
                Ok(quick_xml::events::Event::End(e)) => {
                    if e.name().as_ref() == b"w:t" {
                        in_wt = false;
                    }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
        texts
    }

    pub fn replace_text_in_xml(xml: &str, replacements: &[(String, String)]) -> String {
        let mut result = xml.to_string();
        for (old, new) in replacements {
            result = result.replace(old, new);
        }
        result
    }

    pub fn rewrite_docx(
        path: &Path,
        xml_replacements: &[(String, Vec<(String, String)>)],
    ) -> Result<(), String> {
        let replace_map: std::collections::HashMap<&str, &[(String, String)]> = xml_replacements
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_slice()))
            .collect();

        let file = std::fs::File::open(path).map_err(|e| format!("open docx: {}", e))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("read docx: {}", e))?;

        let tmp_path = path.with_extension("bashm_docx_tmp");
        let tmp_file =
            std::fs::File::create(&tmp_path).map_err(|e| format!("create tmp docx: {}", e))?;
        let mut writer = zip::ZipWriter::new(tmp_file);

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| format!("docx entry {}: {}", i, e))?;

            let name = entry.name().to_string();
            let options =
                zip::write::SimpleFileOptions::default().compression_method(entry.compression());

            writer
                .start_file(&name, options)
                .map_err(|e| format!("write docx entry {}: {}", name, e))?;

            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| format!("read docx entry: {}", e))?;

            if let Some(repls) = replace_map.get(name.as_str()) {
                let xml =
                    String::from_utf8(buf).map_err(|_| format!("non-utf8 xml in {}", name))?;
                let modified = Self::replace_text_in_xml(&xml, repls);
                writer
                    .write_all(modified.as_bytes())
                    .map_err(|e| format!("write docx xml: {}", e))?;
            } else {
                writer
                    .write_all(&buf)
                    .map_err(|e| format!("write docx entry: {}", e))?;
            }
        }

        writer
            .finish()
            .map_err(|e| format!("finalize docx: {}", e))?;
        std::fs::rename(&tmp_path, path).map_err(|e| format!("rename docx: {}", e))?;
        Ok(())
    }
}

impl Adapter for DocxAdapter {
    fn name(&self) -> &'static str {
        "docx"
    }
    fn extensions(&self) -> &'static [&'static str] {
        DOCX_EXTENSIONS
    }
    fn category(&self) -> FileCategory {
        FileCategory::Docs
    }
    fn can_write(&self) -> bool {
        true
    }

    fn probe(&self, _path: &Path, first_bytes: &[u8]) -> bool {
        first_bytes.len() >= 4 && first_bytes[..4] == [0x50, 0x4B, 0x03, 0x04]
    }

    fn read_content(&self, path: &Path) -> Result<String, String> {
        let entries = Self::read_document_xml(path)?;
        let mut texts = Vec::new();
        for (_, xml) in &entries {
            texts.extend(Self::extract_text_nodes(xml));
        }
        Ok(texts.join("\n"))
    }

    fn write_back(&self, _path: &Path, _content: &str) -> Result<(), String> {
        Err("use DocxAdapter::rewrite_docx() for docx write-back".into())
    }
}
