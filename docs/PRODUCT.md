# Bash-EM — Product

**Bash-EM** ("bash them") is a local-first AI typography exterminator. It hunts the smug little glyphs and habits that large language models spray into your files — especially the em-dash — and replaces them in place, with backups, diffs, and a TUI that refuses to be boring.

```
  bash═══m
  find the tells. bash them. keep the receipt.
```

---

## Why this exists

LLMs love decorative punctuation. They insert **em-dashes (—)** like confetti, sprinkle **en-dashes (–)** into ranges that already had hyphens, curl your quotes, inject zero-width junk, and occasionally leave behind "As an AI…" residue.

Em-dashes are the worst offenders. They scream *machine wrote this* in prose, docs, slide decks, and half the Markdown on the internet. Bash-EM exists so you can point it at a directory, see the damage, and scrub it without praying that search-and-replace remembered HTML entities.

This started as a PoC ratatui scanner for dashes. The product is the grown-up version: **pluggable artifact rules**, **format adapters**, **backup/restore**, **YAML profiles**, and a **playful mission-control TUI**.

---

## What it does

| Capability | Intent |
|------------|--------|
| Scan a tree | Classify files, count artifacts by rule and category |
| Review | Pick targets, preview diffs, toggle rules/files |
| Apply | In-place replace via adapters (atomic writes) |
| Backup / restore | Snapshot originals before mutate; restore by run |
| Diff / report | Before/after hunks, logs, optional machine-readable export |
| Profiles | YAML-defined rule sets + prefs; editable in TUI or on disk |
| Hub UI | Welcome, directory browse, health bar, history, settings |
| Add-ons | Optional play modules (e.g. Tetris) — never in the engine |

**Not in scope (v1 product promise):** OCR on scanned PDFs, cloud sync, auto-commit to git, "rewrite my essay to sound human."

---

## Architecture (locked)

Workspace crates. No monolithic scripts. Business logic stays out of the UI.

```
bash-em-engine      rules + pipeline (find → replace spans)
bash-em-backup      snapshots / restore (core system)
bash-em-diff        hunks / reports (core system)
bash-em-config      YAML profiles, prefs, ignore
bash-em-adapters    text, pdf (text-only), xlsx, docx, …
bash-em-tui         hub + review + mouse (consumes core)
bash-em-addon-*     toys (Tetris, etc.)
bash-em             binary composition root + CLI
```

**Rules** decide *what* to find and what to put back.  
**Adapters** decide *how* to open a format and write edits back.  
**Core systems** (backup, diff, config) sit beside the engine — not inside adapters or widgets.

See phase docs for how this lands over time.

---

## Supported content (product intent)

| Category | Examples | Notes |
|----------|----------|-------|
| Text / docs | `.md`, `.txt`, `.rst`, `.tex` | Primary path |
| Code | `.rs`, `.ts`, `.js`, `.py`, … | Fence / string awareness via rules |
| Web assets | `.html`, `.css`, `.astro`, … | Entities + text nodes |
| Office | `.xlsx`, `.docx`, `.odt` | Structured adapters |
| PDF | `.pdf` | **Text PDFs only — no OCR** |

File categories feed the **directory health bar** on the welcome screen (artifact load + mix of text / code / web / docs / office / pdf).

---

## Artifact families (rules hook into the engine)

1. **Typographic (default)** — em/en/bar dashes, HTML entities, curly quotes, ellipsis, optional exotic spaces / soft hyphens  
2. **LLM tells (opt-in)** — boilerplate phrases, zero-width / bidi junk  
3. **Code-aware guards** — skip fences, URLs, SHAs; don't "helpfully" smash syntax  

Engine contract is deliberately dumb and powerful: **find spans → replace in place**. Rules supply the candidates; adapters map spans back into their container format.

---

## UI design language

**Fun. Colorful. Not boring.**

This is not a grey sysadmin panel that apologizes for existing. It is a small arcade cabinet that happens to delete em-dashes.

### Visual principles

- **Punchy palette** — coral/red for "guilty" glyphs, lime for cleaned, cyan for nav focus, yellow for kill counts. Near-black chrome is fine; dead grey-on-grey is not.
- **Pane theatre** — bordered panels, clear focus ring (bright border on the active pane), tabs where modes switch. Inspired by dense TUIs (binsider / openapi-tui energy) but louder and more playful.
- **Motion with purpose** — spinners, count-up tickers, progress fills, a short "shatter" flash when dashes die. No screensaver soup during apply.
- **Health as a character** — the welcome health bar should feel like a boss HP meter for the selected directory, not a muted progress widget.
- **One joke, then ship** — crossed-out em-dashes in the logo, sarcastic flash lines. Wit in copy; clarity in controls.

### Layout grammar

```
┌ header / brand / tabs / path ──────────────────────────┐
│ sidebar or list │ main stage (diff / browse / config)  │
│                 │ secondary stats / health / add-on    │
├ footer: keys · mouse hints · flash ────────────────────┤
└────────────────────────────────────────────────────────┘
```

Mouse and keyboard are both first-class. Footer always shows what the focused pane accepts.

### Anti-patterns

- Flat single-color emptiness with one lonely list  
- Purple SaaS gradients transplanted into a terminal  
- Walls of identical bordered cards with no hierarchy  
- Tetris stealing focus during apply (pause or background only)

Mockups: `docs/mockups/*.html`

---

## Tone of voice

**Exciting, a little sarcastic, never cruel to the user.**

| Situation | Tone |
|-----------|------|
| Found 400 em-dashes | Glee. Name and shame the glyph. |
| Clean tree | Mild disbelief. Congratulate sparingly. |
| Backup saved | Straight and trustworthy — safety copy is not a bit. |
| Error / restore | Clear, adult, no jokes that obscure the fix. |
| Tetris high score | Chaotic neutral. |

Example microcopy:

- `scanning… the em-dashes know.`  
- `142 offenders. space to spare them. a to bash.`  
- `backup sealed. now we get messy.`  
- `restored. the dashes may return. you know where to find us.`

Brand name stays **Bash-EM** / `Bash-EM` with the dash visually defeated (struck / cracked) in the header.

---

## Success looks like

1. Adding a new file extension = new adapter (+ tests), not a rewrite.  
2. Adding a new artifact = new rule impl + YAML profile entry.  
3. Every apply has a restore path.  
4. Opening the TUI feels like boot-up of a game, then does real work.  
5. CI can run headless: scan / apply / report without the arcade.

---

## Doc map

| Doc | Role |
|-----|------|
| [PRODUCT.md](./PRODUCT.md) | This file — identity, tone, architecture north star |
| [phases/PHASE-1.md](./phases/PHASE-1.md) | Engine, config, backup/diff, text adapter, thin CLI |
| [phases/PHASE-2.md](./phases/PHASE-2.md) | Format adapters, health model, richer rules |
| [phases/PHASE-3.md](./phases/PHASE-3.md) | Hub TUI, mouse, play add-ons, polish |
| [mockups/](./mockups/) | HTML starting points for the visual language |
