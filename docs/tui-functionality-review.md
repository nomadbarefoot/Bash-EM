# Bash-EM TUI Functionality Review

**Date:** 2026-07-12
**Method:** Full static review of `crates/bash-em-tui`, `crates/backup`, `crates/config`, `crates/adapters`, `crates/bash-em`, followed by a live session driven end-to-end in tmux against a fixture tree (nested dirs 4 levels deep, hidden dir, dirty + clean files). Every finding tagged **[verified live]** was reproduced in the running TUI; the rest are confirmed by code reading with file:line references.

**Verdict:** The rendering layer is largely complete and looks finished, which masks the fact that roughly half of the interaction layer does not exist. The TUI currently supports exactly one workflow: launch on a directory → one scan → apply → restore. Everything else — browsing, re-scanning, changing roots, editing profiles, toggling rules, inspecting backups — is either inert chrome or actively misleading. Several UI strings promise behavior that has no handler behind it, which is the biggest source of the "trust issues" you're feeling.

---

## 1. Browse tab — decorative, not functional

### 1.1 Locked to the launch root; no filesystem navigation **[verified live]** — BLOCKER
`load_browse_entries` (`crates/bash-em-tui/src/run.rs:255`) walks only `app.root`, which is fixed at CLI launch (`crates/bash-em/src/main.rs:461`). There is no way to go up a level, jump to `/`, `~`, or any sibling directory. The user's core ask — browse the system freely, pick a dir, load and scan it — is entirely absent.

### 1.2 Selection does nothing **[verified live]** — BLOCKER
`handle_browse_key` (`crates/bash-em-tui/src/event.rs:117-132`) handles only `j`/`k`/arrows. Enter, `h`/`l`, Left/Right, and Backspace were all sent to the live app: nothing happens. There is no "select this dir as root", no expand/collapse, no drill-in. The cursor moves over a list that cannot be acted on.

### 1.3 Depth cap silently hides files and corrupts the artifact count **[verified live]**
`collect_dir` recurses only while `depth < 3` (`run.rs:307`). In the fixture, `sub/deep/deeper/deepest/d4.txt` (which contains an em-dash) is invisible in Browse, while the Scan tab happily finds it. Result on screen: Browse header says **6 artifacts**, Welcome/Scan say **7**. Two tabs of the same app disagree about the same directory — this is exactly the kind of inconsistency that erodes trust.

### 1.4 Hidden entries unconditionally excluded, no toggle
`run.rs:285` drops every dotfile/dotdir with no way to show them. Combined with 1.1, a `.backups` dir (see §3) could never be inspected from Browse even if it existed.

### 1.5 Stats bar counts only visible entries
"4 dirs 3 files" counts the truncated entry list, not the directory's real contents (there are 4+ files). The numbers look authoritative but describe the rendering, not the filesystem.

### 1.6 Never refreshed
Browse entries are loaded once at startup (`run.rs:30`). After an apply or restore, artifact counts and the clean/dirty coloring are stale for the rest of the session.

### 1.7 No mouse support
`LayoutCache` has no browse area; clicks in the Browse list are ignored while the Scan and Backups lists advertise "click works too".

**What Browse needs to be real:** a cwd-based directory pager (start at launch root, `Backspace`/`h` → parent, `Enter`/`l` → descend into dir, so the whole filesystem is reachable), a distinct "set as scan root" action (e.g. `s` or Enter on a dir) that updates `app.root`, resets `scan_phase` to Idle, re-runs health + browse load, and a `.`/hidden toggle. Depth-1 listing per level (pager style) also removes the depth-3 cap problem entirely.

---

## 2. Scan tab — works once, then never again

### 2.1 One scan per process; re-scan is impossible **[verified live]** — BLOCKER
`switch_screen` only kicks off a scan when `scan_phase == Idle` (`app.rs:214`), and nothing ever resets the phase after it reaches `Done`. Verified live: after an apply, pressing `1` then `3` (or Enter from Welcome) just re-shows the stale DONE panel. To scan again — even the same directory — you must quit and relaunch. Combined with 1.1/1.2 this means one directory, one scan, per process launch.

### 2.2 Scan surfaces only offenders, not directory contents
`run_scan` populates `app.files` from dirty files only. Clean files and the overall tree are invisible, so there's no context for what was scanned (your ask: "Scan tab should surface files in the dir contents"). Minimum fix: show scanned-clean count per dir or an "all files" toggle; better: reuse the Browse tree with per-file offender badges.

### 2.3 Scanning blocks the UI thread
`run_scan` and `run_health_scan` run synchronously inside the event loop (`run.rs:27-28, 58-68`). The braille spinner and "N files found" counter in `draw_scanning` can never actually animate — on a large tree the app freezes at launch (health scan runs before the first frame) and again on scan. The scanning screen is theater; the work happens in one blocking call between frames.

### 2.4 Rules row is display-only
`Pane::RulesRow` is Tab-focusable on the Scan screen (`app.rs:222`), but no key anywhere mutates `rule_toggles` — grep confirms it is written once in `App::new` and never again. Focusing the pane and pressing Space still toggles *file exclusion* in the offender list (`event.rs:101` ignores `focused_pane`). A focusable, highlighted control that cannot be operated is worse than no control.

### 2.5 Diff preview scroll defects
- `diff_scroll` is never reset when the selected file changes — scroll down on file A, select file B, and you're looking at an empty/mid-file view.
- Scroll is mouse-only; no keys scroll the diff even when `DiffPreview` is focused.
- `diff_scroll += 3` is unbounded past the end of the change list.

### 2.6 Mouse click uses a stale offset — selects the wrong file
`draw_offender_list` computes the viewport offset locally each frame (`scan.rs:200-206`) but can never write it back (`&App`), so `app.list_offset` is permanently 0. The click handler maps `row - area.y + app.list_offset` (`event.rs:215`). Once the list has scrolled, clicks select a file N rows above the one under the cursor. Same latent pattern in `browse_offset`.

### 2.7 Footer advertises `l log` — no handler **[verified live]**
No `l` binding exists in `handle_scan_key` or globals. Dead hint.

---

## 3. Backups & restore — the trust gap

### 3.1 Backup location is never shown anywhere — BLOCKER (trust)
Backups go to `~/.bash-em/backups` (default in `crates/config/src/lib.rs:42`), a hidden directory in `$HOME`. No screen, dialog, footer, or done-panel ever prints this path — the DONE panel shows only the run UUID. You correctly observed "I am unable to see where the backup files are." Fixes, in order of impact:
1. Print the resolved backup dir in the Backups panel title/footer and in the DONE panel.
2. Per your requirement: default `backup_dir` to a root-level `<scan-root>/.backups`. Note two consequences the implementation must handle: the walker already skips dot-dirs (`crates/adapters/src/walker.rs:50`) so scans won't self-ingest backups, **but** `backup_dir` is currently global while runs are per-root — a per-root `.backups` means `list_runs` must aggregate or scope by the active root; and `prune_old_runs` scoping changes too.

### 3.2 Restore does not delete the restore point **[verified live]** — your stated requirement
`backup::restore` (`crates/backup/src/lib.rs:90-113`) copies files back and returns; `run.rs:78-90` flashes a message. The run dir `478500af…` was still on disk and still listed after a successful live restore. Per your spec, a successful restore should consume the restore point (`fs::remove_dir_all(run_dir)` after the copy loop succeeds) and `load_backup_runs` must be re-called so the list updates. Today the runs list isn't even reloaded after restore.

### 3.3 Restore dialog lies about what it will do
`draw_restore_dialog` (`crates/bash-em-tui/src/screens/backups.rs:280-293`) renders `"{n} files ← {app.root}/"` — but restore writes to **`manifest.root`**, not the current root. Select one of the older 32-file runs (made from a different directory) and the dialog claims it will restore into `fixture/` while it actually overwrites 32 files in a completely different tree. It also prints "profile hash matched" — nothing in the codebase computes or compares any profile hash, and restore never verifies the stored SHA-256 `FileEntry.hash` values it went to the trouble of recording. This is fabricated reassurance in a destructive-action dialog.

### 3.4 Runs table hides the run's root and hardcodes the profile
The table has no root/path column, so runs from different directories are indistinguishable **[verified live]** (my fixture run sat next to three unrelated runs). The profile column is the literal string `"typographic"` (`backups.rs:144`) instead of `manifest.profile_name`.

### 3.5 "Open in $EDITOR" button is a no-op **[verified live]**
Third button in the restore dialog; `handle_confirm_key` (`event.rs:181-191`) only acts on button 0 — selecting "Open in $EDITOR" silently cancels the dialog.

### 3.6 Footer advertises `e edit profile` — no handler **[verified live]**
`handle_backups_key` has no `e` binding.

### 3.7 Mouse click on runs is off by two rows
`layout_cache.runs_area` is set to the full panel inner (`backups.rs:62`), but rows render starting at `y + 2` (header + separator). Click mapping `row - area.y` (`event.rs:225`) means clicking the header selects run 0 and clicking run N selects run N+2.

### 3.8 Silent pruning of restore points
`prune_old_runs` keeps `keep_last_n = 10` and silently deletes older runs on every apply (`run.rs:229`). Nothing in the UI mentions retention. Backups that quietly vanish + a hidden backup dir = compounded trust damage. Surface it: "keeping last 10 runs (pruned 2)".

### 3.9 Missing basic vault operations
No delete-run key, no run-detail view (which files, sizes, hashes), no reveal-in-Finder / print-path action.

---

## 4. Profiles tab — pure façade

### 4.1 Zero input handling **[verified live]**
`handle_key` routes `Screen::Profiles` to `_ => {}` (`event.rs:48`). Verified: `j`, Space, and every other key do nothing. Yet the pane says **"# editable here or on disk"** and the Backups copy of the same pane renders a hardcoded **"VALID YAML"** badge (`backups.rs:186`) that no validator ever computes. The rules list renders ON/OFF states that cannot be toggled here either (see 2.4).

### 4.2 Not in the pane cycle
`cycle_pane` (`app.rs:224`) returns early for Profiles, so even Tab does nothing, though the footer offers "tab focus".

### 4.3 Rules render in random order
`profile.rules` is a `HashMap`; both the rules list and the YAML dump reorder on every launch. Cosmetic but adds to the "unfinished" feel. Use a `BTreeMap` or fixed rule ordering.

### 4.4 No persistence path
Even if editing existed, there is no save/write-profile code in the TUI, and no indication of which file (if any) the profile came from.

---

## 5. Welcome screen

### 5.1 Corruption gauge pegs at 100% almost immediately **[verified live]**
`health.score = min(100, artifacts*100/scanned_files)` (`crates/engine/src/health.rs:104`) — an average of one artifact per file reads as **100% corruption**. My 4-file fixture with 7 dashes showed a fully red 100% bar. As a "corruption %" it's meaningless above tiny densities; either rescale (e.g. % of files dirty) or relabel as artifact density.

### 5.2 Navigation panel lists a screen that doesn't exist
"Stats museum" appears in NAVIGATE; there are only 5 screens and no key reaches any museum.

### 5.3 Startup freeze
The synchronous health scan runs before the first frame (`run.rs:27-28`), so on a large tree the terminal sits blank in the alternate screen for the whole walk. Same fix as 2.3: move scanning off-thread with incremental updates.

---

## 6. Global / cross-cutting

- **`q` and Esc hard-quit from everywhere** (`event.rs:54-57`), including mid-review with per-file exclusions staged. Only `Applying` is guarded. One stray Esc discards the session. Esc should back out (close dialog → back to Welcome → then quit), with `q` alone quitting.
- **Flash messages never expire** — `app.flash` persists until overwritten; "restored 3 files" is still on screen 10 minutes later.
- **Apply/restore don't refresh health or browse** — Welcome's tiles and Browse's counts keep the pre-apply numbers for the rest of the session.
- **State after restore is incoherent**: Scan tab still says "3 files cleaned / DONE" for files that were just un-cleaned.

## 7. Bonus: CLI `apply` ignores profile rules (found while cross-checking)
`cmd_apply` scans with the profile's `FixOptions`, but then applies via `adapters::TextAdapter::apply(path, 0)` (`crates/bash-em/src/main.rs:277`), which re-runs `fix_content` with **default options** — profile-disabled rules (e.g. `curly_quotes: false`) still get applied to disk. The TUI path is correct (writes the pipeline's `new_content`); the CLI path is not. Also, unused dead module: `vscroll.rs` is exported but never referenced.

---

## Priority order (what actually unblocks the tool)

| # | Fix | Findings |
|---|-----|----------|
| 1 | Browse = real navigator: parent/descend, hidden toggle, "select dir → set root → rescan" | 1.1, 1.2, 1.4, 2.1 |
| 2 | Re-scan capability (reset `scan_phase`, `s` to rescan) | 2.1 |
| 3 | Backup visibility: show resolved path everywhere; move default to `<root>/.backups`; root column in runs table; dialog uses `manifest.root` | 3.1, 3.3, 3.4 |
| 4 | Restore consumes the restore point + reloads runs + refreshes state | 3.2, 6 |
| 5 | Remove or implement every lying affordance: `$EDITOR` button, `e`, `l`, "editable here", "VALID YAML", "profile hash matched", "Stats museum" | 3.5, 3.6, 2.7, 4.1, 5.2 |
| 6 | Rule toggling (Space on RulesRow / Profiles list) with pipeline rebuild | 2.4, 4.1 |
| 7 | Async scan/health + honest progress | 2.3, 5.3 |
| 8 | Mouse offset bugs, diff-scroll reset, score rescale, CLI apply options | 2.5, 2.6, 3.7, 5.1, 7 |

The unifying theme: **every string on screen must be backed by a handler, and every number must be backed by the filesystem.** The visual layer is ahead of the interaction layer; closing that gap is what turns this from a demo into a tool.
