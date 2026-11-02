//! AeroShell configuration.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellConfig {
    pub panel: PanelConfig,
    pub launcher: LauncherConfig,
    pub notifications: NotificationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelConfig {
    /// Panel height in pixels.
    pub height: u32,
    /// Panel position: "top" or "bottom".
    pub position: String,
    /// Show workspace switcher buttons.
    pub show_workspaces: bool,
    /// Show clock.
    pub show_clock: bool,
    /// Show system tray.
    pub show_tray: bool,
    /// Key to open the launcher (shown in panel).
    pub launcher_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherConfig {
    /// Max number of recent apps to show.
    pub recent_count: usize,
    /// Max number of favourite apps pinned.
    pub favorites_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationConfig {
    /// Duration to show notifications in milliseconds.
    pub duration_ms: u64,
    /// Corner: "top-right", "top-left", "bottom-right", "bottom-left".
    pub corner: String,
    /// Max simultaneous notifications.
    pub max_visible: usize,
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            height: 48,
            position: "bottom".to_string(),
            show_workspaces: true,
            show_clock: true,
            show_tray: true,
            launcher_key: "Super+Space".to_string(),
        }
    }
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            recent_count: 6,
            favorites_count: 8,
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            duration_ms: 5000,
            corner: "top-right".to_string(),
            max_visible: 5,
        }
    }
}

impl ShellConfig {
    pub fn load() -> Result<Self> {
        let path = config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)?;
        Ok(toml::from_str(&text)?)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

fn config_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("aeroshell")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let c = ShellConfig::default();
        assert_eq!(c.panel.height, 48);
        assert_eq!(c.panel.position, "bottom");
        assert_eq!(c.notifications.max_visible, 5);
    }

    #[test]
    fn config_roundtrip() {
        let c = ShellConfig::default();
        let s = toml::to_string_pretty(&c).unwrap();
        let p: ShellConfig = toml::from_str(&s).unwrap();
        assert_eq!(p.panel.height, c.panel.height);
    }
}
