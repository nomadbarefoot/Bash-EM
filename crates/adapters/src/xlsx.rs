use std::path::Path;
use std::io::{Read as IoRead, Write as IoWrite};
use crate::adapter::{Adapter, FileCategory};

const XLSX_EXTENSIONS: &[&str] = &["xlsx"];

pub struct XlsxAdapter;

impl XlsxAdapter {
    pub fn read_strings(path: &Path) -> Result<Vec<(String, String)>, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("open xlsx: {}", e))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("read xlsx zip: {}", e))?;

        let mut results = Vec::new();

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| format!("xlsx entry {}: {}", i, e))?;

            let name = entry.name().to_string();
            let is_shared_strings = name == "xl/sharedStrings.xml";
            let is_sheet = name.starts_with("xl/worksheets/") && name.ends_with(".xml");

            if !is_shared_strings && !is_sheet {
                continue;
            }

            let mut xml = String::new();
            entry.read_to_string(&mut xml)
                .map_err(|e| format!("read xlsx xml: {}", e))?;
            results.push((name, xml));
        }

        Ok(results)
    }

    pub fn extract_text_from_xml(xml: &str) -> Vec<String> {
        let mut texts = Vec::new();
        let mut reader = quick_xml::Reader::from_str(xml);
        let mut in_t = false;
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Start(e)) if e.name().as_ref() == b"t" => {
                    in_t = true;
                }
                Ok(quick_xml::events::Event::Text(e)) if in_t => {
                    if let Ok(text) = e.unescape() {
                        texts.push(text.to_string());
                    }
                }
                Ok(quick_xml::events::Event::End(e)) if e.name().as_ref() == b"t" => {
                    in_t = false;
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

    pub fn rewrite_xlsx(
        path: &Path,
        xml_replacements: &[(String, Vec<(String, String)>)],
    ) -> Result<(), String> {
        let replace_map: std::collections::HashMap<&str, &[(String, String)]> = xml_replacements
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_slice()))
            .collect();

        let file = std::fs::File::open(path)
            .map_err(|e| format!("open xlsx: {}", e))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("read xlsx: {}", e))?;

        let tmp_path = path.with_extension("bashm_xlsx_tmp");
        let tmp_file = std::fs::File::create(&tmp_path)
            .map_err(|e| format!("create tmp xlsx: {}", e))?;
        let mut writer = zip::ZipWriter::new(tmp_file);

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| format!("xlsx entry {}: {}", i, e))?;

            let name = entry.name().to_string();
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(entry.compression());

            writer.start_file(&name, options)
                .map_err(|e| format!("write xlsx entry {}: {}", name, e))?;

            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)
                .map_err(|e| format!("read xlsx entry: {}", e))?;

            if let Some(repls) = replace_map.get(name.as_str()) {
                let xml = String::from_utf8(buf)
                    .map_err(|_| format!("non-utf8 xml in {}", name))?;
                let modified = Self::replace_text_in_xml(&xml, repls);
                writer.write_all(modified.as_bytes())
                    .map_err(|e| format!("write xlsx xml: {}", e))?;
            } else {
                writer.write_all(&buf)
                    .map_err(|e| format!("write xlsx entry: {}", e))?;
            }
        }

        writer.finish()
            .map_err(|e| format!("finalize xlsx: {}", e))?;
        std::fs::rename(&tmp_path, path)
            .map_err(|e| format!("rename xlsx: {}", e))?;
        Ok(())
    }
}

impl Adapter for XlsxAdapter {
    fn name(&self) -> &'static str { "xlsx" }
    fn extensions(&self) -> &'static [&'static str] { XLSX_EXTENSIONS }
    fn category(&self) -> FileCategory { FileCategory::Office }
    fn can_write(&self) -> bool { true }

    fn probe(&self, _path: &Path, first_bytes: &[u8]) -> bool {
        first_bytes.len() >= 4 && first_bytes[..4] == [0x50, 0x4B, 0x03, 0x04]
    }

    fn read_content(&self, path: &Path) -> Result<String, String> {
        let entries = Self::read_strings(path)?;
        let mut texts = Vec::new();
        for (_, xml) in &entries {
            texts.extend(Self::extract_text_from_xml(xml));
        }
        Ok(texts.join("\n"))
    }

    fn write_back(&self, _path: &Path, _content: &str) -> Result<(), String> {
        Err("use XlsxAdapter::rewrite_xlsx() for xlsx write-back".into())
    }
}
