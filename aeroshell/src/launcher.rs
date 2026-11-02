//! AeroStart — application launcher for FrutigerOS.
//!
//! Reads installed applications from XDG .desktop files,
//! provides search, and launches them.
//!
//! Phase 3: Application discovery and search logic.
//! Phase 4: Full UI (rendered Wayland popup surface).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A discovered desktop application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct App {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    pub comment: Option<String>,
    /// Source .desktop file path.
    pub desktop_file: PathBuf,
}

/// Scan XDG application directories for .desktop files.
///
/// Searches: ~/.local/share/applications, /usr/share/applications,
///           /usr/local/share/applications
pub fn discover_apps() -> Vec<App> {
    let dirs = xdg_app_dirs();
    let mut apps = Vec::new();

    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("desktop") {
                    if let Some(app) = parse_desktop_file(&path) {
                        apps.push(app);
                    }
                }
            }
        }
    }

    // Sort alphabetically.
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

/// Search apps by name or comment (case-insensitive substring).
pub fn search<'a>(apps: &'a [App], query: &str) -> Vec<&'a App> {
    if query.is_empty() {
        return apps.iter().collect();
    }
    let q = query.to_lowercase();
    apps.iter()
        .filter(|a| {
            a.name.to_lowercase().contains(&q)
                || a.comment
                    .as_deref()
                    .map(|c| c.to_lowercase().contains(&q))
                    .unwrap_or(false)
        })
        .collect()
}

/// Launch an application by running its Exec= command.
///
/// Returns the child process handle. Detaches from AeroShell so closing
/// the launcher does not kill the app.
pub fn launch(app: &App) -> Result<()> {
    // Strip .desktop field codes (%u, %f, %F, %U, etc.)
    let exec = clean_exec(&app.exec);
    let parts: Vec<&str> = exec.split_whitespace().collect();
    if parts.is_empty() {
        anyhow::bail!("Empty Exec= in {}", app.name);
    }
    std::process::Command::new(parts[0])
        .args(&parts[1..])
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to launch {}: {}", app.name, e))?;
    Ok(())
}

// ── Internals ─────────────────────────────────────────────────────────────────

fn xdg_app_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // ~/.local/share/applications
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }

    // $XDG_DATA_DIRS (default: /usr/local/share:/usr/share)
    let data_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for d in data_dirs.split(':') {
        dirs.push(PathBuf::from(d).join("applications"));
    }

    dirs
}

/// Parse a .desktop file into an App.
/// Returns None if the file is hidden, NoDisplay, or missing Name/Exec.
fn parse_desktop_file(path: &PathBuf) -> Option<App> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut categories = Vec::new();
    let mut comment = None;
    let mut no_display = false;
    let mut in_entry = false;

    for line in text.lines() {
        let line = line.trim();
        if line == "[Desktop Entry]" {
            in_entry = true;
            continue;
        }
        if line.starts_with('[') {
            in_entry = false;
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "Name" => {
                    if name.is_none() {
                        name = Some(value.trim().to_string());
                    }
                }
                "Exec" => {
                    exec = Some(value.trim().to_string());
                }
                "Icon" => {
                    icon = Some(value.trim().to_string());
                }
                "Categories" => {
                    categories = value
                        .trim()
                        .split(';')
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                        .collect();
                }
                "Comment" => {
                    comment = Some(value.trim().to_string());
                }
                "NoDisplay" => {
                    no_display = value.trim().eq_ignore_ascii_case("true");
                }
                "Hidden" if value.trim().eq_ignore_ascii_case("true") => return None,
                _ => {}
            }
        }
    }

    if no_display {
        return None;
    }

    Some(App {
        name: name?,
        exec: exec?,
        icon,
        categories,
        comment,
        desktop_file: path.clone(),
    })
}

/// Remove .desktop field codes from an Exec= string.
fn clean_exec(exec: &str) -> String {
    let mut out = String::with_capacity(exec.len());
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            chars.next(); // skip the code letter
        } else {
            out.push(c);
        }
    }
    // Collapse multiple spaces left by removed codes, then trim.
    let collapsed: String = out.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_exec_removes_codes() {
        assert_eq!(clean_exec("firefox %u"), "firefox");
        assert_eq!(clean_exec("code %F --new-window"), "code --new-window");
        assert_eq!(clean_exec("gedit"), "gedit");
    }

    #[test]
    fn search_is_case_insensitive() {
        let apps = vec![
            App {
                name: "Firefox".into(),
                exec: "firefox".into(),
                icon: None,
                categories: vec![],
                comment: Some("Web browser".into()),
                desktop_file: PathBuf::from("firefox.desktop"),
            },
            App {
                name: "Files".into(),
                exec: "nautilus".into(),
                icon: None,
                categories: vec![],
                comment: None,
                desktop_file: PathBuf::from("files.desktop"),
            },
        ];
        let results = search(&apps, "fire");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Firefox");
    }

    #[test]
    fn search_empty_returns_all() {
        let apps = vec![
            App {
                name: "A".into(),
                exec: "a".into(),
                icon: None,
                categories: vec![],
                comment: None,
                desktop_file: PathBuf::from("a.desktop"),
            },
            App {
                name: "B".into(),
                exec: "b".into(),
                icon: None,
                categories: vec![],
                comment: None,
                desktop_file: PathBuf::from("b.desktop"),
            },
        ];
        assert_eq!(search(&apps, "").len(), 2);
    }
}
