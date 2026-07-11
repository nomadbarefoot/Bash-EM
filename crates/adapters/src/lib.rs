pub mod adapter;
mod text;
mod walker;
pub mod registry;
pub mod zip_adapter;
pub mod xlsx;
pub mod docx;
pub mod pdf;

pub use adapter::{Adapter, FileCategory};
pub use text::TextAdapter;
pub use walker::{walk_tree, FileCandidate};
pub use registry::Registry;
pub use zip_adapter::ZipAdapter;
pub use xlsx::XlsxAdapter;
pub use docx::DocxAdapter;
pub use pdf::PdfAdapter;
