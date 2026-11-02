//! AeroInstaller — graphical installer for FrutigerOS.
//!
//! Installation flow:
//!   Welcome → Language → Keyboard → Network → Disk → Partitioning →
//!   Install type → User creation → Timezone → Installation → Complete
//!
//! This binary is the orchestrator: it drives the installation state machine
//! and coordinates the backend (disk ops, pacstrap, bootloader) with the UI.
//!
//! ## Phase 5 scope
//! - Full state machine and step definitions
//! - Disk probe (lsblk-based, no destructive ops)
//! - Partition plan builder (auto + manual)
//! - Command planner for sgdisk/mkfs/mount/pacstrap/arch-chroot/GRUB
//! - Dry-run by default; `--apply --yes` executes the plan
//!
//! ## Phase 6+
//! - GTK4 / Aero Glass UI on top of this backend

mod disk;
mod install;
mod steps;
mod ui;

use anyhow::{anyhow, Result};
use slog::{info, o, Drain};
use steps::{InstallStep, InstallerState};

use crate::install::ExecutionMode;

pub fn build_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("app" => "aero-installer"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let log = build_logger();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("aero-installer {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let mode = parse_mode(&args)?;
    ui::print_header("FrutigerOS Installation Wizard");
    info!(log, "AeroInstaller v{}", env!("CARGO_PKG_VERSION"));
    info!(log, "FrutigerOS Installation Wizard");
    info!(
        log,
        "Mode: {}",
        match mode {
            ExecutionMode::DryRun => "dry-run (no disk changes)",
            ExecutionMode::Apply => "APPLY (destructive)",
        }
    );
    if mode == ExecutionMode::Apply {
        ui::print_warning("This will erase the selected disk. Press Ctrl+C to abort.");
    }

    let mut state = InstallerState::with_mode(mode);

    // Drive the installation state machine in CLI mode.
    // The graphical UI will replace this loop in Phase 6.
    loop {
        let current = state.current_step();
        ui::print_progress(current.index(), InstallStep::TOTAL_STEPS, current.title());
        info!(log, "Step: {:?}", current);

        match current {
            InstallStep::Complete => {
                info!(log, "Installation complete. Reboot to start FrutigerOS.");
                break;
            }
            InstallStep::Aborted => {
                info!(
                    log,
                    "Installation aborted. No changes were made to your disk."
                );
                break;
            }
            _ => {
                // In CLI mode, advance automatically with defaults for testing.
                // The real UI will call step-specific methods interactively.
                state.advance_with_defaults(&log).await?;
            }
        }
    }

    Ok(())
}

fn parse_mode(args: &[String]) -> Result<ExecutionMode> {
    let apply = args.iter().any(|a| a == "--apply");
    let confirmed = args.iter().any(|a| a == "--yes" || a == "-y");

    let unknown: Vec<&String> = args
        .iter()
        .filter(|a| !matches!(a.as_str(), "--apply" | "--yes" | "-y" | "--dry-run"))
        .collect();
    if !unknown.is_empty() {
        return Err(anyhow!(
            "unknown argument(s): {}. See --help.",
            unknown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }

    if apply && !confirmed {
        return Err(anyhow!(
            "--apply is destructive and requires --yes. \
             Run without flags for a safe dry-run."
        ));
    }

    Ok(if apply {
        ExecutionMode::Apply
    } else {
        ExecutionMode::DryRun
    })
}

fn print_help() {
    println!(
        "AeroInstaller {}\n\
         \n\
         Usage: aero-installer [OPTIONS]\n\
         \n\
         Options:\n\
           --dry-run    Print the installation plan without changing the disk (default)\n\
           --apply      Execute the plan (DESTRUCTIVE; requires --yes)\n\
           -y, --yes    Confirm destructive execution\n\
           -h, --help   Show this help\n\
           -V, --version Show version\n",
        env!("CARGO_PKG_VERSION")
    );
}
