use std::io::Write;

use diretree::cli::Mode;
use diretree::{ScanConfig, scan_tree};

fn main() {
    let cli = diretree::cli::parse();
    let cfg = ScanConfig {
        follow_symlinks: cli.follow_symlinks,
        max_depth: cli.max_depth,
        ignore_dirs: !cli.no_ignore,
    };
    let scanned = if cli.no_cache {
        scan_tree(&cli.path, cfg)
    } else if let Some(t) = diretree::scan::cache::load(
        &cli.cache_dir,
        &cli.path,
        cfg.ignore_dirs,
        cfg.follow_symlinks,
    ) {
        Ok(t)
    } else {
        scan_tree(&cli.path, cfg).inspect(|t| {
            diretree::scan::cache::save(
                &cli.cache_dir,
                &cli.path,
                cfg.ignore_dirs,
                cfg.follow_symlinks,
                t,
            );
        })
    };
    let tree = match scanned {
        Ok(t) => t,
        Err(e) => {
            eprintln!("diretree: {e}");
            std::process::exit(1);
        }
    };

    let result: std::io::Result<()> = match cli.mode {
        Mode::Tui => diretree::tui::run(tree, cfg).map_err(|e| std::io::Error::other(format!("{e}"))),
        Mode::Report => {
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            diretree::cli::report::print_report(&tree, cli.top, cli.min_size, &mut lock)
        }
        Mode::Json => {
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            diretree::cli::json::print_json(&tree, &mut lock)
        }
    };

    if let Err(e) = result {
        eprintln!("diretree: {e}");
        std::process::exit(1);
    }
    let _ = std::io::stdout().flush();
}
