//! AeroScreenshot — screenshot tool for FrutigerOS.
//!
//! Captures through `grim` on Wayland, optionally copies to the clipboard with
//! `wl-copy`, and shows a `notify-send` notification. Region/window captures use
//! `slurp`; all arguments are passed without a shell.
//!
//! ```text
//! aero-screenshot [fullscreen|region|window|delayed] [--no-clipboard] [--no-notify]
//! ```
//!
//! ## Phase 7+
//! - Aero Glass UI with annotate/crop tools
//! - Recording via wf-recorder

mod capture;

use anyhow::Result;
use slog::{info, o, Drain};

fn build_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("app" => "aero-screenshot"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let log = build_logger();
    info!(log, "AeroScreenshot v{}", env!("CARGO_PKG_VERSION"));

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "usage: aero-screenshot [fullscreen|region|window|delayed] [--no-clipboard] [--no-notify]"
        );
        return Ok(());
    }

    let mode = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
        .unwrap_or("fullscreen");
    let mode = capture::CaptureMode::from_str(mode);

    let mut cfg = capture::CaptureConfig::default();
    if args.iter().any(|a| a == "--no-clipboard") {
        cfg.copy_to_clipboard = false;
    }
    if args.iter().any(|a| a == "--no-notify") {
        cfg.show_notification = false;
    }

    info!(log, "Mode: {:?}", mode);
    let path = capture::capture(&cfg, mode, &log).await?;
    info!(log, "Screenshot written to {}", path.display());
    Ok(())
}
