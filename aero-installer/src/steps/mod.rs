//! Installation state machine.
//!
//! Each `InstallStep` variant represents one screen/phase of the installer.
//! `InstallerState` holds all user choices and drives transitions.

pub mod config;
pub use config::InstallConfig;

use anyhow::Result;
use slog::{info, warn};

use crate::install::{self, ExecutionMode};

/// Every step in the installation wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstallStep {
    Welcome,
    Language,
    Keyboard,
    Network,
    DiskSelect,
    Partitioning,
    InstallType,
    UserCreation,
    Timezone,
    Installing,
    Complete,
    Aborted,
}

impl InstallStep {
    /// Returns the next step in the default forward order.
    pub fn next(self) -> Self {
        match self {
            Self::Welcome => Self::Language,
            Self::Language => Self::Keyboard,
            Self::Keyboard => Self::Network,
            Self::Network => Self::DiskSelect,
            Self::DiskSelect => Self::Partitioning,
            Self::Partitioning => Self::InstallType,
            Self::InstallType => Self::UserCreation,
            Self::UserCreation => Self::Timezone,
            Self::Timezone => Self::Installing,
            Self::Installing => Self::Complete,
            Self::Complete => Self::Complete,
            Self::Aborted => Self::Aborted,
        }
    }

    /// Human-readable title shown in the progress bar.
    pub fn title(self) -> &'static str {
        match self {
            Self::Welcome => "Welcome",
            Self::Language => "Language",
            Self::Keyboard => "Keyboard Layout",
            Self::Network => "Network",
            Self::DiskSelect => "Select Disk",
            Self::Partitioning => "Partitioning",
            Self::InstallType => "Installation Type",
            Self::UserCreation => "Create User",
            Self::Timezone => "Timezone",
            Self::Installing => "Installing…",
            Self::Complete => "Complete",
            Self::Aborted => "Aborted",
        }
    }

    /// Step index (0-based) for progress display.
    pub fn index(self) -> usize {
        match self {
            Self::Welcome => 0,
            Self::Language => 1,
            Self::Keyboard => 2,
            Self::Network => 3,
            Self::DiskSelect => 4,
            Self::Partitioning => 5,
            Self::InstallType => 6,
            Self::UserCreation => 7,
            Self::Timezone => 8,
            Self::Installing => 9,
            Self::Complete => 10,
            Self::Aborted => 10,
        }
    }

    pub const TOTAL_STEPS: usize = 10;
}

/// All user choices collected across wizard steps.
#[derive(Debug, Default, Clone)]
pub struct UserChoices {
    pub language: String,
    pub keyboard_layout: String,
    pub target_disk: Option<String>,
    pub partition_plan: Option<crate::disk::PartitionPlan>,
    pub install_type: InstallType,
    pub username: String,
    pub hostname: String,
    pub timezone: String,
    pub root_password_hash: String,
    pub user_password_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[allow(dead_code)] // Alongside/Manual are accepted by the UI in a later phase.
pub enum InstallType {
    #[default]
    Clean, // Wipe and install (automatic partitioning)
    Alongside, // Shrink existing and dual boot
    Manual,    // User manages partitions manually
}
/// Live state of the installer.
pub struct InstallerState {
    step: InstallStep,
    pub choices: UserChoices,
    pub config: InstallConfig,
    mode: ExecutionMode,
}

impl InstallerState {
    pub fn with_mode(mode: ExecutionMode) -> Self {
        Self {
            step: InstallStep::Welcome,
            choices: UserChoices::default(),
            config: InstallConfig::default(),
            mode,
        }
    }

    pub fn current_step(&self) -> InstallStep {
        self.step
    }

    /// Advance to the next step using default/safe values.
    /// Used in CLI test mode; the GUI calls step-specific methods instead.
    pub async fn advance_with_defaults(&mut self, log: &slog::Logger) -> Result<()> {
        match self.step {
            InstallStep::Welcome => {
                info!(log, "Welcome to FrutigerOS installer");
                self.step = self.step.next();
            }
            InstallStep::Language => {
                self.choices.language = "en_US.UTF-8".into();
                info!(log, "Language: {}", self.choices.language);
                self.step = self.step.next();
            }
            InstallStep::Keyboard => {
                self.choices.keyboard_layout = "us".into();
                info!(log, "Keyboard: {}", self.choices.keyboard_layout);
                self.step = self.step.next();
            }
            InstallStep::Network => {
                info!(log, "Network: skipped in CLI mode");
                self.step = self.step.next();
            }
            InstallStep::DiskSelect => {
                let disks = crate::disk::probe_disks().await?;
                if disks.is_empty() {
                    warn!(log, "No disks found — aborting");
                    self.step = InstallStep::Aborted;
                    return Ok(());
                }
                info!(log, "Found {} disk(s):", disks.len());
                for d in &disks {
                    info!(log, "  {} — {} ({} GB)", d.path, d.model, d.size_gb);
                }
                // In CLI mode: select the first disk. GUI will ask the user.
                self.choices.target_disk = Some(disks[0].path.clone());
                info!(log, "Selected: {}", disks[0].path);
                self.step = self.step.next();
            }
            InstallStep::Partitioning => {
                let disk = self.choices.target_disk.clone().unwrap_or_default();
                let plan = crate::disk::auto_partition_plan(&disk, crate::disk::Filesystem::Btrfs);
                info!(
                    log,
                    "Partition plan ({} partitions):",
                    plan.partitions.len()
                );
                for p in &plan.partitions {
                    info!(
                        log,
                        "  {} {} {:?} {}",
                        p.path,
                        p.size_desc,
                        p.role,
                        p.filesystem.name()
                    );
                }
                self.choices.partition_plan = Some(plan);
                self.step = self.step.next();
            }
            InstallStep::InstallType => {
                self.choices.install_type = InstallType::Clean;
                info!(log, "Install type: Clean");
                self.step = self.step.next();
            }
            InstallStep::UserCreation => {
                self.choices.username = "user".into();
                self.choices.hostname = "frutigeros".into();
                info!(
                    log,
                    "User: {} @ {}", self.choices.username, self.choices.hostname
                );
                self.step = self.step.next();
            }
            InstallStep::Timezone => {
                self.choices.timezone = "UTC".into();
                info!(log, "Timezone: {}", self.choices.timezone);
                self.step = self.step.next();
            }
            InstallStep::Installing => {
                let uefi = std::path::Path::new("/sys/firmware/efi").exists();
                let plan = install::full_plan(&self.choices, &self.config, uefi)?;
                info!(
                    log,
                    "Installation plan: {} steps ({} destructive), mode={:?}",
                    plan.steps.len(),
                    plan.destructive_steps().count(),
                    self.mode
                );
                install::execute(&plan, self.mode, log).await?;
                self.step = self.step.next();
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_ordering() {
        let mut s = InstallStep::Welcome;
        let order = vec![
            InstallStep::Language,
            InstallStep::Keyboard,
            InstallStep::Network,
            InstallStep::DiskSelect,
            InstallStep::Partitioning,
            InstallStep::InstallType,
            InstallStep::UserCreation,
            InstallStep::Timezone,
            InstallStep::Installing,
            InstallStep::Complete,
        ];
        for expected in order {
            s = s.next();
            assert_eq!(s, expected);
        }
    }

    #[test]
    fn complete_is_terminal() {
        assert_eq!(InstallStep::Complete.next(), InstallStep::Complete);
        assert_eq!(InstallStep::Aborted.next(), InstallStep::Aborted);
    }

    #[test]
    fn step_titles_unique() {
        use std::collections::HashSet;
        let titles: HashSet<_> = [
            InstallStep::Welcome,
            InstallStep::Language,
            InstallStep::Keyboard,
            InstallStep::Network,
            InstallStep::DiskSelect,
            InstallStep::Partitioning,
            InstallStep::InstallType,
            InstallStep::UserCreation,
            InstallStep::Timezone,
            InstallStep::Installing,
            InstallStep::Complete,
        ]
        .iter()
        .map(|s| s.title())
        .collect();
        assert_eq!(titles.len(), 11);
    }
}
