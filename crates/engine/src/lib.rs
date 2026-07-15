pub mod boilerplate;
pub mod health;
mod pipeline;
mod replacer;
mod rule;

pub use pipeline::{BatchEdits, FileEdits, Pipeline, TextUnit};
pub use replacer::{
    decode_entities, fix_content, fix_content_with_options, fix_line, fix_line_with_options,
    Counts, FixOptions, FixResult, LineChange,
};
pub use rule::{Match, Rule, RuleId};
