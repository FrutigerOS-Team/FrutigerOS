//! Music library — scans a directory for audio files and reads metadata.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: u32,
    pub track_number: Option<u32>,
    pub year: Option<u32>,
}

impl Track {
    pub fn duration_str(&self) -> String {
        let m = self.duration_secs / 60;
        let s = self.duration_secs % 60;
        format!("{}:{:02}", m, s)
    }
}

/// Maximum directory depth walked by the scanner.
const MAX_DEPTH: usize = 8;

fn is_audio(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str(),
        "mp3" | "flac" | "ogg" | "opus" | "wav" | "m4a"
    )
}

fn scan_sync(root: &str) -> Vec<Track> {
    let mut tracks = Vec::new();
    let mut queue = vec![(std::path::PathBuf::from(root), 0usize)];

    while let Some((dir, depth)) = queue.pop() {
        if depth > MAX_DEPTH {
            continue;
        }

        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }

            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            if file_type.is_dir() && !file_type.is_symlink() {
                queue.push((path, depth + 1));
                continue;
            }
            if !file_type.is_file() || !is_audio(&path) {
                continue;
            }

            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string();

            // Parse the common "Artist - Title" filename convention.
            let (artist, title) = if let Some((a, t)) = stem.split_once(" - ") {
                (a.trim().to_string(), t.trim().to_string())
            } else {
                ("Unknown Artist".to_string(), stem)
            };

            let album = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .filter(|n| *n != root)
                .unwrap_or("Unknown Album")
                .to_string();

            tracks.push(Track {
                path: path.to_string_lossy().to_string(),
                title,
                artist,
                album,
                duration_secs: 0, // real duration needs a tag reader
                track_number: None,
                year: None,
            });
        }
    }

    tracks.sort_by(|a, b| a.artist.cmp(&b.artist).then(a.title.cmp(&b.title)));
    tracks
}

pub struct MusicLibrary {
    tracks: Vec<Track>,
}

impl MusicLibrary {
    /// Scan a directory tree for audio files.
    ///
    /// Recursion is bounded to `MAX_DEPTH`, hidden directories are skipped, and
    /// symlinked directories are not followed (avoids cycles). Tag parsing is
    /// still filename-based; `lofty` integration is tracked for Phase 7.
    pub async fn scan(dir: &str) -> Self {
        let dir = dir.to_string();
        match tokio::task::spawn_blocking(move || scan_sync(&dir)).await {
            Ok(tracks) => Self { tracks },
            Err(_) => Self { tracks: Vec::new() },
        }
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    pub fn tracks(&self) -> impl Iterator<Item = &Track> {
        self.tracks.iter()
    }

    pub fn search(&self, query: &str) -> Vec<&Track> {
        let q = query.to_lowercase();
        self.tracks
            .iter()
            .filter(|t| {
                t.title.to_lowercase().contains(&q)
                    || t.artist.to_lowercase().contains(&q)
                    || t.album.to_lowercase().contains(&q)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_str_format() {
        let t = Track {
            path: String::new(),
            title: "X".into(),
            artist: "Y".into(),
            album: String::new(),
            duration_secs: 185,
            track_number: None,
            year: None,
        };
        assert_eq!(t.duration_str(), "3:05");
    }

    #[test]
    fn duration_str_zero() {
        let t = Track {
            path: String::new(),
            title: "X".into(),
            artist: "Y".into(),
            album: String::new(),
            duration_secs: 0,
            track_number: None,
            year: None,
        };
        assert_eq!(t.duration_str(), "0:00");
    }

    #[test]
    fn is_audio_checks_extension_case_insensitively() {
        assert!(is_audio(Path::new("/x/Song.MP3")));
        assert!(is_audio(Path::new("/x/Song.flac")));
        assert!(!is_audio(Path::new("/x/cover.png")));
    }

    #[tokio::test]
    async fn scan_is_recursive_and_skips_hidden() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("aero-music-{}-{}", std::process::id(), nanos));
        let nested = root.join("Album").join("Disc 1");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.join("Loose.mp3"), b"x").unwrap();
        std::fs::write(nested.join("Aero Band - Blue Sky.flac"), b"x").unwrap();
        std::fs::write(nested.join("cover.png"), b"x").unwrap();
        let hidden = root.join(".hidden");
        std::fs::create_dir_all(&hidden).unwrap();
        std::fs::write(hidden.join("Secret.mp3"), b"x").unwrap();

        let lib = MusicLibrary::scan(root.to_str().unwrap()).await;
        assert_eq!(
            lib.track_count(),
            2,
            "tracks: {:?}",
            lib.tracks().collect::<Vec<_>>()
        );
        assert!(lib.search("blue sky").len() == 1);
        assert!(lib.search("aero band").len() == 1);
        assert!(lib.search("secret").is_empty());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn scan_empty_dir() {
        let lib = MusicLibrary::scan("/tmp/no-music-here-xyzzy").await;
        assert_eq!(lib.track_count(), 0);
    }

    #[test]
    fn search_is_case_insensitive() {
        let lib = MusicLibrary {
            tracks: vec![Track {
                path: String::new(),
                title: "Blue Sky".into(),
                artist: "Aero Band".into(),
                album: String::new(),
                duration_secs: 200,
                track_number: None,
                year: None,
            }],
        };
        assert_eq!(lib.search("blue").len(), 1);
        assert_eq!(lib.search("AERO").len(), 1);
        assert_eq!(lib.search("xyz").len(), 0);
    }
}
