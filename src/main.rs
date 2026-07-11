//! Bash-EM: bash them (dashes).
//!
//! Usage:  Bash-EM [path]        (defaults to current directory)
//!
//! main.rs owns the terminal lifecycle and the event loop. The pattern:
//!   1. spawn scanner on a worker thread, results stream over an mpsc channel
//!   2. loop: poll keyboard (with timeout) -> drain channel -> redraw
//!   3. restore the terminal NO MATTER WHAT (raw mode left on = broken shell)

mod app;
mod replacer;
mod scanner;
mod ui;

use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::{App, Phase};
use scanner::ScanEvent;

/// Events the apply worker streams back.
enum ApplyEvent {
    Applied(replacer::Counts),
    Failed,
    Done,
}

fn main() -> io::Result<()> {
    // --- args: just a path, default "." ---
    let root: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let root = root.canonicalize().unwrap_or(root);

    if !root.is_dir() {
        eprintln!("Bash-EM: '{}' is not a directory", root.display());
        std::process::exit(1);
    }

    // --- terminal setup ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app; capture the result so we ALWAYS restore the terminal
    // before propagating any error. (Poor man's RAII guard.)
    let result = run(&mut terminal, root);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, root: PathBuf) -> io::Result<()> {
    let mut app = App::new(root.clone());

    // Scanner worker. `move` transfers ownership of root+tx into the thread —
    // this is the borrow checker forcing thread-safety on you, and it's right.
    let (scan_tx, scan_rx) = mpsc::channel::<ScanEvent>();
    {
        let root = root.clone();
        std::thread::spawn(move || scanner::scan(root, scan_tx));
    }

    // Apply worker channel — created lazily when the user hits 'a'.
    let mut apply_rx: Option<mpsc::Receiver<ApplyEvent>> = None;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;
        app.tick = app.tick.wrapping_add(1);

        // Poll keyboard with a 50ms timeout: this is both our input check
        // AND our animation clock (~20fps redraw).
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Windows sends Press+Release; only act on Press.
                if key.kind == KeyEventKind::Press {
                    handle_key(key.code, &mut app, &mut apply_rx);
                }
            }
        }

        // Drain scanner events (non-blocking).
        while let Ok(ev) = scan_rx.try_recv() {
            match ev {
                ScanEvent::Dirty(report) => app.add_file(report),
                ScanEvent::Progress(scanned, skipped) => {
                    app.scanned = scanned;
                    app.skipped = skipped;
                }
                ScanEvent::Done(scanned, skipped, ms) => {
                    app.scanned = scanned;
                    app.skipped = skipped;
                    app.scan_ms = ms;
                    if app.phase == Phase::Scanning {
                        app.phase = Phase::Review;
                    }
                }
            }
        }

        // Drain apply events if an apply is running.
        if let Some(rx) = &apply_rx {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    ApplyEvent::Applied(counts) => {
                        app.applied_files += 1;
                        app.applied_counts.add(counts);
                    }
                    ApplyEvent::Failed => app.apply_errors += 1,
                    ApplyEvent::Done => {
                        app.phase = Phase::Done;
                        app.flash = format!(
                            "{} dashes purged from {} files",
                            app.applied_counts.total(),
                            app.applied_files
                        );
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_key(code: KeyCode, app: &mut App, apply_rx: &mut Option<mpsc::Receiver<ApplyEvent>>) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => {
            // Don't allow quitting mid-apply — half-applied trees are confusing.
            if app.phase != Phase::Applying {
                app.should_quit = true;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Char(' ') => {
            if app.phase == Phase::Review {
                app.toggle_selected();
            }
        }
        KeyCode::Char('l') => {
            if app.phase == Phase::Review || app.phase == Phase::Done {
                app.flash = match app.write_log() {
                    Ok(p) => format!("log saved: {}", p.display()),
                    Err(e) => format!("log failed: {e}"),
                };
            }
        }
        KeyCode::Char('a') => {
            if app.phase == Phase::Review && !app.files.is_empty() {
                app.phase = Phase::Applying;
                let paths: Vec<PathBuf> = app
                    .apply_set()
                    .into_iter()
                    .map(|i| app.files[i].path.clone())
                    .collect();
                let (tx, rx) = mpsc::channel::<ApplyEvent>();
                *apply_rx = Some(rx);
                std::thread::spawn(move || {
                    for p in paths {
                        match scanner::apply_file(&p) {
                            Ok(c) => { let _ = tx.send(ApplyEvent::Applied(c)); }
                            Err(_) => { let _ = tx.send(ApplyEvent::Failed); }
                        }
                        // Tiny stagger so the gauge visibly fills instead of
                        // teleporting to 100% — pure theater, remove if impatient.
                        std::thread::sleep(Duration::from_millis(4));
                    }
                    let _ = tx.send(ApplyEvent::Done);
                });
            }
        }
        _ => {}
    }
}
