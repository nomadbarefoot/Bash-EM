use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROJECT_PROFILE_FILE: &str = ".bash-em.yaml";

#[derive(Debug, Clone)]
pub struct ProfileDocument {
    pub profile: Profile,
    pub path: PathBuf,
    pub explicit: bool,
    pub persisted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub rules: HashMap<String, RuleConfig>,
    #[serde(default)]
    pub prefs: Prefs,
    #[serde(default)]
    pub ignore: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefs {
    #[serde(default = "default_max_file_bytes")]
    pub max_file_bytes: u64,
    #[serde(default = "default_preview_lines")]
    pub preview_lines: usize,
    #[serde(default = "default_backup_dir")]
    pub backup_dir: String,
    #[serde(default = "default_skip_dirs")]
    pub skip_dirs: Vec<String>,
    #[serde(default = "default_keep_last_n")]
    pub keep_last_n: usize,
    #[serde(default = "default_true")]
    pub fence_guard: bool,
}

fn default_max_file_bytes() -> u64 {
    10 * 1024 * 1024
}
fn default_preview_lines() -> usize {
    8
}
fn default_backup_dir() -> String {
    "~/.bash-em/backups".to_string()
}
fn default_keep_last_n() -> usize {
    10
}
fn default_skip_dirs() -> Vec<String> {
    vec![
        ".git",
        ".svn",
        ".hg",
        "node_modules",
        "target",
        "__pycache__",
        ".cache",
        ".venv",
        "venv",
        ".idea",
        ".vscode",
        "dist",
        "build",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            max_file_bytes: default_max_file_bytes(),
            preview_lines: default_preview_lines(),
            backup_dir: default_backup_dir(),
            skip_dirs: default_skip_dirs(),
            keep_last_n: default_keep_last_n(),
            fence_guard: true,
        }
    }
}

pub fn load_profile(path: &Path) -> Result<Profile, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read profile {}: {}", path.display(), e))?;
    serde_yaml::from_str(&content).map_err(|e| format!("invalid profile YAML: {}", e))
}

pub fn project_profile_path(root: &Path) -> PathBuf {
    root.join(PROJECT_PROFILE_FILE)
}

pub fn resolve_profile(
    root: &Path,
    explicit_path: Option<&Path>,
) -> Result<ProfileDocument, String> {
    let explicit = explicit_path.is_some();
    let path = explicit_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| project_profile_path(root));
    let persisted = path.is_file();
    let profile = if explicit || persisted {
        load_profile(&path)?
    } else {
        default_profile()
    };
    Ok(ProfileDocument {
        profile,
        path,
        explicit,
        persisted,
    })
}

pub fn save_profile(path: &Path, profile: &Profile) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("create profile directory {}: {error}", parent.display()))?;
    let yaml =
        serde_yaml::to_string(profile).map_err(|error| format!("serialize profile: {error}"))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("profile.yaml");
    let temp = parent.join(format!(".{file_name}.tmp-{}-{nonce}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|error| format!("create temporary profile {}: {error}", temp.display()))?;
        file.write_all(yaml.as_bytes())
            .map_err(|error| format!("write temporary profile {}: {error}", temp.display()))?;
        if let Ok(metadata) = fs::metadata(path) {
            fs::set_permissions(&temp, metadata.permissions()).map_err(|error| {
                format!("preserve profile permissions {}: {error}", path.display())
            })?;
        }
        file.sync_all()
            .map_err(|error| format!("sync temporary profile {}: {error}", temp.display()))?;
        fs::rename(&temp, path)
            .map_err(|error| format!("replace profile {}: {error}", path.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

pub fn default_profile() -> Profile {
    Profile {
        name: "typographic".to_string(),
        rules: HashMap::from([
            ("em_dash".into(), RuleConfig { enabled: true }),
            ("en_dash".into(), RuleConfig { enabled: true }),
            ("horizontal_bar".into(), RuleConfig { enabled: true }),
            ("html_dash_entities".into(), RuleConfig { enabled: true }),
            ("curly_quotes".into(), RuleConfig { enabled: false }),
            ("ellipsis".into(), RuleConfig { enabled: false }),
            ("zero_width".into(), RuleConfig { enabled: true }),
            ("llm_boilerplate".into(), RuleConfig { enabled: false }),
        ]),
        prefs: Prefs::default(),
        ignore: vec!["**/*.min.js".into(), "**/package-lock.json".into()],
    }
}

pub fn resolve_backup_dir(prefs: &Prefs) -> PathBuf {
    let raw = &prefs.backup_dir;
    if raw.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&raw[2..]);
        }
    }
    PathBuf::from(raw)
}

pub fn is_rule_enabled(profile: &Profile, rule_name: &str) -> bool {
    profile
        .rules
        .get(rule_name)
        .map_or_else(|| default_rule_enabled(rule_name), |rule| rule.enabled)
}

fn default_rule_enabled(rule_name: &str) -> bool {
    !matches!(rule_name, "curly_quotes" | "ellipsis" | "llm_boilerplate")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_has_dashes_enabled() {
        let p = default_profile();
        assert!(is_rule_enabled(&p, "em_dash"));
        assert!(is_rule_enabled(&p, "en_dash"));
        assert!(!is_rule_enabled(&p, "curly_quotes"));
    }

    #[test]
    fn unknown_rule_defaults_enabled() {
        let p = default_profile();
        assert!(is_rule_enabled(&p, "nonexistent_rule"));
    }

    #[test]
    fn omitted_opt_in_rules_stay_disabled() {
        let profile: Profile = serde_yaml::from_str("name: partial\nrules: {}\n").unwrap();
        assert!(!is_rule_enabled(&profile, "curly_quotes"));
        assert!(!is_rule_enabled(&profile, "ellipsis"));
        assert!(!is_rule_enabled(&profile, "llm_boilerplate"));
        assert!(is_rule_enabled(&profile, "zero_width"));
    }

    #[test]
    fn yaml_roundtrip() {
        let p = default_profile();
        let yaml = serde_yaml::to_string(&p).unwrap();
        let loaded: Profile = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(loaded.name, "typographic");
    }

    #[test]
    fn invalid_yaml_errors() {
        let result: Result<Profile, _> = serde_yaml::from_str("{{{{not yaml");
        assert!(result.is_err());
    }

    #[test]
    fn omitted_fence_guard_stays_enabled() {
        let yaml = "name: safe\nrules: {}\nprefs: {}\n";
        let profile: Profile = serde_yaml::from_str(yaml).unwrap();
        assert!(profile.prefs.fence_guard);
    }

    #[test]
    fn project_profile_is_discovered_and_saved_atomically() {
        let root = std::env::temp_dir().join(format!(
            "bash-em-profile-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let document = resolve_profile(&root, None).unwrap();
        assert!(!document.persisted);
        assert_eq!(document.path, root.join(PROJECT_PROFILE_FILE));

        let mut profile = document.profile;
        profile.name = "saved-project".to_string();
        save_profile(&document.path, &profile).unwrap();
        let loaded = resolve_profile(&root, None).unwrap();
        assert!(loaded.persisted);
        assert_eq!(loaded.profile.name, "saved-project");
        assert!(!loaded.explicit);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn explicit_profile_takes_precedence_over_project_profile() {
        let root = std::env::temp_dir().join(format!(
            "bash-em-explicit-profile-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let project_path = project_profile_path(&root);
        let explicit_path = root.join("explicit.yaml");
        let mut project = default_profile();
        project.name = "project".to_string();
        save_profile(&project_path, &project).unwrap();
        let mut explicit = default_profile();
        explicit.name = "explicit".to_string();
        save_profile(&explicit_path, &explicit).unwrap();

        let loaded = resolve_profile(&root, Some(&explicit_path)).unwrap();
        assert_eq!(loaded.profile.name, "explicit");
        assert!(loaded.explicit);
        let _ = fs::remove_dir_all(root);
    }
}
