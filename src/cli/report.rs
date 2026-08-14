use std::io::Write;

use crate::scan::Tree;
use crate::units::human_bytes;

/// Top-N largest entries across the whole tree, sorted by subtree size desc.
pub fn top_entries(tree: &Tree, n: usize, min_size: u64) -> Vec<usize> {
    let mut all: Vec<usize> = tree
        .indices()
        .filter(|&i| tree.node(i).size >= min_size)
        .collect();
    all.sort_by_key(|&i| std::cmp::Reverse(tree.node(i).size));
    all.truncate(n);
    all
}

pub fn print_report(
    tree: &Tree,
    top: usize,
    min_size: u64,
    out: &mut dyn Write,
) -> std::io::Result<()> {
    let root = tree.node(tree.root());
    writeln!(out, "diretree — {}", root.path.display())?;
    writeln!(
        out,
        "total: {}   files: {}   dirs: {}",
        human_bytes(root.size),
        root.file_count,
        root.dir_count
    )?;
    writeln!(out, "---")?;

    let entries = top_entries(tree, top, min_size);
    let root_size = root.size.max(1);
    for (rank, idx) in entries.iter().enumerate() {
        let n = tree.node(*idx);
        let pct = n.size as f64 / root_size as f64 * 100.0;
        let kind = match n.kind {
            crate::scan::FileKind::Dir => "DIR",
            crate::scan::FileKind::File => "FILE",
            crate::scan::FileKind::Symlink => "LINK",
        };
        writeln!(
            out,
            "{:>3}. [{:>4} {:>6.2}%] {:>10}  {}  {}",
            rank + 1,
            kind,
            pct,
            human_bytes(n.size),
            n.file_count,
            n.path.display()
        )?;
    }
    Ok(())
}
