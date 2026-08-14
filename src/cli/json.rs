use std::io::Write;

use serde::Serialize;

use crate::scan::Tree;

#[derive(Serialize)]
struct JsonNode<'a> {
    name: &'a str,
    #[serde(rename = "type")]
    kind: &'static str,
    size: u64,
    file_count: u64,
    dir_count: u64,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<JsonNode<'a>>>,
}

fn to_json(tree: &Tree, idx: usize) -> JsonNode<'_> {
    let n = tree.node(idx);
    let kind = match n.kind {
        crate::scan::FileKind::Dir => "dir",
        crate::scan::FileKind::File => "file",
        crate::scan::FileKind::Symlink => "symlink",
    };
    let children: Vec<_> = tree.children(idx).map(|c| to_json(tree, c)).collect();
    let children = if children.is_empty() {
        None
    } else {
        Some(children)
    };
    JsonNode {
        name: &n.name,
        kind,
        size: n.size,
        file_count: n.file_count,
        dir_count: n.dir_count,
        path: n.path.to_string_lossy().into_owned(),
        children,
    }
}

pub fn print_json(tree: &Tree, out: &mut dyn Write) -> std::io::Result<()> {
    let root = to_json(tree, tree.root());
    serde_json::to_writer_pretty(&mut *out, &root)?;
    writeln!(out)
}
