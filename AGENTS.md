# AGENTS.md

Rust binary + lib: `diretree` — interactive disk-usage analyzer (ratatui TUI + plain-text report + JSON). Cargo workspace, single crate, edition 2024.

## Commands

- Build: `cargo build` (Windows target; builds `diretree.exe`)
- Run: `cargo run -- <path>` — defaults to TUI. Use `--mode report` / `--mode json` for non-interactive output.
- Test: `cargo test` (unit tests only, in `src/scan/tree.rs`, `src/units.rs`, `src/tui/theme.rs`)
- No CI, no lint config, no custom formatter config (stock rustfmt). Nothing else to run.

## Gotchas an agent will likely miss

- **Cache writes by default.** `scan_tree` caches results to `./cache/diretree/` (flag `--cache-dir`, disable with `--no-cache`). `.gitignore` only ignores `/target` — `cache/` is NOT ignored. Cache files are keyed by hashed canonical target path + `ignore_dirs` + `follow_symlinks`, and invalidated only when the target's mtime is newer. When testing scan changes, use `--no-cache` or a temp `--cache-dir`; do not commit `cache/`.
- **Ignored dirs are skipped by default.** `IGNORE_DIRS` in `src/scan/walker.rs` (target, node_modules, venv, .git, dist, build, …) is applied unless `--no-ignore`. Scanning this repo itself will never show the `target/` directory — this is expected, not a bug.
- **`eza.txt` is a stray debug dump** (a full `eza -T` listing of this repo incl. `target/`). Not documentation; ignore/delete it.
- **Symlinks handled via `symlink_metadata`** (doesn't follow links). Following is opt-in (`--follow-symlinks`) and bounded by `max_depth` (default 32) plus a canonical-path visited set to stop loops.

## Architecture

- `src/lib.rs` — crate root, re-exports `scan_tree`, `ScanConfig`, `Tree`, `human_bytes`. Keep these public API names stable.
- `src/main.rs` — thin: parse CLI → load-or-scan (cache) → dispatch to TUI/report/json.
- `src/scan/` — traversal. `walker.rs` owns the parallel (rayon) filesystem walk producing an arena `Tree`; `tree.rs` owns the arena data structure; `cache.rs` the JSON disk cache.
- `src/cli/` — `mod.rs` (clap args + `Mode` enum), `report.rs` (`top_entries` + plain-text), `json.rs` (serde dump).
- `src/tui/` — ratatui app (`app.rs`), palette (`theme.rs`), squarified treemap (`treemap/`).
- `src/units.rs` — `human_bytes` (binary KiB/MiB/GiB/TiB).

## `Tree` data structure invariants (edit with care)

`Tree` is an arena (`Vec<Node>`) using first-child/next-sibling encoding:
- `add_child` head-links, so children are stored in **reverse insertion order**; `children(i)` re-reverses to oldest-first. Always go through `children()` / `indices()`, never walk `first_child`/`next_sibling` directly.
- **Children always have larger indices than their parent.** `compute_sizes` relies on iterating indices in reverse to fold sizes bottom-up — do not break this ordering (e.g. by inserting nodes out of order).
- `tree.rs` and `units.rs` / `theme.rs` contain unit tests asserting these invariants; keep them green when touching the arena.
