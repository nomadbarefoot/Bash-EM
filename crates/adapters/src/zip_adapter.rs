use std::fs;
use std::io::{Read as IoRead, Write as IoWrite};
use std::path::Path;
use zip::read::ZipArchive;
use zip::write::ZipWriter;
use crate::adapter::{Adapter, FileCategory};

pub struct ZipAdapter;

const ZIP_EXTENSIONS: &[&str] = &["zip"];

fn is_text_entry(name: &str, bytes: &[u8]) -> bool {
    let text_exts = [
        ".md", ".txt", ".rst", ".csv", ".tsv", ".log",
        ".yml", ".yaml", ".toml", ".json", ".xml", ".ini", ".cfg",
        ".rs", ".py", ".js", ".ts", ".go", ".rb", ".lua",
        ".sh", ".bash", ".zsh",
        ".c", ".cpp", ".h", ".hpp", ".java",
        ".html", ".htm", ".css", ".scss",
        ".sql", ".graphql",
    ];

    let has_text_ext = text_exts.iter().any(|ext| name.ends_with(ext));
    if !has_text_ext {
        return false;
    }
    !bytes.contains(&0) && std::str::from_utf8(bytes).is_ok()
}

impl ZipAdapter {
    pub fn read_entries(path: &Path) -> Result<Vec<(String, String)>, String> {
        let file = fs::File::open(path)
            .map_err(|e| format!("open zip: {}", e))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| format!("read zip: {}", e))?;

        let mut entries = Vec::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| format!("zip entry {}: {}", i, e))?;

            if entry.is_dir() || entry.encrypted() {
                continue;
            }

            let name = entry.name().to_string();
            let mut buf = Vec::new();
            if entry.read_to_end(&mut buf).is_err() {
                continue;
            }

            if is_text_entry(&name, &buf) {
                if let Ok(text) = String::from_utf8(buf) {
                    entries.push((name, text));
                }
            }
        }
        Ok(entries)
    }

    pub fn rewrite_zip(
        path: &Path,
        replacements: &[(String, String)],
    ) -> Result<(), String> {
        let replace_map: std::collections::HashMap<&str, &str> = replacements
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let file = fs::File::open(path)
            .map_err(|e| format!("open zip: {}", e))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| format!("read zip: {}", e))?;

        let tmp_path = path.with_extension("bashm_zip_tmp");
        let tmp_file = fs::File::create(&tmp_path)
            .map_err(|e| format!("create tmp zip: {}", e))?;
        let mut writer = ZipWriter::new(tmp_file);

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| format!("zip entry {}: {}", i, e))?;

            let name = entry.name().to_string();
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(entry.compression());

            writer.start_file(&name, options)
                .map_err(|e| format!("write zip entry {}: {}", name, e))?;

            if let Some(&new_content) = replace_map.get(name.as_str()) {
                writer.write_all(new_content.as_bytes())
                    .map_err(|e| format!("write entry content: {}", e))?;
            } else {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)
                    .map_err(|e| format!("read entry: {}", e))?;
                writer.write_all(&buf)
                    .map_err(|e| format!("write entry: {}", e))?;
            }
        }

        writer.finish()
            .map_err(|e| format!("finalize zip: {}", e))?;

        fs::rename(&tmp_path, path)
            .map_err(|e| format!("rename zip: {}", e))?;
        Ok(())
    }
}

impl Adapter for ZipAdapter {
    fn name(&self) -> &'static str { "zip" }
    fn extensions(&self) -> &'static [&'static str] { ZIP_EXTENSIONS }
    fn category(&self) -> FileCategory { FileCategory::Archive }
    fn can_write(&self) -> bool { true }

    fn probe(&self, _path: &Path, first_bytes: &[u8]) -> bool {
        first_bytes.len() >= 4 && first_bytes[..2] == [0x50, 0x4B]
    }

    fn read_content(&self, path: &Path) -> Result<String, String> {
        let entries = Self::read_entries(path)?;
        Ok(entries.into_iter().map(|(_, content)| content).collect::<Vec<_>>().join("\n"))
    }

    fn write_back(&self, _path: &Path, _content: &str) -> Result<(), String> {
        Err("use ZipAdapter::rewrite_zip() for zip write-back".into())
    }
}
