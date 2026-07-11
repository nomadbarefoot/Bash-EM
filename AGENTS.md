# Bash-EM — Agent Instructions

Rust ratatui TUI that scans a directory tree for LLM typography (em-dashes, en-dashes, HTML entities) and replaces them with hyphens after explicit user review.

## Project layout

| Path | Role |
|---|---|
| `src/replacer.rs` | Pure transform logic + unit tests |
| `src/scanner.rs` | Tree walk, text sniffing, atomic apply |
| `src/app.rs` | State machine, log writer |
| `src/ui.rs` | Drawing only |
| `src/main.rs` | Terminal lifecycle, event loop |
| `docs/` | Product docs, phase plans, UI mockups |
| `scripts/` | Repo maintenance scripts |
| `_examples/` | **Local-only fixtures — see boundary below** |

Build: `cargo build --release` · Test: `cargo test`

## `_examples/` boundary (mandatory)

`_examples/` is **gitignored**. It holds private local test fixtures (HTML, PDF, XLSX, website snapshots) used to exercise the scanner on real-world files.

**Default rule — do not read contents:**

- Do **not** open, read, grep, or search inside any file under `_examples/`
- Do **not** use those files as implementation context unless the user **explicitly** asks you to read a specific path there

**Allowed without asking:**

- List the directory **tree** only (folder names, file names, counts, sizes)
- Refer to `_examples/` as "local fixtures exist" in docs or explanations

**Only when the user explicitly requests it:**

- Read, edit, or run tools against a named file or subtree under `_examples/`
- Example triggers: "read `_examples/HTML/foo.html`", "scan `_examples/`", "fix the PDF in examples"

When in doubt, list the tree and ask before reading.

## Conventions

- Package/binary name: **Bash-EM** (not `bash-m`)
- Minimize scope; match existing Rust style in `src/`
- Scan is read-only; apply is a separate explicit step — preserve that safety model
- Do not commit `_examples/`, `target/`, or `.DS_Store`
