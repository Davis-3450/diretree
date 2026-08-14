use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rayon::prelude::*;

use super::tree::{FileKind, Tree};
use crate::scan::ScanError;

/// Directory names never recursed into by default (matching is case-insensitive).
const IGNORE_DIRS: &[&str] = &[
    "target",
    "node_modules",
    "venv",
    ".venv",
    "__pycache__",
    ".git",
    ".svn",
    ".hg",
    "dist",
    "build",
];

#[derive(Clone, Copy, Debug, Default)]
pub struct ScanConfig {
    /// Follow symlinks (with loop guard via max depth). Default: do not follow.
    pub follow_symlinks: bool,
    /// Max recursion depth for all directory traversal. Guards against
    /// runaway recursion and symlink/junction loops.
    pub max_depth: u32,
    /// Stop recursion into well-known dependency/build directories.
    pub ignore_dirs: bool,
}

fn is_ignored(name: &str) -> bool {
    IGNORE_DIRS.iter().any(|d| name.eq_ignore_ascii_case(d))
}

#[derive(Debug)]
struct REntry {
    name: String,
    kind: FileKind,
    path: PathBuf,
    /// direct size contribution (file bytes or symlink len); 0 for dirs
    size: u64,
    children: Vec<REntry>,
}

/// Owns traversal state: config + canonical-path visited set for cycle detection.
struct Walker {
    cfg: ScanConfig,
    visited: Mutex<HashSet<PathBuf>>,
}

/// Scan `path` into a fully-built tree.
pub fn scan_tree(path: &Path, cfg: ScanConfig) -> Result<Tree, ScanError> {
    if !path.exists() {
        return Err(ScanError::NotFound(path.to_path_buf()));
    }
    let walker = Walker {
        cfg,
        visited: Mutex::new(HashSet::new()),
    };
    let root = walker.scan_dir(path, 0).unwrap_or_else(|_e| REntry {
        name: path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        kind: FileKind::Dir,
        path: path.to_path_buf(),
        size: 0,
        children: Vec::new(),
    });
    let root_path = path.to_path_buf();
    let mut tree = Tree::new_root(root.name.clone(), root_path);
    let root_idx = tree.root();
    append_children(&mut tree, root_idx, &root.children);
    tree.node_mut(tree.root()).size = root.size;
    tree.compute_sizes();
    Ok(tree)
}

fn append_children(tree: &mut Tree, parent: usize, children: &[REntry]) {
    for c in children {
        let idx = tree.add_child(parent, c.name.clone(), c.kind, c.path.clone());
        tree.node_mut(idx).size = c.size;
        append_children(tree, idx, &c.children);
    }
}

impl Walker {
    fn scan_dir(&self, path: &Path, depth: u32) -> Result<REntry, ScanError> {
        // depth cap: terminate runaway/looping recursion
        if depth >= self.cfg.max_depth {
            return self.leaf_dir(path);
        }
        // cycle detection via canonical real path (catches junctions/symlink loops)
        if let Ok(canon) = path.canonicalize()
            && !self.visited.lock().unwrap().insert(canon)
        {
            return self.leaf_dir(path);
        }

        let rd = match std::fs::read_dir(path) {
            Ok(rd) => rd,
            Err(_) => return Err(ScanError::Unreadable(path.to_path_buf())),
        };

        let entries: Vec<_> = rd.flatten().collect();
        let results: Vec<REntry> = entries
            .par_iter()
            .map(|e| {
                self.scan_entry(
                    &e.path(),
                    e.file_name().to_string_lossy().into_owned(),
                    depth,
                )
            })
            .collect();

        Ok(REntry {
            name: path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            kind: FileKind::Dir,
            path: path.to_path_buf(),
            size: 0,
            children: results,
        })
    }

    fn leaf_dir(&self, path: &Path) -> Result<REntry, ScanError> {
        Ok(REntry {
            name: path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            kind: FileKind::Dir,
            path: path.to_path_buf(),
            size: 0,
            children: vec![],
        })
    }

    fn scan_entry(&self, path: &Path, name: String, depth: u32) -> REntry {
        // symlink_metadata does not follow the link
        let md = match std::fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(_) => {
                return REntry {
                    name,
                    kind: FileKind::File,
                    path: path.to_path_buf(),
                    size: 0,
                    children: vec![],
                };
            }
        };

        if md.file_type().is_symlink() {
            let follow = self.cfg.follow_symlinks
                && depth < self.cfg.max_depth
                && !(self.cfg.ignore_dirs && is_ignored(&name));
            if follow && let Ok(rmd) = std::fs::metadata(path) {
                if rmd.is_dir()
                    && let Ok(sub) = self.scan_dir(path, depth + 1)
                {
                    return sub;
                } else if rmd.is_file() {
                    return REntry {
                        name,
                        kind: FileKind::File,
                        path: path.to_path_buf(),
                        size: rmd.len(),
                        children: vec![],
                    };
                }
            }
            // don't follow / not a file: record as symlink with its own len
            return REntry {
                name,
                kind: FileKind::Symlink,
                path: path.to_path_buf(),
                size: md.len(),
                children: vec![],
            };
        }

        if md.is_dir() {
            if self.cfg.ignore_dirs && is_ignored(&name) {
                // keep the node visible but do not recurse
                return REntry {
                    name,
                    kind: FileKind::Dir,
                    path: path.to_path_buf(),
                    size: 0,
                    children: vec![],
                };
            }
            return self.scan_dir(path, depth + 1).unwrap_or_else(|_| REntry {
                name,
                kind: FileKind::Dir,
                path: path.to_path_buf(),
                size: 0,
                children: vec![],
            });
        }

        REntry {
            name,
            kind: FileKind::File,
            path: path.to_path_buf(),
            size: md.len(),
            children: vec![],
        }
    }
}
