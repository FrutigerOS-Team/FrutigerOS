//! AeroMonitor — system resource monitor for FrutigerOS.
//!
//! Reads CPU, memory, disk, and network stats from /proc and /sys.
//! Phase 6: backend data collection.
//! Phase 7: GTK4 + Aero Glass UI with live graphs.

mod stats;

use anyhow::Result;
use slog::{info, o, Drain};

fn build_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("app" => "aero-monitor"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let log = build_logger();
    info!(log, "AeroMonitor v{}", env!("CARGO_PKG_VERSION"));

    // Sample once and print — the UI loop will call this on a timer.
    let snapshot = stats::SystemSnapshot::collect().await?;
    info!(log, "CPU:    {:.1}%", snapshot.cpu_percent);
    info!(
        log,
        "Memory: {:.1} / {:.1} GB  ({:.1}%)",
        snapshot.mem_used_gb,
        snapshot.mem_total_gb,
        snapshot.mem_percent
    );
    info!(
        log,
        "Swap:   {:.1} / {:.1} GB", snapshot.swap_used_gb, snapshot.swap_total_gb
    );
    for disk in &snapshot.disks {
        info!(
            log,
            "Disk {} ({}): {:.1}/{:.1} GB  ({:.0}%)",
            disk.mount,
            disk.device,
            disk.used_gb,
            disk.total_gb,
            disk.percent
        );
    }
    Ok(())
}
