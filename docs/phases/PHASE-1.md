# Phase 1 — Foundation (engine, core systems, text)

**Goal:** Replace the PoC monolith with a workspace where the engine, backup, diff, and config are real crates — and plain-text scrubbing still works end-to-end.

**Exit criteria:** `cargo test` green across crates; CLI can `scan` / `apply` / `restore` a Markdown tree with typographic rules; YAML profile loads; every apply writes a restoreable snapshot; TUI can stay as a thin temporary shell or deferred to a stub binary.

**Depends on:** nothing (starts from current PoC).  
**Unlocks:** Phase 2 adapters + Phase 3 hub.

---

## Scope

### In

- Cargo workspace + crate split (see PRODUCT.md)
- Engine: `Rule` trait, pipeline, `Edit` / span model, `Counts`
- Ship **Typographic** rules ported from PoC (`replacer.rs`)
- `bash-em-config`: load/validate YAML profiles + prefs + ignore patterns
- `bash-em-backup`: content-addressed (or simple hashed) file snapshots + manifest
- `bash-em-diff`: line hunks from before/after text units
- `bash-em-adapters`: **text adapter only** (UTF-8 sniff, line-preserving like today)
- `bash-em` binary: clap CLI (`scan`, `apply`, `restore`, `rules`, `profile`)
- Migrate PoC unit tests into `bash-em-engine`
- Default profile YAML under `config/`

### Out

- PDF / XLSX / DOCX adapters
- Full hub UI, mouse, Tetris
- LLM-tell phrase rules (stub trait ok; implementation Phase 2)
- Directory health bar (model types may be sketched; UI is Phase 3)

---

## Target layout

```
Bash-EM/
├── Cargo.toml                 # workspace
├── config/
│   └── default.profile.yaml
├── crates/
│   ├── bash-em-engine/
│   ├── bash-em-backup/
│   ├── bash-em-diff/
│   ├── bash-em-config/
│   ├── bash-em-adapters/      # text module only this phase
│   └── bash-em/               # CLI binary
├── docs/
└── src/                       # REMOVE or thin-wrap after migration
```

PoC `src/{replacer,scanner,app,ui,main}.rs` is the donor. Logic moves out; do not grow the monolith.

---

## Engine contract

```
TextUnit { id, text, meta }
Rule::find(unit, ctx) -> Vec<Match>      # spans + rule_id + replacement
Pipeline::run(units, rules, profile) -> BatchEdits
Adapter::read(path) -> Vec<TextUnit>
Adapter::write(path, units, edits) -> ()
```

Rules never open files. Adapters never choose replacements. Backup wraps apply:

```
plan → backup.snapshot(paths) → adapter.write per file → diff.record(run)
```

---

## Config sketch (YAML)

```yaml
# config/default.profile.yaml
name: typographic
rules:
  em_dash: { enabled: true }
  en_dash: { enabled: true }
  horizontal_bar: { enabled: true }
  html_dash_entities: { enabled: true }
  curly_quotes: { enabled: false }    # ready, off by default in P1
prefs:
  max_file_bytes: 10485760
  preview_lines: 8
  backup_dir: "~/.bash-em/backups"
  skip_dirs: [".git", "node_modules", "target", "dist", "build"]
ignore:
  - "**/*.min.js"
  - "**/package-lock.json"
```

CLI and (later) TUI both read/write this schema via `bash-em-config`.

---

## Backup / restore (minimum viable core)

```
~/.bash-em/backups/<run_id>/
  manifest.json    # root, timestamp, profile hash, files[]
  files/<sha256>   # original bytes
```

- Snapshot **before** first write of each file in the run  
- `bash-em restore <run_id>` copies bytes back  
- Manifest lists relative paths + hashes + rule profile id  

Diff crate stores optional sidecar `run_report.json` (counts + per-file hunk summaries) next to the backup or in cwd when `--report` is set.

---

## CLI surface (Phase 1)

```
bash-em scan   <path> [--profile FILE] [--json]
bash-em apply  <path> [--profile FILE] [--yes]
bash-em restore <run_id>
bash-em rules  list
bash-em profile show|validate [FILE]
```

Interactive TUI optional in P1: either keep a minimal Review screen wired to new crates, or CLI-only until Phase 3. Prefer **CLI-first green path** so adapters can land without UI blockers.

---

## Work sequence

1. Create workspace + empty crates with correct dependency edges  
2. Move dash transform into `engine` as `TypographicDashRule` (+ tests)  
3. Define `TextUnit` / `Edit` / `Pipeline`  
4. Implement text adapter (sniff, walk, skip dirs) using config  
5. Implement backup + wire into apply  
6. Implement diff hunk builder for reports/logs  
7. YAML config load/validate  
8. CLI commands  
9. Delete or stub old PoC `src/` once parity proven  
10. README pointer to docs + `cargo test` / example scan on a fixture tree  

---

## Verification

- [ ] Engine tests cover all former `replacer` cases  
- [ ] Apply on fixture mutates files and creates restoreable run  
- [ ] Restore returns byte-identical originals  
- [ ] Invalid YAML profile fails loudly  
- [ ] Binaries/NUL files skipped; CRLF preserved on text  
- [ ] No crate in `adapters` depends on `tui`  

---

## Risks

| Risk | Mitigation |
|------|------------|
| Span model too line-naive for later PDF | Keep `TextUnit` opaque id + byte/char range; adapters own mapping |
| Backup disk bloat | Hash dedupe + document retention as P2 enhancement |
| Over-building TUI in P1 | CLI exit criteria only; UI polish is Phase 3 |
