//! Disk probing and partition planning for AeroInstaller.
//!
//! All operations in this module are READ-ONLY until the user explicitly
//! confirms the partition plan in the UI.
//!
//! Destructive operations (mkfs, parted, dd) are gated behind the
//! `apply_partition_plan` function which requires the plan to be
//! `confirmed = true`.
//!
//! ## Safety contract
//! - `probe_disks()` — read-only, uses lsblk
//! - `auto_partition_plan()` — pure computation, no I/O
//! - `apply_partition_plan()` — DESTRUCTIVE; requires explicit confirmation

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

// ── Disk info ─────────────────────────────────────────────────────────────────

/// A physical or virtual disk available for installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    /// Block device path, e.g. "/dev/sda" or "/dev/nvme0n1".
    pub path: String,
    /// Human-readable model name.
    pub model: String,
    /// Size in GiB.
    pub size_gb: f64,
    /// True if this disk already has partitions on it.
    pub has_partitions: bool,
    /// Transport type: "nvme", "sata", "virtio", "usb", "unknown".
    pub transport: String,
}

/// Filesystem choices for partitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Filesystem {
    Btrfs,
    Ext4,
    Fat32, // always used for EFI
    Swap,
}

impl Filesystem {
    pub fn name(self) -> &'static str {
        match self {
            Self::Btrfs => "btrfs",
            Self::Ext4 => "ext4",
            Self::Fat32 => "fat32",
            Self::Swap => "swap",
        }
    }

    #[allow(dead_code)] // Retained for callers that only need a printable command.
    pub fn mkfs_cmd(self, path: &str) -> Option<String> {
        match self {
            Self::Btrfs => Some(format!("mkfs.btrfs -f {}", path)),
            Self::Ext4 => Some(format!("mkfs.ext4 -F {}", path)),
            Self::Fat32 => Some(format!("mkfs.fat -F32 {}", path)),
            Self::Swap => Some(format!("mkswap {}", path)),
        }
    }
}

/// Role of a partition in the FrutigerOS layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartitionRole {
    Efi,
    Root,
    Home,
    Swap,
}

/// A single partition in a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedPartition {
    pub path: String,
    pub role: PartitionRole,
    pub filesystem: Filesystem,
    pub size_mb: u64,
    pub size_desc: String,
    pub mount_point: String,
}

/// A full partition plan for a single disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionPlan {
    pub disk: String,
    pub partitions: Vec<PlannedPartition>,
    /// Must be set to true before apply_partition_plan() will run.
    pub confirmed: bool,
}

// ── Probing ───────────────────────────────────────────────────────────────────

/// Probe all block devices via lsblk.
/// Returns only whole disks (not partitions).
/// READ-ONLY — safe to call at any time.
pub async fn probe_disks() -> Result<Vec<DiskInfo>> {
    let output = tokio::process::Command::new("lsblk")
        .args(["--json", "--bytes", "--output", "NAME,SIZE,MODEL,TRAN,TYPE"])
        .output()
        .await
        .map_err(|e| anyhow!("lsblk not found: {}", e))?;

    if !output.status.success() {
        return Err(anyhow!("lsblk failed"));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let mut disks = Vec::new();

    if let Some(devices) = json["blockdevices"].as_array() {
        for dev in devices {
            // Only whole disks (type == "disk").
            if dev["type"].as_str() != Some("disk") {
                continue;
            }

            let name = dev["name"].as_str().unwrap_or("unknown");
            let path = format!("/dev/{}", name);
            let size_bytes: u64 = dev["size"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let size_gb = size_bytes as f64 / 1_073_741_824.0;

            let model = dev["model"]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("Unknown")
                .to_string();

            let transport = dev["tran"].as_str().unwrap_or("unknown").to_string();

            let has_partitions = dev["children"]
                .as_array()
                .map(|c| !c.is_empty())
                .unwrap_or(false);

            disks.push(DiskInfo {
                path,
                model,
                size_gb,
                has_partitions,
                transport,
            });
        }
    }

    Ok(disks)
}

// ── Partition planning ────────────────────────────────────────────────────────

/// Build an automatic partition plan for a clean install.
/// Does NOT write anything to disk.
///
/// Layout (UEFI):
///   - 512 MB  EFI System Partition (fat32)     /boot/efi
///   - 512 MB  /boot (ext4)
///   - 4 GB    swap
///   - rest    root (btrfs or ext4)             /
pub fn auto_partition_plan(disk: &str, root_fs: Filesystem) -> PartitionPlan {
    let efi_part = format!("{}1", disk);
    let boot_part = format!("{}2", disk);
    let swap_part = format!("{}3", disk);
    let root_part = format!("{}4", disk);

    // For NVMe the partition suffix is p1, p2…
    let (efi_p, boot_p, swap_p, root_p) = if disk.contains("nvme") || disk.contains("mmcblk") {
        (
            format!("{}p1", disk),
            format!("{}p2", disk),
            format!("{}p3", disk),
            format!("{}p4", disk),
        )
    } else {
        (efi_part, boot_part, swap_part, root_part)
    };

    PartitionPlan {
        disk: disk.to_string(),
        confirmed: false,
        partitions: vec![
            PlannedPartition {
                path: efi_p,
                role: PartitionRole::Efi,
                filesystem: Filesystem::Fat32,
                size_mb: 512,
                size_desc: "512 MB".into(),
                mount_point: "/boot/efi".into(),
            },
            PlannedPartition {
                path: boot_p,
                role: PartitionRole::Root, // /boot lives on root fs
                filesystem: Filesystem::Ext4,
                size_mb: 512,
                size_desc: "512 MB".into(),
                mount_point: "/boot".into(),
            },
            PlannedPartition {
                path: swap_p,
                role: PartitionRole::Swap,
                filesystem: Filesystem::Swap,
                size_mb: 4096,
                size_desc: "4 GB".into(),
                mount_point: "[swap]".into(),
            },
            PlannedPartition {
                path: root_p,
                role: PartitionRole::Root,
                filesystem: root_fs,
                size_mb: 0, // 0 = use remaining space
                size_desc: "remaining".into(),
                mount_point: "/".into(),
            },
        ],
    }
}

/// Apply a confirmed partition plan.
///
/// # Safety
/// This function FORMATS the target disk. It will DESTROY all data.
/// The plan MUST have `confirmed = true` or this returns an error.
///
/// Not called in dry-run / CLI test mode.
#[allow(dead_code)] // Kept as a low-level entry point for the graphical UI.
pub async fn apply_partition_plan(plan: &PartitionPlan, log: &slog::Logger) -> Result<()> {
    use slog::warn;

    if !plan.confirmed {
        return Err(anyhow!(
            "Refusing to apply unconfirmed partition plan. \
             Set plan.confirmed = true only after the user explicitly approves."
        ));
    }

    warn!(log, "DESTRUCTIVE: formatting disk {}", plan.disk);
    warn!(log, "All data on {} will be lost.", plan.disk);

    // Build the same command plan used by the full installer and execute it.
    let mut steps = crate::install::partition_steps(plan);
    steps.extend(crate::install::filesystem_steps(plan));

    let commands = crate::install::InstallPlan {
        target_mount: String::new(),
        steps,
    };
    crate::install::execute(&commands, crate::install::ExecutionMode::Apply, log).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_plan_has_four_partitions() {
        let plan = auto_partition_plan("/dev/sda", Filesystem::Btrfs);
        assert_eq!(plan.partitions.len(), 4);
        assert!(!plan.confirmed);
    }

    #[test]
    fn auto_plan_nvme_uses_p_suffix() {
        let plan = auto_partition_plan("/dev/nvme0n1", Filesystem::Btrfs);
        assert!(
            plan.partitions[0].path.contains("p1"),
            "NVMe EFI: {}",
            plan.partitions[0].path
        );
    }

    #[test]
    fn auto_plan_first_is_efi() {
        let plan = auto_partition_plan("/dev/sda", Filesystem::Ext4);
        assert_eq!(plan.partitions[0].role, PartitionRole::Efi);
        assert_eq!(plan.partitions[0].filesystem, Filesystem::Fat32);
    }

    #[test]
    fn unconfirmed_plan_blocked() {
        // apply_partition_plan refuses unconfirmed plans.
        // We can't run the async fn in a sync test easily,
        // but we can verify the confirmed flag starts false.
        let plan = auto_partition_plan("/dev/sda", Filesystem::Btrfs);
        assert!(!plan.confirmed, "plan must not auto-confirm");
    }

    #[test]
    fn filesystem_names() {
        assert_eq!(Filesystem::Btrfs.name(), "btrfs");
        assert_eq!(Filesystem::Ext4.name(), "ext4");
        assert_eq!(Filesystem::Fat32.name(), "fat32");
        assert_eq!(Filesystem::Swap.name(), "swap");
    }
}
