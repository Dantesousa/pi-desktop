//! File snapshots so an `edit` / `write` tool call can be reverted from the UI.
//!
//! The snapshot is taken when the tool call is announced (before the tool runs),
//! and restoring simply writes the previous content back.

use serde::Serialize;
use std::fs;
use std::path::PathBuf;

const MAX_SNAPSHOT: u64 = 4 * 1024 * 1024;

#[derive(Serialize, Clone)]
pub struct FileSnapshot {
    pub path: String,
    pub exists: bool,
    pub content: Option<String>,
    pub size: u64,
    pub can_restore: bool,
}

pub fn snapshot(path: &str) -> Result<FileSnapshot, String> {
    let p = PathBuf::from(path);
    if !p.is_absolute() {
        return Err("the path must be absolute".into());
    }
    if !p.exists() {
        return Ok(FileSnapshot {
            path: path.to_string(),
            exists: false,
            content: None,
            size: 0,
            can_restore: true,
        });
    }
    if p.is_dir() {
        return Ok(FileSnapshot {
            path: path.to_string(),
            exists: true,
            content: None,
            size: 0,
            can_restore: false,
        });
    }
    let meta = fs::metadata(&p).map_err(|e| e.to_string())?;
    if meta.len() > MAX_SNAPSHOT {
        return Ok(FileSnapshot {
            path: path.to_string(),
            exists: true,
            content: None,
            size: meta.len(),
            can_restore: false,
        });
    }
    let bytes = fs::read(&p).map_err(|e| e.to_string())?;
    match String::from_utf8(bytes) {
        Ok(text) if !text.contains('\u{0}') => Ok(FileSnapshot {
            path: path.to_string(),
            exists: true,
            content: Some(text),
            size: meta.len(),
            can_restore: true,
        }),
        _ => Ok(FileSnapshot {
            path: path.to_string(),
            exists: true,
            content: None,
            size: meta.len(),
            can_restore: false,
        }),
    }
}

/// `content = Some(text)` restores that text; `content = None` removes the file
/// (used when the file did not exist before the tool call).
pub fn restore(path: &str, content: Option<String>) -> Result<(), String> {
    let p = PathBuf::from(path);
    if !p.is_absolute() {
        return Err("the path must be absolute".into());
    }
    match content {
        Some(text) => {
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&p, text).map_err(|e| format!("failed to restore: {e}"))
        }
        None => {
            if p.exists() {
                fs::remove_file(&p).map_err(|e| format!("failed to remove: {e}"))
            } else {
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("pi-desktop-test-{}-{}", std::process::id(), name))
    }

    #[test]
    fn snapshot_and_restore_existing_file() {
        let path = tmp("edit.txt");
        fs::write(&path, "before\n").unwrap();

        let snap = snapshot(path.to_str().unwrap()).unwrap();
        assert!(snap.exists);
        assert!(snap.can_restore);
        assert_eq!(snap.content.as_deref(), Some("before\n"));

        fs::write(&path, "after\n").unwrap();
        restore(path.to_str().unwrap(), snap.content.clone()).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "before\n");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn snapshot_of_missing_file_restores_by_deleting() {
        let path = tmp("novo.txt");
        let _ = fs::remove_file(&path);

        let snap = snapshot(path.to_str().unwrap()).unwrap();
        assert!(!snap.exists);
        assert!(snap.can_restore);

        fs::write(&path, "criado pela tool\n").unwrap();
        restore(path.to_str().unwrap(), None).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn relative_paths_are_rejected() {
        assert!(snapshot("relativo.txt").is_err());
        assert!(restore("relativo.txt", None).is_err());
    }
}

/// Writes a diagnostics bundle to `~/.cache/pi-desktop/diagnostics/` and returns its path.
/// Lets the user hand the agent a complete snapshot of the UI state instead of a screenshot.
pub fn write_diagnostics(payload: &str) -> Result<String, String> {
    let base = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cache")
        })
        .join("pi-desktop")
        .join("diagnostics");
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = base.join(format!("diag-{ts}.json"));
    fs::write(&path, payload).map_err(|e| format!("failed to write diagnostics: {e}"))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Opens a URL in the user's browser instead of navigating the app webview.
pub fn open_external(url: &str) -> Result<(), String> {
    let url = url.trim();
    let allowed = url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("mailto:")
        || url.starts_with("ftp://");
    if !allowed {
        return Err("only http(s), mailto and ftp are opened".into());
    }
    let candidates: [(&str, Vec<&str>); 3] = [
        ("xdg-open", vec![url]),
        ("gio", vec!["open", url]),
        ("kde-open5", vec![url]),
    ];
    for (bin, args) in candidates {
        if std::process::Command::new(bin)
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .is_ok()
        {
            return Ok(());
        }
    }
    Err("no opener available (xdg-open/gio)".into())
}
