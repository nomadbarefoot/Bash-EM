use crate::adapter::{Adapter, FileCategory};
use engine::fix_content;
use engine::Counts;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const TEXT_EXTENSIONS: &[&str] = &[
    "md",
    "txt",
    "rst",
    "adoc",
    "org",
    "csv",
    "tsv",
    "log",
    "yml",
    "yaml",
    "toml",
    "json",
    "xml",
    "ini",
    "cfg",
    "rs",
    "py",
    "js",
    "ts",
    "tsx",
    "jsx",
    "go",
    "rb",
    "lua",
    "sh",
    "bash",
    "zsh",
    "fish",
    "ps1",
    "bat",
    "cmd",
    "c",
    "cpp",
    "h",
    "hpp",
    "java",
    "kt",
    "swift",
    "cs",
    "html",
    "htm",
    "css",
    "scss",
    "less",
    "astro",
    "svelte",
    "vue",
    "sql",
    "graphql",
    "proto",
    "tf",
    "hcl",
    "env",
    "gitignore",
    "dockerignore",
    "editorconfig",
];

pub struct TextAdapterImpl;

impl TextAdapterImpl {
    fn looks_like_text_bytes(bytes: &[u8]) -> bool {
        !bytes.contains(&0) && std::str::from_utf8(bytes).is_ok()
    }
}

impl Adapter for TextAdapterImpl {
    fn name(&self) -> &'static str {
        "text"
    }
    fn extensions(&self) -> &'static [&'static str] {
        TEXT_EXTENSIONS
    }
    fn category(&self) -> FileCategory {
        FileCategory::Text
    }
    fn can_write(&self) -> bool {
        true
    }

    fn probe(&self, _path: &Path, first_bytes: &[u8]) -> bool {
        Self::looks_like_text_bytes(first_bytes)
    }

    fn read_content(&self, path: &Path) -> Result<String, String> {
        TextAdapter::read(path)
    }

    fn write_back(&self, path: &Path, content: &str) -> Result<(), String> {
        atomic_write(path, content.as_bytes())
    }
}

pub struct TextAdapter;

impl TextAdapter {
    pub fn looks_like_text(bytes: &[u8]) -> bool {
        TextAdapterImpl::looks_like_text_bytes(bytes)
    }

    pub fn read(path: &Path) -> Result<String, String> {
        let bytes = fs::read(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
        if !Self::looks_like_text(&bytes) {
            return Err("binary file".to_string());
        }
        String::from_utf8(bytes).map_err(|_| "invalid utf-8".to_string())
    }

    pub fn apply(path: &Path, preview_cap: usize) -> Result<Counts, String> {
        let content = Self::read(path)?;
        let result = fix_content(&content, preview_cap);
        if result.counts.total() == 0 {
            return Ok(result.counts);
        }
        atomic_write(path, result.new_content.as_bytes())?;
        Ok(result.counts)
    }
}

fn temp_path(path: &Path) -> PathBuf {
    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(
        ".bashm.tmp.{}.{}",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    PathBuf::from(name)
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let permissions = fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());
    let tmp = temp_path(path);

    let result = (|| {
        fs::write(&tmp, bytes).map_err(|e| format!("write tmp: {e}"))?;
        if let Some(permissions) = permissions {
            fs::set_permissions(&tmp, permissions)
                .map_err(|e| format!("preserve permissions: {e}"))?;
        }
        fs::rename(&tmp, path).map_err(|e| format!("rename: {e}"))
    })();

    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}
