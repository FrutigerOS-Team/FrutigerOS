//! AeroWelcome — first-boot wizard for FrutigerOS.
//!
//! Shown once after first login. Guides the user through:
//!   1. Welcome message
//!   2. Theme selection
//!   3. Keyboard layout confirmation
//!   4. Network check
//!   5. Optional: install AUR helper
//!   6. Done — mark completed so it doesn't re-launch

mod wizard;

use anyhow::Result;
use slog::{info, o, Drain};

fn build_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("app" => "aero-welcome"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let log = build_logger();

    if wizard::already_completed() {
        info!(log, "AeroWelcome already completed — exiting");
        return Ok(());
    }

    info!(log, "Welcome to FrutigerOS!");
    info!(log, "Tomorrow, Reimagined.");
    info!(log, "Phase 7: GTK4 UI will replace this log output");

    for step in wizard::Step::all() {
        info!(log, "Step {}: {}", step.index() + 1, step.title());
    }

    // Mark completed (writes ~/.config/frutigeros/welcome-done).
    wizard::mark_completed()?;
    info!(log, "Welcome wizard complete.");
    Ok(())
}
