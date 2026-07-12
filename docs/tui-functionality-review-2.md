# Bash-EM Secondary Review — Engine, Adapters, Config, CLI, and the Corners

**Date:** 2026-07-12
**Scope:** Everything the first review (`tui-functionality-review.md`) skimmed: the `engine` crate (replacer, pipeline, health, boilerplate), the `adapters` crate (walker, registry, docx/xlsx/pdf/zip), `config`, `diff`, the CLI command paths, remaining TUI modules (`welcome`, `common`, `vscroll`, `addon`), docs, and test coverage. Findings tagged **[verified live]** were reproduced by running the built binary against fixtures; the rest are pinned to file:line.

**Headline:** The engine core is the healthiest part of the codebase — well-tested, CRLF-safe, atomic-write correct. But it has one real counting bug that inflates every number the UI shows, the apply path silently destroys file permissions, the CLI apply ignores the profile it was given, and a whole adapter subsystem (docx/xlsx/pdf/zip) plus a scroll-state module exist as fully-written dead code that the UI advertises as live features.

---

## 1. Engine correctness

### 1.1 HTML entities are double-counted — every total in the app is inflated **[verified live]**
`decode_entities` (`crates/engine/src/replacer.rs:85-112`) counts `entities += 1` and rewrites `&mdash;` to a literal U+2014 — which the very next pass then counts *again* as `em += 1` (`replacer.rs:158`). Verified: a file containing exactly one `&mdash;` reports `em:1, entities:1, Σ2`. The existing test (`entity_mdash`, replacer.rs:305) asserts `entities == 2` but never asserts `em == 0`, so the bug is test-invisible. Consequences ripple everywhere: the Scan stats bar, the DONE panel's "N replacements", the Welcome corruption total and score, and the health JSON all overstate whenever entities are present. (This also explains round 1's fixture reporting Σ7 for 6 real artifacts.)

Fix direction: either don't re-count the decoded char (decode to a marker and count once), or treat "entities" as a sub-attribution and stop adding it into `total()`.

### 1.2 Apply destroys file permissions **[verified live]** — data-integrity class
The write path is `fs::write(tmp) + fs::rename` (`crates/adapters/src/text.rs:73-77`, duplicated in `run.rs:208-211` for the TUI). The tmp file is created with default mode, so the target's mode is replaced: a `chmod 755` script went in executable and came out **644** after one apply. Restore doesn't heal it — `fs::copy` from the backup blob (itself written 644) keeps 644. Any repo with executable scripts, git hooks, or restrictive perms is silently damaged by a clean/restore cycle. Fix: copy the source file's metadata onto the tmp before rename (`fs::metadata` → `set_permissions`), in both write paths, and preserve mode in `snapshot_file`/`restore`.

### 1.3 Em-dash policy note (by design, but document it)
Em-dashes become **spaced** hyphens (`word—word` → `word - word`, `replacer.rs:157-178`), including inside string literals of code files. The README documents this; just flagging that for `code` category files the space-insertion can change semantics (e.g. inside string constants) and there's no per-category policy knob.

### 1.4 Fence guard is decent but shallow
Toggle-on-``` logic (`replacer.rs:246-251`) doesn't pair fence markers (a ```` ``` ```` inside a `~~~` block flips state), doesn't handle indented fences per CommonMark, and an unterminated fence guards the rest of the file. Acceptable heuristic — but see 3.1 for how easily the guard silently turns off entirely.

### 1.5 Boilerplate detection is CLI-only; the TUI toggle is a placebo
`llm_boilerplate` appears in the TUI rules row and Profiles list, but `run_health_scan` (`run.rs:122-143`) never calls `health.add_boilerplate`, and no scan/apply path in the TUI touches `engine::boilerplate`. Only `bash-em health` / `bash-em scan` (CLI) honor it (`main.rs:188-214, 355-358`). In the TUI the toggle displays state that controls nothing (compounding round 1's finding 2.4 that toggles can't be toggled anyway).

## 2. Adapters — a subsystem built and never plugged in

### 2.1 The registry and all non-text adapters are dead code
`Registry`, the `Adapter` trait, and full implementations for docx (154 lines), xlsx (151), pdf (45), zip (130) exist — but the **only** consumer of `Registry::default()` is the `bash-em adapters list` CLI command (`main.rs:448`). The walker (`crates/adapters/src/walker.rs:71`) reads every file through `TextAdapter::read` directly; `Registry::resolve`, `probe`, `read_content`, `write_back` are never called by any scan or apply path.

Cascading dishonesty from this one gap:
- The Welcome screen's category grid renders tiles for **office / xlsx cells / pdf (text)** (`welcome.rs:219`) — these can never be non-zero, because binary files fail the UTF-8 sniff and are dropped as "skipped" before categorization.
- `bash-em adapters list` prints docx/xlsx/pdf as "registered adapters" with read/write capability — none of them will ever run.
- `TEXT_EXTENSIONS` (`text.rs:7-16`) is likewise never consulted by the walker; scanning is pure content-sniffing. (Harmless — arguably better — but the allowlist is dead weight.)

Decision needed: wire `Registry::resolve` into the walker (real work: the apply path must then route through `write_back` per adapter), or delete the adapters and the UI tiles that promise them.

### 2.2 Binary/oversized/empty files all lumped into one "skipped" number
`walker.rs:66-78`: >10 MB, zero-byte, and non-UTF-8 files all increment the same `skipped` counter. The UI presents "skipped N" with no breakdown, so users can't tell "skipped my 400 docx files" from "skipped node_modules cruft". Cheap fix: `skipped_binary` / `skipped_large` counters.

### 2.3 Symlinked trees are silently invisible
`walker.rs:42-44` skips all symlinks (loop safety — good) but nothing ever reports it. A root that is itself mostly symlinks (common in `~/Documents` setups) scans as near-empty with zero explanation.

## 3. Config traps

### 3.1 `fence_guard` silently flips off for any loaded profile that omits the key **[verified live]**
`Prefs::default()` sets `fence_guard: true` (`config/src/lib.rs:59`), but the field is `#[serde(default)]` (`lib.rs:36-37`) → **false** when deserialized without the key. Verified: same file, same content — default profile guards fenced code, `--profile` with a YAML lacking `fence_guard` rewrites dashes *inside* code fences. Worse: `bash-em profile show`'s own output round-trips fine, but any hand-trimmed profile hits this. Fix: `#[serde(default = "default_true")]` (the helper already exists).

### 3.2 `profile.ignore` globs are parsed and never applied
`ignore: ["**/*.min.js", "**/package-lock.json"]` ships in the default profile and `config/default.profile.yaml`, but no code path reads `profile.ignore` (grep confirms: zero references outside the struct definition). Minified JS and lockfiles — the exact files most likely to be full of legitimate en-dashes and entities — are scanned and offered for rewriting.

### 3.3 `config/default.profile.yaml` is never auto-loaded
The checked-in profile file is only used if explicitly passed via `--profile`. No search path (`./bash-em.yaml`, `~/.bash-em/profile.yaml`, etc.) exists. Users editing the repo's config file will see no effect — consistent with the TUI's "editable here or on disk" string being false on both counts.

## 4. CLI command paths

### 4.1 `bash-em apply` ignores the profile it scanned with **[verified live]** — correctness bug
`cmd_apply` scans with profile-derived `FixOptions`, then applies via `adapters::TextAdapter::apply(path, 0)` (`main.rs:277`), which re-runs `fix_content` with **defaults**. Verified with a profile enabling `curly_quotes`: apply reported "✔ 1 files bashed. 0 errors" and **left the file byte-identical** (curly quotes still there) — while still creating a backup run for it. Inverse hazard too: a profile that *disables* a default-on rule still gets that rule applied to disk. The TUI path is correct (writes the pipeline's `new_content`); make the CLI do the same instead of re-fixing.

### 4.2 `bash-em restore` can't see custom backup dirs
`cmd_restore` (`main.rs:306-308`) hardcodes `Prefs::default()` — no `--profile` flag exists on the subcommand. Anyone who set `backup_dir` to something custom can apply (backups land in the custom dir) but never restore from the CLI ("read manifest: No such file or directory").

### 4.3 No `backups list` command
The CLI can create and restore runs but not enumerate them — the run UUID must be scraped from old terminal output. Pairs with round 1's §3.1 (invisible backup location) as the CLI half of the trust gap.

## 5. TUI corners not covered in round 1

### 5.1 Flash/error messages are invisible on the two screens that produce them — trust-critical
`draw_footer` (`common.rs:230-266`) renders `app.flash` **only when the screen has no tagline and no second key row** — i.e. only on Welcome/Browse/Profiles. Scan and Backups always show their static taglines ("backup first. then we get messy.", "safety copy is not a bit.") instead. Consequences, all real paths: `restore error: …` (run.rs:86), `backup error: …` (run.rs:187), apply `error: …` (run.rs:218) are set while you're on Backups/Scan — **and never displayed**. During round 1's live restore, the "restored 3 files" confirmation only became visible after switching to the Profiles tab. A tool whose error channel is suppressed exactly where errors happen will feel untrustworthy even when it works.

### 5.2 The Browse footer documents the unimplemented feature set
`common.rs:183-193` hints `enter open · backspace up · s select dir` — none of these keys have handlers (round 1 §1.2 verified they're dead). Same pattern as `l log` (Scan), `e edit profile` (Backups), `t tetris focus` / "click everywhere" (Welcome — `app.addon` is always `None`; no addon implementation exists anywhere, only the trait in `addon.rs`). The footer is a spec, not a legend.

### 5.3 `vscroll.rs` already solves round 1's scroll bugs — it's just not wired in
`VScrollState` (`crates/bash-em-tui/src/vscroll.rs`) implements exactly the selection/offset/ensure-visible logic whose absence causes the stale-offset mouse-click bug (round 1 §2.6) and the local-offset recomputation in three draw functions. It's exported from `lib.rs` and referenced nowhere. Adopting it for offender list, runs table, and browse list fixes the offset family of bugs with code that already exists and even has `page_up/page_down` (also currently missing as keys).

### 5.4 Welcome details
- `relative_time_short` (`welcome.rs:358-360`) ignores its argument and returns the literal `"recent"` — "last run: recent" is hardcoded fiction.
- The "⌘ Scan & bash / Browse… / Backups" buttons (`welcome.rs:170-190`) are painted as buttons but are not click targets — no rects are registered in `LayoutCache` (only tabs are clickable).
- The mission-control spinner during health scan (`welcome.rs:37-67`) can never be seen spinning: the health scan blocks before the first frame (round 1 §5.3).

### 5.5 Diff crate: positional, not structural
`diff::build_diff` compares line N to line N (`diff/src/lib.rs:30-48`) — fine for this engine (pure 1:1 line rewrites, never inserts/deletes), but it's only used by `scan --json` with empty `run_id`/`timestamp` fields emitted as `""`. If a future rule ever removes lines (e.g. boilerplate deletion), every hunk after the first removal misreports.

## 6. Docs and repo hygiene

- **Root `README.md` documents the `poc/`, not the product**: usage is `cargo run --release -- /path/to/dir` (actual: `bash-em tui <path>` or `./run-tui.sh`), and it documents the `l` save-log key that exists only in `poc/ui.rs`. The `poc/` directory itself (5 `.rs` files, own README) is a stale duplicate of the whole app at root level — archive or delete.
- `docs/PRODUCT.md`, `docs/phases/`, `docs/mockups/` not audited line-by-line, but the mockups describe the browse/profile interactions that were never built — worth a pass when implementing round 1's priority list so docs and code converge.
- Version string `0.2.0-hub` is hardcoded in the titlebar (`common.rs:26`) while every crate is `0.1.0`.

## 7. Test coverage map

Good: engine unit tests are real and catch CRLF/trailing-newline/fence cases; `tests/integration.rs` (504 lines) covers replacer, pipeline, walker basics (dotdirs, large files), and one backup→restore roundtrip.
Gaps, in priority order:
1. **No test would catch 1.1** (entity double-count) — add `assert_eq!(c.em, 0)` to entity tests.
2. **No permission-preservation test** (would catch 1.2).
3. **No test that CLI apply honors FixOptions** (would catch 4.1).
4. **No serde-default test for `fence_guard`** (would catch 3.1).
5. Zero tests for: restore-to-missing-root, prune boundary (`keep_last_n` exactly), `list_runs` with corrupt manifest, boilerplate line numbers, registry resolution.

---

## Priority order for this round

| # | Fix | Findings | Class |
|---|-----|----------|-------|
| 1 | Preserve file permissions through apply/backup/restore | 1.2 | data integrity |
| 2 | Fix entity double-count (+ regression test) | 1.1, 7.1 | correctness |
| 3 | CLI apply writes the pipeline's content instead of re-fixing with defaults | 4.1 | correctness |
| 4 | Render `app.flash` on every screen (errors must be visible where they occur) | 5.1 | trust |
| 5 | `fence_guard` serde default → true | 3.1 | correctness |
| 6 | Implement or remove `profile.ignore` globs | 3.2 | correctness |
| 7 | Wire `VScrollState` into all three lists | 5.3 | existing-code win |
| 8 | Decide adapters: wire registry into walker+apply, or delete adapters + welcome tiles + `adapters list` | 2.1 | scope honesty |
| 9 | `restore --profile` / `backups list` CLI; auto-load a default profile path | 4.2, 4.3, 3.3 | usability |
| 10 | README rewrite for the workspace binary; archive `poc/` | 6 | hygiene |

Combined with round 1: the codebase has two personalities — a solid, tested engine core, and a presentation/integration layer where features were painted on before being built. Round 1's list makes the TUI functional; this round's list makes what it reports *true*.

*Housekeeping: this round's live tests added 2 backup runs to `~/.bash-em/backups` (perm test `07734b48…`, curly-quotes test) plus round 1's `478500af…` — all from scratchpad fixtures, safe to delete.*
