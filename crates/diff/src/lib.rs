use std::path::PathBuf;
use engine::Counts;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineHunk {
    pub line_no: usize,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: PathBuf,
    pub hunks: Vec<LineHunk>,
    pub counts: Counts,
    pub lines_changed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunReport {
    pub run_id: String,
    pub timestamp: String,
    pub root: PathBuf,
    pub profile: String,
    pub files: Vec<FileDiff>,
    pub totals: Counts,
}

pub fn build_diff(before: &str, after: &str) -> Vec<LineHunk> {
    let before_lines: Vec<&str> = before.split('\n').collect();
    let after_lines: Vec<&str> = after.split('\n').collect();
    let mut hunks = Vec::new();

    let max = before_lines.len().max(after_lines.len());
    for i in 0..max {
        let b = before_lines.get(i).copied().unwrap_or("");
        let a = after_lines.get(i).copied().unwrap_or("");
        if b != a {
            hunks.push(LineHunk {
                line_no: i + 1,
                before: b.to_string(),
                after: a.to_string(),
            });
        }
    }
    hunks
}

impl RunReport {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_content_no_hunks() {
        let hunks = build_diff("hello\nworld", "hello\nworld");
        assert!(hunks.is_empty());
    }

    #[test]
    fn single_line_change() {
        let hunks = build_diff("a\u{2014}b\nplain", "a - b\nplain");
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].line_no, 1);
    }

    #[test]
    fn multiple_changes() {
        let hunks = build_diff("a\u{2014}b\nc\u{2013}d\nok", "a - b\nc-d\nok");
        assert_eq!(hunks.len(), 2);
    }
}
