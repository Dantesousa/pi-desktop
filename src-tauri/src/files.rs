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
        return Err("o caminho precisa ser absoluto".into());
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
        return Err("o caminho precisa ser absoluto".into());
    }
    match content {
        Some(text) => {
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&p, text).map_err(|e| format!("falha ao restaurar: {e}"))
        }
        None => {
            if p.exists() {
                fs::remove_file(&p).map_err(|e| format!("falha ao remover: {e}"))
            } else {
                Ok(())
            }
        }
    }
}
