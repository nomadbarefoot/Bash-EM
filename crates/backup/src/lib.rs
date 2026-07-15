use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ManifestSummary {
    pub run_id: String,
    pub timestamp: String,
    pub root: PathBuf,
    pub file_count: usize,
    pub profile_name: String,
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

pub fn snapshot_file(run_dir: &Path, file_path: &Path, root: &Path) -> Result<FileEntry, String> {
    let bytes = fs::read(file_path).map_err(|e| format!("read {}: {}", file_path.display(), e))?;
    let hash = hash_bytes(&bytes);

    let files_dir = run_dir.join("files");
    let dest = files_dir.join(&hash);
    if !dest.exists() {
        fs::create_dir_all(&files_dir).map_err(|e| format!("create backup dir: {}", e))?;
        fs::write(&dest, &bytes).map_err(|e| format!("write backup: {}", e))?;
    }

    let relative = file_path
        .strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| file_path.to_string_lossy().to_string());

    Ok(FileEntry {
        relative_path: relative,
        hash,
        size: bytes.len() as u64,
        mode: file_mode(file_path),
    })
}

#[cfg(unix)]
fn file_mode(path: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions().mode())
}

#[cfg(not(unix))]
fn file_mode(_path: &Path) -> Option<u32> {
    None
}

pub fn seal_manifest(run_dir: &Path, manifest: &Manifest) -> Result<(), String> {
    let json =
        serde_json::to_string_pretty(manifest).map_err(|e| format!("serialize manifest: {}", e))?;
    fs::write(run_dir.join("manifest.json"), json).map_err(|e| format!("write manifest: {}", e))
}

pub fn load_manifest(run_dir: &Path) -> Result<Manifest, String> {
    let content = fs::read_to_string(run_dir.join("manifest.json"))
        .map_err(|e| format!("read manifest: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("parse manifest: {}", e))
}

pub fn restore(backup_dir: &Path, run_id: &str) -> Result<usize, String> {
    let run_dir = backup_dir.join(run_id);
    let manifest = load_manifest(&run_dir)?;
    if !manifest.root.is_absolute() {
        return Err(format!(
            "backup root is not absolute: {}",
            manifest.root.display()
        ));
    }

    // Validate the complete run before writing a single destination.
    for entry in &manifest.files {
        if entry.hash.len() != 64 || !entry.hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("invalid backup hash: {}", entry.hash));
        }
        let src = run_dir.join("files").join(&entry.hash);
        if !src.exists() {
            return Err(format!("backup file missing: {}", entry.hash));
        }

        let relative = safe_relative_path(&entry.relative_path)?;
        validate_destination(&manifest.root, &relative)?;
        let bytes = fs::read(&src).map_err(|e| format!("read backup {}: {e}", entry.hash))?;
        if hash_bytes(&bytes) != entry.hash {
            return Err(format!("backup hash mismatch: {}", entry.hash));
        }
        let _ = manifest.root.join(relative);
    }

    let mut restored = 0;
    for entry in &manifest.files {
        let src = run_dir.join("files").join(&entry.hash);
        let relative = safe_relative_path(&entry.relative_path)?;
        let dest = manifest.root.join(relative);
        let bytes = fs::read(&src).map_err(|e| format!("read backup {}: {e}", entry.hash))?;

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("create dir {}: {}", parent.display(), e))?;
        }

        atomic_restore(&dest, &bytes, entry.mode)
            .map_err(|e| format!("restore {}: {e}", entry.relative_path))?;
        restored += 1;
    }
    Ok(restored)
}

fn safe_relative_path(raw: &str) -> Result<PathBuf, String> {
    use std::path::Component;

    let path = Path::new(raw);
    if path.as_os_str().is_empty()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("unsafe backup path: {raw}"));
    }
    Ok(path.to_path_buf())
}

fn validate_destination(root: &Path, relative: &Path) -> Result<(), String> {
    let mut current = root.to_path_buf();
    if fs::symlink_metadata(&current)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(format!("backup root is a symlink: {}", root.display()));
    }
    for component in relative.components() {
        current.push(component.as_os_str());
        if fs::symlink_metadata(&current)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err(format!(
                "restore path crosses a symlink: {}",
                current.display()
            ));
        }
    }
    Ok(())
}

fn atomic_restore(path: &Path, bytes: &[u8], mode: Option<u32>) -> Result<(), String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let mut name = path.as_os_str().to_os_string();
    name.push(format!(
        ".bashm.restore.{}.{}",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let tmp = PathBuf::from(name);
    let existing_permissions = fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());

    let result = (|| {
        fs::write(&tmp, bytes).map_err(|e| format!("write temp: {e}"))?;
        set_restored_permissions(&tmp, mode, existing_permissions)?;
        fs::rename(&tmp, path).map_err(|e| format!("rename temp: {e}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(unix)]
fn set_restored_permissions(
    path: &Path,
    mode: Option<u32>,
    fallback: Option<fs::Permissions>,
) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    if let Some(mode) = mode {
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|e| format!("set permissions: {e}"))?;
    } else if let Some(permissions) = fallback {
        fs::set_permissions(path, permissions).map_err(|e| format!("set permissions: {e}"))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_restored_permissions(
    path: &Path,
    _mode: Option<u32>,
    fallback: Option<fs::Permissions>,
) -> Result<(), String> {
    if let Some(permissions) = fallback {
        fs::set_permissions(path, permissions).map_err(|e| format!("set permissions: {e}"))?;
    }
    Ok(())
}

pub fn list_runs(backup_dir: &Path) -> Result<Vec<ManifestSummary>, String> {
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut runs = Vec::new();
    let entries = fs::read_dir(backup_dir).map_err(|e| format!("read backup dir: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Ok(manifest) = load_manifest(&path) {
                runs.push(ManifestSummary {
                    run_id: manifest.run_id,
                    timestamp: manifest.timestamp,
                    root: manifest.root,
                    file_count: manifest.files.len(),
                    profile_name: manifest.profile_name,
                });
            }
        }
    }
    runs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(runs)
}

pub fn create_run(backup_dir: &Path, run_id: &str) -> Result<PathBuf, String> {
    let run_dir = backup_dir.join(run_id);
    fs::create_dir_all(&run_dir).map_err(|e| format!("create run dir: {}", e))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_rejects_parent_directory_paths() {
        let root = std::env::temp_dir().join(format!("bash-em-safe-{}", std::process::id()));
        let vault = root.join("vault");
        let data = root.join("data");
        fs::create_dir_all(&data).unwrap();
        let source = data.join("note.md");
        fs::write(&source, "original").unwrap();
        let (run_dir, run_id, mut manifest) = begin_run(&vault, &data, "test").unwrap();
        let mut entry = snapshot_file(&run_dir, &source, &data).unwrap();
        entry.relative_path = "../escape.md".to_string();
        manifest.files.push(entry);
        seal_manifest(&run_dir, &manifest).unwrap();
        let error = restore(&vault, &run_id).unwrap_err();
        assert!(error.contains("unsafe backup path"));
        assert!(!root.join("escape.md").exists());
        let _ = fs::remove_dir_all(root);
    }
}
