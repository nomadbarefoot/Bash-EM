use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub run_id: String,
    pub root: PathBuf,
    pub timestamp: String,
    pub profile_name: String,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub relative_path: String,
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct ManifestSummary {
    pub run_id: String,
    pub timestamp: String,
    pub root: PathBuf,
    pub file_count: usize,
}

pub fn new_run_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn now_timestamp() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", d.as_secs())
}

pub fn snapshot_file(
    run_dir: &Path,
    file_path: &Path,
    root: &Path,
) -> Result<FileEntry, String> {
    let bytes = fs::read(file_path)
        .map_err(|e| format!("read {}: {}", file_path.display(), e))?;
    let hash = hash_bytes(&bytes);

    let files_dir = run_dir.join("files");
    let dest = files_dir.join(&hash);
    if !dest.exists() {
        fs::create_dir_all(&files_dir)
            .map_err(|e| format!("create backup dir: {}", e))?;
        fs::write(&dest, &bytes)
            .map_err(|e| format!("write backup: {}", e))?;
    }

    let relative = file_path.strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| file_path.to_string_lossy().to_string());

    Ok(FileEntry {
        relative_path: relative,
        hash,
        size: bytes.len() as u64,
    })
}

pub fn seal_manifest(run_dir: &Path, manifest: &Manifest) -> Result<(), String> {
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("serialize manifest: {}", e))?;
    fs::write(run_dir.join("manifest.json"), json)
        .map_err(|e| format!("write manifest: {}", e))
}

pub fn load_manifest(run_dir: &Path) -> Result<Manifest, String> {
    let content = fs::read_to_string(run_dir.join("manifest.json"))
        .map_err(|e| format!("read manifest: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("parse manifest: {}", e))
}

pub fn restore(backup_dir: &Path, run_id: &str) -> Result<usize, String> {
    let run_dir = backup_dir.join(run_id);
    let manifest = load_manifest(&run_dir)?;
    let mut restored = 0;

    for entry in &manifest.files {
        let src = run_dir.join("files").join(&entry.hash);
        let dest = manifest.root.join(&entry.relative_path);

        if !src.exists() {
            return Err(format!("backup file missing: {}", entry.hash));
        }

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("create dir {}: {}", parent.display(), e))?;
        }

        fs::copy(&src, &dest)
            .map_err(|e| format!("restore {}: {}", entry.relative_path, e))?;
        restored += 1;
    }
    Ok(restored)
}

pub fn list_runs(backup_dir: &Path) -> Result<Vec<ManifestSummary>, String> {
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut runs = Vec::new();
    let entries = fs::read_dir(backup_dir)
        .map_err(|e| format!("read backup dir: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Ok(manifest) = load_manifest(&path) {
                runs.push(ManifestSummary {
                    run_id: manifest.run_id,
                    timestamp: manifest.timestamp,
                    root: manifest.root,
                    file_count: manifest.files.len(),
                });
            }
        }
    }
    runs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(runs)
}

pub fn create_run(backup_dir: &Path, run_id: &str) -> Result<PathBuf, String> {
    let run_dir = backup_dir.join(run_id);
    fs::create_dir_all(&run_dir)
        .map_err(|e| format!("create run dir: {}", e))?;
    Ok(run_dir)
}

pub fn begin_run(
    backup_dir: &Path,
    root: &Path,
    profile_name: &str,
) -> Result<(PathBuf, String, Manifest), String> {
    let run_id = new_run_id();
    let run_dir = create_run(backup_dir, &run_id)?;
    let manifest = Manifest {
        run_id: run_id.clone(),
        root: root.to_path_buf(),
        timestamp: now_timestamp(),
        profile_name: profile_name.to_string(),
        files: Vec::new(),
    };
    Ok((run_dir, run_id, manifest))
}

pub fn prune_old_runs(backup_dir: &Path, keep_last_n: usize) -> Result<usize, String> {
    if keep_last_n == 0 || !backup_dir.exists() {
        return Ok(0);
    }
    let mut runs = list_runs(backup_dir)?;
    if runs.len() <= keep_last_n {
        return Ok(0);
    }
    runs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let to_remove = &runs[keep_last_n..];
    let mut removed = 0;
    for run in to_remove {
        let run_dir = backup_dir.join(&run.run_id);
        if fs::remove_dir_all(&run_dir).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}
