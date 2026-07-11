use std::ops::Range;
use crate::replacer::fix_line;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleId {
    EmDash,
    EnDash,
    HorizontalBar,
    HtmlDashEntities,
    CurlyQuotes,
}

impl RuleId {
    pub fn name(&self) -> &'static str {
        match self {
            RuleId::EmDash => "em_dash",
            RuleId::EnDash => "en_dash",
            RuleId::HorizontalBar => "horizontal_bar",
            RuleId::HtmlDashEntities => "html_dash_entities",
            RuleId::CurlyQuotes => "curly_quotes",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            RuleId::EmDash => "Em-dash (U+2014) to spaced hyphen",
            RuleId::EnDash => "En-dash (U+2013) to hyphen",
            RuleId::HorizontalBar => "Horizontal bar (U+2015) to spaced hyphen",
            RuleId::HtmlDashEntities => "HTML entities (&mdash; etc.) to their characters",
            RuleId::CurlyQuotes => "Curly quotes to straight quotes",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Match {
    pub rule_id: RuleId,
    pub span: Range<usize>,
    pub replacement: String,
}

pub trait Rule {
    fn id(&self) -> RuleId;
    fn find(&self, text: &str) -> Vec<Match>;
}

#[allow(dead_code)]
pub struct TypographicDashRule;

impl Rule for TypographicDashRule {
    fn id(&self) -> RuleId {
        RuleId::EmDash
    }

    fn find(&self, text: &str) -> Vec<Match> {
        let (fixed, counts) = fix_line(text);
        if counts.total() == 0 {
            return Vec::new();
        }
        vec![Match {
            rule_id: RuleId::EmDash,
            span: 0..text.len(),
            replacement: fixed,
        }]
    }
}
