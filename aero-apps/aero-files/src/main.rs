//! AeroFiles — file manager for FrutigerOS.
//!
//! The current binary is a command-line front end for the same backend that the
//! future Aero Glass UI will use.
//!
//! ```text
//! aero-files [PATH]                    list a directory
//! aero-files --search QUERY [PATH]     recursive name/MIME search
//! aero-files --mkdir PATH              create a directory
//! aero-files --copy SRC DST            copy a regular file
//! aero-files --move SRC DST            rename or move
//! aero-files --delete PATH             delete a file or directory tree
//! ```
//!
//! ## Phase 7+
//! - GTK4 + Aero Glass UI
//! - Thumbnail generation
//! - Archive support (zip, tar.gz)
//! - Network drives (SMB, SFTP)

mod browser;
mod ops;

use std::path::Path;

use anyhow::{anyhow, Result};
use slog::{info, o, Drain};

fn build_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("app" => "aero-files"))
}

fn usage() -> &'static str {
    "usage:\n  \
     aero-files [PATH]\n  \
     aero-files --search QUERY [PATH]\n  \
     aero-files --mkdir PATH\n  \
     aero-files --copy SRC DST\n  \
     aero-files --move SRC DST\n  \
     aero-files --delete PATH"
}

#[tokio::main]
async fn main() -> Result<()> {
    let log = build_logger();
    info!(log, "AeroFiles v{}", env!("CARGO_PKG_VERSION"));

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", usage());
        return Ok(());
    }

    match args.first().map(String::as_str) {
        None => list(&log, default_dir()).await,
        Some("--search") => {
            let query = args.get(1).ok_or_else(|| anyhow!("{}", usage()))?;
            let root = args.get(2).cloned().unwrap_or_else(default_dir);
            search(&log, query, &root).await
        }
        Some("--mkdir") => {
            let path = args.get(1).ok_or_else(|| anyhow!("{}", usage()))?;
            ops::create_dir(Path::new(path)).await?;
            info!(log, "Created {}", path);
            Ok(())
        }
        Some("--copy") => {
            let (from, to) = pair(&args)?;
            ops::copy_file(Path::new(from), Path::new(to)).await?;
            info!(log, "Copied {} -> {}", from, to);
            Ok(())
        }
        Some("--move") => {
            let (from, to) = pair(&args)?;
            ops::rename(Path::new(from), Path::new(to)).await?;
            info!(log, "Moved {} -> {}", from, to);
            Ok(())
        }
        Some("--delete") => {
            let path = args.get(1).ok_or_else(|| anyhow!("{}", usage()))?;
            ops::delete(Path::new(path)).await?;
            info!(log, "Deleted {}", path);
            Ok(())
        }
        Some(flag) if flag.starts_with('-') => Err(anyhow!("unknown option {}\n{}", flag, usage())),
        Some(path) => list(&log, path.to_string()).await,
    }
}

async fn list(log: &slog::Logger, path: String) -> Result<()> {
    let entries = browser::list_directory(&path).await?;
    for e in &entries {
        let kind = if e.is_dir { "DIR " } else { "FILE" };
        let link = if e.is_symlink { " ->" } else { "" };
        info!(
            log,
            "{} {:>12}  {}{}  [{}]", kind, e.size_human, e.name, link, e.mime_type
        );
    }
    info!(log, "{} entries in {}", entries.len(), path);
    Ok(())
}

async fn search(log: &slog::Logger, query: &str, root: &str) -> Result<()> {
    let hits = ops::search(
        Path::new(root),
        query,
        ops::DEFAULT_SEARCH_LIMIT,
        ops::DEFAULT_SEARCH_DEPTH,
    )
    .await?;
    for e in &hits {
        info!(log, "{}  [{}]", e.path, e.mime_type);
    }
    info!(
        log,
        "{} match(es) for {:?} under {}",
        hits.len(),
        query,
        root
    );
    Ok(())
}

fn pair(args: &[String]) -> Result<(&str, &str)> {
    match (args.get(1), args.get(2)) {
        (Some(from), Some(to)) => Ok((from, to)),
        _ => Err(anyhow!("{}", usage())),
    }
}

fn default_dir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/".into())
}
