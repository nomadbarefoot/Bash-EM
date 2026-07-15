use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, poll, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::{App, ApplyStats, RunRow, ScanPhase};
use crate::event::{handle_key, handle_mouse, LayoutCache};
use crate::screens;

const TICK_MS: u64 = 100;

enum WorkerEvent {
    ScanProgress {
        generation: u64,
        progress: workflow::ScanProgress,
    },
    ScanComplete {
        generation: u64,
        result: Result<workflow::ScanReport, String>,
    },
    ApplyComplete(Result<workflow::ApplyReport, String>),
    RestoreComplete {
        root: std::path::PathBuf,
        result: Result<workflow::RestoreReport, String>,
    },
}

pub fn run(mut app: App) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app);

    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    );
    let _ = terminal.show_cursor();
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let mut layout_cache = LayoutCache::new();
    let mut last_tick = Instant::now();
    let (worker_tx, worker_rx) = mpsc::channel::<WorkerEvent>();
    let mut scan_generation = 0u64;
    let mut scan_cancel: Option<Arc<AtomicBool>> = None;

    load_backup_runs(app);

    loop {
        terminal.draw(|frame| screens::draw(frame, app, &mut layout_cache))?;
        let timeout = Duration::from_millis(TICK_MS).saturating_sub(last_tick.elapsed());
        if poll(timeout)? {
            match event::read()? {
                Event::Key(key) => handle_key(app, key, &layout_cache),
                Event::Mouse(mouse) => handle_mouse(app, mouse, &layout_cache),
                _ => {}
            }
        }

        if last_tick.elapsed() >= Duration::from_millis(TICK_MS) {
            app.tick = app.tick.wrapping_add(1);
            last_tick = Instant::now();
        }

        if app.reload_backups {
            app.reload_backups = false;
            load_backup_runs(app);
        }

        if let Some(root) = app.pending_scan.take() {
            if let Some(cancel) = scan_cancel.take() {
                cancel.store(true, Ordering::Relaxed);
            }
            scan_generation = scan_generation.wrapping_add(1);
            let generation = scan_generation;
            let cancel = Arc::new(AtomicBool::new(false));
            scan_cancel = Some(cancel.clone());
            let sender = worker_tx.clone();
            let profile = app.profile.clone();
            std::thread::spawn(move || {
                let result = workflow::scan(
                    workflow::ScanRequest { root, profile },
                    |progress| {
                        let _ = sender.send(WorkerEvent::ScanProgress {
                            generation,
                            progress,
                        });
                    },
                    &cancel,
                );
                let _ = sender.send(WorkerEvent::ScanComplete { generation, result });
            });
        }

        if app.pending_apply {
            app.pending_apply = false;
            if let Some(report) = app.scan_report.clone() {
                let included = app.included_paths();
                let sender = worker_tx.clone();
                std::thread::spawn(move || {
                    let _ = sender.send(WorkerEvent::ApplyComplete(workflow::apply(
                        &report, &included,
                    )));
                });
            } else {
                app.scan_phase = ScanPhase::Review;
                set_error(app, "scan result unavailable; rescan required".to_string());
            }
        }

        if let Some((run_id, root)) = app.pending_restore.take() {
            let sender = worker_tx.clone();
            let profile = app.profile.clone();
            std::thread::spawn(move || {
                let result = workflow::restore(&profile, &run_id);
                let _ = sender.send(WorkerEvent::RestoreComplete { root, result });
            });
        }

        while let Ok(message) = worker_rx.try_recv() {
            match message {
                WorkerEvent::ScanProgress {
                    generation,
                    progress,
                } if generation == scan_generation => {
                    app.scan_stats.progress = match progress {
                        workflow::ScanProgress::Walking => "walking directory…".to_string(),
                        workflow::ScanProgress::Discovered { files } => {
                            format!("{files} readable files found")
                        }
                        workflow::ScanProgress::Processing { completed, total } => {
                            format!("processing {completed}/{total}")
                        }
                    };
                }
                WorkerEvent::ScanComplete { generation, result }
                    if generation == scan_generation =>
                {
                    scan_cancel = None;
                    match result {
                        Ok(report) => {
                            let offenders = report.totals.total();
                            let files = report.files.len();
                            app.accept_scan_report(report);
                            if app.preserve_next_scan_flash {
                                app.preserve_next_scan_flash = false;
                            } else if app.apply_stats.backup_run_id.is_none() {
                                app.flash = if offenders == 0 {
                                    "clean tree".to_string()
                                } else if files == 0 {
                                    format!("{offenders} review-only LLM flags")
                                } else {
                                    format!("{offenders} offenders in {files} files")
                                };
                                app.flash_color = if offenders == 0 {
                                    app.theme.clean
                                } else {
                                    app.theme.guilty
                                };
                            }
                        }
                        Err(error) if error == "scan cancelled" => {}
                        Err(error) => {
                            app.preserve_next_scan_flash = false;
                            app.scan_phase = if app.finish_scan_as_done {
                                app.finish_scan_as_done = false;
                                ScanPhase::Done
                            } else {
                                ScanPhase::Idle
                            };
                            set_error(app, error);
                        }
                    }
                }
                WorkerEvent::ApplyComplete(result) => match result {
                    Ok(report) => {
                        app.apply_stats = ApplyStats {
                            applied_files: report.applied_files,
                            applied_counts: report.applied_counts,
                            errors: report.errors.clone(),
                            backup_run_id: report.run_id.clone(),
                            backup_dir: Some(report.backup_dir.clone()),
                            pruned_runs: report.pruned_runs,
                        };
                        app.flash = if report.errors.is_empty() {
                            format!(
                                "{} files cleaned · backup retained{}",
                                report.applied_files,
                                if report.pruned_runs > 0 {
                                    format!(" · pruned {} old run(s)", report.pruned_runs)
                                } else {
                                    String::new()
                                }
                            )
                        } else {
                            format!("partial apply: {}", report.errors.join("; "))
                        };
                        app.flash_color = if report.errors.is_empty() {
                            app.theme.clean
                        } else {
                            app.theme.guilty
                        };
                        load_backup_runs(app);
                        let _ = app.reload_browse();
                        queue_refresh_scan(app);
                    }
                    Err(error) => {
                        app.scan_phase = ScanPhase::Review;
                        set_error(app, error);
                    }
                },
                WorkerEvent::RestoreComplete { root, result } => match result {
                    Ok(report) => {
                        app.restore_in_progress = false;
                        app.flash = format!(
                            "restored {} files · backup retained in {}",
                            report.restored_files,
                            report.backup_dir.display()
                        );
                        app.flash_color = app.theme.clean;
                        app.apply_stats = ApplyStats::default();
                        load_backup_runs(app);
                        let _ = app.reload_browse();
                        if root == app.root {
                            queue_refresh_scan(app);
                        }
                    }
                    Err(error) => {
                        app.restore_in_progress = false;
                        set_error(app, format!("restore error: {error}"));
                    }
                },
                _ => {}
            }
        }

        if app.should_quit {
            if let Some(cancel) = scan_cancel {
                cancel.store(true, Ordering::Relaxed);
            }
            break;
        }
    }
    Ok(())
}

fn queue_refresh_scan(app: &mut App) {
    app.finish_scan_as_done = true;
    app.preserve_next_scan_flash = true;
    app.pending_scan = Some(app.root.clone());
    app.scan_phase = ScanPhase::Scanning;
    app.scan_stats.progress = "refreshing scan state…".to_string();
    app.scan_report = None;
    app.files.clear();
    app.excluded.clear();
    app.list_selected = 0;
    app.diff_scroll = 0;
    app.health = None;
}

fn load_backup_runs(app: &mut App) {
    let runs = workflow::list_backups(&app.profile).unwrap_or_default();
    app.runs = runs
        .into_iter()
        .map(|run| RunRow {
            when_relative: relative_time(&run.timestamp),
            run_id: run.run_id,
            timestamp: run.timestamp,
            file_count: run.file_count,
            root: run.root,
            profile_name: run.profile_name,
        })
        .collect();
    app.runs_selected = app.runs_selected.min(app.runs.len().saturating_sub(1));
}

fn relative_time(epoch: &str) -> String {
    let timestamp = epoch.parse::<u64>().unwrap_or(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    match now.saturating_sub(timestamp) {
        seconds if seconds < 60 => "just now".to_string(),
        seconds if seconds < 3_600 => format!("{}m ago", seconds / 60),
        seconds if seconds < 86_400 => format!("{}h ago", seconds / 3_600),
        seconds => format!("{}d ago", seconds / 86_400),
    }
}

fn set_error(app: &mut App, error: String) {
    app.flash = error;
    app.flash_color = app.theme.guilty;
}
