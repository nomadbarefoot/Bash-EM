use std::fs;
use std::path::Path;
use engine::Counts;
use engine::fix_content;
use crate::adapter::{Adapter, FileCategory};

const TEXT_EXTENSIONS: &[&str] = &[
    "md", "txt", "rst", "adoc", "org", "csv", "tsv", "log",
    "yml", "yaml", "toml", "json", "xml", "ini", "cfg",
    "rs", "py", "js", "ts", "tsx", "jsx", "go", "rb", "lua",
    "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd",
    "c", "cpp", "h", "hpp", "java", "kt", "swift", "cs",
    "html", "htm", "css", "scss", "less", "astro", "svelte", "vue",
    "sql", "graphql", "proto", "tf", "hcl",
    "env", "gitignore", "dockerignore", "editorconfig",
];

pub struct TextAdapterImpl;

impl TextAdapterImpl {
    fn looks_like_text_bytes(bytes: &[u8]) -> bool {
        !bytes.contains(&0) && std::str::from_utf8(bytes).is_ok()
    }
}

impl Adapter for TextAdapterImpl {
    fn name(&self) -> &'static str { "text" }
    fn extensions(&self) -> &'static [&'static str] { TEXT_EXTENSIONS }
    fn category(&self) -> FileCategory { FileCategory::Text }
    fn can_write(&self) -> bool { true }

    fn probe(&self, _path: &Path, first_bytes: &[u8]) -> bool {
        Self::looks_like_text_bytes(first_bytes)
    }

    fn read_content(&self, path: &Path) -> Result<String, String> {
        TextAdapter::read(path)
    }

    fn write_back(&self, path: &Path, content: &str) -> Result<(), String> {
        let tmp = path.with_extension("bashm_tmp");
        fs::write(&tmp, content)
            .map_err(|e| format!("write tmp: {}", e))?;
        fs::rename(&tmp, path)
            .map_err(|e| format!("rename: {}", e))?;
        Ok(())
    }
}

pub struct TextAdapter;

impl TextAdapter {
    pub fn looks_like_text(bytes: &[u8]) -> bool {
        TextAdapterImpl::looks_like_text_bytes(bytes)
    }

    pub fn read(path: &Path) -> Result<String, String> {
        let bytes = fs::read(path)
            .map_err(|e| format!("read {}: {}", path.display(), e))?;
        if !Self::looks_like_text(&bytes) {
            return Err("binary file".to_string());
        }
        String::from_utf8(bytes)
            .map_err(|_| "invalid utf-8".to_string())
    }

    pub fn apply(path: &Path, preview_cap: usize) -> Result<Counts, String> {
        let content = Self::read(path)?;
        let result = fix_content(&content, preview_cap);
        if result.counts.total() == 0 {
            return Ok(result.counts);
        }
        let tmp = path.with_extension("bashm_tmp");
        fs::write(&tmp, &result.new_content)
            .map_err(|e| format!("write tmp: {}", e))?;
        fs::rename(&tmp, path)
            .map_err(|e| format!("rename: {}", e))?;
        Ok(result.counts)
    }
}
