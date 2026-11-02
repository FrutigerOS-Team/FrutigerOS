//! Installer configuration (non-interactive defaults).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InstallConfig {
    /// Mount point for the live install target.
    pub target_mount: String,
    /// Minimum disk size in GB.
    pub min_disk_gb: u64,
    /// Default packages (base install).
    pub base_packages: Vec<String>,
    /// FrutigerOS-specific packages.
    pub frutigeros_packages: Vec<String>,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            target_mount: "/mnt".into(),
            min_disk_gb: 10,
            base_packages: vec![
                "base".into(),
                "base-devel".into(),
                "linux".into(),
                "linux-firmware".into(),
                "networkmanager".into(),
                "pipewire".into(),
                "pipewire-pulse".into(),
                "wireplumber".into(),
                "grub".into(),
                "efibootmgr".into(),
                "btrfs-progs".into(),
                "xdg-utils".into(),
                "xdg-user-dirs".into(),
            ],
            frutigeros_packages: vec!["aerowm".into(), "aeroshell".into(), "aero-installer".into()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_base() {
        let c = InstallConfig::default();
        assert!(c.base_packages.contains(&"base".to_string()));
        assert!(c.base_packages.contains(&"linux".to_string()));
        assert!(c.min_disk_gb >= 10);
    }
}
