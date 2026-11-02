//! AeroSettings — system configuration app for FrutigerOS.

mod pages;

use anyhow::Result;
use slog::{info, o, Drain};

fn build_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("app" => "aero-settings"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let log = build_logger();
    info!(log, "AeroSettings v{}", env!("CARGO_PKG_VERSION"));
    info!(log, "Available sections:");
    for page in pages::all_pages() {
        info!(
            log,
            "  {} {} — {} ({})", page.icon, page.title, page.id, page.description
        );
    }
    Ok(())
}
