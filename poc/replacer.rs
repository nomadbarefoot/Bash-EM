//! replacer.rs — the pure heart of Bash-EM.
//!
//! No I/O here. Takes text in, gives text + stats out. This makes it
//! trivially unit-testable and reusable (CLI, TUI, future daemon, whatever).
//!
//! Rules:
//!   em-dash  (U+2014) —  -> collapsed to exactly " - " (spaces normalized)
//!   horiz bar(U+2015) ―  -> same treatment as em-dash
//!   en-dash  (U+2013) –  -> bare "-" (preserves "2019–2024" -> "2019-2024")
//!   HTML entities (&mdash; &ndash; &#8212; &#x2014; etc.) -> same as their chars

/// Per-species kill counts. `#[derive]` auto-implements boilerplate traits:
/// Default gives us zeroed counts, Clone/Copy let us pass by value cheaply.
#[derive(Debug, Default, Clone, Copy)]
pub struct Counts {
    pub em: usize,
    pub en: usize,
    pub bar: usize,
    pub entities: usize,
}

impl Counts {
    pub fn total(&self) -> usize {
        self.em + self.en + self.bar + self.entities
    }
    /// Merge another Counts into this one (used to aggregate per-line -> per-file -> global).
    pub fn add(&mut self, other: Counts) {
        self.em += other.em;
        self.en += other.en;
        self.bar += other.bar;
        self.entities += other.entities;
    }
}

/// One changed line, kept for previews and the log.
#[derive(Debug, Clone)]
pub struct LineChange {
    pub line_no: usize, // 1-based, like editors show
    pub before: String,
    pub after: String,
}

/// Step 1: decode HTML entities for our dash family into the actual chars,
/// counting how many we found. Runs before the char-level pass so entities
/// get identical whitespace treatment.
fn decode_entities(line: &str, counts: &mut Counts) -> String {
    // (entity, replacement char) pairs. &str literals live in the binary — no allocation.
    const ENTITIES: &[(&str, char)] = &[
        ("&mdash;", '\u{2014}'),
        ("&#8212;", '\u{2014}'),
        ("&#x2014;", '\u{2014}'),
        ("&#X2014;", '\u{2014}'),
        ("&ndash;", '\u{2013}'),
        ("&#8211;", '\u{2013}'),
        ("&#x2013;", '\u{2013}'),
        ("&#X2013;", '\u{2013}'),
        ("&horbar;", '\u{2015}'),
        ("&#8213;", '\u{2015}'),
        ("&#x2015;", '\u{2015}'),
    ];

    // Fast path: no '&' means no entities. Avoids allocating a new String
    // for the overwhelmingly common case.
    if !line.contains('&') {
        return line.to_string();
    }

    let mut out = line.to_string();
    for (ent, ch) in ENTITIES {
        if out.contains(ent) {
            counts.entities += out.matches(ent).count();
            // Rust strings are immutable-ish; replace() allocates a new one.
            out = out.replace(ent, &ch.to_string());
        }
    }
    out
}

/// Is this char one of the "collapse to spaced hyphen" family?
fn is_wide_dash(c: char) -> bool {
    c == '\u{2014}' || c == '\u{2015}'
}

/// Transform a single line. Returns (new_line, counts_for_this_line).
pub fn fix_line(line: &str) -> (String, Counts) {
    let mut counts = Counts::default();
    let line = decode_entities(line, &mut counts);

    // Nothing to do? Return early without walking chars.
    if !line.contains(|c: char| is_wide_dash(c) || c == '\u{2013}') {
        return (line, counts);
    }

    let mut out = String::with_capacity(line.len());
    // Peekable lets us look at the next char without consuming it —
    // essential for "eat the spaces after the dash" logic.
    let mut chars = line.chars().peekable();
    // Tracks whether the last thing we emitted was our own " - " insertion,
    // so we can strip the trailing space if the line ends right after a dash.
    let mut inserted_dash_at_end = false;

    while let Some(c) = chars.next() {
        if is_wide_dash(c) {
            if c == '\u{2014}' { counts.em += 1 } else { counts.bar += 1 }

            // Collapse: spaces BEFORE the dash that we already emitted...
            while out.ends_with(' ') {
                out.pop();
            }
            // ...any run of further dashes/spaces AFTER it (—— or " — — " -> one hyphen)...
            while let Some(&next) = chars.peek() {
                if next == ' ' {
                    chars.next();
                } else if is_wide_dash(next) {
                    if next == '\u{2014}' { counts.em += 1 } else { counts.bar += 1 }
                    chars.next();
                } else {
                    break;
                }
            }
            // ...then emit exactly one spaced hyphen. At line start, skip the
            // leading space ("—dialogue" -> "- dialogue", not " - dialogue").
            if out.is_empty() {
                out.push_str("- ");
            } else {
                out.push_str(" - ");
            }
            inserted_dash_at_end = true;
        } else if c == '\u{2013}' {
            counts.en += 1;
            out.push('-'); // bare swap: ranges stay tight
            inserted_dash_at_end = false;
        } else {
            out.push(c);
            inserted_dash_at_end = false;
        }
    }

    // "word—" at end of line would leave "word - " — drop OUR trailing space.
    // (We never touch pre-existing trailing whitespace, e.g. markdown hard breaks.)
    if inserted_dash_at_end && out.ends_with(' ') {
        out.pop();
    }

    (out, counts)
}

/// Result of transforming a whole file's content.
pub struct FixResult {
    pub new_content: String,
    pub counts: Counts,
    pub changes: Vec<LineChange>,
    pub lines_changed: usize,
}

/// Transform full file content. `preview_cap` limits how many LineChange
/// entries we keep (memory guard for huge files); counts are always complete.
pub fn fix_content(content: &str, preview_cap: usize) -> FixResult {
    let mut counts = Counts::default();
    let mut changes = Vec::new();
    let mut lines_changed = 0usize;
    // Rough pre-allocation avoids repeated growth reallocations.
    let mut new_content = String::with_capacity(content.len());

    // split('\n') (not lines()) preserves exact structure — lines() would
    // silently swallow a missing trailing newline and eat '\r' inconsistently.
    // We handle '\r' manually so CRLF files round-trip byte-identical.
    let mut first = true;
    for (idx, raw_line) in content.split('\n').enumerate() {
        if !first {
            new_content.push('\n');
        }
        first = false;

        // Peel off a trailing '\r' (CRLF) and reattach after transforming.
        let (line, cr) = match raw_line.strip_suffix('\r') {
            Some(stripped) => (stripped, true),
            None => (raw_line, false),
        };

        let (fixed, line_counts) = fix_line(line);
        if line_counts.total() > 0 {
            lines_changed += 1;
            if changes.len() < preview_cap {
                changes.push(LineChange {
                    line_no: idx + 1,
                    before: line.to_string(),
                    after: fixed.clone(),
                });
            }
            counts.add(line_counts);
        }
        new_content.push_str(&fixed);
        if cr {
            new_content.push('\r');
        }
    }

    FixResult { new_content, counts, changes, lines_changed }
}

// ------------------------- tests -------------------------
// `cargo test` runs these. cfg(test) means they're compiled out of release builds.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn em_dash_no_spaces() {
        let (out, c) = fix_line("word—word");
        assert_eq!(out, "word - word");
        assert_eq!(c.em, 1);
    }

    #[test]
    fn em_dash_with_spaces_no_doubling() {
        let (out, _) = fix_line("word — word");
        assert_eq!(out, "word - word");
    }

    #[test]
    fn en_dash_range_stays_tight() {
        let (out, c) = fix_line("pages 40–60, 2019–2024");
        assert_eq!(out, "pages 40-60, 2019-2024");
        assert_eq!(c.en, 2);
    }

    #[test]
    fn entity_mdash() {
        let (out, c) = fix_line("a&mdash;b and c &#8212; d");
        assert_eq!(out, "a - b and c - d");
        assert_eq!(c.entities, 2);
    }

    #[test]
    fn dash_run_collapses() {
        let (out, _) = fix_line("wait——what");
        assert_eq!(out, "wait - what");
    }

    #[test]
    fn line_start_dialogue() {
        let (out, _) = fix_line("—hello there");
        assert_eq!(out, "- hello there");
    }

    #[test]
    fn line_end_no_trailing_space() {
        let (out, _) = fix_line("and then—");
        assert_eq!(out, "and then -");
    }

    #[test]
    fn clean_line_untouched() {
        let (out, c) = fix_line("perfectly normal - text with hyphens");
        assert_eq!(out, "perfectly normal - text with hyphens");
        assert_eq!(c.total(), 0);
    }

    #[test]
    fn crlf_preserved() {
        let r = fix_content("a—b\r\nplain\r\n", 10);
        assert_eq!(r.new_content, "a - b\r\nplain\r\n");
        assert_eq!(r.lines_changed, 1);
    }

    #[test]
    fn no_trailing_newline_preserved() {
        let r = fix_content("x—y", 10);
        assert_eq!(r.new_content, "x - y");
    }
}
