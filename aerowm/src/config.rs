//! AeroWM configuration.
//!
//! Loaded from `~/.config/aerowm/config.toml`.
//! Falls back to compiled-in defaults if the file is missing.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub appearance: AppearanceConfig,
    pub workspaces: WorkspaceConfig,
    pub keybindings: KeybindingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Backend: "winit" (nested/testing) or "udev" (real hardware, Phase 3+).
    pub backend: String,
    /// Enable XWayland for legacy X11 application support.
    pub xwayland: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    /// Window corner radius in pixels.
    pub corner_radius: u32,
    /// Background blur strength for the glass effect.
    pub blur_strength: u32,
    /// Glass surface opacity (0.0 – 1.0).
    pub glass_opacity: f32,
    /// Active window border color (RGBA hex, e.g. "#00AEEFCC").
    pub border_active: String,
    /// Inactive window border color.
    pub border_inactive: String,
    /// Animation speed multiplier (1.0 = normal, 0.0 = off).
    pub animation_speed: f32,
    /// Window shadow blur radius.
    pub shadow_blur: u32,
    /// Window shadow opacity (0.0 – 1.0).
    pub shadow_opacity: f32,
    /// Height reserved for the AeroShell panel at the bottom edge.
    pub panel_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceConfig {
    /// Number of workspaces to create at startup.
    pub count: usize,
    /// Default layout: "floating", "tiling", or "hybrid".
    pub default_layout: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingConfig {
    /// Modifier key: "super", "alt", or "ctrl".
    pub modifier: String,
}

// ── Defaults ──────────────────────────────────────────────────────────────────

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            backend: "winit".to_string(), // safe default: works without root/DRM
            xwayland: false,
        }
    }
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            corner_radius: 12,
            blur_strength: 20,
            glass_opacity: 0.82,
            border_active: "#00AEEFcc".to_string(),
            border_inactive: "#7DDCFF44".to_string(),
            animation_speed: 1.0,
            shadow_blur: 24,
            shadow_opacity: 0.4,
            panel_height: 48,
        }
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            count: 4,
            default_layout: "floating".to_string(),
        }
    }
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            modifier: "super".to_string(),
        }
    }
}

// ── Loading ───────────────────────────────────────────────────────────────────

impl Config {
    /// Load config from disk, falling back to defaults on any error.
    pub fn load() -> Result<Self> {
        let path = config_path();
        if !path.exists() {
            return Ok(Config::default());
        }
        let text = std::fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&text)?;
        Ok(config)
    }

    /// Write the current config to disk (creates directories as needed).
    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
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
        .join("aerowm")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let c = Config::default();
        assert_eq!(c.workspaces.count, 4);
        assert_eq!(c.general.backend, "winit");
        assert!(c.appearance.glass_opacity > 0.0 && c.appearance.glass_opacity <= 1.0);
    }

    #[test]
    fn config_roundtrip() {
        let original = Config::default();
        let serialized = toml::to_string_pretty(&original).unwrap();
        let parsed: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(parsed.workspaces.count, original.workspaces.count);
        assert_eq!(parsed.general.backend, original.general.backend);
    }
}
