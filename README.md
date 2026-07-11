# Bash-EM ("Bash-EM" — bash them)

A ratatui TUI that hunts down LLM typography — em-dashes, en-dashes,
horizontal bars, and their HTML entities — across a directory tree and
replaces them with honest hyphens.

## Replacement rules
| target | example | becomes |
|---|---|---|
| em-dash U+2014 | `word—word`, `word — word` | `word - word` (spaces normalized) |
| horizontal bar U+2015 | same as em-dash | `word - word` |
| en-dash U+2013 | `2019–2024` | `2019-2024` (stays tight) |
| entities | `&mdash; &ndash; &#8212; &#x2014; &horbar;` … | same as their chars |
| dash runs | `wait——what` | `wait - what` |

## Usage
```
cargo run --release -- /path/to/dir     # defaults to . if omitted
```
Scan runs immediately (read-only). Nothing is written until you press `a`.

Keys: `↑↓`/`jk` navigate · `space` include/exclude file · `a` apply ·
`l` save log (`Bash-EM-<epoch>.log`, full before/after per file) · `q` quit

## Safety model
- Scan phase is pure read; apply is a separate explicit step
- Text sniffing: valid UTF-8 + no NUL bytes = text; binaries auto-skipped
- Skips `.git`, `node_modules`, `target`, hidden dirs, symlinks, files >10MB
- Atomic writes: temp file + rename, never a half-written file
- CRLF line endings and missing trailing newlines round-trip exactly

## Layout
- `replacer.rs` — pure transform logic + 10 unit tests (`cargo test`)
- `scanner.rs` — tree walk + sniff on worker thread, mpsc streaming
- `app.rs` — state machine (Scanning → Review → Applying → Done), log writer
- `ui.rs` — all drawing; reads state, never mutates
- `main.rs` — terminal lifecycle, event loop, keybinds
