use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(name = "bash-em", version, about = "bash them dashes")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a directory tree for typographic artifacts
    Scan {
        path: PathBuf,
        #[arg(long)]
        profile: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Apply replacements after creating a backup
    Apply {
        path: PathBuf,
        #[arg(long)]
        profile: Option<PathBuf>,
        #[arg(long)]
        yes: bool,
    },
    /// Restore files from a retained backup run
    Restore {
        run_id: String,
        #[arg(long)]
        profile: Option<PathBuf>,
    },
    /// Manage retained backup runs
    Backups {
        #[command(subcommand)]
        action: BackupsAction,
    },
    /// Show a health report for a directory
    Health {
        path: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        profile: Option<PathBuf>,
    },
    /// Manage rules
    Rules {
        #[command(subcommand)]
        action: RulesAction,
    },
    /// Manage profiles
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },
    /// List active adapters
    Adapters {
        #[command(subcommand)]
        action: AdaptersAction,
    },
    /// Launch the interactive TUI
    Tui {
        path: Option<PathBuf>,
        #[arg(long)]
        profile: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum RulesAction {
    List,
}

#[derive(Subcommand)]
enum ProfileAction {
    Show { file: Option<PathBuf> },
    Validate { file: PathBuf },
}

#[derive(Subcommand)]
enum AdaptersAction {
    List,
}

#[derive(Subcommand)]
enum BackupsAction {
    /// List retained backup runs
    List {
        #[arg(long)]
        profile: Option<PathBuf>,
    },
}

fn load_profile_or_default(path: Option<&PathBuf>) -> config::Profile {
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    resolve_profile_or_exit(&root, path).profile
}

fn resolve_profile_or_exit(
    root: &std::path::Path,
    explicit: Option<&PathBuf>,
) -> config::ProfileDocument {
    config::resolve_profile(root, explicit.map(PathBuf::as_path)).unwrap_or_else(|error| {
        eprintln!("{} {}", "error:".red().bold(), error);
        std::process::exit(1);
    })
}

fn scan_path(path: PathBuf, profile: config::Profile) -> workflow::ScanReport {
    workflow::scan(
        workflow::ScanRequest {
            root: path,
            profile,
        },
        |_| {},
        &AtomicBool::new(false),
    )
    .unwrap_or_else(|error| {
        eprintln!("{} {}", "error:".red().bold(), error);
        std::process::exit(1);
    })
}

fn cmd_scan(path: PathBuf, profile_path: Option<PathBuf>, json: bool) {
    let profile = resolve_profile_or_exit(&path, profile_path.as_ref()).profile;
    let report = scan_path(path, profile);

    if json {
        let output = diff::RunReport {
            run_id: String::new(),
            timestamp: String::new(),
            root: report.root.clone(),
            profile: report.profile.name.clone(),
            files: report
                .files
                .iter()
                .map(|file| diff::FileDiff {
                    path: file.path.clone(),
                    hunks: file
                        .changes
                        .iter()
                        .map(|change| diff::LineHunk {
                            line_no: change.line_no,
                            before: change.before.clone(),
                            after: change.after.clone(),
                        })
                        .collect(),
                    counts: file.counts,
                    lines_changed: file.lines_changed,
                })
                .collect(),
            totals: report.totals,
        };
        println!("{}", output.to_json().expect("serialize scan report"));
        return;
    }

    println!("{}", "bash-em scan".bold());
    println!("  root: {}", report.root.display());
    println!(
        "  scanned: {}  skipped: {}  ignored: {}  binary: {}",
        report.stats.scanned,
        report.stats.skipped,
        report.stats.ignored + report.stats.hidden_dirs,
        report.stats.binary
    );
    println!();

    if report.files.is_empty() {
        println!("  {} no offenders found.", "✓".green());
        return;
    }

    println!(
        "  {} dirty files:",
        report.files.len().to_string().yellow().bold()
    );
    for file in &report.files {
        let relative = file.path.strip_prefix(&report.root).unwrap_or(&file.path);
        println!(
            "    {} {} (em:{} en:{} bar:{} ent:{} cq:{} ell:{} zw:{})",
            "●".red(),
            relative.display(),
            file.counts.em,
            file.counts.en,
            file.counts.bar,
            file.counts.entities,
            file.counts.curly_quotes,
            file.counts.ellipsis,
            file.counts.zero_width
        );
        for change in &file.changes {
            println!(
                "      L{}: {} {} {}",
                change.line_no,
                change.before.red(),
                "→".dimmed(),
                change.after.green()
            );
        }
    }
    println!();
    println!(
        "  total offenders: {}",
        report.totals.total().to_string().yellow()
    );
}

fn cmd_apply(path: PathBuf, profile_path: Option<PathBuf>, yes: bool) {
    let profile = resolve_profile_or_exit(&path, profile_path.as_ref()).profile;
    let report = scan_path(path, profile);
    if report.files.is_empty() {
        println!("{} nothing to bash.", "✓".green());
        return;
    }

    println!(
        "{} dirty files. {} total offenders.",
        report.files.len().to_string().yellow().bold(),
        report.totals.total().to_string().red().bold()
    );
    if !yes {
        eprint!("apply? [y/N] ");
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("read confirmation");
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("aborted.");
            return;
        }
    }

    let included = workflow::included_paths(&report);
    let applied = workflow::apply(&report, &included).unwrap_or_else(|error| {
        eprintln!("{} {}", "error:".red().bold(), error);
        std::process::exit(1);
    });
    println!("  backup vault: {}", applied.backup_dir.display());
    if let Some(run_id) = &applied.run_id {
        println!("  backup run: {}", run_id.dimmed());
    }
    println!(
        "  {} {} files bashed. {} errors.",
        "✓".green(),
        applied.applied_files.to_string().green().bold(),
        applied.errors.len()
    );
    if applied.pruned_runs > 0 {
        println!("  pruned {} old backup(s).", applied.pruned_runs);
    }
    for error in &applied.errors {
        eprintln!("  {} {}", "!".red(), error);
    }
}

fn cmd_restore(run_id: String, profile_path: Option<PathBuf>) {
    let profile = load_profile_or_default(profile_path.as_ref());
    match workflow::restore(&profile, &run_id) {
        Ok(report) => {
            println!("{} restored {} files.", "✓".green(), report.restored_files);
            println!(
                "  backup retained: {}/{}",
                report.backup_dir.display(),
                run_id
            );
        }
        Err(error) => {
            eprintln!("{} {}", "error:".red().bold(), error);
            std::process::exit(1);
        }
    }
}

fn cmd_backups(profile_path: Option<PathBuf>) {
    let profile = load_profile_or_default(profile_path.as_ref());
    let backup_dir = config::resolve_backup_dir(&profile.prefs);
    println!("{}", "bash-em backups".bold());
    println!("  vault: {}", backup_dir.display());
    match workflow::list_backups(&profile) {
        Ok(runs) if runs.is_empty() => println!("  no backup runs found."),
        Ok(runs) => {
            for run in runs {
                println!(
                    "  {}  {} files  {}  {}",
                    run.run_id,
                    run.file_count,
                    run.profile_name,
                    run.root.display()
                );
            }
        }
        Err(error) => {
            eprintln!("{} {}", "error:".red().bold(), error);
            std::process::exit(1);
        }
    }
}

fn cmd_health(path: PathBuf, profile_path: Option<PathBuf>, json: bool) {
    let profile = resolve_profile_or_exit(&path, profile_path.as_ref()).profile;
    let report = scan_path(path, profile);
    let health = report.health;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&health).expect("serialize health report")
        );
        return;
    }

    println!("{}", "bash-em health".bold());
    println!("  root: {}", health.root.display());
    println!("  scanned: {}  skipped: {}", health.scanned, health.skipped);
    println!(
        "  dirty files: {}  health score: {}/100",
        health.dirty_files,
        score_colored(health.score)
    );
    println!("  artifacts: {}", health.corruption.total());
    for (path, count) in health.top_files.iter().take(10) {
        let relative = path.strip_prefix(&health.root).unwrap_or(path);
        println!("    {} {} ({})", "●".red(), relative.display(), count);
    }
}

fn score_colored(score: u8) -> colored::ColoredString {
    let value = score.to_string();
    if score == 0 {
        value.green().bold()
    } else if score < 50 {
        value.yellow()
    } else {
        value.red().bold()
    }
}

fn cmd_rules_list() {
    for (name, rule) in &config::default_profile().rules {
        let status = if rule.enabled {
            "on".green()
        } else {
            "off".dimmed()
        };
        println!("  {} [{}]", name, status);
    }
}

fn cmd_profile_show(file: Option<PathBuf>) {
    let profile = load_profile_or_default(file.as_ref());
    println!(
        "{}",
        serde_yaml::to_string(&profile).expect("serialize profile")
    );
}

fn cmd_profile_validate(file: PathBuf) {
    match config::load_profile(&file) {
        Ok(profile) => println!("{} profile '{}' is valid.", "✓".green(), profile.name),
        Err(error) => {
            eprintln!("{} {}", "error:".red().bold(), error);
            std::process::exit(1);
        }
    }
}

fn cmd_adapters_list() {
    println!("{}", "active adapters:".bold());
    for (name, extensions, category, can_write) in adapters::Registry::default().list() {
        let mode = if can_write { "read/write" } else { "read-only" };
        println!(
            "  {} [{}] ({}) .{}",
            name,
            category,
            mode,
            extensions
                .iter()
                .take(8)
                .copied()
                .collect::<Vec<_>>()
                .join(", .")
        );
    }
}

fn cmd_tui(path: Option<PathBuf>, profile_path: Option<PathBuf>) {
    let root =
        path.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let root = root.canonicalize().unwrap_or(root);
    let document = resolve_profile_or_exit(&root, profile_path.as_ref());
    let app = bash_em_tui::app::App::with_profile_document(root, document);
    if let Err(error) = bash_em_tui::run::run(app) {
        eprintln!("{} TUI: {}", "error:".red().bold(), error);
        std::process::exit(1);
    }
}

fn main() {
    match Cli::parse().command {
        None => cmd_tui(None, None),
        Some(Commands::Scan {
            path,
            profile,
            json,
        }) => cmd_scan(path, profile, json),
        Some(Commands::Apply { path, profile, yes }) => cmd_apply(path, profile, yes),
        Some(Commands::Restore { run_id, profile }) => cmd_restore(run_id, profile),
        Some(Commands::Backups {
            action: BackupsAction::List { profile },
        }) => cmd_backups(profile),
        Some(Commands::Health {
            path,
            json,
            profile,
        }) => cmd_health(path, profile, json),
        Some(Commands::Rules {
            action: RulesAction::List,
        }) => cmd_rules_list(),
        Some(Commands::Profile {
            action: ProfileAction::Show { file },
        }) => cmd_profile_show(file),
        Some(Commands::Profile {
            action: ProfileAction::Validate { file },
        }) => cmd_profile_validate(file),
        Some(Commands::Adapters {
            action: AdaptersAction::List,
        }) => cmd_adapters_list(),
        Some(Commands::Tui { path, profile }) => cmd_tui(path, profile),
    }
}
