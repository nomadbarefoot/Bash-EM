#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Counts {
    pub em: usize,
    pub en: usize,
    pub bar: usize,
    pub entities: usize,
    pub curly_quotes: usize,
    pub ellipsis: usize,
    pub zero_width: usize,
}

impl Counts {
    pub fn total(&self) -> usize {
        self.em
            + self.en
            + self.bar
            + self.entities
            + self.curly_quotes
            + self.ellipsis
            + self.zero_width
    }

    pub fn replaceable(&self) -> usize {
        self.total()
    }

    pub fn add(&mut self, other: Counts) {
        self.em += other.em;
        self.en += other.en;
        self.bar += other.bar;
        self.entities += other.entities;
        self.curly_quotes += other.curly_quotes;
        self.ellipsis += other.ellipsis;
        self.zero_width += other.zero_width;
    }
}

#[derive(Debug, Clone)]
pub struct FixOptions {
    pub em_dash: bool,
    pub en_dash: bool,
    pub horizontal_bar: bool,
    pub html_entities: bool,
    pub curly_quotes: bool,
    pub ellipsis: bool,
    pub zero_width: bool,
}

impl Default for FixOptions {
    fn default() -> Self {
        Self {
            em_dash: true,
            en_dash: true,
            horizontal_bar: true,
            html_entities: true,
            curly_quotes: false,
            ellipsis: false,
            zero_width: true,
        }
    }
}

impl FixOptions {
    pub fn from_profile(profile: &[(String, bool)]) -> Self {
        let mut opts = Self::default();
        for (name, enabled) in profile {
            match name.as_str() {
                "em_dash" => opts.em_dash = *enabled,
                "en_dash" => opts.en_dash = *enabled,
                "horizontal_bar" => opts.horizontal_bar = *enabled,
                "html_dash_entities" => opts.html_entities = *enabled,
                "curly_quotes" => opts.curly_quotes = *enabled,
                "ellipsis" => opts.ellipsis = *enabled,
                "zero_width" => opts.zero_width = *enabled,
                _ => {}
            }
        }
        opts
    }
}

#[derive(Debug, Clone)]
pub struct LineChange {
    pub line_no: usize,
    pub before: String,
    pub after: String,
}

pub fn decode_entities(line: &str, counts: &mut Counts) -> String {
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

    if !line.contains('&') {
        return line.to_string();
    }

    let mut out = line.to_string();
    for (ent, ch) in ENTITIES {
        if out.contains(ent) {
            counts.entities += out.matches(ent).count();
            out = out.replace(ent, &ch.to_string());
        }
    }
    out
}

const ENTITY_EM_MARKER: char = '\u{E000}';
const ENTITY_EN_MARKER: char = '\u{E001}';
const ENTITY_BAR_MARKER: char = '\u{E002}';

fn decode_entities_for_fix(line: &str, counts: &mut Counts) -> String {
    const ENTITIES: &[(&str, char)] = &[
        ("&mdash;", ENTITY_EM_MARKER),
        ("&#8212;", ENTITY_EM_MARKER),
        ("&#x2014;", ENTITY_EM_MARKER),
        ("&#X2014;", ENTITY_EM_MARKER),
        ("&ndash;", ENTITY_EN_MARKER),
        ("&#8211;", ENTITY_EN_MARKER),
        ("&#x2013;", ENTITY_EN_MARKER),
        ("&#X2013;", ENTITY_EN_MARKER),
        ("&horbar;", ENTITY_BAR_MARKER),
        ("&#8213;", ENTITY_BAR_MARKER),
        ("&#x2015;", ENTITY_BAR_MARKER),
    ];

    if !line.contains('&') {
        return line.to_string();
    }

    let mut out = line.to_string();
    for (entity, marker) in ENTITIES {
        let found = out.matches(entity).count();
        if found > 0 {
            counts.entities += found;
            out = out.replace(entity, &marker.to_string());
        }
    }
    out
}

fn is_enabled_wide_dash(c: char, opts: &FixOptions) -> bool {
    match c {
        '\u{2014}' => opts.em_dash,
        '\u{2015}' => opts.horizontal_bar,
        ENTITY_EM_MARKER | ENTITY_BAR_MARKER => true,
        _ => false,
    }
}

fn is_zero_width(c: char) -> bool {
    matches!(c,
        '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' |
        '\u{2066}'..='\u{2069}' | '\u{202A}'..='\u{202E}'
    )
}

fn is_curly_quote(c: char) -> bool {
    matches!(c, '\u{201C}' | '\u{201D}' | '\u{2018}' | '\u{2019}')
}

pub fn fix_line(line: &str) -> (String, Counts) {
    fix_line_with_options(line, &FixOptions::default())
}

pub fn fix_line_with_options(line: &str, opts: &FixOptions) -> (String, Counts) {
    let mut counts = Counts::default();

    let line = if opts.html_entities {
        decode_entities_for_fix(line, &mut counts)
    } else {
        line.to_string()
    };

    let needs_dashes = line.contains(|c: char| {
        is_enabled_wide_dash(c, opts) || (opts.en_dash && c == '\u{2013}') || c == ENTITY_EN_MARKER
    });
    let needs_quotes = opts.curly_quotes && line.contains(is_curly_quote);
    let needs_ellipsis = opts.ellipsis && line.contains('\u{2026}');
    let needs_zw = opts.zero_width && line.contains(is_zero_width);

    if !needs_dashes && !needs_quotes && !needs_ellipsis && !needs_zw && counts.entities == 0 {
        return (line, counts);
    }

    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut inserted_dash_at_end = false;

    while let Some(c) = chars.next() {
        if is_enabled_wide_dash(c, opts) {
            if c == '\u{2014}' {
                counts.em += 1;
            } else if c == '\u{2015}' {
                counts.bar += 1;
            }

            while out.ends_with(' ') {
                out.pop();
            }
            while let Some(&next) = chars.peek() {
                if next == ' ' {
                    chars.next();
                } else if is_enabled_wide_dash(next, opts) {
                    if next == '\u{2014}' {
                        counts.em += 1;
                    } else if next == '\u{2015}' {
                        counts.bar += 1;
                    }
                    chars.next();
                } else {
                    break;
                }
            }
            if out.is_empty() {
                out.push_str("- ");
            } else {
                out.push_str(" - ");
            }
            inserted_dash_at_end = true;
        } else if (c == '\u{2013}' && opts.en_dash) || c == ENTITY_EN_MARKER {
            if c == '\u{2013}' {
                counts.en += 1;
            }
            out.push('-');
            inserted_dash_at_end = false;
        } else if is_curly_quote(c) && opts.curly_quotes {
            counts.curly_quotes += 1;
            match c {
                '\u{201C}' | '\u{201D}' => out.push('"'),
                '\u{2018}' | '\u{2019}' => out.push('\''),
                _ => unreachable!(),
            }
            inserted_dash_at_end = false;
        } else if c == '\u{2026}' && opts.ellipsis {
            counts.ellipsis += 1;
            out.push_str("...");
            inserted_dash_at_end = false;
        } else if is_zero_width(c) && opts.zero_width {
            counts.zero_width += 1;
            inserted_dash_at_end = false;
        } else {
            out.push(c);
            inserted_dash_at_end = false;
        }
    }

    if inserted_dash_at_end && out.ends_with(' ') {
        out.pop();
    }

    (out, counts)
}

pub struct FixResult {
    pub new_content: String,
    pub counts: Counts,
    pub changes: Vec<LineChange>,
    pub lines_changed: usize,
}

pub fn fix_content(content: &str, preview_cap: usize) -> FixResult {
    fix_content_with_options(content, preview_cap, &FixOptions::default(), false)
}

pub fn fix_content_with_options(
    content: &str,
    preview_cap: usize,
    opts: &FixOptions,
    fence_guard: bool,
) -> FixResult {
    let mut counts = Counts::default();
    let mut changes = Vec::new();
    let mut lines_changed = 0usize;
    let mut new_content = String::with_capacity(content.len());
    let mut in_fence = false;

    let mut first = true;
    for (idx, raw_line) in content.split('\n').enumerate() {
        if !first {
            new_content.push('\n');
        }
        first = false;

        let (line, cr) = match raw_line.strip_suffix('\r') {
            Some(stripped) => (stripped, true),
            None => (raw_line, false),
        };

        if fence_guard {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                in_fence = !in_fence;
            }
        }

        if in_fence {
            new_content.push_str(line);
            if cr {
                new_content.push('\r');
            }
            continue;
        }

        let (fixed, line_counts) = fix_line_with_options(line, opts);
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

    FixResult {
        new_content,
        counts,
        changes,
        lines_changed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn em_dash_no_spaces() {
        let (out, c) = fix_line("word\u{2014}word");
        assert_eq!(out, "word - word");
        assert_eq!(c.em, 1);
    }

    #[test]
    fn em_dash_with_spaces_no_doubling() {
        let (out, _) = fix_line("word \u{2014} word");
        assert_eq!(out, "word - word");
    }

    #[test]
    fn en_dash_range_stays_tight() {
        let (out, c) = fix_line("pages 40\u{2013}60, 2019\u{2013}2024");
        assert_eq!(out, "pages 40-60, 2019-2024");
        assert_eq!(c.en, 2);
    }

    #[test]
    fn entity_mdash() {
        let (out, c) = fix_line("a&mdash;b and c &#8212; d");
        assert_eq!(out, "a - b and c - d");
        assert_eq!(c.entities, 2);
        assert_eq!(c.em, 0);
        assert_eq!(c.total(), 2);
    }

    #[test]
    fn dash_run_collapses() {
        let (out, _) = fix_line("wait\u{2014}\u{2014}what");
        assert_eq!(out, "wait - what");
    }

    #[test]
    fn em_dash_and_horizontal_bar_options_are_independent() {
        let em_disabled = FixOptions {
            em_dash: false,
            ..FixOptions::default()
        };
        let (out, counts) = fix_line_with_options("em\u{2014}dash bar\u{2015}done", &em_disabled);
        assert_eq!(out, "em\u{2014}dash bar - done");
        assert_eq!(counts.em, 0);
        assert_eq!(counts.bar, 1);

        let bar_disabled = FixOptions {
            horizontal_bar: false,
            ..FixOptions::default()
        };
        let (out, counts) = fix_line_with_options("em\u{2014}dash bar\u{2015}done", &bar_disabled);
        assert_eq!(out, "em - dash bar\u{2015}done");
        assert_eq!(counts.em, 1);
        assert_eq!(counts.bar, 0);
    }

    #[test]
    fn line_start_dialogue() {
        let (out, _) = fix_line("\u{2014}hello there");
        assert_eq!(out, "- hello there");
    }

    #[test]
    fn line_end_no_trailing_space() {
        let (out, _) = fix_line("and then\u{2014}");
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
        let r = fix_content("a\u{2014}b\r\nplain\r\n", 10);
        assert_eq!(r.new_content, "a - b\r\nplain\r\n");
        assert_eq!(r.lines_changed, 1);
    }

    #[test]
    fn no_trailing_newline_preserved() {
        let r = fix_content("x\u{2014}y", 10);
        assert_eq!(r.new_content, "x - y");
    }
}
