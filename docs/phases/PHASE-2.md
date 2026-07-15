# Phase 2 — Formats, richer rules, health model

**Goal:** Everyday formats become first-class via adapters; rule packs expand; the engine can describe **directory health** (counts + categories) for the hub.

**Exit criteria:** Text + **PDF (text-only)** + **XLSX** + **DOCX** (and HTML as text/web) round-trip scrub under typographic profile; LLM-tell + curly-quote rules exist behind YAML flags; `HealthReport` API returns artifact totals and category mix for a path; backups still wrap every apply.

**Depends on:** Phase 1 complete.  
**Unlocks:** Phase 3 welcome health bar and multi-format review UI.

---

## Scope

### In

- Adapter registry keyed by extension + sniff/magic fallback  
- **PdfAdapter** — extract text from text PDFs only; no OCR; skip/flag image-only PDFs  
- **XlsxAdapter** — shared strings + inline strings; preserve workbook structure  
- **DocxAdapter** — word/document.xml text nodes (and notes if cheap)  
- Optional **HtmlAdapter** refinement (entity-aware units) if text adapter is too blunt  
- Rules: curly quotes, ellipsis, zero-width/bidi scrub, opt-in LLM boilerplate pack  
- Code-aware context flags (fenced regions in Markdown / Astro) passed into rule ctx  
- `HealthReport` + file categorization taxonomy  
- Ignore file: `.bash-emignore` (gitignore semantics; implemented for the text MVP)
- Backup retention policy in prefs (`keep_last_n`)  
- CLI: `bash-em health <path>`, `bash-em adapters list`  
- Fixture packs under `testdata/` per format (small, committed)

### Out

- OCR / scanned PDF rewriting  
- PPTX / EPUB (nice-to-have backlog; only if time)  
- Full playful hub UI (consume `HealthReport` in Phase 3)  
- Tetris / mouse chrome  

---

## Adapter checklist (make extension adds trivial)

Every adapter implements:

```
fn probe(path, bytes) -> Option<Self::Meta>
fn read(path) -> Vec<TextUnit>          # unit ids stable for write-back
fn write(path, edits) -> Result<()>     # apply only listed spans
fn category() -> FileCategory           # Text | Code | Web | Docs | Office | Pdf
```

Registry:

```
adapters.register(".md", TextAdapter)
adapters.register(".xlsx", XlsxAdapter)
adapters.register(".pdf", PdfAdapter)
...
```

Adding `.pptx` later = new file + register + fixtures. No engine edits.

---

## PDF policy (product-locked)

```
text-extractable PDF  → scrub text operators / content streams carefully
encrypted / empty text → skip + reason in report
scanned image-only    → skip; message: "no OCR — export text or skip"
```

Prefer a maintained Rust PDF crate that can extract and rewrite text with tests; if rewrite is too risky in P2, allow **extract → scrub → regenerate simple text PDF** only when user opts in — default should preserve layout where possible. Document the chosen strategy in adapter README.

---

## XLSX / DOCX notes

- **XLSX:** mutate string table entries; never break formula cells; keep zip mimetype order  
- **DOCX:** edit `w:t` nodes; preserve `w:r` styling; don't touch unrelated parts  
- Round-trip tests: unzip listing + selected XML nodes equal aside from intended string changes  

---

## Health model

```
HealthReport {
  root,
  scanned, skipped,
  by_category: { text, code, web, docs, office, pdf },
  by_rule: { em_dash, en_dash, … },
  top_files: [ … ],
  score: 0..=100   # playful "corruption" meter for UI
}
```

Category heuristics (extension tables + optional content hints). Score is presentation sugar derived from density (artifacts / files), not ML.

---

## Rules expansion

| Rule pack | Default | Notes |
|-----------|---------|-------|
| Typographic dashes + entities | on | from P1 |
| Curly quotes / ellipsis | off | enable in profile |
| Zero-width / BOM / bidi isolates | on (safe) | strip |
| LLM boilerplate phrases | off | high false-positive risk; preview-heavy |
| Fence guard | on when md/astro | rules see `in_code_fence` |

---

## Work sequence

1. Adapter trait + registry in `bash-em-adapters`  
2. Categorize API + `health` command using text-only first  
3. XLSX adapter + fixtures  
4. DOCX adapter + fixtures  
5. PDF text-only adapter + explicit skip reasons  
6. New rules behind YAML; profile examples (`aggressive.yaml`, `safe.yaml`)  
7. Fence-aware context for Markdown  
8. `.bash-emignore` + retention cleanup on backup  
9. Integration test: mixed tree apply + restore  

---

## Verification

- [ ] `adapters list` shows registered formats  
- [ ] Mixed fixture tree: only intended string cells / text nodes change  
- [ ] Image-only PDF skipped with clear reason  
- [ ] `health --json` stable schema for TUI consumption  
- [ ] Disabled rules produce zero matches  
- [ ] Restore after xlsx/docx/pdf apply succeeds  

---

## Risks

| Risk | Mitigation |
|------|------------|
| PDF rewrite corrupts layout | Feature-flag rewrite; extensive fixtures; skip when unsure |
| Shared string index bugs in xlsx | Round-trip unzip diffs in CI |
| LLM phrase false positives | Off by default; require profile opt-in |
| Scope creep (pptx/epub) | Backlog only unless P2 finishes early |
