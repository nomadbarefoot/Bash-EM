use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn tempdir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("bash-em-cli-{name}-{nonce}"));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn profile_apply_lists_and_restores_the_same_run() {
    let temp = tempdir("roundtrip");
    let root = temp.join("data");
    let vault = temp.join("vault");
    fs::create_dir_all(&root).unwrap();
    let source = root.join("note.md");
    fs::write(&source, "say \u{201c}hello\u{201d}").unwrap();

    let mut profile = config::default_profile();
    profile.prefs.backup_dir = vault.display().to_string();
    profile.rules.get_mut("curly_quotes").unwrap().enabled = true;
    let profile_path = temp.join("profile.yaml");
    fs::write(&profile_path, serde_yaml::to_string(&profile).unwrap()).unwrap();

    let binary = env!("CARGO_BIN_EXE_bash-em");
    let apply = Command::new(binary)
        .args(["apply"])
        .arg(&root)
        .args(["--profile"])
        .arg(&profile_path)
        .arg("--yes")
        .output()
        .unwrap();
    assert!(
        apply.status.success(),
        "{}",
        String::from_utf8_lossy(&apply.stderr)
    );
    assert_eq!(fs::read_to_string(&source).unwrap(), "say \"hello\"");

    let run_id = fs::read_dir(&vault)
        .unwrap()
        .flatten()
        .find(|entry| entry.path().is_dir())
        .unwrap()
        .file_name()
        .to_string_lossy()
        .to_string();
    let list = Command::new(binary)
        .args(["backups", "list"])
        .args(["--profile"])
        .arg(&profile_path)
        .output()
        .unwrap();
    assert!(list.status.success());
    assert!(String::from_utf8_lossy(&list.stdout).contains(&run_id));

    let restore = Command::new(binary)
        .args(["restore", &run_id, "--profile"])
        .arg(&profile_path)
        .output()
        .unwrap();
    assert!(
        restore.status.success(),
        "{}",
        String::from_utf8_lossy(&restore.stderr)
    );
    assert_eq!(
        fs::read_to_string(&source).unwrap(),
        "say \u{201c}hello\u{201d}"
    );
    assert!(vault.join(run_id).exists());
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn active_adapter_list_is_honest() {
    let output = Command::new(env!("CARGO_BIN_EXE_bash-em"))
        .args(["adapters", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("text"));
    assert!(!stdout.contains("docx"));
    assert!(!stdout.contains("xlsx"));
    assert!(!stdout.contains("pdf"));
}

#[test]
fn project_profile_and_ignore_file_are_loaded_automatically() {
    let root = tempdir("project-config");
    let mut profile = config::default_profile();
    profile.name = "project-auto".to_string();
    profile.rules.get_mut("curly_quotes").unwrap().enabled = true;
    config::save_profile(&config::project_profile_path(&root), &profile).unwrap();
    fs::write(root.join(".bash-emignore"), "ignored.md\n").unwrap();
    fs::write(root.join("ignored.md"), "\u{201c}ignored\u{201d}").unwrap();
    fs::write(root.join("keep.md"), "\u{201c}keep\u{201d}").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bash-em"))
        .args(["scan"])
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["profile"], "project-auto");
    let files = report["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0]["path"].as_str().unwrap().ends_with("keep.md"));
    let _ = fs::remove_dir_all(root);
}
