//! AeroWM — Wayland compositor for FrutigerOS
//!
//! Entry point. Initialises logging, loads config, starts the compositor.

mod compositor;
mod config;
// Visual primitives are compiled and unit-tested but not yet consumed by the
// Smithay render path. Keeping them lint-clean flags accidental rot; wiring is
// tracked as Phase 4b (blur/rounded corners/animation uniforms).
#[allow(dead_code)]
mod effects;
mod input;
#[allow(dead_code)]
mod render;
mod statefile;
mod workspace;

use anyhow::Result;
use slog::{info, o, Drain};

pub fn build_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("app" => "aerowm"))
}

fn usage() -> &'static str {
    "usage: aerowm [--save-config]\n  \
     --save-config  write the current/default configuration to \
     ~/.config/aerowm/config.toml and exit"
}

fn main() -> Result<()> {
    let log = build_logger();
    info!(log, "AeroWM v{} starting", env!("CARGO_PKG_VERSION"));

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", usage());
        return Ok(());
    }

    let config = config::Config::load()?;
    info!(
        log,
        "Config: {} workspaces, backend={}", config.workspaces.count, config.general.backend
    );

    if args.iter().any(|a| a == "--save-config") {
        config.save()?;
        info!(log, "Configuration written to ~/.config/aerowm/config.toml");
        return Ok(());
    }

    compositor::run(config, log)
}
