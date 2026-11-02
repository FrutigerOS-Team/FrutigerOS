//! AeroShell — desktop shell for FrutigerOS.
//!
//! Runs as a Wayland client connected to AeroWM.
//! Provides: panel (taskbar), AeroStart launcher, notification center.

mod config;
mod glyphs;
// The AeroStart popup (Phase 4) consumes the launcher API; it is compiled and
// unit-tested today so discovery/launch behaviour does not rot.
#[allow(dead_code)]
mod launcher;
// Only `run` is active until the D-Bus front end and toast surface land (Phase 5).
#[allow(dead_code)]
mod notifications;
mod panel;
mod panel_render;
// Design-system primitives; panel_render uses the palette today, and the
// remaining surfaces/controls are consumed by AeroStart and toasts.
#[allow(dead_code)]
mod widgets;

use anyhow::Result;
use slog::{info, o, Drain};

pub fn build_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("app" => "aeroshell"))
}

fn usage() -> &'static str {
    "usage: aeroshell [--save-config]\n  \
     --save-config  write the current/default configuration to \
     ~/.config/aeroshell/config.toml and exit"
}

#[tokio::main]
async fn main() -> Result<()> {
    let log = build_logger();
    info!(log, "AeroShell v{} starting", env!("CARGO_PKG_VERSION"));

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", usage());
        return Ok(());
    }

    let cfg = config::ShellConfig::load()?;
    if args.iter().any(|a| a == "--save-config") {
        cfg.save()?;
        info!(
            log,
            "Configuration written to ~/.config/aeroshell/config.toml"
        );
        return Ok(());
    }
    info!(
        log,
        "Panel height: {}px, launcher key: {}", cfg.panel.height, cfg.panel.launcher_key
    );

    // Start the notification daemon in the background.
    let notif_log = log.new(o!("module" => "notifications"));
    tokio::spawn(async move {
        notifications::run(notif_log).await;
    });

    // Run the panel on a blocking thread. It owns the Wayland connection and
    // blocks until the compositor closes it.
    let panel_log = log.new(o!("module" => "panel"));
    tokio::task::spawn_blocking(move || panel::run(cfg, panel_log))
        .await
        .map_err(|e| anyhow::anyhow!("panel task panicked: {}", e))??;

    Ok(())
}
