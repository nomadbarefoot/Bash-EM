use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FileCategory {
    Text,
    Code,
    Web,
    Office,
    Docs,
    Pdf,
    Archive,
}

impl FileCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Code => "code",
            Self::Web => "web",
            Self::Office => "office",
            Self::Docs => "docs",
            Self::Pdf => "pdf",
            Self::Archive => "archive",
        }
    }
}

impl std::fmt::Display for FileCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

pub trait Adapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn category(&self) -> FileCategory;
    fn can_write(&self) -> bool;
    fn probe(&self, path: &Path, first_bytes: &[u8]) -> bool;
    fn read_content(&self, path: &Path) -> Result<String, String>;
    fn write_back(&self, path: &Path, content: &str) -> Result<(), String>;
}
