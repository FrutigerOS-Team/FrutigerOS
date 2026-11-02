//! Screenshot capture configuration, command building and execution.

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{anyhow, Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureConfig {
    /// Directory for saved screenshots.
    pub save_dir: String,
    /// File format: "png", "jpg", "webp".
    pub format: String,
    /// Countdown seconds before capture (0 = immediate).
    pub countdown: u8,
    /// Copy to clipboard after capture.
    pub copy_to_clipboard: bool,
    /// Show notification after capture.
    pub show_notification: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        Self {
            save_dir: format!("{}/Pictures/Screenshots", home),
            format: "png".into(),
            countdown: 0,
            copy_to_clipboard: true,
            show_notification: true,
        }
    }
}

impl CaptureConfig {
    /// Generate the next screenshot filename with a timestamp.
    pub fn next_filename(&self) -> String {
        let ts = Local::now().format("%Y-%m-%d_%H-%M-%S");
        format!("{}/Screenshot_{}.{}", self.save_dir, ts, self.format)
    }

    pub fn save_path(&self) -> PathBuf {
        PathBuf::from(&self.save_dir)
    }
}

/// Screenshot capture mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CaptureMode {
    /// Capture the entire display.
    Fullscreen,
    /// Capture the focused window only.
    Window,
    /// User selects a region with the mouse.
    Region,
    /// Capture after a countdown.
    Delayed { seconds: u8 },
}

impl CaptureMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "window" => Self::Window,
            "region" => Self::Region,
            "delayed" => Self::Delayed { seconds: 3 },
            _ => Self::Fullscreen,
        }
    }

    /// Modes that need a geometry selected with `slurp`.
    pub fn needs_region(self) -> bool {
        matches!(self, Self::Region | Self::Window)
    }
}

/// Build the `grim` invocation without executing it.
///
/// `geometry` is the output of `slurp` for region/window captures. It is passed
/// as a single argument, never through a shell.
pub fn build_grim_command(
    cfg: &CaptureConfig,
    mode: CaptureMode,
    output: &Path,
    geometry: Option<&str>,
) -> (String, Vec<String>) {
    let mut args = vec!["-t".to_string(), cfg.format.clone()];
    if mode.needs_region() {
        if let Some(geometry) = geometry {
            args.push("-g".to_string());
            args.push(geometry.to_string());
        }
    }
    args.push(output.to_string_lossy().to_string());
    ("grim".to_string(), args)
}

fn mime_for(format: &str) -> &'static str {
    match format {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    }
}

/// Run `slurp` and return the selected geometry, or an error if cancelled.
pub async fn select_region() -> Result<String> {
    let output = tokio::process::Command::new("slurp")
        .output()
        .await
        .context("slurp is required for region captures (install slurp)")?;
    if !output.status.success() {
        return Err(anyhow!("region selection cancelled"));
    }
    let geometry = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if geometry.is_empty() {
        return Err(anyhow!("region selection was empty"));
    }
    Ok(geometry)
}

/// Capture a screenshot, save it, and optionally copy it to the clipboard.
pub async fn capture(
    cfg: &CaptureConfig,
    mode: CaptureMode,
    log: &slog::Logger,
) -> Result<PathBuf> {
    use slog::{info, warn};

    let save_dir = cfg.save_path();
    tokio::fs::create_dir_all(&save_dir)
        .await
        .with_context(|| format!("create screenshot directory {}", save_dir.display()))?;

    if let CaptureMode::Delayed { seconds } = mode {
        info!(log, "Capturing in {} second(s)…", seconds);
        tokio::time::sleep(std::time::Duration::from_secs(seconds as u64)).await;
    } else if cfg.countdown > 0 {
        info!(log, "Capturing in {} second(s)…", cfg.countdown);
        tokio::time::sleep(std::time::Duration::from_secs(cfg.countdown as u64)).await;
    }

    let geometry = if mode.needs_region() {
        Some(select_region().await?)
    } else {
        None
    };

    let output = PathBuf::from(cfg.next_filename());
    let (program, args) = build_grim_command(cfg, mode, &output, geometry.as_deref());
    info!(log, "Running {} {}", program, args.join(" "));

    let status = tokio::process::Command::new(&program)
        .args(&args)
        .status()
        .await
        .context("grim is required for Wayland screenshots (install grim)")?;
    if !status.success() {
        return Err(anyhow!("grim failed with status {}", status));
    }

    if cfg.copy_to_clipboard {
        match copy_to_clipboard(&output, &cfg.format).await {
            Ok(()) => info!(log, "Copied screenshot to clipboard"),
            Err(err) => warn!(log, "Could not copy to clipboard: {}", err),
        }
    }

    if cfg.show_notification {
        let _ = tokio::process::Command::new("notify-send")
            .args(["Screenshot saved", &output.to_string_lossy()])
            .status()
            .await;
    }

    info!(log, "Saved {}", output.display());
    Ok(output)
}

async fn copy_to_clipboard(path: &Path, format: &str) -> Result<()> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("open {} for clipboard", path.display()))?;
    let status = tokio::process::Command::new("wl-copy")
        .args(["--type", mime_for(format)])
        .stdin(Stdio::from(file))
        .status()
        .await
        .context("wl-copy not available")?;
    if !status.success() {
        return Err(anyhow!("wl-copy exited with {}", status));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_contains_png() {
        let cfg = CaptureConfig::default();
        let name = cfg.next_filename();
        assert!(name.ends_with(".png"), "got: {}", name);
        assert!(name.contains("Screenshot_"));
    }

    #[test]
    fn capture_mode_parse() {
        assert_eq!(CaptureMode::from_str("window"), CaptureMode::Window);
        assert_eq!(CaptureMode::from_str("region"), CaptureMode::Region);
        assert_eq!(CaptureMode::from_str("fullscreen"), CaptureMode::Fullscreen);
        assert_eq!(CaptureMode::from_str("unknown"), CaptureMode::Fullscreen);
    }

    #[test]
    fn default_format_is_png() {
        assert_eq!(CaptureConfig::default().format, "png");
    }

    #[test]
    fn grim_command_fullscreen() {
        let cfg = CaptureConfig::default();
        let out = Path::new("/tmp/shot.png");
        let (program, args) = build_grim_command(&cfg, CaptureMode::Fullscreen, out, None);
        assert_eq!(program, "grim");
        assert_eq!(args, vec!["-t", "png", "/tmp/shot.png"]);
    }

    #[test]
    fn grim_command_region_passes_geometry_as_one_argument() {
        let cfg = CaptureConfig::default();
        let out = Path::new("/tmp/shot.png");
        let (_, args) = build_grim_command(&cfg, CaptureMode::Region, out, Some("10,20 300x200"));
        assert!(args.contains(&"-g".to_string()));
        assert!(args.contains(&"10,20 300x200".to_string()));
    }

    #[test]
    fn mime_types_match_format() {
        assert_eq!(mime_for("png"), "image/png");
        assert_eq!(mime_for("jpg"), "image/jpeg");
        assert_eq!(mime_for("webp"), "image/webp");
    }
}
