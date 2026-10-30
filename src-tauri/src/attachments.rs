//! Attachment handling: images go inline as base64, text is inlined, binaries
//! are staged on disk so the agent can reach them with its own tools.

use base64::Engine;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const IMAGE_MAX: usize = 12 * 1024 * 1024;
const TEXT_MAX: usize = 400 * 1024;

fn cache_root() -> PathBuf {
    let base = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cache")
        });
    base.join("pi-desktop").join("attachments")
}

fn stamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn mime_for(name: &str) -> String {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "json" => "application/json",
        "md" | "markdown" => "text/markdown",
        "txt" | "log" => "text/plain",
        "rs" => "text/x-rust",
        "ts" | "tsx" => "text/typescript",
        "js" | "jsx" | "mjs" => "text/javascript",
        "py" => "text/x-python",
        "cs" => "text/x-csharp",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "xml" | "csproj" | "xaml" => "text/xml",
        "yaml" | "yml" => "text/yaml",
        "toml" => "text/toml",
        "sh" | "bash" => "text/x-sh",
        "csv" => "text/csv",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn is_image(mime: &str) -> bool {
    mime.starts_with("image/") && !mime.contains("svg")
}

#[derive(Serialize, Clone)]
pub struct Attachment {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub mime: String,
    pub size: u64,
    pub data: Option<String>,
    pub text: Option<String>,
    pub truncated: bool,
    pub staged_path: Option<String>,
    pub preview_url: Option<String>,
}

fn build(path: &Path) -> Result<Attachment, String> {
    let meta = fs::metadata(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    if meta.is_dir() {
        return Err("folders are not supported as attachments".into());
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    let mime = mime_for(&name);
    let size = meta.len();

    if is_image(&mime) && size as usize <= IMAGE_MAX {
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        return Ok(Attachment {
            name,
            path: path.to_string_lossy().into_owned(),
            kind: "image".into(),
            mime: mime.clone(),
            size,
            data: Some(b64.clone()),
            text: None,
            truncated: false,
            staged_path: None,
            preview_url: Some(format!("data:{mime};base64,{b64}")),
        });
    }

    if size as usize <= TEXT_MAX {
        if let Ok(bytes) = fs::read(path) {
            if let Ok(text) = String::from_utf8(bytes) {
                if !text.contains('\u{0}') {
                    return Ok(Attachment {
                        name,
                        path: path.to_string_lossy().into_owned(),
                        kind: "text".into(),
                        mime,
                        size,
                        data: None,
                        text: Some(text),
                        truncated: false,
                        staged_path: None,
                        preview_url: None,
                    });
                }
            }
        }
    }

    // Large file or binary: stage a copy the agent can open itself.
    let staged = stage(path)?;
    Ok(Attachment {
        name,
        path: path.to_string_lossy().into_owned(),
        kind: "binary".into(),
        mime,
        size,
        data: None,
        text: None,
        truncated: false,
        staged_path: Some(staged),
        preview_url: None,
    })
}

pub fn read(path: String) -> Result<Attachment, String> {
    build(Path::new(&path))
}

pub fn stage(path: &Path) -> Result<String, String> {
    let dir = cache_root().join(format!("{}", stamp()));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    let dest = dir.join(name);
    fs::copy(path, &dest).map_err(|e| format!("failed to copy attachment: {e}"))?;
    Ok(dest.to_string_lossy().into_owned())
}

pub fn save_b64(name: String, mime: String, data: String) -> Result<Attachment, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data.as_bytes())
        .map_err(|e| format!("invalid base64: {e}"))?;
    let dir = cache_root().join(format!("{}", stamp()));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ext = mime.rsplit('/').next().unwrap_or("png").split('+').next().unwrap_or("png");
    let safe_name = if name.trim().is_empty() {
        format!("paste-{}.{}", stamp(), ext)
    } else {
        name
    };
    let dest = dir.join(&safe_name);
    fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
    build(&dest)
}
