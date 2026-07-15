use crate::boilerplate::BoilerplateReport;
use crate::replacer::Counts;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CorruptionBreakdown {
    pub em_dash: usize,
    pub en_dash: usize,
    pub horizontal_bar: usize,
    pub html_entities: usize,
    pub curly_quotes: usize,
    pub ellipsis: usize,
    pub zero_width: usize,
    pub llm_flags: usize,
}

impl CorruptionBreakdown {
    pub fn from_counts(counts: &Counts) -> Self {
        Self {
            em_dash: counts.em,
            en_dash: counts.en,
            horizontal_bar: counts.bar,
            html_entities: counts.entities,
            curly_quotes: counts.curly_quotes,
            ellipsis: counts.ellipsis,
            zero_width: counts.zero_width,
            llm_flags: 0,
        }
    }

    pub fn add_counts(&mut self, counts: &Counts) {
        self.em_dash += counts.em;
        self.en_dash += counts.en;
        self.horizontal_bar += counts.bar;
        self.html_entities += counts.entities;
        self.curly_quotes += counts.curly_quotes;
        self.ellipsis += counts.ellipsis;
        self.zero_width += counts.zero_width;
    }

    pub fn add_boilerplate(&mut self, report: &BoilerplateReport) {
        self.llm_flags += report.matches.len();
    }

    pub fn total(&self) -> usize {
        self.em_dash
            + self.en_dash
            + self.horizontal_bar
            + self.html_entities
            + self.curly_quotes
            + self.ellipsis
            + self.zero_width
            + self.llm_flags
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CategoryStats {
    pub file_count: usize,
    pub artifact_count: usize,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HealthReport {
    pub root: PathBuf,
    pub scanned: usize,
    pub skipped: usize,
    pub dirty_files: usize,
    pub corruption: CorruptionBreakdown,
    pub by_category: HashMap<String, CategoryStats>,
    pub top_files: Vec<(PathBuf, usize)>,
    pub score: u8,
}

impl HealthReport {
    pub fn new(root: PathBuf, scanned: usize, skipped: usize) -> Self {
        Self {
            root,
            scanned,
            skipped,
            dirty_files: 0,
            corruption: CorruptionBreakdown::default(),
            by_category: HashMap::new(),
            top_files: Vec::new(),
            score: 0,
        }
    }

    pub fn add_file(&mut self, path: PathBuf, counts: &Counts, category: &str) {
        self.add_file_with_boilerplate(path, counts, category, None);
    }

    pub fn add_file_with_boilerplate(
        &mut self,
        path: PathBuf,
        counts: &Counts,
        category: &str,
        boilerplate: Option<&BoilerplateReport>,
    ) {
        self.corruption.add_counts(counts);
        let llm_flags = boilerplate.map_or(0, |report| report.matches.len());
        if let Some(report) = boilerplate {
            self.corruption.add_boilerplate(report);
        }
        let total = counts.total() + llm_flags;
        if total > 0 {
            self.dirty_files += 1;
            self.top_files.push((path, total));
        }
        let cat = self.by_category.entry(category.to_string()).or_default();
        cat.file_count += 1;
        cat.artifact_count += total;
    }

    pub fn add_boilerplate(&mut self, report: &BoilerplateReport) {
        self.corruption.add_boilerplate(report);
    }

    pub fn finalize(&mut self) {
        self.top_files.sort_by(|a, b| b.1.cmp(&a.1));
        self.top_files.truncate(20);
        self.score = std::cmp::min(
            100,
            (self.dirty_files * 100) / std::cmp::max(self.scanned, 1),
        ) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_report_score_zero() {
        let mut r = HealthReport::new("/tmp".into(), 10, 0);
        r.finalize();
        assert_eq!(r.score, 0);
    }

    #[test]
    fn score_scales_with_density() {
        let mut r = HealthReport::new("/tmp".into(), 100, 0);
        let counts = Counts {
            em: 1,
            ..Default::default()
        };
        for i in 0..50 {
            r.add_file(format!("{i}.md").into(), &counts, "text");
        }
        r.finalize();
        assert_eq!(r.score, 50);
    }

    #[test]
    fn corruption_breakdown_tracks_types() {
        let mut r = HealthReport::new("/tmp".into(), 10, 0);
        let counts = Counts {
            em: 3,
            curly_quotes: 2,
            ..Default::default()
        };
        r.add_file("a.md".into(), &counts, "text");
        r.finalize();
        assert_eq!(r.corruption.em_dash, 3);
        assert_eq!(r.corruption.curly_quotes, 2);
        assert_eq!(r.corruption.total(), 5);
    }

    #[test]
    fn boilerplate_flags_contribute_to_file_health() {
        let mut report = HealthReport::new("/tmp".into(), 1, 0);
        let boilerplate = crate::boilerplate::scan_content("As an AI assistant, I can help.");
        report.add_file_with_boilerplate(
            "a.md".into(),
            &Counts::default(),
            "text",
            Some(&boilerplate),
        );
        report.finalize();

        assert_eq!(report.corruption.llm_flags, 1);
        assert_eq!(report.dirty_files, 1);
        assert_eq!(report.score, 100);
        assert_eq!(report.top_files, vec![(PathBuf::from("a.md"), 1)]);
        assert_eq!(report.by_category["text"].artifact_count, 1);
    }

    #[test]
    fn top_files_sorted_and_truncated() {
        let mut r = HealthReport::new("/tmp".into(), 100, 0);
        for i in 0..30 {
            let counts = Counts {
                em: i,
                ..Default::default()
            };
            r.add_file(format!("{}.md", i).into(), &counts, "text");
        }
        r.finalize();
        assert_eq!(r.top_files.len(), 20);
        assert!(r.top_files[0].1 >= r.top_files[19].1);
    }
}
