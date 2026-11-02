//! Lightweight runtime state shared with AeroShell.
//!
//! AeroWM and AeroShell are separate processes with separate Wayland
//! connections. A tiny key/value file in `$XDG_RUNTIME_DIR` is enough to keep
//! the panel's workspace indicator in sync without adding a socket protocol.
//! Writes are atomic (temp file + rename) so the shell never reads a partial
//! record.

use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeState {
    pub active_workspace: usize,
    pub workspace_count: usize,
}

impl RuntimeState {
    pub fn to_text(&self) -> String {
        format!(
            "version=1\nactive_workspace={}\nworkspace_count={}\n",
            self.active_workspace, self.workspace_count
        )
    }

    /// Parse the key/value format. Unknown keys are ignored; missing keys
    /// default to zero so older/newer writers remain compatible.
    pub fn parse(text: &str) -> Self {
        let mut state = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "active_workspace" => {
                    state.active_workspace = value.trim().parse().unwrap_or(0);
                }
                "workspace_count" => {
                    state.workspace_count = value.trim().parse().unwrap_or(0);
                }
                _ => {}
            }
        }
        state
    }
}

/// Directory shared by the compositor and shell.
pub fn runtime_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("frutigeros");
    }
    let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
    std::env::temp_dir().join(format!("frutigeros-{user}"))
}

pub fn state_path() -> PathBuf {
    runtime_dir().join("aerowm-state")
}

/// Atomically publish the current state. Best-effort: a failure is reported to
/// the caller but must never stop the compositor.
pub fn publish(state: RuntimeState) -> Result<()> {
    let path = state_path();
    let dir = path
        .parent()
        .context("runtime state path has no parent directory")?;
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;

    let tmp = path.with_extension("tmp");
    {
        let mut file =
            std::fs::File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
        file.write_all(state.to_text().as_bytes())
            .with_context(|| format!("write {}", tmp.display()))?;
        file.sync_all().ok();
    }
    std::fs::rename(&tmp, &path)
        .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let state = RuntimeState {
            active_workspace: 2,
            workspace_count: 4,
        };
        assert_eq!(RuntimeState::parse(&state.to_text()), state);
    }

    #[test]
    fn ignores_unknown_and_malformed_lines() {
        let parsed = RuntimeState::parse("garbage\nactive_workspace=3\nnope=x\nworkspace_count=5\n");
        assert_eq!(
            parsed,
            RuntimeState {
                active_workspace: 3,
                workspace_count: 5
            }
        );
    }

    #[test]
    fn state_path_is_under_runtime_dir() {
        let path = state_path();
        assert!(path.ends_with("frutigeros/aerowm-state"));
    }
}
