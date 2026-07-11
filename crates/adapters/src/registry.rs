use std::collections::HashMap;
use std::path::Path;
use crate::adapter::{Adapter, FileCategory};

pub struct Registry {
    by_ext: HashMap<&'static str, usize>,
    adapters: Vec<Box<dyn Adapter>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            by_ext: HashMap::new(),
            adapters: Vec::new(),
        }
    }

    pub fn register(&mut self, adapter: Box<dyn Adapter>) {
        let idx = self.adapters.len();
        for ext in adapter.extensions() {
            self.by_ext.insert(ext, idx);
        }
        self.adapters.push(adapter);
    }

    pub fn resolve(&self, path: &Path, first_bytes: &[u8]) -> Option<&dyn Adapter> {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_ascii_lowercase();
            if let Some(&idx) = self.by_ext.get(ext_lower.as_str()) {
                return Some(self.adapters[idx].as_ref());
            }
        }
        for adapter in &self.adapters {
            if adapter.probe(path, first_bytes) {
                return Some(adapter.as_ref());
            }
        }
        None
    }

    pub fn list(&self) -> Vec<(&'static str, &'static [&'static str], FileCategory, bool)> {
        self.adapters.iter().map(|a| {
            (a.name(), a.extensions(), a.category(), a.can_write())
        }).collect()
    }
}

impl Default for Registry {
    fn default() -> Self {
        let mut reg = Self::new();
        reg.register(Box::new(crate::text::TextAdapterImpl));
        reg.register(Box::new(crate::zip_adapter::ZipAdapter));
        reg.register(Box::new(crate::xlsx::XlsxAdapter));
        reg.register(Box::new(crate::docx::DocxAdapter));
        reg.register(Box::new(crate::pdf::PdfAdapter));
        reg
    }
}
