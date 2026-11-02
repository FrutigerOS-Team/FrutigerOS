//! AeroTerminal configuration.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TermConfig {
    pub shell: String,
    pub font_family: String,
    pub font_size: f32,
    pub glass_opacity: f32,
    pub blur_strength: u32,
    pub scrollback_lines: usize,
    pub cursor_blink: bool,
    pub bell_enabled: bool,
}

impl Default for TermConfig {
    fn default() -> Self {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
        Self {
            shell,
            font_family: "JetBrains Mono".into(),
            font_size: 14.0,
            glass_opacity: 0.88,
            blur_strength: 20,
            scrollback_lines: 10_000,
            cursor_blink: true,
            bell_enabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let c = TermConfig::default();
        assert!(!c.shell.is_empty());
        assert!(c.glass_opacity > 0.0 && c.glass_opacity <= 1.0);
        assert!(c.scrollback_lines > 0);
    }
}
