# Bash-EM next features

**Status:** working draft for product discussion  
**Starting point:** the text workflow is functional; the next risk is letting the interface promise more than the engine can prove.

## Product rule for the next cycle

Every screen should answer four questions without guesswork:

1. What root and profile am I operating on?
2. What did the scan find, and which results are included?
3. What will change if I confirm?
4. Where is the safety copy, and how do I restore it?

The UI overhaul is a change to information architecture, not a palette pass. The current five peer tabs split one linear safety workflow across several places. Fable's consultation recommends testing three alternatives in [the comparison prototype](./mockups/04-fable-ui-directions.html):

- **Pipeline:** a guided `Target > Scan > Review > Receipt` flow. Lowest migration cost and the default candidate.
- **Workbench:** one persistent, dense workspace with root, rules, files, diff, vault, and a severity ledger visible together.
- **Case File:** a run-centric model where scans, apply receipts, and restores share one docket.

Regardless of the winner, retain Workbench's severity ledger and Case File's receipt framing.

## Phase 0: choose the interaction model

**Outcome:** one tested direction and a short UI contract before ratatui implementation begins.

- Walk through all three prototypes at `140x40` and `100x30`.
- Test four tasks: select a root, spare one file, change one rule and rescan, then apply and restore.
- Compare task completion, wrong-pane visits, backup-path recall, error notice rate, and subjective confidence.
- Choose the primary information architecture; record useful pieces borrowed from the other variants.
- Define the minimum terminal size and exact collapse behavior.
- Freeze the screen/state map, focus order, global keys, and destructive-action copy.

**Exit gate**

- A first-time user can complete scan, review, apply, and restore without coaching.
- At least four of five test users can say where the backup lives after applying.
- Errors are noticed immediately at both target terminal sizes.
- Every visible control has an implemented action in the proposed state map.

## Phase 1: rebuild the safety workflow shell

**Outcome:** the chosen navigation model replaces the five-tab PoC shell without changing engine behavior.

- Make target, profile, scan freshness, and apply eligibility persistent state in the chrome.
- Put rule controls beside scan results; a rule change must visibly mark results stale and block apply until rescan.
- Replace the single flash line with a small severity-aware event ledger for success, warning, and error messages.
- Expand apply confirmation into a safety manifest showing:
  - included files and replacement counts by rule;
  - destination root;
  - resolved backup path and retention policy;
  - atomic-write and permission-preservation guarantees;
  - partial-failure behavior.
- Turn the post-apply state into a receipt with run ID, changed-file count, backup path, and restore action.
- Make keyboard and mouse hit the same actions and share the same focus model.
- Specify honest loading, empty, stale, partial-success, and error states.

**Exit gate**

- Happy path works at `100x30` and `140x40` with keyboard only and mouse only.
- Apply cannot start from stale results.
- Destructive actions use straight, unambiguous copy; jokes remain in non-critical chrome.
- Focus, active, disabled, loading, warning, error, and success states are visually distinct.

## Phase 2: deepen review and recovery

**Outcome:** users can understand the proposed edits and recover confidently without leaving the TUI.

- Add an `Offenders / All files` view so clean and skipped content is not invisible.
- Add per-rule and per-file pivots without duplicating scan state.
- Improve diff navigation: next hunk, previous hunk, bounded scrolling, and preserved selection.
- Explain skipped files by reason: ignored, hidden, binary, oversized, unreadable, and symlink.
- Add backup run details: root, profile, file list, timestamp, hashes, and restore target.
- Add explicit reveal/copy-path and delete-run operations with confirmation.
- Keep restore points after restore by default; make retention behavior visible and configurable.
- Add a durable activity history only if user testing shows the three-line ledger is insufficient.

**Exit gate**

- Counts agree across home, review, receipt, CLI report, and backup manifest.
- A user can trace any proposed replacement to its rule and source line.
- A restore confirmation names the manifest root, never merely the currently selected root.
- Partial apply and partial restore failures leave a clear recovery path.

## Phase 3: profiles and automation parity

**Outcome:** TUI and CLI express the same profile and workflow contracts.

- Make the active profile source and unsaved state obvious.
- Support deterministic rule ordering, ignore-pattern editing, validation, and atomic save/reload.
- Add CLI backup listing and run inspection if they remain absent from the final command surface.
- Keep CLI apply, TUI apply, health, and report on the shared workflow orchestration path.
- Add machine-readable receipts suitable for CI without adding cloud state.
- Document config discovery, project profile precedence, `.bash-emignore`, backup location, and retention.

**Exit gate**

- The same root and profile produce identical counts and intended output in TUI and CLI.
- Invalid profiles and ignore patterns fail loudly without mutating files.
- Headless scan, apply, receipt, and restore have integration coverage.

## Phase 4: formats, then delight

**Outcome:** expand only after the text workflow and redesigned shell are trustworthy.

- Decide each structured adapter independently: integrate end to end or keep it dormant and unadvertised.
- Prove DOCX and XLSX extraction, preview, write-back, backup, and restore before exposing their tiles.
- Keep PDF read-only unless a lossless write policy is demonstrated; no OCR in this cycle.
- Add richer rules only with false-positive fixtures and per-rule preview counts.
- Add short, purposeful scan/apply motion and restrained arcade feedback after performance profiling.
- Treat add-ons such as Tetris as optional modules that never compete with review, apply, or restore.

**Exit gate**

- No format appears as supported until scan, preview, apply, backup, and restore all work together.
- A structured-format restore is byte-identical to its original.
- UI effects add no input lag and stop during destructive operations.

## Cross-phase verification

- Focused unit tests for counts, rules, profile defaults, ignore matching, and permission preservation.
- Workflow integration tests for scan to receipt to restore.
- Ratatui render snapshots at the minimum and target terminal sizes.
- Event tests proving every footer hint and click target has a handler.
- Fixture matrix for clean, dirty, ignored, binary, oversized, unreadable, symlinked, and partially failing trees.
- Manual smoke test before each phase closes; `_examples/` remains opt-in and is not part of automated inspection.

## Decisions to make together

1. Which model wins Phase 0: Pipeline, Workbench, or Case File?
2. Should restore points be retained after restore, consumed, or configurable?
3. Is rule-first review important enough to ship as a primary pivot, or should it remain a secondary view?
4. Should project history persist scan-only runs, or only apply receipts?
5. Which structured format earns the first end-to-end adapter investment: DOCX or XLSX?

## Recommended first slice

Choose **Pipeline** as the implementation baseline unless testing clearly favors another model. Build only the stage rail, in-review rules with stale blocking, safety manifest, three-line severity ledger, and receipt. This slice changes the mental model while reusing most current screen data and workflow code.
