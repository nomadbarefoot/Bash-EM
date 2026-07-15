pub mod adapter;
pub mod docx;
pub mod pdf;
pub mod registry;
mod text;
mod walker;
pub mod xlsx;
pub mod zip_adapter;

pub use adapter::{Adapter, FileCategory};
pub use docx::DocxAdapter;
pub use pdf::PdfAdapter;
pub use registry::Registry;
pub use text::{atomic_write, TextAdapter};
pub use walker::{
    walk_tree, walk_tree_with_ignores, walk_tree_with_ignores_cancelled, FileCandidate, WalkStats,
};
pub use xlsx::XlsxAdapter;
pub use zip_adapter::ZipAdapter;
