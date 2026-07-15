# Phase 3 — Hub TUI, playfulness, add-ons

**Goal:** Ship the arcade: welcome hub with **directory health bar**, browse, review with mouse, YAML editing surfaces, history/backups UI, and optional add-ons (Tetris) — without putting any of that into the engine.

**Exit criteria:** `bash-em` with no args opens the hub; user can browse, see health, scan/review/apply/restore entirely in-TUI; mouse + keyboard; visual language matches PRODUCT.md + mockups; Tetris loads as an add-on module and cannot mutate scrub state.

**Depends on:** Phase 1–2 APIs (`HealthReport`, adapters, backup, config).  
**Unlocks:** public “fun” release polish.

---

## Scope

### In

- `bash-em-tui` crate: screens, focus system, mouse, animations  
- Welcome / hub: brand, path picker, **health bar**, category breakdown, recent runs  
- Browse mode: directory tree, category badges, artifact hints  
- Review mode: offender list, diff preview, rule toggles, apply gauge  
- Backups mode: list runs, inspect manifest, restore with confirm  
- Profiles mode: rule form with atomic `.bash-em.yaml` save/reload (implemented)
- Stats / history: lifetime kills, last run, sarcasm-friendly flash copy  
- Crossterm mouse: click select, toggle, scroll, tab buttons  
- `bash-em-addon-tetris`: side panel or modal; pauses when focus is Review-apply  
- Design tokens in one theme module (colors, borders, focus style)  
- Align visuals with `docs/mockups/*.html`

### Out

- New format adapters (those are Phase 2 / later)  
- Engine changes beyond small display DTOs if needed  
- OCR, cloud, plugins marketplace  

---

## Screen map

```
Hub
 ├─ Welcome     health meter + categories + CTA [Scan] [Browse] [Backups]
 ├─ Browse      tree / file list → set root or multi-select
 ├─ Scan/Review offenders + diff + rules
 ├─ Backups     runs → restore
 ├─ Profiles    YAML / toggles
 ├─ Stats       museum of carnage
 └─ Add-ons     Tetris (optional pane)
```

Navigation: tabs (openapi-tui vibe) + pane focus ring (green/cyan bright border) + footer key/mouse legend (binsider density without the boredom).

---

## Health bar (welcome)

Consumes Phase 2 `HealthReport`:

```
┌ corruption ████████████░░░░  74%  — 1,284 artifacts ─┐
│ text 402  code 88  web 120  docs 40  office 19  pdf 7 │
│ top: posts/essay.md (142) · deck.xlsx (89) · …        │
└───────────────────────────────────────────────────────┘
```

Copy examples: `directory looking suspicious.` / `mostly clean. mostly.`

---

## Focus + input model

- Explicit `FocusedPane` enum; Tab / mouse click moves focus  
- Keys always apply to focused pane (hjkl / arrows / space / a)  
- Mouse:  
  - click row → select  
  - click marker → toggle include  
  - scroll wheel → list / diff  
  - click footer actions / tabs  
- During `Applying`: ignore quit; pause Tetris; show gauge  

---

## Add-on contract

```
trait Addon {
  fn name(&self) -> &str;
  fn tick(&mut self, dt);
  fn handle_key/mouse(...);
  fn draw(frame, area);
}
```

Host provides a rectangle and time. Add-ons **must not** call apply/backup. Tetris is the reference implementation.

---

## Work sequence

1. Scaffold `bash-em-tui` + theme tokens from mockups  
2. App shell: tabs, focus, footer, tick loop (~20–30fps)  
3. Welcome + health widgets wired to `health` API  
4. Browse + root selection  
5. Port Review from PoC against new engine (all files, not top-20 only — virtual scroll)  
6. Mouse event wiring  
7. Backups + Profiles screens  
8. Flash copy / micro-animations (count-up, apply flash)  
9. Tetris add-on + toggle in prefs  
10. Polish pass vs mockups; README gifs/screenshots optional  

---

## Verification

- [ ] No-arg launch → hub, not silent scan of `.` unless prefs say so  
- [ ] Health bar matches CLI `health --json` for same path  
- [ ] Apply from TUI creates backup; restore from Backups screen works  
- [ ] Mouse-only happy path possible for scan → toggle → apply  
- [ ] Tetris toggle off = zero draw cost / no ticks  
- [ ] `bash-em-tui` does not depend on adapter internals (only public APIs)  

---

## Risks

| Risk | Mitigation |
|------|------------|
| UI rewrite stalls useful CLI | P1/P2 already ship value; P3 is additive |
| Animation noise | Prefer ticks on status numbers, not whole-screen flicker |
| YAML editor pain in terminal | Form toggles + “open in $EDITOR” escape hatch |
| Scope: too many screens | MVP hub = Welcome + Review + Backups; Profiles/Stats/Tetris can trail |

---

## Phase MVP vs stretch

| MVP | Stretch |
|-----|---------|
| Welcome + health + Review + Backups | Profiles live editor |
| Keyboard + mouse | Clipboard scrub mode |
| Theme + animations | Stats museum, particles |
| Tetris behind flag | More add-ons |
