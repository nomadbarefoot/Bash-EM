use crate::replacer::{fix_content_with_options, Counts, FixOptions, LineChange};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TextUnit {
    pub id: usize,
    pub text: String,
    pub meta: TextUnitMeta,
}

#[derive(Debug, Clone)]
pub struct TextUnitMeta {
    pub path: PathBuf,
    pub line_offset: usize,
}

pub struct FileEdits {
    pub path: PathBuf,
    pub new_content: String,
    pub counts: Counts,
    pub changes: Vec<LineChange>,
    pub lines_changed: usize,
}

pub struct BatchEdits {
    pub files: Vec<FileEdits>,
    pub totals: Counts,
}

pub struct Pipeline {
    pub preview_cap: usize,
    pub options: FixOptions,
    pub fence_guard: bool,
}

impl Pipeline {
    pub fn new(preview_cap: usize) -> Self {
        Self {
            preview_cap,
            options: FixOptions::default(),
            fence_guard: false,
        }
    }

    pub fn with_options(preview_cap: usize, options: FixOptions, fence_guard: bool) -> Self {
        Self {
            preview_cap,
            options,
            fence_guard,
        }
    }

    fn should_fence_guard(&self, path: &std::path::Path) -> bool {
        if !self.fence_guard {
            return false;
        }
        match path.extension().and_then(|e| e.to_str()) {
            Some("md" | "mdx" | "astro" | "markdown") => true,
            _ => false,
        }
    }

    pub fn process_content(&self, path: PathBuf, content: &str) -> Option<FileEdits> {
        let fence = self.should_fence_guard(&path);
        let result = fix_content_with_options(content, self.preview_cap, &self.options, fence);
        if result.counts.total() == 0 {
            return None;
        }
        Some(FileEdits {
            path,
            new_content: result.new_content,
            counts: result.counts,
            changes: result.changes,
            lines_changed: result.lines_changed,
        })
    }

    pub fn process_batch(&self, files: Vec<(PathBuf, String)>) -> BatchEdits {
        let mut totals = Counts::default();
        let mut edits = Vec::new();
        for (path, content) in files {
            if let Some(file_edits) = self.process_content(path, &content) {
                totals.add(file_edits.counts);
                edits.push(file_edits);
            }
        }
        BatchEdits {
            files: edits,
            totals,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_skips_clean() {
        let p = Pipeline::new(8);
        let result = p.process_content("clean.md".into(), "no dashes here");
        assert!(result.is_none());
    }

    #[test]
    fn pipeline_finds_dirty() {
        let p = Pipeline::new(8);
        let result = p.process_content("dirty.md".into(), "word\u{2014}word");
        let edits = result.unwrap();
        assert_eq!(edits.counts.em, 1);
        assert_eq!(edits.new_content, "word - word");
    }

    #[test]
    fn batch_aggregates() {
        let p = Pipeline::new(8);
        let batch = p.process_batch(vec![
            ("a.md".into(), "x\u{2014}y".to_string()),
            ("b.md".into(), "clean".to_string()),
            ("c.md".into(), "a\u{2013}b".to_string()),
        ]);
        assert_eq!(batch.files.len(), 2);
        assert_eq!(batch.totals.em, 1);
        assert_eq!(batch.totals.en, 1);
    }

    #[test]
    fn fence_guard_skips_code_blocks() {
        let opts = FixOptions::default();
        let p = Pipeline::with_options(8, opts, true);
        let content = "before\u{2014}after\n```\ninside\u{2014}fence\n```\noutside\u{2014}fence\n";
        let edits = p.process_content("test.md".into(), content).unwrap();
        assert_eq!(edits.counts.em, 2);
        assert!(edits.new_content.contains("inside\u{2014}fence"));
        assert!(edits.new_content.contains("outside - fence"));
    }

    #[test]
    fn fence_guard_inactive_for_non_markdown() {
        let opts = FixOptions::default();
        let p = Pipeline::with_options(8, opts, true);
        let content = "```\ninside\u{2014}fence\n```\n";
        let edits = p.process_content("test.rs".into(), content).unwrap();
        assert_eq!(edits.counts.em, 1);
    }
}
