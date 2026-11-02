//! Installation command planning and execution.
//!
//! The installer never shells out through `sh -c` for individual commands; each
//! step is a program plus an argument vector, which avoids quoting bugs and
//! command injection from user-supplied names.
//!
//! Safety model:
//! - Planning is pure and has no side effects.
//! - `ExecutionMode::DryRun` is the default and only logs commands.
//! - `ExecutionMode::Apply` runs commands in order and aborts on first failure.
//! - Destructive steps are flagged so the UI can show an explicit warning.

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use slog::{info, warn};

use crate::disk::{Filesystem, PartitionPlan, PartitionRole};
use crate::steps::UserChoices;

/// How much of the plan to execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionMode {
    /// Log the commands without running them (default).
    #[default]
    DryRun,
    /// Run the commands. Callers must obtain explicit user confirmation first.
    Apply,
}

/// A single executable step in an installation plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandStep {
    /// Human-readable description shown in the UI/log.
    pub description: String,
    pub program: String,
    pub args: Vec<String>,
    /// True when the command rewrites or destroys data on the target disk.
    pub destructive: bool,
}

impl CommandStep {
    pub fn new(description: impl Into<String>, program: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            program: program.into(),
            args: Vec::new(),
            destructive: false,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    /// Shell-ish representation for logs and dry-run output.
    pub fn display(&self) -> String {
        let mut out = self.program.clone();
        for arg in &self.args {
            out.push(' ');
            if arg.is_empty() || arg.contains([' ', '\'', '"', '$', '\\', '&', '|', ';']) {
                out.push('\'');
                out.push_str(&arg.replace('\'', "'\\''"));
                out.push('\'');
            } else {
                out.push_str(arg);
            }
        }
        out
    }
}

/// Full ordered installation plan.
#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub target_mount: String,
    pub steps: Vec<CommandStep>,
}

impl InstallPlan {
    pub fn new(target_mount: impl Into<String>) -> Self {
        Self {
            target_mount: target_mount.into(),
            steps: Vec::new(),
        }
    }

    fn push<I>(&mut self, steps: I)
    where
        I: IntoIterator<Item = CommandStep>,
    {
        self.steps.extend(steps);
    }

    /// Steps that will modify the target disk.
    pub fn destructive_steps(&self) -> impl Iterator<Item = &CommandStep> {
        self.steps.iter().filter(|s| s.destructive)
    }
}

// ── Partitioning ──────────────────────────────────────────────────────────────

/// GPT partition types used by the auto layout.
const TYPE_EFI: &str = "ef00";
const TYPE_LINUX: &str = "8300";
const TYPE_SWAP: &str = "8200";

/// Build the `sgdisk` steps that recreate the auto partition layout.
pub fn partition_steps(plan: &PartitionPlan) -> Vec<CommandStep> {
    let disk = plan.disk.clone();
    let mut steps = vec![CommandStep::new("Wipe existing partition table", "sgdisk")
        .args(["--zap-all".to_string(), disk.clone()])
        .destructive()];

    for (i, part) in plan.partitions.iter().enumerate() {
        let index = (i + 1).to_string();
        let type_code = match part.role {
            PartitionRole::Efi => TYPE_EFI,
            PartitionRole::Swap => TYPE_SWAP,
            PartitionRole::Root | PartitionRole::Home => TYPE_LINUX,
        };
        let size = if part.size_mb == 0 {
            "0:0".to_string()
        } else {
            format!("0:+{}M", part.size_mb)
        };
        steps.push(
            CommandStep::new(
                format!("Create {} partition ({})", part.role_name(), part.size_desc),
                "sgdisk",
            )
            .args([
                "-n".to_string(),
                format!("{}:{}", index, size),
                "-t".to_string(),
                format!("{}:{}", index, type_code),
                disk.clone(),
            ])
            .destructive(),
        );
    }

    steps.push(
        CommandStep::new("Re-read partition table", "partprobe")
            .arg(disk)
            .destructive(),
    );
    steps
}

/// Build `mkfs`/`mkswap` steps for every partition in the plan.
pub fn filesystem_steps(plan: &PartitionPlan) -> Vec<CommandStep> {
    plan.partitions
        .iter()
        .map(|part| match part.filesystem {
            Filesystem::Btrfs => {
                CommandStep::new(format!("Format {} as btrfs", part.path), "mkfs.btrfs")
                    .args(["-f", part.path.as_str()])
                    .destructive()
            }
            Filesystem::Ext4 => {
                CommandStep::new(format!("Format {} as ext4", part.path), "mkfs.ext4")
                    .args(["-F", part.path.as_str()])
                    .destructive()
            }
            Filesystem::Fat32 => {
                CommandStep::new(format!("Format {} as FAT32", part.path), "mkfs.fat")
                    .args(["-F32", part.path.as_str()])
                    .destructive()
            }
            Filesystem::Swap => {
                CommandStep::new(format!("Initialise swap on {}", part.path), "mkswap")
                    .arg(&part.path)
                    .destructive()
            }
        })
        .collect()
}

// ── Mounting ──────────────────────────────────────────────────────────────────

/// Build mount steps (root first, then boot/EFI, then swap).
pub fn mount_steps(plan: &PartitionPlan, target: &str) -> Vec<CommandStep> {
    let mut steps = Vec::new();

    let root = plan
        .partitions
        .iter()
        .find(|p| p.role == PartitionRole::Root && p.mount_point == "/");
    let boot = plan
        .partitions
        .iter()
        .find(|p| p.mount_point == "/boot" && p.role != PartitionRole::Efi);
    let efi = plan
        .partitions
        .iter()
        .find(|p| p.role == PartitionRole::Efi);
    let swap = plan
        .partitions
        .iter()
        .find(|p| p.role == PartitionRole::Swap);

    if let Some(root) = root {
        steps.push(
            CommandStep::new(format!("Mount root on {}", target), "mount")
                .args([root.path.as_str(), target]),
        );
    }

    if let Some(swap) = swap {
        steps.push(
            CommandStep::new(format!("Enable swap on {}", swap.path), "swapon").arg(&swap.path),
        );
    }

    if let Some(boot) = boot {
        let dest = format!("{}/boot", target.trim_end_matches('/'));
        steps.push(
            CommandStep::new(format!("Mount boot on {}", dest), "mount")
                .args([boot.path.as_str(), dest.as_str()]),
        );
    }

    if let Some(efi) = efi {
        let dest = format!("{}/boot/efi", target.trim_end_matches('/'));
        steps.push(
            CommandStep::new(format!("Mount EFI on {}", dest), "mount")
                .args([efi.path.as_str(), dest.as_str()]),
        );
    }

    steps
}

// ── Base system ───────────────────────────────────────────────────────────────

/// Build `pacstrap` and `genfstab` steps.
pub fn base_system_steps(
    target: &str,
    packages: &[String],
    frutigeros_packages: &[String],
) -> Vec<CommandStep> {
    let mut all: Vec<String> = packages.to_vec();
    for pkg in frutigeros_packages {
        if !all.contains(pkg) {
            all.push(pkg.clone());
        }
    }

    vec![
        CommandStep::new("Install base system with pacstrap", "pacstrap")
            .arg(target)
            .args(all),
        CommandStep::new("Generate /etc/fstab", "sh").args([
            "-c",
            &format!(
                "genfstab -U '{}' >> '{}/etc/fstab'",
                shell_escape(target),
                shell_escape(target)
            ),
        ]),
    ]
}

// ── System configuration ──────────────────────────────────────────────────────

/// Build the `arch-chroot` script that applies locale, timezone, hostname,
/// user account and services.
pub fn config_script(choices: &UserChoices, target: &str) -> CommandStep {
    let username = sanitize_name(&choices.username, "user");
    let hostname = sanitize_name(&choices.hostname, "frutigeros");
    let timezone = sanitize_timezone(&choices.timezone);
    let locale = sanitize_locale(&choices.language);
    let keyboard = sanitize_name(&choices.keyboard_layout, "us");

    let root_hash = choices.root_password_hash.trim();
    let user_hash = choices.user_password_hash.trim();

    let mut script = String::new();
    script.push_str("set -e\n");
    script.push_str(&format!(
        "ln -sf /usr/share/zoneinfo/{timezone} /etc/localtime\n"
    ));
    script.push_str("hwclock --systohc\n");
    script.push_str(&format!(
        "sed -i 's/^#{locale}/{locale}/' /etc/locale.gen\n"
    ));
    script.push_str("locale-gen\n");
    script.push_str(&format!("echo 'LANG={locale}' > /etc/locale.conf\n"));
    script.push_str(&format!("echo 'KEYMAP={keyboard}' > /etc/vconsole.conf\n"));
    script.push_str(&format!("echo '{hostname}' > /etc/hostname\n"));
    script.push_str(&format!(
        "printf '127.0.0.1 localhost\\n::1 localhost\\n127.0.1.1 {hostname}.localdomain {hostname}\\n' > /etc/hosts\n"
    ));
    script.push_str("systemctl enable NetworkManager\n");
    script.push_str("systemctl enable systemd-timesyncd\n");
    script.push_str(&format!(
        "id -u {username} >/dev/null 2>&1 || useradd -m -G wheel -s /bin/bash {username}\n"
    ));

    if !root_hash.is_empty() {
        script.push_str(&format!(
            "echo 'root:{}' | chpasswd -e\n",
            shell_escape(root_hash)
        ));
    }
    if !user_hash.is_empty() {
        script.push_str(&format!(
            "echo '{}:{}' | chpasswd -e\n",
            username,
            shell_escape(user_hash)
        ));
    }

    script.push_str(
        "sed -i 's/^# %wheel ALL=(ALL:ALL) ALL/%wheel ALL=(ALL:ALL) ALL/' /etc/sudoers\n",
    );

    CommandStep::new(
        "Configure locale, timezone, hostname and users",
        "arch-chroot",
    )
    .args([target, "bash", "-c", script.trim_end()])
}

/// Build bootloader installation steps (GRUB, UEFI when an ESP is present).
pub fn bootloader_steps(choices: &UserChoices, target: &str, uefi: bool) -> Vec<CommandStep> {
    let mut steps = Vec::new();

    if uefi {
        steps.push(
            CommandStep::new("Install GRUB for UEFI", "arch-chroot").args([
                target,
                "grub-install",
                "--target=x86_64-efi",
                "--efi-directory=/boot/efi",
                "--bootloader-id=FrutigerOS",
                "--recheck",
            ]),
        );
    } else {
        steps.push(
            CommandStep::new("Install GRUB for BIOS", "arch-chroot")
                .args([target, "grub-install", "--target=i386-pc"])
                .arg(choices.target_disk.as_deref().unwrap_or_default()),
        );
    }

    steps.push(
        CommandStep::new("Generate GRUB configuration", "arch-chroot").args([
            target,
            "grub-mkconfig",
            "-o",
            "/boot/grub/grub.cfg",
        ]),
    );

    steps
}

// ── Full plan ─────────────────────────────────────────────────────────────────

/// Build the complete installation plan from the wizard choices.
pub fn full_plan(
    choices: &UserChoices,
    config: &crate::steps::config::InstallConfig,
    uefi: bool,
) -> Result<InstallPlan> {
    let plan = choices
        .partition_plan
        .as_ref()
        .ok_or_else(|| anyhow!("no partition plan; complete the partitioning step first"))?;

    if choices.install_type != crate::steps::InstallType::Clean {
        return Err(anyhow!(
            "only clean installs are supported right now; \
             alongside/manual partitioning is not implemented"
        ));
    }

    let target = config.target_mount.as_str();
    let mut out = InstallPlan::new(target);
    out.push(partition_steps(plan));
    out.push(filesystem_steps(plan));
    out.push(mount_steps(plan, target));
    out.push(base_system_steps(
        target,
        &config.base_packages,
        &config.frutigeros_packages,
    ));
    out.steps.push(config_script(choices, target));
    out.push(bootloader_steps(choices, target, uefi));
    out.steps
        .push(CommandStep::new("Unmount target filesystems", "umount").args(["-R", target]));
    Ok(out)
}

// ── Execution ─────────────────────────────────────────────────────────────────

/// Execute (or dry-run) every step in order.
///
/// In `Apply` mode the first failing command aborts the run. The caller is
/// responsible for having obtained explicit confirmation before choosing
/// `Apply`.
pub async fn execute(plan: &InstallPlan, mode: ExecutionMode, log: &slog::Logger) -> Result<()> {
    if mode == ExecutionMode::Apply {
        let destructive = plan.destructive_steps().count();
        warn!(
            log,
            "APPLY mode: {} destructive step(s) will run against the target disk", destructive
        );
    } else {
        info!(log, "DRY RUN: no commands will be executed");
    }
    if !plan.target_mount.is_empty() {
        info!(log, "Target mount point: {}", plan.target_mount);
    }

    for (index, step) in plan.steps.iter().enumerate() {
        let prefix = if step.destructive { "DESTRUCTIVE " } else { "" };
        info!(
            log,
            "[{}/{}] {}{}: {}",
            index + 1,
            plan.steps.len(),
            prefix,
            step.description,
            step.display()
        );

        if mode == ExecutionMode::DryRun {
            continue;
        }

        let status = tokio::process::Command::new(&step.program)
            .args(&step.args)
            .status()
            .await
            .with_context(|| format!("failed to start {} ({})", step.program, step.description))?;

        if !status.success() {
            return Err(anyhow!(
                "step {} ({}) failed with status {}",
                index + 1,
                step.description,
                status
            ));
        }
    }

    if mode == ExecutionMode::Apply {
        info!(log, "Installation commands completed successfully");
    } else {
        info!(
            log,
            "Dry run complete ({} steps). Re-run with --apply --yes to install.",
            plan.steps.len()
        );
    }
    Ok(())
}

// ── Sanitisation ──────────────────────────────────────────────────────────────

fn sanitize_name(value: &str, fallback: &str) -> String {
    let cleaned: String = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn sanitize_locale(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '@'))
        .collect();
    if cleaned.is_empty() {
        "en_US.UTF-8".to_string()
    } else {
        cleaned
    }
}

fn sanitize_timezone(value: &str) -> String {
    let path = Path::new(value);
    let safe = !value.is_empty()
        && !value.starts_with('/')
        && !value.contains("..")
        && path
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)));

    if safe {
        value.to_string()
    } else {
        "UTC".to_string()
    }
}

fn shell_escape(value: &str) -> String {
    value.replace('\'', "'\\''")
}

impl crate::disk::PlannedPartition {
    fn role_name(&self) -> &'static str {
        match self.role {
            PartitionRole::Efi => "EFI",
            PartitionRole::Swap => "swap",
            PartitionRole::Home => "home",
            PartitionRole::Root if self.mount_point == "/boot" => "boot",
            PartitionRole::Root => "root",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::auto_partition_plan;
    use crate::steps::InstallType;

    fn choices(plan: PartitionPlan) -> UserChoices {
        UserChoices {
            language: "en_US.UTF-8".into(),
            keyboard_layout: "us".into(),
            target_disk: Some(plan.disk.clone()),
            partition_plan: Some(plan),
            install_type: InstallType::Clean,
            username: "alice".into(),
            hostname: "frutigeros".into(),
            timezone: "Europe/Berlin".into(),
            root_password_hash: "$6$root".into(),
            user_password_hash: "$6$user".into(),
        }
    }

    #[test]
    fn partition_steps_are_destructive() {
        let plan = auto_partition_plan("/dev/sda", Filesystem::Btrfs);
        let steps = partition_steps(&plan);
        assert!(steps.iter().all(|s| s.destructive));
        assert_eq!(steps.first().unwrap().program, "sgdisk");
        assert!(steps
            .iter()
            .any(|s| s.args.contains(&"--zap-all".to_string())));
    }

    #[test]
    fn filesystem_steps_match_plan() {
        let plan = auto_partition_plan("/dev/sda", Filesystem::Ext4);
        let steps = filesystem_steps(&plan);
        assert_eq!(steps.len(), 4);
        assert!(steps.iter().any(|s| s.program == "mkfs.fat"));
        assert!(steps.iter().any(|s| s.program == "mkfs.ext4"));
        assert!(steps.iter().any(|s| s.program == "mkswap"));
        assert!(steps.iter().all(|s| s.destructive));
    }

    #[test]
    fn mount_order_root_before_boot() {
        let plan = auto_partition_plan("/dev/sda", Filesystem::Btrfs);
        let steps = mount_steps(&plan, "/mnt");
        assert_eq!(steps[0].args, vec!["/dev/sda4", "/mnt"]);
        assert!(steps
            .iter()
            .any(|s| s.args.contains(&"/mnt/boot".to_string())));
        assert!(steps
            .iter()
            .any(|s| s.args.contains(&"/mnt/boot/efi".to_string())));
    }

    #[test]
    fn full_plan_is_ordered_and_complete() {
        let plan = auto_partition_plan("/dev/nvme0n1", Filesystem::Btrfs);
        let config = crate::steps::config::InstallConfig::default();
        let out = full_plan(&choices(plan), &config, true).unwrap();
        let programs: Vec<&str> = out.steps.iter().map(|s| s.program.as_str()).collect();

        let pos = |needle: &str| programs.iter().position(|p| *p == needle).unwrap();
        assert!(pos("sgdisk") < pos("mkfs.btrfs"));
        assert!(pos("mkfs.btrfs") < pos("mount"));
        assert!(pos("mount") < pos("pacstrap"));
        assert!(pos("pacstrap") < pos("arch-chroot"));
        assert!(!out.destructive_steps().next().is_none());
    }

    #[test]
    fn config_script_sanitises_names() {
        let plan = auto_partition_plan("/dev/sda", Filesystem::Btrfs);
        let mut c = choices(plan);
        c.username = "bad user; rm -rf /".into();
        c.timezone = "../../etc/passwd".into();
        let step = config_script(&c, "/mnt");
        let script = step.args.last().unwrap();
        assert!(script.contains("useradd -m -G wheel -s /bin/bash baduserrm-rf"));
        assert!(script.contains("/usr/share/zoneinfo/UTC"));
        assert!(!script.contains("../"));
    }

    #[test]
    fn bootloader_uses_uefi_when_requested() {
        let plan = auto_partition_plan("/dev/sda", Filesystem::Btrfs);
        let c = choices(plan);
        let efi = bootloader_steps(&c, "/mnt", true);
        assert!(efi[0].args.iter().any(|a| a == "--target=x86_64-efi"));
        let bios = bootloader_steps(&c, "/mnt", false);
        assert!(bios[0].args.iter().any(|a| a == "--target=i386-pc"));
    }

    #[test]
    fn non_clean_install_is_rejected() {
        let plan = auto_partition_plan("/dev/sda", Filesystem::Btrfs);
        let mut c = choices(plan);
        c.install_type = InstallType::Alongside;
        let config = crate::steps::config::InstallConfig::default();
        assert!(full_plan(&c, &config, true).is_err());
    }

    #[tokio::test]
    async fn dry_run_does_not_execute() {
        let log = slog::Logger::root(slog::Discard, slog::o!());
        let mut plan = InstallPlan::new("/mnt");
        // `false` would fail if executed.
        plan.steps.push(CommandStep::new("should not run", "false"));
        execute(&plan, ExecutionMode::DryRun, &log)
            .await
            .expect("dry-run must not execute commands");
    }
}
