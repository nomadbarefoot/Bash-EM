use adapters::{walk_tree, Registry, TextAdapter};
use config::{default_profile, load_profile, Prefs};
use diff::build_diff;
use engine::{
    fix_content, fix_content_with_options, fix_line, fix_line_with_options, Counts, FixOptions,
    Pipeline,
};
use std::fs;
use std::path::Path;

// ── engine: dashes ──

#[test]
fn fix_line_em_dash() {
    let (out, c) = fix_line("word\u{2014}word");
    assert_eq!(out, "word - word");
    assert_eq!(c.em, 1);
}

#[test]
fn fix_line_en_dash_range() {
    let (out, c) = fix_line("2019\u{2013}2024");
    assert_eq!(out, "2019-2024");
    assert_eq!(c.en, 1);
}

#[test]
fn fix_line_horizontal_bar() {
    let (out, c) = fix_line("stop\u{2015}now");
    assert_eq!(out, "stop - now");
    assert_eq!(c.bar, 1);
}

#[test]
fn fix_line_html_entities() {
    let (out, c) = fix_line("a&mdash;b &#8211;c");
    assert_eq!(out, "a - b -c");
    assert_eq!(c.entities, 2);
}

#[test]
fn fix_line_clean_passthrough() {
    let (out, c) = fix_line("nothing special here");
    assert_eq!(out, "nothing special here");
    assert_eq!(c.total(), 0);
}

#[test]
fn fix_content_crlf_roundtrip() {
    let input = "a\u{2014}b\r\nplain\r\n";
    let r = fix_content(input, 10);
    assert_eq!(r.new_content, "a - b\r\nplain\r\n");
    assert_eq!(r.lines_changed, 1);
}

#[test]
fn fix_content_no_trailing_newline() {
    let r = fix_content("x\u{2014}y", 10);
    assert_eq!(r.new_content, "x - y");
    assert!(!r.new_content.ends_with('\n'));
}

#[test]
fn fix_content_multiline_counts() {
    let input = "a\u{2014}b\nc\u{2013}d\nclean\ne\u{2015}f\n";
    let r = fix_content(input, 10);
    assert_eq!(r.counts.em, 1);
    assert_eq!(r.counts.en, 1);
    assert_eq!(r.counts.bar, 1);
    assert_eq!(r.lines_changed, 3);
}

#[test]
fn pipeline_skips_clean_files() {
    let p = Pipeline::new(8);
    let result = p.process_content("clean.md".into(), "no dashes here\n");
    assert!(result.is_none());
}

#[test]
fn pipeline_batch_aggregates() {
    let p = Pipeline::new(8);
    let batch = p.process_batch(vec![
        ("a.md".into(), "x\u{2014}y".into()),
        ("b.md".into(), "clean".into()),
        ("c.md".into(), "a\u{2013}b".into()),
    ]);
    assert_eq!(batch.files.len(), 2);
    assert_eq!(batch.totals.total(), 2);
}

// ── engine: curly quotes ──

#[test]
fn curly_quotes_when_enabled() {
    let opts = FixOptions {
        curly_quotes: true,
        ..Default::default()
    };
    let (out, c) = fix_line_with_options("\u{201C}hello\u{201D} and \u{2018}world\u{2019}", &opts);
    assert_eq!(out, "\"hello\" and 'world'");
    assert_eq!(c.curly_quotes, 4);
}

#[test]
fn curly_quotes_off_by_default() {
    let (out, c) = fix_line("\u{201C}hello\u{201D}");
    assert_eq!(out, "\u{201C}hello\u{201D}");
    assert_eq!(c.curly_quotes, 0);
}

// ── engine: ellipsis ──

#[test]
fn ellipsis_when_enabled() {
    let opts = FixOptions {
        ellipsis: true,
        ..Default::default()
    };
    let (out, c) = fix_line_with_options("wait\u{2026} what", &opts);
    assert_eq!(out, "wait... what");
    assert_eq!(c.ellipsis, 1);
}

#[test]
fn ellipsis_off_by_default() {
    let (out, c) = fix_line("wait\u{2026}");
    assert_eq!(out, "wait\u{2026}");
    assert_eq!(c.ellipsis, 0);
}

// ── engine: zero-width ──

#[test]
fn zero_width_stripped_by_default() {
    let (out, c) = fix_line("hel\u{200B}lo w\u{FEFF}orld");
    assert_eq!(out, "hello world");
    assert_eq!(c.zero_width, 2);
}

#[test]
fn zero_width_bidi_stripped() {
    let (out, c) = fix_line("abc\u{202A}def\u{202C}ghi");
    assert_eq!(out, "abcdefghi");
    assert_eq!(c.zero_width, 2);
}

// ── engine: fence guard ──

#[test]
fn fence_guard_skips_code_blocks() {
    let content = "outside\u{2014}dash\n```\ninside\u{2014}code\n```\nafter\u{2014}fence\n";
    let r = fix_content_with_options(content, 10, &FixOptions::default(), true);
    assert!(r.new_content.contains("outside - dash"));
    assert!(r.new_content.contains("inside\u{2014}code"));
    assert!(r.new_content.contains("after - fence"));
    assert_eq!(r.counts.em, 2);
}

#[test]
fn fence_guard_tilde_fences() {
    let content = "before\u{2014}a\n~~~\nfenced\u{2014}b\n~~~\nafter\u{2014}c\n";
    let r = fix_content_with_options(content, 10, &FixOptions::default(), true);
    assert!(r.new_content.contains("before - a"));
    assert!(r.new_content.contains("fenced\u{2014}b"));
    assert!(r.new_content.contains("after - c"));
}

#[test]
fn fence_guard_off_processes_everything() {
    let content = "```\nfenced\u{2014}code\n```\n";
    let r = fix_content_with_options(content, 10, &FixOptions::default(), false);
    assert!(r.new_content.contains("fenced - code"));
}

// ── engine: boilerplate ──

#[test]
fn boilerplate_detects_tier1() {
    let report = engine::boilerplate::scan_content("As an AI language model, I can help.");
    assert!(!report.matches.is_empty());
    assert_eq!(
        report.matches[0].confidence,
        engine::boilerplate::Confidence::NearCertain
    );
}

#[test]
fn boilerplate_detects_tier2_phrases() {
    let report = engine::boilerplate::scan_content("It's important to note that this is key.");
    assert!(!report.matches.is_empty());
    assert_eq!(
        report.matches[0].confidence,
        engine::boilerplate::Confidence::High
    );
}

#[test]
fn boilerplate_tell_words_below_threshold() {
    let report = engine::boilerplate::scan_content("We delve into the tapestry.");
    assert!(report.matches.is_empty());
    assert_eq!(report.tell_word_count, 2);
}

#[test]
fn boilerplate_tell_words_above_threshold() {
    let text = "Let us delve into the tapestry of this multifaceted landscape.";
    let report = engine::boilerplate::scan_content(text);
    assert!(report.tell_word_count >= 3);
    assert!(!report.matches.is_empty());
}

#[test]
fn boilerplate_clean_text_no_flags() {
    let report = engine::boilerplate::scan_content("Normal text without any LLM artifacts.");
    assert!(report.matches.is_empty());
}

// ── engine: health model ──

#[test]
fn health_score_zero_when_clean() {
    let mut h = engine::health::HealthReport::new("/tmp".into(), 100, 0);
    h.finalize();
    assert_eq!(h.score, 0);
}

#[test]
fn health_corruption_breakdown() {
    let mut h = engine::health::HealthReport::new("/tmp".into(), 100, 0);
    let counts = Counts {
        em: 5,
        curly_quotes: 3,
        ..Default::default()
    };
    h.add_file("test.md".into(), &counts, "text");
    h.finalize();
    assert_eq!(h.corruption.em_dash, 5);
    assert_eq!(h.corruption.curly_quotes, 3);
    assert_eq!(h.score, 1);
}

#[test]
fn health_report_serializable() {
    let mut h = engine::health::HealthReport::new("/tmp".into(), 10, 0);
    let counts = Counts {
        em: 2,
        ..Default::default()
    };
    h.add_file("a.md".into(), &counts, "text");
    h.finalize();
    let json = serde_json::to_string(&h).unwrap();
    assert!(json.contains("\"em_dash\":2"));
}

// ── config ──

#[test]
fn default_profile_rules() {
    let p = default_profile();
    assert_eq!(p.name, "typographic");
    assert!(config::is_rule_enabled(&p, "em_dash"));
    assert!(!config::is_rule_enabled(&p, "curly_quotes"));
    assert!(!config::is_rule_enabled(&p, "llm_boilerplate"));
    assert!(config::is_rule_enabled(&p, "zero_width"));
}

#[test]
fn profile_yaml_roundtrip() {
    let p = default_profile();
    let yaml = serde_yaml::to_string(&p).unwrap();
    let loaded: config::Profile = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(loaded.name, p.name);
}

#[test]
fn invalid_profile_yaml_rejected() {
    let result: Result<config::Profile, _> = serde_yaml::from_str("{{bad");
    assert!(result.is_err());
}

#[test]
fn load_profile_missing_file() {
    let result = load_profile(Path::new("/nonexistent/profile.yaml"));
    assert!(result.is_err());
}

#[test]
fn config_has_keep_last_n() {
    let p = default_profile();
    assert_eq!(p.prefs.keep_last_n, 10);
}

#[test]
fn config_fence_guard_on_by_default() {
    let p = default_profile();
    assert!(p.prefs.fence_guard);
}

// ── diff ──

#[test]
fn diff_identical_no_hunks() {
    let hunks = build_diff("hello\nworld", "hello\nworld");
    assert!(hunks.is_empty());
}

#[test]
fn diff_single_change() {
    let hunks = build_diff("a\u{2014}b\nplain", "a - b\nplain");
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].line_no, 1);
}

#[test]
fn diff_report_json_serializable() {
    let report = diff::RunReport {
        run_id: "test".into(),
        timestamp: "0".into(),
        root: "/tmp".into(),
        profile: "typographic".into(),
        files: vec![],
        totals: Counts::default(),
    };
    let json = report.to_json().unwrap();
    assert!(json.contains("\"run_id\""));
}

// ── adapters ──

#[test]
fn text_adapter_rejects_binary() {
    let bytes: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x00];
    assert!(!TextAdapter::looks_like_text(bytes));
}

#[test]
fn text_adapter_accepts_utf8() {
    assert!(TextAdapter::looks_like_text(b"hello world"));
    assert!(TextAdapter::looks_like_text(
        "utf8 with em\u{2014}dash".as_bytes()
    ));
}

#[test]
fn walker_skips_dotdirs() {
    let dir = tempdir("walker_dotdir");
    fs::create_dir_all(dir.join(".hidden")).unwrap();
    fs::write(dir.join(".hidden/secret.md"), "x\u{2014}y").unwrap();
    fs::write(dir.join("visible.md"), "a\u{2014}b").unwrap();

    let prefs = Prefs::default();
    let (candidates, _) = walk_tree(&dir, &prefs).unwrap();
    assert_eq!(candidates.len(), 1);
    assert!(candidates[0].path.ends_with("visible.md"));
    cleanup(&dir);
}

#[test]
fn walker_skips_large_files() {
    let dir = tempdir("walker_large");
    let big = "x".repeat(11 * 1024 * 1024);
    fs::write(dir.join("big.txt"), &big).unwrap();
    fs::write(dir.join("small.txt"), "a\u{2014}b").unwrap();

    let prefs = Prefs::default();
    let (candidates, stats) = walk_tree(&dir, &prefs).unwrap();
    assert_eq!(candidates.len(), 1);
    assert!(stats.skipped >= 1);
    cleanup(&dir);
}

// ── registry ──

#[test]
fn registry_resolves_text_by_extension() {
    let reg = Registry::default();
    let adapter = reg.resolve(Path::new("test.md"), b"hello");
    assert!(adapter.is_some());
    assert_eq!(adapter.unwrap().name(), "text");
}

#[test]
fn registry_resolves_xlsx() {
    let reg = Registry::experimental();
    let adapter = reg.resolve(Path::new("data.xlsx"), &[0x50, 0x4B, 0x03, 0x04]);
    assert!(adapter.is_some());
    assert_eq!(adapter.unwrap().name(), "xlsx");
}

#[test]
fn registry_resolves_docx() {
    let reg = Registry::experimental();
    let adapter = reg.resolve(Path::new("doc.docx"), &[0x50, 0x4B, 0x03, 0x04]);
    assert!(adapter.is_some());
    assert_eq!(adapter.unwrap().name(), "docx");
}

#[test]
fn registry_resolves_pdf() {
    let reg = Registry::experimental();
    let adapter = reg.resolve(Path::new("report.pdf"), b"%PDF-1.4");
    assert!(adapter.is_some());
    assert_eq!(adapter.unwrap().name(), "pdf");
    assert!(!adapter.unwrap().can_write());
}

#[test]
fn registry_resolves_zip() {
    let reg = Registry::experimental();
    let adapter = reg.resolve(Path::new("archive.zip"), &[0x50, 0x4B, 0x03, 0x04]);
    assert!(adapter.is_some());
    assert_eq!(adapter.unwrap().name(), "zip");
}

#[test]
fn registry_lists_all_adapters() {
    let reg = Registry::experimental();
    let list = reg.list();
    assert!(list.len() >= 5);
    let names: Vec<&str> = list.iter().map(|(n, _, _, _)| *n).collect();
    assert!(names.contains(&"text"));
    assert!(names.contains(&"xlsx"));
    assert!(names.contains(&"docx"));
    assert!(names.contains(&"pdf"));
    assert!(names.contains(&"zip"));
}

// ── backup + restore cycle ──

#[test]
fn backup_restore_roundtrip() {
    let dir = tempdir("backup_rt");
    let backup_dir = dir.join("backups");
    let data_dir = dir.join("data");
    fs::create_dir_all(&data_dir).unwrap();

    let original = "hello \u{2014} world\r\nrange 1\u{2013}10\r\n";
    fs::write(data_dir.join("test.md"), original).unwrap();

    let (run_dir, run_id, mut manifest) =
        backup::begin_run(&backup_dir, &data_dir, "typographic").unwrap();

    let entry = backup::snapshot_file(&run_dir, &data_dir.join("test.md"), &data_dir).unwrap();
    manifest.files.push(entry);

    TextAdapter::apply(&data_dir.join("test.md"), 0).unwrap();
    let after = fs::read_to_string(data_dir.join("test.md")).unwrap();
    assert_ne!(after, original);
    assert!(after.contains(" - "));

    backup::seal_manifest(&run_dir, &manifest).unwrap();

    let restored = backup::restore(&backup_dir, &run_id).unwrap();
    assert_eq!(restored, 1);

    let final_content = fs::read(data_dir.join("test.md")).unwrap();
    assert_eq!(final_content, original.as_bytes());

    cleanup(&dir);
}

#[test]
fn backup_list_runs() {
    let dir = tempdir("backup_list");
    let data_dir = dir.join("data");
    fs::create_dir_all(&data_dir).unwrap();
    fs::write(data_dir.join("f.md"), "a\u{2014}b").unwrap();

    let (run_dir, _run_id, manifest) = backup::begin_run(&dir, &data_dir, "typographic").unwrap();
    backup::seal_manifest(&run_dir, &manifest).unwrap();

    let runs = backup::list_runs(&dir).unwrap();
    assert_eq!(runs.len(), 1);

    cleanup(&dir);
}

#[test]
fn backup_prune_keeps_n() {
    let dir = tempdir("backup_prune");
    let data_dir = dir.join("data");
    fs::create_dir_all(&data_dir).unwrap();
    fs::write(data_dir.join("f.md"), "a\u{2014}b").unwrap();

    for _ in 0..5 {
        let (run_dir, _id, manifest) = backup::begin_run(&dir, &data_dir, "typographic").unwrap();
        backup::seal_manifest(&run_dir, &manifest).unwrap();
    }

    let pruned = backup::prune_old_runs(&dir, 2).unwrap();
    assert_eq!(pruned, 3);

    let remaining = backup::list_runs(&dir).unwrap();
    assert_eq!(remaining.len(), 2);

    cleanup(&dir);
}

// ── fix options from profile ──

#[test]
fn fix_options_from_profile_mapping() {
    let pairs = vec![
        ("em_dash".into(), true),
        ("curly_quotes".into(), true),
        ("zero_width".into(), false),
    ];
    let opts = FixOptions::from_profile(&pairs);
    assert!(opts.em_dash);
    assert!(opts.curly_quotes);
    assert!(!opts.zero_width);
}

// ── helpers ──

fn tempdir(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("bash-em-test-{}-{}", name, std::process::id()));
    fs::create_dir_all(&p).unwrap();
    p
}

fn cleanup(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}
