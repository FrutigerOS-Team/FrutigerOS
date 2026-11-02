//! AeroInstaller UI layer.
//!
//! Phase 5: CLI/text mode for testing in QEMU without a GPU.
//! Phase 6: GTK4 + Aero Glass graphical UI.

/// Progress display for the CLI mode.
pub fn print_progress(step_index: usize, total: usize, title: &str) {
    let pct = (step_index as f32 / total as f32 * 100.0) as usize;
    let filled = step_index * 30 / total;
    let bar: String = "█".repeat(filled) + &"░".repeat(30 - filled);
    println!("\n[{bar}] {pct}%  Step {step_index}/{total}: {title}");
}

/// Print a section header.
pub fn print_header(text: &str) {
    let line = "─".repeat(60);
    println!("\n{line}");
    println!("  {text}");
    println!("{line}");
}

/// Print a warning prominently.
pub fn print_warning(text: &str) {
    println!("\n⚠  WARNING: {text}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_does_not_panic() {
        // Just verify these don't panic at edge values.
        print_progress(0, 10, "Welcome");
        print_progress(10, 10, "Complete");
        print_progress(5, 10, "Installing");
    }
}
