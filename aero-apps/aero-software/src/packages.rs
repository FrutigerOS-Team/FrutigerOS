//! Package database — reads pacman's local database.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub size_kb: u64,
    pub installed: bool,
}

pub struct PackageDb {
    packages: Vec<Package>,
    updates: usize,
}

impl PackageDb {
    /// Load installed packages via `pacman -Q`.
    /// Falls back gracefully if pacman is not available (non-Arch systems).
    pub async fn load() -> Result<Self> {
        let output = tokio::process::Command::new("pacman")
            .args(["-Q"])
            .output()
            .await;

        let packages = match output {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| {
                    let mut parts = line.splitn(2, ' ');
                    let name = parts.next()?.to_string();
                    let version = parts.next()?.trim().to_string();
                    Some(Package {
                        name,
                        version,
                        description: String::new(),
                        size_kb: 0,
                        installed: true,
                    })
                })
                .collect(),
            _ => {
                // Not on Arch or pacman not available — return empty list.
                vec![]
            }
        };

        // `pacman -Qu` lists installed packages with a newer version available.
        // It exits non-zero when there are no updates, so only trust stdout.
        let updates = match tokio::process::Command::new("pacman")
            .args(["-Qu"])
            .output()
            .await
        {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count(),
            _ => 0,
        };

        Ok(Self { packages, updates })
    }

    pub fn installed_count(&self) -> usize {
        self.packages.len()
    }

    pub fn updates_available(&self) -> usize {
        self.updates
    }

    /// Search by name (case-insensitive).
    pub fn search(&self, query: &str) -> Vec<&Package> {
        let q = query.to_lowercase();
        self.packages
            .iter()
            .filter(|p| p.name.to_lowercase().contains(&q))
            .collect()
    }

    /// Install a package via `pkexec pacman -S --noconfirm`.
    /// Only runs if the user has confirmed in the UI.
    pub async fn install(&self, name: &str) -> Result<()> {
        let status = tokio::process::Command::new("pkexec")
            .args(["pacman", "-S", "--noconfirm", name])
            .status()
            .await
            .map_err(|e| anyhow!("pkexec failed: {}", e))?;
        if !status.success() {
            return Err(anyhow!("pacman install of {} failed", name));
        }
        Ok(())
    }

    /// Remove a package via `pkexec pacman -R --noconfirm`.
    pub async fn remove(&self, name: &str) -> Result<()> {
        let status = tokio::process::Command::new("pkexec")
            .args(["pacman", "-R", "--noconfirm", name])
            .status()
            .await
            .map_err(|e| anyhow!("pkexec failed: {}", e))?;
        if !status.success() {
            return Err(anyhow!("pacman remove of {} failed", name));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_case_insensitive() {
        let db = PackageDb {
            packages: vec![
                Package {
                    name: "Firefox".into(),
                    version: "1.0".into(),
                    description: String::new(),
                    size_kb: 0,
                    installed: true,
                },
                Package {
                    name: "gimp".into(),
                    version: "2.0".into(),
                    description: String::new(),
                    size_kb: 0,
                    installed: true,
                },
            ],
            updates: 0,
        };
        assert_eq!(db.search("fire").len(), 1);
        assert_eq!(db.search("GIMP").len(), 1);
        assert_eq!(db.search("zzz").len(), 0);
    }

    #[tokio::test]
    async fn load_doesnt_panic_without_pacman() {
        // Succeeds on non-Arch (returns empty list).
        let db = PackageDb::load().await.unwrap();
        // count may be 0 or N — just must not panic
        let _ = db.installed_count();
    }
}
