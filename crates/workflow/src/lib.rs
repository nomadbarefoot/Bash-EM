use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct ScanRequest {
    pub root: PathBuf,
    pub profile: config::Profile,
}

#[derive(Debug, Clone)]
pub enum ScanProgress {
    Walking,
    Discovered { files: usize },
    Processing { completed: usize, total: usize },
}

#[derive(Debug, Clone)]
pub struct PlannedFile {
    pub path: PathBuf,
    pub source_hash: String,
    pub new_content: String,
    pub counts: engine::Counts,
    pub changes: Vec<engine::LineChange>,
    pub lines_changed: usize,
}

#[derive(Debug, Clone)]
pub struct ScanReport {
    pub root: PathBuf,
    pub profile: config::Profile,
    pub stats: adapters::WalkStats,
    pub health: engine::health::HealthReport,
    pub files: Vec<PlannedFile>,
    pub totals: engine::Counts,
    pub scan_ms: u128,
}

#[derive(Debug, Clone)]
pub struct ApplyReport {
    pub backup_dir: PathBuf,
    pub run_id: Option<String>,
    pub applied_files: usize,
    pub applied_counts: engine::Counts,
    pub errors: Vec<String>,
    pub pruned_runs: usize,
}

impl ApplyReport {
    pub fn partial(&self) -> bool {
        !self.errors.is_empty() && self.applied_files > 0
    }
}

#[derive(Debug, Clone)]
pub struct RestoreReport {
    pub backup_dir: PathBuf,
    pub run_id: String,
    pub restored_files: usize,
}

pub fn scan<F>(
    request: ScanRequest,
    mut progress: F,
    cancelled: &AtomicBool,
) -> Result<ScanReport, String>
where
    F: FnMut(ScanProgress),
{
    let started = Instant::now();
    let root = request
        .root
        .canonicalize()
        .map_err(|error| format!("open {}: {error}", request.root.display()))?;
    if !root.is_dir() {
        return Err(format!("not a directory: {}", root.display()));
    }

    progress(ScanProgress::Walking);
    let (candidates, stats) = adapters::walk_tree_with_ignores_cancelled(
        &root,
        &request.profile.prefs,
        &request.profile.ignore,
        cancelled,
    )?;
    if cancelled.load(Ordering::Relaxed) {
        return Err("scan cancelled".to_string());
    }
    progress(ScanProgress::Discovered {
        files: candidates.len(),
    });

    let options = build_fix_options(&request.profile);
    let pipeline = engine::Pipeline::with_options(
        request.profile.prefs.preview_lines,
        options,
        request.profile.prefs.fence_guard,
    );
    let mut health = engine::health::HealthReport::new(root.clone(), stats.scanned, stats.skipped);
    let mut files = Vec::new();
    let mut totals = engine::Counts::default();
    let total = candidates.len();
    let llm_enabled = config::is_rule_enabled(&request.profile, "llm_boilerplate");

    for (index, candidate) in candidates.into_iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            return Err("scan cancelled".to_string());
        }

        let source_hash = hash_bytes(candidate.content.as_bytes());
        let edits = pipeline.process_content(candidate.path.clone(), &candidate.content);
        let counts = edits.as_ref().map(|edits| edits.counts).unwrap_or_default();
        let boilerplate =
            llm_enabled.then(|| engine::boilerplate::scan_content(&candidate.content));
        health.add_file_with_boilerplate(
            candidate.path.clone(),
            &counts,
            categorize(&candidate.path),
            boilerplate.as_ref(),
        );

        if let Some(edits) = edits {
            totals.add(edits.counts);
            files.push(PlannedFile {
                path: edits.path,
                source_hash,
                new_content: edits.new_content,
                counts: edits.counts,
                changes: edits.changes,
                lines_changed: edits.lines_changed,
            });
        }
        progress(ScanProgress::Processing {
            completed: index + 1,
            total,
        });
    }

    files.sort_by(|left, right| {
        right
            .counts
            .total()
            .cmp(&left.counts.total())
            .then_with(|| left.path.cmp(&right.path))
    });
    health.finalize();

    Ok(ScanReport {
        root,
        profile: request.profile,
        stats,
        health,
        files,
        totals,
        scan_ms: started.elapsed().as_millis().max(1),
    })
}

pub fn apply(report: &ScanReport, included: &HashSet<PathBuf>) -> Result<ApplyReport, String> {
    let backup_dir = config::resolve_backup_dir(&report.profile.prefs);
    let selected: Vec<&PlannedFile> = report
        .files
        .iter()
        .filter(|file| included.contains(&file.path))
        .collect();
    if selected.is_empty() {
        return Ok(ApplyReport {
            backup_dir,
            run_id: None,
            applied_files: 0,
            applied_counts: engine::Counts::default(),
            errors: Vec::new(),
            pruned_runs: 0,
        });
    }

    for file in &selected {
        validate_source(&report.root, file)?;
    }

    let (run_dir, run_id, mut manifest) =
        backup::begin_run(&backup_dir, &report.root, &report.profile.name)?;
    for file in &selected {
        match backup::snapshot_file(&run_dir, &file.path, &report.root) {
            Ok(entry) => manifest.files.push(entry),
            Err(error) => {
                let _ = fs::remove_dir_all(&run_dir);
                return Err(format!("snapshot {}: {error}", file.path.display()));
            }
        }
    }
    if let Err(error) = backup::seal_manifest(&run_dir, &manifest) {
        let _ = fs::remove_dir_all(&run_dir);
        return Err(format!("seal backup manifest: {error}"));
    }

    let mut applied_files = 0;
    let mut applied_counts = engine::Counts::default();
    let mut errors = Vec::new();
    for file in selected {
        match adapters::atomic_write(&file.path, file.new_content.as_bytes()) {
            Ok(()) => {
                applied_files += 1;
                applied_counts.add(file.counts);
            }
            Err(error) => {
                errors.push(format!("write {}: {error}", file.path.display()));
                break;
            }
        }
    }

    let pruned_runs =
        backup::prune_old_runs(&backup_dir, report.profile.prefs.keep_last_n).unwrap_or_default();
    Ok(ApplyReport {
        backup_dir,
        run_id: Some(run_id),
        applied_files,
        applied_counts,
        errors,
        pruned_runs,
    })
}

pub fn list_backups(profile: &config::Profile) -> Result<Vec<backup::ManifestSummary>, String> {
    backup::list_runs(&config::resolve_backup_dir(&profile.prefs))
}

pub fn restore(profile: &config::Profile, run_id: &str) -> Result<RestoreReport, String> {
    let backup_dir = config::resolve_backup_dir(&profile.prefs);
    let restored_files = backup::restore(&backup_dir, run_id)?;
    Ok(RestoreReport {
        backup_dir,
        run_id: run_id.to_string(),
        restored_files,
    })
}

pub fn included_paths(report: &ScanReport) -> HashSet<PathBuf> {
    report.files.iter().map(|file| file.path.clone()).collect()
}

fn build_fix_options(profile: &config::Profile) -> engine::FixOptions {
    let pairs: Vec<(String, bool)> = profile
        .rules
        .iter()
        .map(|(name, rule)| (name.clone(), rule.enabled))
        .collect();
    engine::FixOptions::from_profile(&pairs)
}

fn validate_source(root: &Path, file: &PlannedFile) -> Result<(), String> {
    file.path
        .strip_prefix(root)
        .map_err(|_| format!("file outside scan root: {}", file.path.display()))?;
    let metadata = fs::symlink_metadata(&file.path)
        .map_err(|error| format!("inspect {}: {error}", file.path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(format!(
            "file type changed since scan: {}",
            file.path.display()
        ));
    }
    let canonical = file
        .path
        .canonicalize()
        .map_err(|error| format!("resolve {}: {error}", file.path.display()))?;
    if canonical != file.path || !canonical.starts_with(root) {
        return Err(format!(
            "file path changed since scan: {}",
            file.path.display()
        ));
    }
    let bytes =
        fs::read(&file.path).map_err(|error| format!("read {}: {error}", file.path.display()))?;
    if hash_bytes(&bytes) != file.source_hash {
        return Err(format!(
            "file changed since scan; rescan required: {}",
            file.path.display()
        ));
    }
    Ok(())
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn categorize(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("md" | "txt" | "rst" | "adoc" | "org") => "text",
        Some("rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "rb" | "sh") => "code",
        Some("html" | "htm" | "css" | "scss" | "astro" | "svelte" | "vue") => "web",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tempdir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("bash-em-workflow-{name}-{nonce}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn profile(backup_dir: &Path) -> config::Profile {
        let mut profile = config::default_profile();
        profile.prefs.backup_dir = backup_dir.display().to_string();
        profile
    }

    #[test]
    fn profile_specific_scan_apply_and_restore_round_trip() {
        let temp = tempdir("roundtrip");
        let root = temp.join("data");
        let vault = temp.join("vault");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("note.md");
        fs::write(&file, "say \u{201c}hello\u{201d}").unwrap();
        let mut profile = profile(&vault);
        profile.rules.get_mut("curly_quotes").unwrap().enabled = true;

        let report = scan(
            ScanRequest {
                root: root.clone(),
                profile: profile.clone(),
            },
            |_| {},
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].counts.curly_quotes, 2);

        let applied = apply(&report, &included_paths(&report)).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "say \"hello\"");
        let run_id = applied.run_id.unwrap();
        restore(&profile, &run_id).unwrap();
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            "say \u{201c}hello\u{201d}"
        );
        assert!(vault.join(run_id).exists());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn changed_file_aborts_before_backup_or_write() {
        let temp = tempdir("stale");
        let root = temp.join("data");
        let vault = temp.join("vault");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("note.md");
        fs::write(&file, "a\u{2014}b").unwrap();
        let report = scan(
            ScanRequest {
                root: root.clone(),
                profile: profile(&vault),
            },
            |_| {},
            &AtomicBool::new(false),
        )
        .unwrap();
        fs::write(&file, "changed elsewhere").unwrap();
        let error = apply(&report, &included_paths(&report)).unwrap_err();
        assert!(error.contains("changed since scan"));
        assert_eq!(fs::read_to_string(&file).unwrap(), "changed elsewhere");
        assert!(!vault.exists());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn profile_ignore_globs_are_enforced() {
        let temp = tempdir("ignore");
        let root = temp.join("data");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep.md"), "a\u{2014}b").unwrap();
        fs::write(root.join("skip.min.js"), "a\u{2014}b").unwrap();
        let report = scan(
            ScanRequest {
                root: root.clone(),
                profile: profile(&temp.join("vault")),
            },
            |_| {},
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(report.stats.ignored, 1);
        assert_eq!(report.files.len(), 1);
        assert!(report.files[0].path.ends_with("keep.md"));
        let _ = fs::remove_dir_all(temp);
    }

    #[cfg(unix)]
    #[test]
    fn apply_and_restore_preserve_executable_mode() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir("mode");
        let root = temp.join("data");
        let vault = temp.join("vault");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("script.sh");
        fs::write(&file, "echo a\u{2014}b\n").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
        let profile = profile(&vault);
        let report = scan(
            ScanRequest {
                root: root.clone(),
                profile: profile.clone(),
            },
            |_| {},
            &AtomicBool::new(false),
        )
        .unwrap();
        let applied = apply(&report, &included_paths(&report)).unwrap();
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o755
        );
        restore(&profile, applied.run_id.as_deref().unwrap()).unwrap();
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o755
        );
        let _ = fs::remove_dir_all(temp);
    }
}
