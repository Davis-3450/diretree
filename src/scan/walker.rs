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
    let mut tree = Tree::new_root(root.name, root_path);
    let root_idx = tree.root();
    append_children(&mut tree, root_idx, root.children);
    tree.node_mut(tree.root()).size = root.size;
    tree.compute_sizes();
    Ok(tree)
}

fn append_children(tree: &mut Tree, parent: usize, children: Vec<REntry>) {
    for c in children {
        let idx = tree.add_child(parent, c.name, c.kind, c.path);
        tree.node_mut(idx).size = c.size;
        append_children(tree, idx, c.children);
    }
}

impl Walker {
    fn scan_dir(&self, path: &Path, depth: u32) -> Result<REntry, ScanError> {
        // depth cap: terminate runaway/looping recursion
        if depth >= self.cfg.max_depth {
            return self.leaf_dir(path);
        }

        // cycle detection via canonical real path: ONLY required when follow_symlinks is enabled
        if self.cfg.follow_symlinks {
            if let Ok(canon) = path.canonicalize()
                && !self.visited.lock().unwrap().insert(canon)
            {
                return self.leaf_dir(path);
            }
        }

        let rd = match std::fs::read_dir(path) {
            Ok(rd) => rd,
            Err(_) => return Err(ScanError::Unreadable(path.to_path_buf())),
        };

        // Partition entries into direct files/symlinks and subdirectories
        let mut subdirs = Vec::new();
        let mut files = Vec::new();

        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if file_type.is_dir() {
                if self.cfg.ignore_dirs && is_ignored(&name) {
                    files.push(REntry {
                        name,
                        kind: FileKind::Dir,
                        path: entry.path(),
                        size: 0,
                        children: vec![],
                    });
                } else {
                    subdirs.push((entry.path(), name));
                }
            } else if file_type.is_symlink() {
                let follow = self.cfg.follow_symlinks
                    && depth < self.cfg.max_depth
                    && !(self.cfg.ignore_dirs && is_ignored(&name));

                if follow && let Ok(rmd) = std::fs::metadata(&entry.path()) {
                    if rmd.is_dir() {
                        subdirs.push((entry.path(), name));
                    } else if rmd.is_file() {
                        files.push(REntry {
                            name,
                            kind: FileKind::File,
                            path: entry.path(),
                            size: rmd.len(),
                            children: vec![],
                        });
                    } else {
                        let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        files.push(REntry {
                            name,
                            kind: FileKind::Symlink,
                            path: entry.path(),
                            size: len,
                            children: vec![],
                        });
                    }
                } else {
                    // Use entry.metadata() directly avoiding redundant syscall on Windows/Unix
                    let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    files.push(REntry {
                        name,
                        kind: FileKind::Symlink,
                        path: entry.path(),
                        size: len,
                        children: vec![],
                    });
                }
            } else {
                // Regular file: entry.metadata() reuses OS cached attributes without second syscall
                let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
                files.push(REntry {
                    name,
                    kind: FileKind::File,
                    path: entry.path(),
                    size: len,
                    children: vec![],
                });
            }
        }

        // Subdirectories recurse in parallel using Rayon divide-and-conquer
        let mut subdir_results: Vec<REntry> = subdirs
            .into_par_iter()
            .map(|(sub_path, sub_name)| {
                self.scan_dir(&sub_path, depth + 1).unwrap_or_else(|_| REntry {
                    name: sub_name,
                    kind: FileKind::Dir,
                    path: sub_path,
                    size: 0,
                    children: vec![],
                })
            })
            .collect();

        // Combine files and directories
        files.append(&mut subdir_results);

        Ok(REntry {
            name: path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            kind: FileKind::Dir,
            path: path.to_path_buf(),
            size: 0,
            children: files,
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
}
