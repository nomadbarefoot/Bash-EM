use std::fs;
use std::io;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, poll};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::{App, BrowseEntry, RunRow, ScanFile, ScanPhase};
use crate::event::{handle_key, handle_mouse, LayoutCache};
use crate::screens;

const TICK_MS: u64 = 100;

pub fn run(mut app: App) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut layout_cache = LayoutCache::new();
    let mut last_tick = Instant::now();

    app.health_scanning = true;
    run_health_scan(&mut app);
    load_backup_runs(&mut app);
    load_browse_entries(&mut app);

    let mut scan_batch: Option<Vec<engine::FileEdits>> = None;

    loop {
        terminal.draw(|frame| {
            screens::draw(frame, &app, &mut layout_cache);
        })?;

        let timeout = Duration::from_millis(TICK_MS)
            .saturating_sub(last_tick.elapsed());

        if poll(timeout)? {
            match event::read()? {
                Event::Key(key) => handle_key(&mut app, key),
                Event::Mouse(mouse) => handle_mouse(&mut app, mouse, &layout_cache),
                _ => {}
            }
        }

        if last_tick.elapsed() >= Duration::from_millis(TICK_MS) {
            app.tick = app.tick.wrapping_add(1);
            if let Some(addon) = &mut app.addon {
                addon.tick(TICK_MS);
            }
            last_tick = Instant::now();
        }

        if app.scan_phase == ScanPhase::Scanning {
            let batch = run_scan(&mut app);
            app.scan_phase = if app.files.is_empty() {
                app.flash = "no offenders found".to_string();
                app.flash_color = app.theme.clean;
                ScanPhase::Done
            } else {
                ScanPhase::Review
            };
            scan_batch = Some(batch);
        }

        if app.scan_phase == ScanPhase::Applying {
            if let Some(batch) = scan_batch.take() {
                run_apply(&mut app, &batch);
            }
            app.scan_phase = ScanPhase::Done;
            load_backup_runs(&mut app);
        }

        if let Some(run_id) = app.pending_restore.take() {
            let backup_dir = config::resolve_backup_dir(&app.profile.prefs);
            match backup::restore(&backup_dir, &run_id) {
                Ok(count) => {
                    app.flash = format!("restored {} files", count);
                    app.flash_color = app.theme.clean;
                }
                Err(e) => {
                    app.flash = format!("restore error: {}", e);
                    app.flash_color = app.theme.guilty;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::event::DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

fn build_fix_options(profile: &config::Profile) -> engine::FixOptions {
    let pairs: Vec<(String, bool)> = profile.rules.iter()
        .map(|(k, v)| (k.clone(), v.enabled))
        .collect();
    engine::FixOptions::from_profile(&pairs)
}

fn categorize_ext(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("md" | "txt" | "rst") => "text",
        Some("rs" | "py" | "js" | "ts" | "go" | "rb") => "code",
        Some("html" | "htm" | "css") => "web",
        Some("xlsx") => "office",
        Some("docx") => "docs",
        Some("pdf") => "pdf",
        _ => "other",
    }
}

fn run_health_scan(app: &mut App) {
    let opts = build_fix_options(&app.profile);
    let (candidates, stats) = adapters::walk_tree(&app.root, &app.profile.prefs);
    let pipeline = engine::Pipeline::with_options(
        app.profile.prefs.preview_lines, opts, app.profile.prefs.fence_guard,
    );

    let mut health = engine::health::HealthReport::new(
        app.root.clone(), stats.scanned, stats.skipped,
    );

    for cand in &candidates {
        let edits = pipeline.process_content(cand.path.clone(), &cand.content);
        let counts = edits.as_ref().map(|e| e.counts).unwrap_or_default();
        let category = categorize_ext(&cand.path);
        health.add_file(cand.path.clone(), &counts, category);
    }

    health.finalize();
    app.health = Some(health);
    app.health_scanning = false;
}

fn run_scan(app: &mut App) -> Vec<engine::FileEdits> {
    let scan_start = Instant::now();
    let opts = build_fix_options(&app.profile);
    let (candidates, stats) = adapters::walk_tree(&app.root, &app.profile.prefs);
    let pipeline = engine::Pipeline::with_options(
        app.profile.prefs.preview_lines, opts, app.profile.prefs.fence_guard,
    );
    let batch = pipeline.process_batch(
        candidates.into_iter().map(|c| (c.path, c.content)).collect()
    );

    app.scan_stats.scanned = stats.scanned;
    app.scan_stats.skipped = stats.skipped;
    app.scan_stats.scan_ms = scan_start.elapsed().as_millis().max(1);
    app.files.clear();
    app.excluded.clear();
    app.list_selected = 0;
    app.list_offset = 0;

    for file_edits in &batch.files {
        app.add_scan_file(ScanFile {
            path: file_edits.path.clone(),
            counts: file_edits.counts,
            lines_changed: file_edits.lines_changed,
            changes: file_edits.changes.clone(),
        });
    }

    app.flash = format!("{} offenders in {} files", batch.totals.total(), batch.files.len());
    app.flash_color = if batch.files.is_empty() { app.theme.clean } else { app.theme.guilty };

    batch.files
}

fn run_apply(app: &mut App, batch: &[engine::FileEdits]) {
    let backup_dir = config::resolve_backup_dir(&app.profile.prefs);
    let apply_indices = app.apply_set();

    let begin = backup::begin_run(&backup_dir, &app.root, &app.profile.name);
    let (run_dir, run_id, mut manifest) = match begin {
        Ok(v) => v,
        Err(e) => {
            app.flash = format!("backup error: {}", e);
            app.flash_color = app.theme.guilty;
            return;
        }
    };

    let mut applied = 0usize;
    let mut errors = 0usize;
    let mut applied_counts = engine::Counts::default();

    for &idx in &apply_indices {
        let scan_file = &app.files[idx];
        let file_edits = match batch.iter().find(|fe| fe.path == scan_file.path) {
            Some(fe) => fe,
            None => continue,
        };

        if let Ok(entry) = backup::snapshot_file(&run_dir, &file_edits.path, &app.root) {
            manifest.files.push(entry);
        }

        let tmp = file_edits.path.with_extension("bashm_tmp");
        match fs::write(&tmp, &file_edits.new_content)
            .and_then(|_| fs::rename(&tmp, &file_edits.path))
        {
            Ok(_) => {
                applied += 1;
                applied_counts.add(file_edits.counts);
            }
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                app.flash = format!("error: {}", e);
                app.flash_color = app.theme.guilty;
                errors += 1;
            }
        }

        app.apply_stats.applied_files = applied;
        app.apply_stats.errors = errors;
    }

    let _ = backup::seal_manifest(&run_dir, &manifest);
    let _ = backup::prune_old_runs(&backup_dir, app.profile.prefs.keep_last_n);

    app.apply_stats.applied_files = applied;
    app.apply_stats.applied_counts = applied_counts;
    app.apply_stats.errors = errors;
    app.apply_stats.backup_run_id = Some(run_id);

    app.flash = format!("{} files cleaned", applied);
    app.flash_color = app.theme.clean;
}

fn load_backup_runs(app: &mut App) {
    let backup_dir = config::resolve_backup_dir(&app.profile.prefs);
    let runs = backup::list_runs(&backup_dir).unwrap_or_default();
    app.runs = runs.into_iter().map(|r| {
        RunRow {
            when_relative: relative_time(&r.timestamp),
            run_id: r.run_id,
            timestamp: r.timestamp.clone(),
            file_count: r.file_count,
            root: r.root,
        }
    }).collect();
    app.runs_selected = 0;
}

fn load_browse_entries(app: &mut App) {
    let mut entries = Vec::new();
    collect_dir(&app.root, &app.root, &app.profile.prefs, 0, &mut entries, &app.health);
    app.browse_entries = entries;
    app.browse_selected = 0;
    app.browse_offset = 0;
}

fn collect_dir(
    dir: &std::path::Path,
    root: &std::path::Path,
    prefs: &config::Prefs,
    depth: usize,
    out: &mut Vec<BrowseEntry>,
    health: &Option<engine::health::HealthReport>,
) {
    let mut dir_entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(rd) => rd.flatten().collect(),
        Err(_) => return,
    };
    dir_entries.sort_by(|a, b| {
        let a_dir = a.path().is_dir();
        let b_dir = b.path().is_dir();
        b_dir.cmp(&a_dir).then_with(|| a.file_name().cmp(&b.file_name()))
    });

    for entry in dir_entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') { continue; }
        if prefs.skip_dirs.iter().any(|d| d == &name) { continue; }

        let is_dir = path.is_dir();
        let artifact_count = health.as_ref()
            .and_then(|h| h.top_files.iter().find(|(p, _)| p == &path).map(|(_, c)| *c))
            .unwrap_or(0);

        let category = if is_dir {
            String::new()
        } else {
            categorize_ext(&path).to_string()
        };

        out.push(BrowseEntry {
            path: path.clone(),
            is_dir,
            category,
            artifact_count,
            depth,
        });

        if is_dir && depth < 3 {
            collect_dir(&path, root, prefs, depth + 1, out, health);
        }
    }
}

fn relative_time(epoch_str: &str) -> String {
    let epoch: u64 = epoch_str.parse().unwrap_or(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let diff = now.saturating_sub(epoch);
    if diff < 60 { return "just now".to_string(); }
    if diff < 3600 { return format!("{}m ago", diff / 60); }
    if diff < 86400 { return format!("{}h ago", diff / 3600); }
    format!("{}d ago", diff / 86400)
}
