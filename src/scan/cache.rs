use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use super::tree::{CacheNode, Tree};

fn cache_file(
    cache_dir: &Path,
    target: &Path,
    ignore_dirs: bool,
    follow_symlinks: bool,
) -> PathBuf {
    let mut h = DefaultHasher::new();
    target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf())
        .hash(&mut h);
    ignore_dirs.hash(&mut h);
    follow_symlinks.hash(&mut h);
    cache_dir.join(format!("{:016x}.json", h.finish()))
}

/// Load a cached tree if fresh (target not modified after the cache file).
pub fn load(
    cache_dir: &Path,
    target: &Path,
    ignore_dirs: bool,
    follow_symlinks: bool,
) -> Option<Tree> {
    let file = cache_file(cache_dir, target, ignore_dirs, follow_symlinks);
    let fmod = std::fs::metadata(&file).ok()?.modified().ok()?;
    let tmod = std::fs::metadata(target).ok()?.modified().ok()?;
    if fmod < tmod {
        return None;
    }
    let bytes = std::fs::read(&file).ok()?;
    let nodes: Vec<CacheNode> = serde_json::from_slice(&bytes).ok()?;
    Some(Tree::from_cache(nodes))
}

/// Persist the scanned tree to the cache. Failures are silently ignored.
pub fn save(
    cache_dir: &Path,
    target: &Path,
    ignore_dirs: bool,
    follow_symlinks: bool,
    tree: &Tree,
) {
    if std::fs::create_dir_all(cache_dir).is_err() {
        return;
    }
    let file = cache_file(cache_dir, target, ignore_dirs, follow_symlinks);
    let nodes = tree.to_cache();
    if let Ok(json) = serde_json::to_vec_pretty(&nodes) {
        let _ = std::fs::write(file, json);
    }
}
