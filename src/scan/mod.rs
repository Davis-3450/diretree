pub mod cache;
pub mod tree;
pub mod walker;

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("path not found: {0}")]
    NotFound(PathBuf),
    #[error("cannot read directory: {0}")]
    Unreadable(PathBuf),
}

pub use tree::{CacheNode, FileKind, Node, Tree};
pub use walker::{ScanConfig, scan_tree};
