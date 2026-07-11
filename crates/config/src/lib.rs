use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

fn default_true() -> bool { true }

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
    #[serde(default)]
    pub fence_guard: bool,
}

fn default_max_file_bytes() -> u64 { 10 * 1024 * 1024 }
fn default_preview_lines() -> usize { 8 }
fn default_backup_dir() -> String { "~/.bash-em/backups".to_string() }
fn default_keep_last_n() -> usize { 10 }
fn default_skip_dirs() -> Vec<String> {
    vec![
        ".git", ".svn", ".hg", "node_modules", "target", "__pycache__",
        ".cache", ".venv", "venv", ".idea", ".vscode", "dist", "build",
    ].into_iter().map(String::from).collect()
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
    serde_yaml::from_str(&content)
        .map_err(|e| format!("invalid profile YAML: {}", e))
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
        ignore: vec![
            "**/*.min.js".into(),
            "**/package-lock.json".into(),
        ],
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
    profile.rules.get(rule_name).map_or(true, |r| r.enabled)
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
}
