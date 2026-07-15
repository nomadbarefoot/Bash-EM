# Bash-EM

Bash-EM is a local-first Rust TUI and CLI that finds typography associated with generated prose, previews the exact changes, and replaces it only after explicit review. Every apply creates a retained restore point first.

The current trustworthy MVP supports UTF-8 text, code, and web files. The TUI can adjust and persist active rules. DOCX, XLSX, PDF, and add-ons remain roadmap work.

## Run

```sh
cargo run -p bash-em --                 # open the TUI in the current directory
cargo run -p bash-em -- tui /path       # open the TUI at a directory
cargo run -p bash-em -- scan /path
cargo run -p bash-em -- apply /path
cargo run -p bash-em -- backups list
```

The convenience script is equivalent to the explicit TUI command:

```sh
./run-tui.sh /path/to/start
```

## TUI workflow

1. Open **Browse**. `Enter` descends into a directory; `Backspace` or `Esc` goes up.
2. Use `~` for home, `r` for the filesystem root, and `.` to show hidden entries.
3. Press `s` to select the current directory and start a background scan.
4. Review offenders and diffs. `Space` excludes or includes a file.
5. Press `a`, confirm, and apply. A backup is sealed before the first write.
6. Open **Backups** and press `r` or `Enter` to restore a retained run.
7. Open **Profiles** with `5`; use `Space` or `Enter` to change rules, `w` to save, and `l` to reload.

`q` is the only quit key and is always shown in red. `Esc` returns to Welcome from task screens, moves up one directory in Browse, and dismisses confirmation prompts. The footer shows the active keys for every screen; tabs `1` through `5` switch between Welcome, Browse, Scan, Backups, and Profiles.

## Project configuration

Bash-EM automatically loads `<scan-root>/.bash-em.yaml`. If it does not exist, the built-in profile is used and pressing `w` in Profiles creates it atomically. An explicit `--profile FILE` takes precedence and remains pinned when browsing to another root.

Add `<scan-root>/.bash-emignore` for gitignore-compatible exclusions, including comments, directory patterns, and `!` negation:

```gitignore
generated/*
*.min.js
!generated/keep.md
```

Both project control files are excluded from scans automatically.

## CLI

```text
bash-em scan <path> [--profile FILE] [--json]
bash-em apply <path> [--profile FILE] [--yes]
bash-em health <path> [--profile FILE] [--json]
bash-em backups list [--profile FILE]
bash-em restore <run-id> [--profile FILE]
bash-em rules list
bash-em profile show [FILE]
bash-em profile validate <FILE>
bash-em adapters list
```

The active adapter list intentionally contains only the text adapter until structured-format write-back is integrated and fixture-tested.

## Safety model

- Scans are read-only, explicit, cancellable, and run off the TUI thread.
- Apply rejects files that changed after preview.
- All selected originals are snapshotted and the manifest is sealed before writing.
- Writes are atomic and preserve existing permissions, including executable bits on Unix.
- Restore validates paths and backup hashes before changing any destination.
- Restore points remain in `~/.bash-em/backups`; the path is displayed in the TUI and CLI.
- Symlinks, hidden directories, profile globs, `.bash-emignore` matches, binary files, empty files, and oversized files are skipped and counted.

Default replacement rules include em dash, en dash, horizontal bar, HTML dash entities, and zero-width characters. Curly quotes and ellipses are opt-in through a YAML profile.

## Workspace

```text
engine       pure replacement and health models
adapters     filesystem walk and permission-safe text I/O
backup       retained manifests, snapshots, and restore
workflow     shared scan/apply/restore orchestration
config       YAML profiles and preferences
diff         report DTOs
bash-em-tui  interactive state, input, and rendering
bash-em      CLI composition root
```

Run the full verification suite with:

```sh
cargo fmt --check
cargo test --workspace --all-targets
```
