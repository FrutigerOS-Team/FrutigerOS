//! PTY (pseudo-terminal) management.
//!
//! Phase 6: spawn a shell in a PTY, read/write bytes.
//! Phase 7: wire to the VT100 parser and Wayland surface.

/// PTY handle — Phase 7 will use the `nix` crate for real PTY creation.
pub struct Pty {
    pub shell: String,
    pub rows: u16,
    pub cols: u16,
}

impl Pty {
    pub fn new(shell: &str, rows: u16, cols: u16) -> Self {
        Self {
            shell: shell.to_string(),
            rows,
            cols,
        }
    }

    /// Resize the PTY.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.rows = rows;
        self.cols = cols;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pty_resize() {
        let mut pty = Pty::new("/bin/bash", 24, 80);
        pty.resize(30, 120);
        assert_eq!(pty.rows, 30);
        assert_eq!(pty.cols, 120);
    }
}
