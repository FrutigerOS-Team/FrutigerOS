//! Workspace management for AeroWM.
//!
//! A workspace is a named virtual desktop. Windows are assigned to exactly one
//! workspace and only the active workspace is rendered.

use std::collections::VecDeque;

/// Layout mode for a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Floating,
    Tiling,
    Hybrid,
}

impl Layout {
    pub fn from_str(s: &str) -> Self {
        match s {
            "tiling" => Layout::Tiling,
            "hybrid" => Layout::Hybrid,
            _ => Layout::Floating,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Layout::Floating => "floating",
            Layout::Tiling => "tiling",
            Layout::Hybrid => "hybrid",
        }
    }
}

/// A single virtual workspace.
#[derive(Debug)]
pub struct Workspace {
    pub index: usize,
    pub name: String,
    pub layout: Layout,
    /// Window IDs (opaque handles) assigned to this workspace.
    /// Ordering determines stacking (last = top).
    pub windows: VecDeque<WindowId>,
    /// The currently focused window, if any.
    pub focused: Option<WindowId>,
}

/// Opaque window identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

impl Workspace {
    pub fn new(index: usize, layout: Layout) -> Self {
        Self {
            index,
            name: format!("{}", index + 1),
            layout,
            windows: VecDeque::new(),
            focused: None,
        }
    }

    /// Add a window to this workspace and focus it.
    pub fn add_window(&mut self, id: WindowId) {
        self.windows.push_back(id);
        self.focused = Some(id);
    }

    /// Remove a window. Focuses the next window in the stack, if any.
    pub fn remove_window(&mut self, id: WindowId) {
        self.windows.retain(|&w| w != id);
        if self.focused == Some(id) {
            self.focused = self.windows.back().copied();
        }
    }

    /// Focus a window and raise it to the top of the stack.
    ///
    /// Does nothing if the window is not part of this workspace.
    pub fn focus_window(&mut self, id: WindowId) {
        if let Some(pos) = self.windows.iter().position(|&w| w == id) {
            self.windows.remove(pos);
            self.windows.push_back(id);
            self.focused = Some(id);
        }
    }

    /// Cycle focus forward through the window stack.
    pub fn focus_next(&mut self) {
        if self.windows.len() < 2 {
            return;
        }
        if let Some(current) = self.focused {
            let pos = self.windows.iter().position(|&w| w == current);
            if let Some(i) = pos {
                let next = (i + 1) % self.windows.len();
                self.focused = Some(self.windows[next]);
            }
        }
    }

    /// Cycle focus backward through the window stack.
    pub fn focus_prev(&mut self) {
        if self.windows.len() < 2 {
            return;
        }
        if let Some(current) = self.focused {
            let pos = self.windows.iter().position(|&w| w == current);
            if let Some(i) = pos {
                let prev = if i == 0 {
                    self.windows.len() - 1
                } else {
                    i - 1
                };
                self.focused = Some(self.windows[prev]);
            }
        }
    }
}

/// Manages all workspaces and the active workspace index.
#[derive(Debug)]
pub struct WorkspaceManager {
    workspaces: Vec<Workspace>,
    active: usize,
}

impl WorkspaceManager {
    pub fn new(count: usize, default_layout: Layout) -> Self {
        assert!(count > 0, "must have at least one workspace");
        let workspaces = (0..count)
            .map(|i| Workspace::new(i, default_layout))
            .collect();
        Self {
            workspaces,
            active: 0,
        }
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active(&self) -> &Workspace {
        &self.workspaces[self.active]
    }

    pub fn active_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active]
    }

    /// Switch to the workspace at `index`. Returns `true` if the index was valid.
    pub fn switch_to(&mut self, index: usize) -> bool {
        if index < self.workspaces.len() {
            self.active = index;
            true
        } else {
            false
        }
    }

    pub fn count(&self) -> usize {
        self.workspaces.len()
    }

    /// Remove a window from every workspace.
    ///
    /// Used for surfaces that are promoted to a shell role (for example the
    /// AeroPanel) and should not appear in the regular window list.
    pub fn remove_window_from_all(&mut self, id: WindowId) {
        for ws in &mut self.workspaces {
            ws.remove_window(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_manager_creation() {
        let wm = WorkspaceManager::new(4, Layout::Floating);
        assert_eq!(wm.count(), 4);
        assert_eq!(wm.active_index(), 0);
    }

    #[test]
    fn switch_workspace() {
        let mut wm = WorkspaceManager::new(4, Layout::Floating);
        assert!(wm.switch_to(2));
        assert_eq!(wm.active_index(), 2);
        assert!(!wm.switch_to(99));
        assert_eq!(wm.active_index(), 2); // unchanged
    }

    #[test]
    fn window_focus_cycle() {
        let mut ws = Workspace::new(0, Layout::Floating);
        let a = WindowId(1);
        let b = WindowId(2);
        let c = WindowId(3);
        ws.add_window(a);
        ws.add_window(b);
        ws.add_window(c);
        assert_eq!(ws.focused, Some(c));

        ws.focus_next();
        assert_eq!(ws.focused, Some(a));

        ws.focus_prev();
        assert_eq!(ws.focused, Some(c));
    }

    #[test]
    fn remove_focused_window() {
        let mut ws = Workspace::new(0, Layout::Floating);
        ws.add_window(WindowId(1));
        ws.add_window(WindowId(2));
        ws.remove_window(WindowId(2));
        assert_eq!(ws.focused, Some(WindowId(1)));
    }
}
