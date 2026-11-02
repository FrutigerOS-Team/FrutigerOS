//! Input handling for AeroWM.
//!
//! Processes keyboard and pointer events from the Smithay input backend
//! and translates them into compositor actions.

/// High-level action the compositor should perform in response to input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Switch to workspace N (0-indexed).
    SwitchWorkspace(usize),
    /// Close the focused window.
    CloseWindow,
    /// Launch the application launcher.
    OpenLauncher,
    /// Open a terminal.
    OpenTerminal,
    /// Cycle focus to the next window (Alt+Tab).
    FocusNext,
    /// Cycle focus to the previous window (Alt+Shift+Tab).
    FocusPrev,
    /// Toggle fullscreen on the focused window.
    ToggleFullscreen,
    /// Toggle floating/tiling for the focused window.
    ToggleFloat,
    /// Quit the compositor.
    Quit,
    /// No action — key was handled but produces no compositor command.
    None,
}

/// Modifier keys currently held.
#[derive(Debug, Default, Clone, Copy)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

/// Process a key press and return the corresponding action.
///
/// `modifier_key` comes from config (e.g. "super" or "alt").
pub fn handle_key(key_name: &str, modifiers: Modifiers, modifier_key: &str) -> Action {
    let mod_held = match modifier_key {
        "alt" => modifiers.alt,
        "ctrl" => modifiers.ctrl,
        _ => modifiers.super_key, // default: Super
    };

    // Super+Shift+Q → quit
    if mod_held && modifiers.shift && key_name == "q" {
        return Action::Quit;
    }

    // Super+Return → terminal
    if mod_held && (key_name == "return" || key_name == "enter") {
        return Action::OpenTerminal;
    }

    // Super+Space → launcher
    if mod_held && key_name == "space" {
        return Action::OpenLauncher;
    }

    // Super+W → close window
    if mod_held && key_name == "w" {
        return Action::CloseWindow;
    }

    // Super+F → fullscreen
    if mod_held && key_name == "f" {
        return Action::ToggleFullscreen;
    }

    // Super+V → toggle float
    if mod_held && key_name == "v" {
        return Action::ToggleFloat;
    }

    // Alt+Tab / Alt+Shift+Tab → cycle focus
    if modifiers.alt && key_name == "tab" {
        return if modifiers.shift {
            Action::FocusPrev
        } else {
            Action::FocusNext
        };
    }

    // Super+1 through Super+9 → switch workspace
    if mod_held {
        if let Some(digit) = key_name.chars().next().and_then(|c| c.to_digit(10)) {
            if digit >= 1 {
                return Action::SwitchWorkspace((digit - 1) as usize);
            }
        }
    }

    Action::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn super_return_opens_terminal() {
        let mods = Modifiers {
            super_key: true,
            ..Default::default()
        };
        assert_eq!(handle_key("return", mods, "super"), Action::OpenTerminal);
    }

    #[test]
    fn super_1_switches_workspace() {
        let mods = Modifiers {
            super_key: true,
            ..Default::default()
        };
        assert_eq!(handle_key("1", mods, "super"), Action::SwitchWorkspace(0));
        assert_eq!(handle_key("4", mods, "super"), Action::SwitchWorkspace(3));
    }

    #[test]
    fn alt_tab_cycles_focus() {
        let mods = Modifiers {
            alt: true,
            ..Default::default()
        };
        assert_eq!(handle_key("tab", mods, "super"), Action::FocusNext);
        let mods_shift = Modifiers {
            alt: true,
            shift: true,
            ..Default::default()
        };
        assert_eq!(handle_key("tab", mods_shift, "super"), Action::FocusPrev);
    }

    #[test]
    fn unbound_key_returns_none() {
        let mods = Modifiers::default();
        assert_eq!(handle_key("a", mods, "super"), Action::None);
    }
}
