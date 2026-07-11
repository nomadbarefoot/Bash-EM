use std::path::PathBuf;
use clap::{Parser, Subcommand};
use colored::Colorize;
use engine::FixOptions;

#[derive(Parser)]
#[command(name = "bash-em", version, about = "bash them dashes")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
    /// Apply replacements (with backup)
    Apply {
        path: PathBuf,
        #[arg(long)]
        profile: Option<PathBuf>,
        #[arg(long)]
        yes: bool,
    },
    /// Restore files from a backup run
    Restore {
        run_id: String,
    },
    /// Show health report for a directory
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
    /// Manage adapters
    Adapters {
        #[command(subcommand)]
        action: AdaptersAction,
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

fn load_profile_or_default(path: Option<&PathBuf>) -> config::Profile {
    match path {
        Some(p) => match config::load_profile(p) {
            Ok(profile) => profile,
            Err(e) => {
                eprintln!("{} {}", "error:".red().bold(), e);
                std::process::exit(1);
            }
        },
        None => config::default_profile(),
    }
}

fn build_fix_options(profile: &config::Profile) -> FixOptions {
    let pairs: Vec<(String, bool)> = profile.rules.iter()
        .map(|(k, v)| (k.clone(), v.enabled))
        .collect();
    FixOptions::from_profile(&pairs)
}

fn cmd_scan(path: PathBuf, profile_path: Option<PathBuf>, json: bool) {
    let profile = load_profile_or_default(profile_path.as_ref());
    let root = match path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}: {}", "error:".red().bold(), path.display(), e);
            std::process::exit(1);
        }
    };

    let opts = build_fix_options(&profile);
    let (candidates, stats) = adapters::walk_tree(&root, &profile.prefs);
    let pipeline = engine::Pipeline::with_options(
        profile.prefs.preview_lines, opts, profile.prefs.fence_guard,
    );
    let batch = pipeline.process_batch(
        candidates.into_iter().map(|c| (c.path, c.content)).collect()
    );

    let llm_enabled = config::is_rule_enabled(&profile, "llm_boilerplate");

    if json {
        let report = diff::RunReport {
            run_id: String::new(),
            timestamp: String::new(),
            root: root.clone(),
            profile: profile.name.clone(),
            files: batch.files.iter().map(|f| diff::FileDiff {
                path: f.path.clone(),
                hunks: f.changes.iter().map(|c| diff::LineHunk {
                    line_no: c.line_no,
                    before: c.before.clone(),
                    after: c.after.clone(),
                }).collect(),
                counts: f.counts,
                lines_changed: f.lines_changed,
            }).collect(),
            totals: batch.totals,
        };
        println!("{}", report.to_json().unwrap());
        return;
    }

    println!("{}", "bash-em scan".bold());
    println!("  root: {}", root.display());
    println!("  scanned: {}  skipped: {}", stats.scanned, stats.skipped);
    println!();

    if batch.files.is_empty() && !llm_enabled {
        println!("  {} no offenders found.", "\u{2714}".green());
        return;
    }

    if !batch.files.is_empty() {
        println!("  {} {} files with artifacts:",
            batch.files.len().to_string().yellow().bold(),
            "dirty");

        for f in &batch.files {
            let rel = f.path.strip_prefix(&root).unwrap_or(&f.path);
            println!("    {} {} (em:{} en:{} bar:{} ent:{} cq:{} ell:{} zw:{})",
                "\u{25cf}".red(),
                rel.display(),
                f.counts.em, f.counts.en, f.counts.bar, f.counts.entities,
                f.counts.curly_quotes, f.counts.ellipsis, f.counts.zero_width);

            for c in &f.changes {
                println!("      L{}: {} {} {}",
                    c.line_no,
                    c.before.red(),
                    "\u{2192}".dimmed(),
                    c.after.green());
            }
        }

        println!();
        println!("  totals: {} em, {} en, {} bar, {} ent, {} cq, {} ell, {} zw",
            batch.totals.em.to_string().yellow(),
            batch.totals.en.to_string().yellow(),
            batch.totals.bar.to_string().yellow(),
            batch.totals.entities.to_string().yellow(),
            batch.totals.curly_quotes.to_string().yellow(),
            batch.totals.ellipsis.to_string().yellow(),
            batch.totals.zero_width.to_string().yellow());
    }

    if llm_enabled {
        println!();
        println!("  {}", "LLM boilerplate flags:".bold());
        let mut any_flags = false;
        for f in &batch.files {
            let content = std::fs::read_to_string(&f.path).unwrap_or_default();
            let report = engine::boilerplate::scan_content(&content);
            if !report.matches.is_empty() {
                any_flags = true;
                let rel = f.path.strip_prefix(&root).unwrap_or(&f.path);
                for m in &report.matches {
                    println!("    {} {}:L{} [{}] {}",
                        "\u{26a0}".yellow(),
                        rel.display(),
                        m.line_no,
                        match m.confidence {
                            engine::boilerplate::Confidence::NearCertain => "near-certain",
                            engine::boilerplate::Confidence::High => "high",
                        },
                        m.phrase);
                }
            }
        }
        if !any_flags {
            println!("    {} no LLM tells detected.", "\u{2714}".green());
        }
    }
}

fn cmd_apply(path: PathBuf, profile_path: Option<PathBuf>, yes: bool) {
    let profile = load_profile_or_default(profile_path.as_ref());
    let root = match path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}: {}", "error:".red().bold(), path.display(), e);
            std::process::exit(1);
        }
    };

    let backup_dir = config::resolve_backup_dir(&profile.prefs);
    let opts = build_fix_options(&profile);
    let (candidates, _stats) = adapters::walk_tree(&root, &profile.prefs);
    let pipeline = engine::Pipeline::with_options(
        profile.prefs.preview_lines, opts.clone(), profile.prefs.fence_guard,
    );
    let batch = pipeline.process_batch(
        candidates.into_iter().map(|c| (c.path, c.content)).collect()
    );

    if batch.files.is_empty() {
        println!("{} nothing to bash.", "\u{2714}".green());
        return;
    }

    println!("{} files with artifacts. {} total offenders.",
        batch.files.len().to_string().yellow().bold(),
        batch.totals.total().to_string().red().bold());

    if !yes {
        eprint!("apply? [y/N] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("aborted.");
            return;
        }
    }

    let (run_dir, run_id, mut manifest) = backup::begin_run(&backup_dir, &root, &profile.name)
        .unwrap_or_else(|e| {
            eprintln!("{} backup init: {}", "error:".red().bold(), e);
            std::process::exit(1);
        });

    println!("backup: {}", run_id.dimmed());

    let mut applied = 0;
    let mut errors = 0;

    for file_edits in &batch.files {
        match backup::snapshot_file(&run_dir, &file_edits.path, &root) {
            Ok(entry) => manifest.files.push(entry),
            Err(e) => {
                eprintln!("  {} snapshot {}: {}", "!".red(), file_edits.path.display(), e);
                errors += 1;
                continue;
            }
        }

        match adapters::TextAdapter::apply(&file_edits.path, 0) {
            Ok(_counts) => applied += 1,
            Err(e) => {
                eprintln!("  {} apply {}: {}", "!".red(), file_edits.path.display(), e);
                errors += 1;
            }
        }
    }

    backup::seal_manifest(&run_dir, &manifest)
        .unwrap_or_else(|e| eprintln!("{} seal manifest: {}", "warn:".yellow(), e));

    match backup::prune_old_runs(&backup_dir, profile.prefs.keep_last_n) {
        Ok(pruned) if pruned > 0 => {
            println!("  pruned {} old backup(s).", pruned);
        }
        _ => {}
    }

    println!();
    println!("  {} {} files bashed. {} errors.",
        "\u{2714}".green(),
        applied.to_string().green().bold(),
        errors);
    println!("  restore with: {} {}",
        "bash-em restore".bold(),
        run_id);
}

fn cmd_restore(run_id: String) {
    let prefs = config::Prefs::default();
    let backup_dir = config::resolve_backup_dir(&prefs);

    match backup::restore(&backup_dir, &run_id) {
        Ok(count) => {
            println!("{} restored {} files.", "\u{2714}".green(), count);
        }
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

fn cmd_health(path: PathBuf, profile_path: Option<PathBuf>, json: bool) {
    let profile = load_profile_or_default(profile_path.as_ref());
    let root = match path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}: {}", "error:".red().bold(), path.display(), e);
            std::process::exit(1);
        }
    };

    let opts = build_fix_options(&profile);
    let (candidates, stats) = adapters::walk_tree(&root, &profile.prefs);
    let pipeline = engine::Pipeline::with_options(
        profile.prefs.preview_lines, opts, profile.prefs.fence_guard,
    );

    let llm_enabled = config::is_rule_enabled(&profile, "llm_boilerplate");
    let mut health = engine::health::HealthReport::new(root.clone(), stats.scanned, stats.skipped);

    for cand in &candidates {
        let edits = pipeline.process_content(cand.path.clone(), &cand.content);
        let counts = edits.as_ref().map(|e| e.counts).unwrap_or_default();
        let ext = cand.path.extension().and_then(|e| e.to_str()).unwrap_or("other");
        let category = match ext {
            "md" | "txt" | "rst" => "text",
            "rs" | "py" | "js" | "ts" | "go" | "rb" => "code",
            "html" | "htm" | "css" => "web",
            "xlsx" => "office",
            "docx" => "docs",
            "pdf" => "pdf",
            _ => "other",
        };
        health.add_file(cand.path.clone(), &counts, category);

        if llm_enabled {
            let bp = engine::boilerplate::scan_content(&cand.content);
            health.add_boilerplate(&bp);
        }
    }

    health.finalize();

    if json {
        let j = serde_json::to_string_pretty(&health).unwrap();
        println!("{}", j);
        return;
    }

    println!("{}", "bash-em health".bold());
    println!("  root: {}", root.display());
    println!("  scanned: {}  skipped: {}", health.scanned, health.skipped);
    println!("  score: {}/100", score_colored(health.score));
    println!();

    let c = &health.corruption;
    println!("  {}", "corruption breakdown:".bold());
    if c.em_dash > 0 { println!("    em-dash:      {}", c.em_dash.to_string().yellow()); }
    if c.en_dash > 0 { println!("    en-dash:      {}", c.en_dash.to_string().yellow()); }
    if c.horizontal_bar > 0 { println!("    horiz-bar:    {}", c.horizontal_bar.to_string().yellow()); }
    if c.html_entities > 0 { println!("    html-entities: {}", c.html_entities.to_string().yellow()); }
    if c.curly_quotes > 0 { println!("    curly-quotes: {}", c.curly_quotes.to_string().yellow()); }
    if c.ellipsis > 0 { println!("    ellipsis:     {}", c.ellipsis.to_string().yellow()); }
    if c.zero_width > 0 { println!("    zero-width:   {}", c.zero_width.to_string().yellow()); }
    if c.llm_flags > 0 { println!("    llm-flags:    {}", c.llm_flags.to_string().yellow()); }
    if c.total() == 0 { println!("    {}", "clean!".green()); }

    if !health.by_category.is_empty() {
        println!();
        println!("  {}", "by category:".bold());
        for (cat, s) in &health.by_category {
            if s.artifact_count > 0 {
                println!("    {}: {} files, {} artifacts", cat, s.file_count, s.artifact_count);
            }
        }
    }

    if !health.top_files.is_empty() {
        println!();
        println!("  {}", "worst offenders:".bold());
        for (path, count) in health.top_files.iter().take(10) {
            let rel = path.strip_prefix(&root).unwrap_or(path);
            println!("    {} {} ({})", "\u{25cf}".red(), rel.display(), count);
        }
    }
}

fn score_colored(score: u8) -> colored::ColoredString {
    let s = score.to_string();
    if score == 0 { s.green().bold() }
    else if score < 25 { s.yellow() }
    else if score < 50 { s.yellow().bold() }
    else { s.red().bold() }
}

fn cmd_rules_list() {
    let profile = config::default_profile();
    println!("{}", "available rules:".bold());
    for (name, rc) in &profile.rules {
        let status = if rc.enabled {
            "on".green().to_string()
        } else {
            "off".dimmed().to_string()
        };
        println!("  {} [{}]", name, status);
    }
}

fn cmd_profile_show(file: Option<PathBuf>) {
    let profile = match file {
        Some(ref p) => load_profile_or_default(Some(p)),
        None => config::default_profile(),
    };
    let yaml = serde_yaml::to_string(&profile).unwrap();
    println!("{}", yaml);
}

fn cmd_profile_validate(file: PathBuf) {
    match config::load_profile(&file) {
        Ok(p) => println!("{} profile '{}' is valid.", "\u{2714}".green(), p.name),
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

fn cmd_adapters_list() {
    let registry = adapters::Registry::default();
    println!("{}", "registered adapters:".bold());
    for (name, exts, category, can_write) in registry.list() {
        let mode = if can_write { "read/write" } else { "read-only" };
        let ext_list: Vec<&str> = exts.iter().take(8).copied().collect();
        println!("  {} [{}] ({}) .{}",
            name, category, mode,
            ext_list.join(", ."));
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Scan { path, profile, json } => cmd_scan(path, profile, json),
        Commands::Apply { path, profile, yes } => cmd_apply(path, profile, yes),
        Commands::Restore { run_id } => cmd_restore(run_id),
        Commands::Health { path, json, profile } => cmd_health(path, profile, json),
        Commands::Rules { action } => match action {
            RulesAction::List => cmd_rules_list(),
        },
        Commands::Profile { action } => match action {
            ProfileAction::Show { file } => cmd_profile_show(file),
            ProfileAction::Validate { file } => cmd_profile_validate(file),
        },
        Commands::Adapters { action } => match action {
            AdaptersAction::List => cmd_adapters_list(),
        },
    }
}
