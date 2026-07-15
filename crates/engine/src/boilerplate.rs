#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Confidence {
    NearCertain,
    High,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoilerplateMatch {
    pub line_no: usize,
    pub phrase: String,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoilerplateReport {
    pub matches: Vec<BoilerplateMatch>,
    pub tell_word_count: usize,
}

const TIER1: &[&str] = &[
    "as an ai language model",
    "as a large language model",
    "i don't have personal opinions",
    "i don't have access to real-time",
    "i'd be happy to help",
    "great question!",
    "that's a great question",
    "i cannot provide",
    "i'm unable to",
    "as an ai,",
    "as an ai assistant",
];

const TIER2_PHRASES: &[&str] = &[
    "it's important to note that",
    "it's worth noting that",
    "it's worth mentioning that",
    "let's delve into",
    "let me delve into",
    "in conclusion,",
    "in today's rapidly",
    "in the ever-evolving",
];

const TELL_WORDS: &[&str] = &[
    "delve",
    "tapestry",
    "multifaceted",
    "utilize",
    "landscape",
    "embark",
    "meticulous",
    "testament",
    "pivotal",
    "underscores",
    "navigate",
    "realm",
    "intricate",
    "furthermore",
    "comprehensive",
    "robust",
    "streamline",
    "leverage",
];

const TELL_WORD_THRESHOLD: usize = 3;

pub fn scan_content(content: &str) -> BoilerplateReport {
    let lower = content.to_lowercase();
    let mut matches = Vec::new();

    for (line_no, line) in content.lines().enumerate() {
        let line_lower = line.to_lowercase();

        for &phrase in TIER1 {
            if line_lower.contains(phrase) {
                matches.push(BoilerplateMatch {
                    line_no: line_no + 1,
                    phrase: phrase.to_string(),
                    confidence: Confidence::NearCertain,
                });
            }
        }

        for &phrase in TIER2_PHRASES {
            if line_lower.contains(phrase) {
                matches.push(BoilerplateMatch {
                    line_no: line_no + 1,
                    phrase: phrase.to_string(),
                    confidence: Confidence::High,
                });
            }
        }
    }

    let mut tell_count = 0;
    for &word in TELL_WORDS {
        if lower.contains(word) {
            tell_count += 1;
        }
    }

    if tell_count >= TELL_WORD_THRESHOLD {
        for (line_no, line) in content.lines().enumerate() {
            let line_lower = line.to_lowercase();
            for &word in TELL_WORDS {
                if line_lower
                    .split_whitespace()
                    .any(|w| w.trim_matches(|c: char| !c.is_alphanumeric()) == word)
                {
                    matches.push(BoilerplateMatch {
                        line_no: line_no + 1,
                        phrase: format!("tell-word: {}", word),
                        confidence: Confidence::High,
                    });
                }
            }
        }
    }

    BoilerplateReport {
        matches,
        tell_word_count: tell_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tier1() {
        let report = scan_content("As an AI language model, I cannot do that.");
        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.matches[0].confidence, Confidence::NearCertain);
    }

    #[test]
    fn detects_tier2_phrase() {
        let report = scan_content("It's important to note that this matters.");
        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.matches[0].confidence, Confidence::High);
    }

    #[test]
    fn tell_words_below_threshold_ignored() {
        let report = scan_content("We delve into the tapestry of life.");
        assert_eq!(report.tell_word_count, 2);
        assert!(report.matches.is_empty());
    }

    #[test]
    fn tell_words_above_threshold_flagged() {
        let text = "We delve into the tapestry of this multifaceted landscape.";
        let report = scan_content(text);
        assert!(report.tell_word_count >= 3);
        assert!(!report.matches.is_empty());
    }

    #[test]
    fn clean_text_no_matches() {
        let report = scan_content("This is normal text without any LLM tells.");
        assert!(report.matches.is_empty());
    }
}
