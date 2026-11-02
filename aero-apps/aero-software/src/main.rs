//! AeroSoftware — graphical package manager for FrutigerOS.
//!
//! A frontend for pacman (and optionally an AUR helper).
//! Phase 6: package query, install, remove logic.
//! Phase 7: GTK4 + Aero Glass UI.
//!
//! ```text
//! aero-software                     summary
//! aero-software --search QUERY      search installed packages
//! aero-software --install NAME --yes
//! aero-software --remove NAME --yes
//! ```
//!
//! Install/remove shell out to `pkexec pacman`, which prompts for
//! authentication. Both require `--yes` to confirm the operation.

mod packages;

use anyhow::{anyhow, Result};
use slog::{info, o, Drain};

fn build_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("app" => "aero-software"))
}

fn usage() -> &'static str {
    "usage:\n  \
     aero-software\n  \
     aero-software --search QUERY\n  \
     aero-software --install NAME --yes\n  \
     aero-software --remove NAME --yes"
}

#[tokio::main]
async fn main() -> Result<()> {
    let log = build_logger();
    info!(log, "AeroSoftware v{}", env!("CARGO_PKG_VERSION"));

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", usage());
        return Ok(());
    }
    let confirmed = args.iter().any(|a| a == "--yes");

    let db = packages::PackageDb::load().await?;

    match args.first().map(String::as_str) {
        None => {
            info!(log, "{} packages installed", db.installed_count());
            info!(log, "{} updates available", db.updates_available());
        }
        Some("--search") => {
            let query = args.get(1).ok_or_else(|| anyhow!("{}", usage()))?;
            let hits = db.search(query);
            for pkg in &hits {
                info!(log, "  {} {}", pkg.name, pkg.version);
            }
            info!(log, "{} match(es) for {:?}", hits.len(), query);
        }
        Some("--install") | Some("--remove") => {
            let action = args[0].clone();
            let name = args.get(1).ok_or_else(|| anyhow!("{}", usage()))?;
            if !confirmed {
                return Err(anyhow!(
                    "refusing to {} {:?} without --yes\n{}",
                    action.trim_start_matches("--"),
                    name,
                    usage()
                ));
            }
            if action == "--install" {
                info!(log, "Installing {}…", name);
                db.install(name).await?;
                info!(log, "Installed {}", name);
            } else {
                info!(log, "Removing {}…", name);
                db.remove(name).await?;
                info!(log, "Removed {}", name);
            }
        }
        Some(flag) if flag.starts_with('-') => {
            return Err(anyhow!("unknown option {}\n{}", flag, usage()));
        }
        Some(other) => return Err(anyhow!("unexpected argument {:?}\n{}", other, usage())),
    }

    Ok(())
}
