mod replacer;
mod rule;
mod pipeline;
pub mod boilerplate;
pub mod health;

pub use replacer::{
    fix_line, fix_line_with_options, fix_content, fix_content_with_options,
    decode_entities, Counts, LineChange, FixResult, FixOptions,
};
pub use rule::{Rule, Match, RuleId};
pub use pipeline::{TextUnit, Pipeline, BatchEdits, FileEdits};
