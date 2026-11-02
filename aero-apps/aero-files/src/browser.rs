//! Directory browser — reads filesystem metadata.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single directory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size_bytes: u64,
    pub size_human: String,
    /// MIME type guess from extension.
    pub mime_type: String,
    /// Last modified as ISO 8601 string.
    pub modified: String,
}

/// Read a directory and return sorted entries (dirs first, then files).
pub async fn list_directory(path: &str) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    let mut read_dir = tokio::fs::read_dir(path).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files (starting with .) unless showing all.
        // Phase 7: config flag for show_hidden.
        if name.starts_with('.') {
            continue;
        }

        if let Ok(entry) = entry_for_path(&entry.path()).await {
            entries.push(entry);
        }
    }

    sort_entries(&mut entries);
    Ok(entries)
}

/// Build an `Entry` from a path without following symlinks for the type check.
pub async fn entry_for_path(path: &Path) -> Result<Entry> {
    let meta = tokio::fs::symlink_metadata(path).await?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());
    let size_bytes = meta.len();
    let is_dir = meta.is_dir();
    let is_symlink = meta.file_type().is_symlink();
    let mime_type = guess_mime(&name, is_dir);

    let modified = if let Ok(time) = meta.modified() {
        let dt: chrono::DateTime<chrono::Local> = time.into();
        dt.format("%Y-%m-%d %H:%M").to_string()
    } else {
        "unknown".to_string()
    };

    Ok(Entry {
        path: path.to_string_lossy().to_string(),
        name,
        is_dir,
        is_symlink,
        size_bytes,
        size_human: human_size(size_bytes),
        mime_type,
        modified,
    })
}

fn sort_entries(entries: &mut [Entry]) {
    // Sort: directories first, then by name (case-insensitive).
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Guess MIME type from file extension.
fn guess_mime(name: &str, is_dir: bool) -> String {
    if is_dir {
        return "inode/directory".into();
    }
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => "image/".to_string() + &ext,
        "mp3" | "flac" | "ogg" | "wav" | "opus" => "audio/".to_string() + &ext,
        "mp4" | "mkv" | "webm" | "avi" => "video/".to_string() + &ext,
        "pdf" => "application/pdf".into(),
        "zip" | "tar" | "gz" | "bz2" | "xz" | "zst" => "application/archive".into(),
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" => "text/code".into(),
        "toml" | "yaml" | "yml" | "json" | "xml" => "text/config".into(),
        "md" | "txt" | "log" => "text/plain".into(),
        "desktop" => "application/x-desktop".into(),
        _ => "application/octet-stream".into(),
    }
}

/// Format bytes as human-readable string.
fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".into();
    }
    let mut val = bytes as f64;
    let mut unit_idx = 0;
    while val >= 1024.0 && unit_idx < UNITS.len() - 1 {
        val /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", val, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_images() {
        assert_eq!(guess_mime("photo.jpg", false), "image/jpg");
        assert_eq!(guess_mime("icon.svg", false), "image/svg");
    }

    #[test]
    fn mime_directory() {
        assert_eq!(guess_mime("Documents", true), "inode/directory");
    }

    #[test]
    fn human_size_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1024), "1.0 KB");
        assert_eq!(human_size(1_048_576), "1.0 MB");
        assert_eq!(human_size(1_073_741_824), "1.0 GB");
    }

    #[test]
    fn mime_code_files() {
        assert_eq!(guess_mime("main.rs", false), "text/code");
        assert_eq!(guess_mime("config.toml", false), "text/config");
    }

    #[tokio::test]
    async fn list_tmp_doesnt_panic() {
        // /tmp always exists, just verify it runs without error.
        let result = list_directory("/tmp").await;
        assert!(result.is_ok(), "{:?}", result);
    }
}
