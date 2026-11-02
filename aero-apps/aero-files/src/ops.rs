//! File operations and recursive search.
//!
//! Destructive operations validate their targets and never follow symlinks when
//! deleting. The UI is expected to confirm user intent; these functions enforce
//! the hard safety floor (no root, no parent-escaping recursion).

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

use crate::browser::{entry_for_path, Entry};

/// Maximum number of entries returned by a search.
pub const DEFAULT_SEARCH_LIMIT: usize = 500;

/// Maximum directory depth walked by `search`.
pub const DEFAULT_SEARCH_DEPTH: usize = 12;

/// Return an error for paths that must never be modified by the file manager.
pub fn ensure_mutable(path: &Path) -> Result<()> {
    let text = path.to_string_lossy();
    if text.trim().is_empty() {
        return Err(anyhow!("refusing to operate on an empty path"));
    }

    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    // Normalise `..` without requiring the path to exist.
    let mut normalised = PathBuf::new();
    for component in absolute.components() {
        use std::path::Component;
        match component {
            Component::ParentDir => {
                normalised.pop();
            }
            Component::CurDir => {}
            other => normalised.push(other.as_os_str()),
        }
    }

    if normalised == Path::new("/") {
        return Err(anyhow!("refusing to modify the filesystem root"));
    }
    if normalised.parent().is_none() {
        return Err(anyhow!("refusing to modify {:?}", normalised));
    }
    Ok(())
}

/// Rename or move a file/directory. Refuses to overwrite an existing target.
pub async fn rename(from: &Path, to: &Path) -> Result<()> {
    ensure_mutable(from)?;
    ensure_mutable(to)?;
    if tokio::fs::try_exists(to).await? {
        return Err(anyhow!("target already exists: {}", to.display()));
    }
    tokio::fs::rename(from, to)
        .await
        .with_context(|| format!("rename {} -> {}", from.display(), to.display()))
}

/// Copy a regular file. Directories must be copied explicitly by the caller.
pub async fn copy_file(from: &Path, to: &Path) -> Result<()> {
    ensure_mutable(to)?;
    let meta = tokio::fs::metadata(from).await?;
    if !meta.is_file() {
        return Err(anyhow!(
            "only regular files can be copied: {}",
            from.display()
        ));
    }
    if tokio::fs::try_exists(to).await? {
        return Err(anyhow!("target already exists: {}", to.display()));
    }
    tokio::fs::copy(from, to)
        .await
        .with_context(|| format!("copy {} -> {}", from.display(), to.display()))?;
    Ok(())
}

/// Create a directory and all parents.
pub async fn create_dir(path: &Path) -> Result<()> {
    ensure_mutable(path)?;
    tokio::fs::create_dir_all(path)
        .await
        .with_context(|| format!("create directory {}", path.display()))
}

/// Delete a file, symlink or directory tree. Symlinks are removed, not followed.
pub async fn delete(path: &Path) -> Result<()> {
    ensure_mutable(path)?;
    let meta = tokio::fs::symlink_metadata(path).await?;
    if meta.file_type().is_symlink() || meta.is_file() {
        tokio::fs::remove_file(path).await?;
    } else if meta.is_dir() {
        tokio::fs::remove_dir_all(path).await?;
    } else {
        return Err(anyhow!("unsupported file type: {}", path.display()));
    }
    Ok(())
}

/// Recursively search for entries whose name or MIME type contains `query`.
///
/// Hidden directories are skipped, depth is bounded, and results are capped.
pub async fn search(
    root: &Path,
    query: &str,
    limit: usize,
    max_depth: usize,
) -> Result<Vec<Entry>> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }

    let root = root.to_path_buf();
    let mut results = Vec::new();
    let mut queue = vec![(root, 0usize)];

    while let Some((dir, depth)) = queue.pop() {
        if depth > max_depth || results.len() >= limit {
            continue;
        }
        let Ok(mut rd) = tokio::fs::read_dir(&dir).await else {
            continue;
        };
        while let Ok(Some(entry)) = rd.next_entry().await {
            if results.len() >= limit {
                break;
            }
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }

            let Ok(item) = entry_for_path(&path).await else {
                continue;
            };

            let matched = item.name.to_lowercase().contains(&needle)
                || item.mime_type.to_lowercase().contains(&needle);
            if matched {
                results.push(item.clone());
            }

            if item.is_dir && !item.is_symlink {
                queue.push((path, depth + 1));
            }
        }
    }

    results.sort_by_key(|a| a.path.to_lowercase());
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "aero-files-{}-{}-{}",
            name,
            std::process::id(),
            nanos
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn refuses_root() {
        assert!(ensure_mutable(Path::new("/")).is_err());
        assert!(ensure_mutable(Path::new("")).is_err());
        assert!(ensure_mutable(Path::new("/tmp/example")).is_ok());
    }

    #[tokio::test]
    async fn copy_rename_delete_roundtrip() {
        let dir = scratch("ops");
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        let c = dir.join("c.txt");
        tokio::fs::write(&a, b"hello").await.unwrap();

        copy_file(&a, &b).await.unwrap();
        assert_eq!(tokio::fs::read_to_string(&b).await.unwrap(), "hello");

        rename(&b, &c).await.unwrap();
        assert!(!b.exists());
        assert_eq!(tokio::fs::read_to_string(&c).await.unwrap(), "hello");

        delete(&c).await.unwrap();
        assert!(!c.exists());
        delete(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn copy_refuses_existing_target() {
        let dir = scratch("existing");
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        tokio::fs::write(&a, b"a").await.unwrap();
        tokio::fs::write(&b, b"b").await.unwrap();
        assert!(copy_file(&a, &b).await.is_err());
        delete(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn search_finds_nested_files() {
        let dir = scratch("search");
        let nested = dir.join("sub").join("deeper");
        tokio::fs::create_dir_all(&nested).await.unwrap();
        tokio::fs::write(nested.join("sunset.png"), b"x")
            .await
            .unwrap();
        tokio::fs::write(dir.join("readme.txt"), b"x")
            .await
            .unwrap();

        let hits = search(&dir, "sunset", DEFAULT_SEARCH_LIMIT, DEFAULT_SEARCH_DEPTH)
            .await
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].path.ends_with("sunset.png"));

        let images = search(&dir, "image/", DEFAULT_SEARCH_LIMIT, DEFAULT_SEARCH_DEPTH)
            .await
            .unwrap();
        assert_eq!(images.len(), 1);

        delete(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn search_skips_hidden_and_respects_limit() {
        let dir = scratch("limit");
        let hidden = dir.join(".secret");
        tokio::fs::create_dir_all(&hidden).await.unwrap();
        tokio::fs::write(hidden.join("hidden-match.txt"), b"x")
            .await
            .unwrap();
        for i in 0..5 {
            tokio::fs::write(dir.join(format!("match-{i}.txt")), b"x")
                .await
                .unwrap();
        }

        let hits = search(&dir, "match", 2, DEFAULT_SEARCH_DEPTH)
            .await
            .unwrap();
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|h| !h.path.contains(".secret")));

        delete(&dir).await.unwrap();
    }
}
