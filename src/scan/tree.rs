use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum FileKind {
    File,
    Dir,
    Symlink,
}

/// Flat, serde-friendly view of the tree used for disk caching.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheNode {
    pub name: String,
    pub kind: FileKind,
    pub size: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub path: PathBuf,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

/// One node in the arena tree. `first_child`/`next_sibling` form a
/// singly-linked children list (first-child/next-sibling encoding).
#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub kind: FileKind,
    pub size: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub parent: Option<usize>,
    pub first_child: Option<usize>,
    pub next_sibling: Option<usize>,
    /// cache of own path, built lazily on demand
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    pub fn new_root(name: String, path: PathBuf) -> Self {
        Self {
            nodes: vec![Node {
                name,
                kind: FileKind::Dir,
                size: 0,
                file_count: 0,
                dir_count: 0,
                parent: None,
                first_child: None,
                next_sibling: None,
                path,
            }],
        }
    }

    pub fn root(&self) -> usize {
        0
    }

    pub fn node(&self, idx: usize) -> &Node {
        &self.nodes[idx]
    }

    pub fn node_mut(&mut self, idx: usize) -> &mut Node {
        &mut self.nodes[idx]
    }

    /// Add a child to `parent`, returning its index. Children are kept in
    /// reverse insertion order (head-linked); caller controls order.
    pub fn add_child(
        &mut self,
        parent: usize,
        name: String,
        kind: FileKind,
        path: PathBuf,
    ) -> usize {
        let idx = self.nodes.len();
        let parent_first = self.nodes[parent].first_child;
        self.nodes.push(Node {
            name,
            kind,
            size: 0,
            file_count: if kind == FileKind::File { 1 } else { 0 },
            dir_count: if kind == FileKind::Dir { 1 } else { 0 },
            parent: Some(parent),
            first_child: None,
            next_sibling: parent_first,
            path,
        });
        self.nodes[parent].first_child = Some(idx);
        idx
    }

    /// Iterate children of `idx` in insertion (oldest-first) order.
    pub fn children(&self, idx: usize) -> impl Iterator<Item = usize> + '_ {
        let mut order = Vec::new();
        let mut next = self.nodes[idx].first_child;
        while let Some(cur) = next {
            order.push(cur);
            next = self.nodes[cur].next_sibling;
        }
        order.reverse();
        order.into_iter()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Iterate every node index (DFS, pre-order).
    pub fn indices(&self) -> impl Iterator<Item = usize> + '_ {
        let mut out = Vec::with_capacity(self.nodes.len());
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            out.push(i);
            let mut kids: Vec<_> = self.children(i).collect();
            kids.reverse();
            stack.extend(kids);
        }
        out.into_iter()
    }

    /// Total size of subtree rooted at `idx`.
    pub fn subtree_size(&self, idx: usize) -> u64 {
        self.node(idx).size
    }

    pub fn has_children(&self, idx: usize) -> bool {
        self.nodes[idx].first_child.is_some()
    }

    /// Aggregate child sizes up into ancestors.
    pub fn compute_sizes(&mut self) {
        // post-order: children before parent. Reverse index guarantees children
        // (which have larger indices than their parent) are folded first.
        for i in (0..self.nodes.len()).rev() {
            let parent = self.nodes[i].parent;
            if let Some(p) = parent {
                let (sz, fc, dc) = {
                    let c = &self.nodes[i];
                    (c.size, c.file_count, c.dir_count)
                };
                let n = &mut self.nodes[p];
                n.size += sz;
                n.file_count += fc;
                n.dir_count += dc;
            }
        }
    }

    /// Flat, serde-friendly snapshot for disk caching.
    pub fn to_cache(&self) -> Vec<CacheNode> {
        (0..self.nodes.len())
            .map(|i| {
                let n = &self.nodes[i];
                CacheNode {
                    name: n.name.clone(),
                    kind: n.kind,
                    size: n.size,
                    file_count: n.file_count,
                    dir_count: n.dir_count,
                    path: n.path.clone(),
                    parent: n.parent,
                    children: self.children(i).collect(),
                }
            })
            .collect()
    }

    /// Rebuild a tree from a `to_cache` snapshot.
    pub fn from_cache(nodes: Vec<CacheNode>) -> Tree {
        let mut t = Tree {
            nodes: Vec::with_capacity(nodes.len()),
        };
        for _ in 0..nodes.len() {
            t.nodes.push(Node {
                name: String::new(),
                kind: FileKind::File,
                size: 0,
                file_count: 0,
                dir_count: 0,
                parent: None,
                first_child: None,
                next_sibling: None,
                path: PathBuf::new(),
            });
        }
        for (i, c) in nodes.into_iter().enumerate() {
            // raw chain is head-linked (reverse of insertion); children() re-reverses
            let first_child = c.children.last().copied();
            {
                let n = &mut t.nodes[i];
                n.name = c.name;
                n.kind = c.kind;
                n.size = c.size;
                n.file_count = c.file_count;
                n.dir_count = c.dir_count;
                n.path = c.path;
                n.parent = c.parent;
                n.first_child = first_child;
            }
            for k in (0..c.children.len()).rev() {
                let child = c.children[k];
                t.nodes[child].next_sibling = c.children.get(k.wrapping_sub(1)).copied();
            }
        }
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_aggregate_up() {
        let mut t = Tree::new_root("root".into(), PathBuf::from("root"));
        let r = t.root();
        let a = t.add_child(r, "a".into(), FileKind::Dir, PathBuf::from("root/a"));
        let b = t.add_child(r, "b".into(), FileKind::Dir, PathBuf::from("root/b"));
        t.add_child(a, "f1".into(), FileKind::File, PathBuf::from("root/a/f1"));
        let f2 = t.add_child(a, "f2".into(), FileKind::File, PathBuf::from("root/a/f2"));
        // dirs hold 0 direct size; files hold their own bytes
        t.node_mut(f2).size = 3;
        t.node_mut(b).size = 7;
        t.compute_sizes();

        // a subtree = 3 (f2 only), b subtree = 7, root = 10
        assert_eq!(t.node(a).size, 3);
        assert_eq!(t.node(b).size, 7);
        assert_eq!(t.node(r).size, 10);
        // file_count: a has f1+f2 = 2, root = 2 files total
        assert_eq!(t.node(a).file_count, 2);
        assert_eq!(t.node(r).file_count, 2);
        assert_eq!(t.node(r).dir_count, 2);
    }

    #[test]
    fn children_iteration_order() {
        let mut t = Tree::new_root("root".into(), PathBuf::from("root"));
        let r = t.root();
        let a = t.add_child(r, "a".into(), FileKind::Dir, PathBuf::from("a"));
        let b = t.add_child(r, "b".into(), FileKind::Dir, PathBuf::from("b"));
        let c = t.add_child(r, "c".into(), FileKind::Dir, PathBuf::from("c"));
        let got: Vec<usize> = t.children(r).collect();
        assert_eq!(got, vec![a, b, c]);
        assert_eq!(t.len(), 4);
    }

    #[test]
    fn cache_roundtrip_preserves_tree() {
        let mut t = Tree::new_root("root".into(), PathBuf::from("root"));
        let r = t.root();
        let a = t.add_child(r, "a".into(), FileKind::Dir, PathBuf::from("root/a"));
        t.add_child(r, "b".into(), FileKind::Dir, PathBuf::from("root/b"));
        t.add_child(a, "f1".into(), FileKind::File, PathBuf::from("root/a/f1"));
        let f2 = t.add_child(a, "f2".into(), FileKind::File, PathBuf::from("root/a/f2"));
        t.node_mut(f2).size = 3;
        t.compute_sizes();

        let cache = t.to_cache();
        let t2 = Tree::from_cache(cache);

        assert_eq!(t2.len(), t.len());
        assert_eq!(t2.node(t2.root()).size, t.node(r).size);
        assert_eq!(t2.node(t2.root()).file_count, t.node(r).file_count);
        let kids2: Vec<_> = t2.children(t2.root()).collect();
        let kids1: Vec<_> = t.children(r).collect();
        assert_eq!(kids2, kids1);
        for (i, j) in kids1.iter().zip(&kids2) {
            assert_eq!(t2.node(*j).name, t.node(*i).name);
        }
    }
}
