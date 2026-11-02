//! Settings page definitions.

#[derive(Debug, Clone)]
pub struct SettingsPage {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    pub description: &'static str,
}

pub fn all_pages() -> Vec<SettingsPage> {
    vec![
        SettingsPage {
            id: "appearance",
            title: "Appearance",
            icon: "🎨",
            description: "Themes, colors, wallpapers",
        },
        SettingsPage {
            id: "display",
            title: "Display",
            icon: "🖥",
            description: "Resolution, scaling, arrangement",
        },
        SettingsPage {
            id: "audio",
            title: "Audio",
            icon: "🔊",
            description: "Volume, outputs, inputs",
        },
        SettingsPage {
            id: "network",
            title: "Network",
            icon: "📡",
            description: "Wi-Fi, Ethernet, VPN",
        },
        SettingsPage {
            id: "bluetooth",
            title: "Bluetooth",
            icon: "🔵",
            description: "Pair and manage devices",
        },
        SettingsPage {
            id: "keyboard",
            title: "Keyboard",
            icon: "⌨️",
            description: "Layout, shortcuts, input methods",
        },
        SettingsPage {
            id: "mouse",
            title: "Mouse & Touchpad",
            icon: "🖱",
            description: "Speed, buttons, gestures",
        },
        SettingsPage {
            id: "power",
            title: "Power",
            icon: "🔋",
            description: "Sleep, hibernate, battery",
        },
        SettingsPage {
            id: "aerowm",
            title: "AeroWM",
            icon: "◈",
            description: "Window manager appearance and behaviour",
        },
        SettingsPage {
            id: "workspaces",
            title: "Workspaces",
            icon: "⊞",
            description: "Number, names, layouts",
        },
        SettingsPage {
            id: "users",
            title: "Users",
            icon: "👤",
            description: "Accounts and passwords",
        },
        SettingsPage {
            id: "system-info",
            title: "System Info",
            icon: "ℹ️",
            description: "Hardware, FrutigerOS version",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_pages_has_aerowm() {
        let pages = all_pages();
        assert!(pages.iter().any(|p| p.id == "aerowm"));
    }

    #[test]
    fn no_duplicate_ids() {
        let pages = all_pages();
        let mut ids = std::collections::HashSet::new();
        for p in &pages {
            assert!(ids.insert(p.id), "duplicate id: {}", p.id);
        }
    }
}
