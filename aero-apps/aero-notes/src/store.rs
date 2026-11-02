//! Note storage — plain Markdown files in a directory.
//! One file per note, filename = slug of title + .md

use anyhow::Result;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String, // filename without .md
    pub title: String,
    pub body: String,
    pub created: DateTime<Local>,
    pub updated: DateTime<Local>,
    pub pinned: bool,
    pub tags: Vec<String>,
}

impl Note {
    pub fn new(title: &str, body: &str) -> Self {
        let now = Local::now();
        Self {
            id: slug(title),
            title: title.to_string(),
            body: body.to_string(),
            created: now,
            updated: now,
            pinned: false,
            tags: vec![],
        }
    }

    /// Render as a Markdown string for storage.
    pub fn to_markdown(&self) -> String {
        let tags = self.tags.join(", ");
        format!(
            "---\ntitle: {}\ncreated: {}\nupdated: {}\npinned: {}\ntags: {}\n---\n\n{}",
            self.title,
            self.created.to_rfc3339(),
            self.updated.to_rfc3339(),
            self.pinned,
            tags,
            self.body,
        )
    }
}

/// Manages a directory of note files.
pub struct NoteStore {
    dir: String,
    notes: Vec<Note>,
}

impl NoteStore {
    /// Open a note directory. Creates it if it doesn't exist.
    pub async fn open(dir: &str) -> Result<Self> {
        tokio::fs::create_dir_all(dir).await?;

        let mut notes = Vec::new();
        let mut read_dir = tokio::fs::read_dir(dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            if let Ok(text) = tokio::fs::read_to_string(&path).await {
                if let Some(note) = parse_markdown_note(&path.to_string_lossy(), &text) {
                    notes.push(note);
                }
            }
        }

        // Sort: pinned first, then by updated date descending.
        notes.sort_by(|a, b| b.pinned.cmp(&a.pinned).then(b.updated.cmp(&a.updated)));

        Ok(Self {
            dir: dir.to_string(),
            notes,
        })
    }

    pub fn count(&self) -> usize {
        self.notes.len()
    }

    pub fn list(&self) -> &[Note] {
        &self.notes
    }

    /// Find a note by id (filename stem).
    pub fn get(&self, id: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    /// Search titles, bodies, and tags (case-insensitive).
    pub fn search(&self, query: &str) -> Vec<&Note> {
        let q = query.to_lowercase();
        self.notes
            .iter()
            .filter(|n| {
                n.title.to_lowercase().contains(&q)
                    || n.body.to_lowercase().contains(&q)
                    || n.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Save a new or updated note to disk.
    pub async fn save(&mut self, note: Note) -> Result<()> {
        let path = format!("{}/{}.md", self.dir, note.id);
        tokio::fs::write(&path, note.to_markdown()).await?;

        // Update in-memory list.
        if let Some(existing) = self.notes.iter_mut().find(|n| n.id == note.id) {
            *existing = note;
        } else {
            self.notes.push(note);
        }
        Ok(())
    }

    /// Delete a note by id.
    pub async fn delete(&mut self, id: &str) -> Result<()> {
        let path = format!("{}/{}.md", self.dir, id);
        tokio::fs::remove_file(&path).await?;
        self.notes.retain(|n| n.id != id);
        Ok(())
    }
}

/// Parse a very simple front-matter Markdown note.
fn parse_markdown_note(path: &str, text: &str) -> Option<Note> {
    // Expect "---\n...frontmatter...\n---\n\nbody"
    let after_first = text.trim_start_matches("---\n");
    let end = after_first.find("\n---\n")?;
    let frontmatter = &after_first[..end];
    let body = after_first[end + 5..].trim_start_matches('\n').to_string();

    let mut title = String::new();
    let now = Local::now();
    let mut created = now;
    let mut updated = now;
    let mut pinned = false;
    let mut tags = Vec::new();

    for line in frontmatter.lines() {
        if let Some((k, v)) = line.split_once(": ") {
            match k.trim() {
                "title" => title = v.trim().to_string(),
                "created" => {
                    if let Ok(dt) = DateTime::parse_from_rfc3339(v.trim()) {
                        created = dt.into();
                    }
                }
                "updated" => {
                    if let Ok(dt) = DateTime::parse_from_rfc3339(v.trim()) {
                        updated = dt.into();
                    }
                }
                "pinned" => pinned = v.trim() == "true",
                "tags" => {
                    tags = v
                        .split(',')
                        .map(|t| t.trim().to_string())
                        .filter(|t| !t.is_empty())
                        .collect()
                }
                _ => {}
            }
        }
    }

    if title.is_empty() {
        return None;
    }

    let id = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    Some(Note {
        id,
        title,
        body,
        created,
        updated,
        pinned,
        tags,
    })
}

/// Convert a title to a filename slug.
fn slug(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_basic() {
        assert_eq!(slug("Hello World"), "hello-world");
        assert_eq!(slug("My Note: v2!"), "my-note-v2");
        assert_eq!(slug("  spaces  "), "spaces");
    }

    #[test]
    fn note_markdown_roundtrip() {
        let note = Note::new("Test Note", "This is the body.");
        let md = note.to_markdown();
        assert!(md.contains("title: Test Note"));
        assert!(md.contains("This is the body."));
        assert!(md.starts_with("---\n"));
    }

    #[tokio::test]
    async fn open_empty_dir() {
        let dir = format!("/tmp/aero-notes-test-{}", std::process::id());
        let store = NoteStore::open(&dir).await.unwrap();
        assert_eq!(store.count(), 0);
        tokio::fs::remove_dir_all(&dir).await.ok();
    }

    #[test]
    fn search_matches_title_and_body() {
        let mut a = Note::new("Groceries", "milk and eggs");
        a.tags.push("home".into());
        let b = Note::new("Work", "finish the roadmap");
        let store = NoteStore {
            dir: String::new(),
            notes: vec![a, b],
        };
        assert_eq!(store.search("milk").len(), 1);
        assert_eq!(store.search("WORK").len(), 1);
        assert_eq!(store.search("home").len(), 1);
        assert_eq!(store.search("absent").len(), 0);
    }

    #[tokio::test]
    async fn save_and_list() {
        let dir = format!("/tmp/aero-notes-test2-{}", std::process::id());
        let mut store = NoteStore::open(&dir).await.unwrap();
        let note = Note::new("My Note", "Hello FrutigerOS");
        store.save(note).await.unwrap();
        assert_eq!(store.count(), 1);
        assert_eq!(store.list()[0].title, "My Note");
        tokio::fs::remove_dir_all(&dir).await.ok();
    }
}
