//! Welcome wizard step definitions and completion tracking.

use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub enum Step {
    Welcome,
    Theme,
    Keyboard,
    Network,
    AurHelper,
    Done,
}

impl Step {
    pub fn all() -> &'static [Step] {
        &[
            Step::Welcome,
            Step::Theme,
            Step::Keyboard,
            Step::Network,
            Step::AurHelper,
            Step::Done,
        ]
    }

    pub fn title(self) -> &'static str {
        match self {
            Step::Welcome => "Welcome to FrutigerOS",
            Step::Theme => "Choose your theme",
            Step::Keyboard => "Keyboard layout",
            Step::Network => "Connect to the internet",
            Step::AurHelper => "Install AUR helper (optional)",
            Step::Done => "You're all set!",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Step::Welcome => 0,
            Step::Theme => 1,
            Step::Keyboard => 2,
            Step::Network => 3,
            Step::AurHelper => 4,
            Step::Done => 5,
        }
    }
}

fn done_file() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("frutigeros")
        .join("welcome-done")
}

pub fn already_completed() -> bool {
    done_file().exists()
}

pub fn mark_completed() -> Result<()> {
    let path = done_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, "1")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_steps_have_unique_titles() {
        let mut titles = std::collections::HashSet::new();
        for s in Step::all() {
            assert!(titles.insert(s.title()), "duplicate: {}", s.title());
        }
    }

    #[test]
    fn step_indices_sequential() {
        for (i, s) in Step::all().iter().enumerate() {
            assert_eq!(s.index(), i);
        }
    }
}
