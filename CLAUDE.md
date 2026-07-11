# Bash-EM — Claude Code Instructions

Same repo as `AGENTS.md`. Read that file for layout and conventions; this file repeats the one rule that matters most for privacy.

## `_examples/` — do not read by default

`_examples/` is **gitignored** local fixture data (HTML, PDF, XLSX, website snapshots) for manual scanner testing. It is not in the remote repo and may contain private content.

### Forbidden unless the user explicitly asks

- `Read`, `cat`, `grep`, `rg`, or semantic search **inside** `_examples/` file contents
- Quoting or summarizing content from files under `_examples/`
- Using `_examples/` files as context for code changes without a direct user request

### Allowed

- **Tree listing only**: `ls`, `find … -print`, `tree`, glob by path/name — structure and metadata, not contents
- Mentioning that local fixtures live in `_examples/` when explaining how to test

### Explicit opt-in required

The user must clearly request content access, e.g.:

- "Read `_examples/HTML/ADBE_tearsheet.html`"
- "Run Bash-EM on `_examples/`"
- "What's in the PDF examples?"

If you need fixture content to proceed, **list the tree and ask** — do not read first.

## Quick reference

- **Build:** `cargo build --release`
- **Test:** `cargo test`
- **Run:** `cargo run --release -- [path]`
- **Core modules:** `replacer.rs`, `scanner.rs`, `app.rs`, `ui.rs`, `main.rs`
