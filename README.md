# diretree

Interactive disk-usage analyzer and optimizer. ratatui TUI + plain text report + JSON.

Inspired by WinDirStat and SpaceSniffer. 

## Install

Requires Rust (`cargo`). Two options:

```sh
cargo install --path .        # install to ~/.cargo/bin
```

Or build from source:

```sh
cargo build --release
```

Binary at `target/release/diretree(.exe)`.

## Usage

```sh
diretree <path>              # interactive TUI
diretree --mode report       # text report
diretree --mode json         # JSON dump
diretree --no-cache          # skip disk cache
diretree --no-ignore         # don't ignore dirs (target, node_modules, ...)
diretree --follow-symlinks   # follow symbolic links
```

Cache at `./cache/diretree/`. Ignores common dirs by default.
