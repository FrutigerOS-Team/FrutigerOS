//! AeroTerminal — terminal emulator for FrutigerOS.
//!
//! Phase 6: PTY management, VT100 parser, config.
//! Phase 7: Wayland window with Aero Glass frame and GPU-rendered text.

mod config;
mod pty;
mod vt;

use anyhow::Result;
use slog::{info, o, Drain};

fn build_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("app" => "aero-terminal"))
}

/// Read a positive terminal dimension from the environment.
fn env_dimension(key: &str, default: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(default)
}

#[tokio::main]
async fn main() -> Result<()> {
    let log = build_logger();
    info!(log, "AeroTerminal v{}", env!("CARGO_PKG_VERSION"));

    let cfg = config::TermConfig::default();
    info!(log, "Shell: {}", cfg.shell);
    info!(log, "Font: {} {}pt", cfg.font_family, cfg.font_size);
    info!(log, "Glass opacity: {}", cfg.glass_opacity);

    // Terminal size comes from the environment when launched inside a window
    // manager, falling back to the classic 80x24.
    let rows = env_dimension("LINES", 24);
    let cols = env_dimension("COLUMNS", 80);
    let mut pty = pty::Pty::new(&cfg.shell, rows, cols);
    pty.resize(rows, cols);
    info!(
        log,
        "PTY: {} rows x {} cols, shell {}", pty.rows, pty.cols, pty.shell
    );

    // Exercise the parser so startup validates the backend without a UI.
    let mut parser = vt::ParserState::default();
    for byte in b"AeroTerminal ready\r\n" {
        if let Some(vt::TermAction::Print(c)) = vt::parse_byte(&mut parser, *byte) {
            print!("{c}");
        }
    }
    println!();

    info!(
        log,
        "Phase 7: GTK4 window with GPU text rendering not yet implemented"
    );

    Ok(())
}
