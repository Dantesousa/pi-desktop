//! MCP server configuration (~/.pi/agent/mcp.json).

use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default())
}

pub fn config_path() -> PathBuf {
    home().join(".pi").join("agent").join("mcp.json")
}

#[derive(Serialize, Clone)]
pub struct McpServer {
    pub name: String,
    pub kind: String,
    pub target: String,
    pub enabled: bool,
    pub exposure: Option<String>,
    pub description: Option<String>,
}

pub fn read_config() -> serde_json::Value {
    let path = config_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({ "mcpServers": {} }))
}

pub fn save_config(config: serde_json::Value) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if path.exists() {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backup = path.with_extension(format!("json.{ts}.bak"));
        fs::copy(&path, backup).ok();
    }
    let pretty = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, pretty).map_err(|e| format!("failed to write mcp.json: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("failed to save mcp.json: {e}"))?;
    Ok(())
}

pub fn list_servers() -> Vec<McpServer> {
    let config = read_config();
    let mut out = Vec::new();
    if let Some(map) = config.get("mcpServers").and_then(|v| v.as_object()) {
        for (name, value) in map {
            let command = value.get("command").and_then(|v| v.as_str());
            let url = value.get("url").and_then(|v| v.as_str());
            let kind = if command.is_some() {
                "stdio"
            } else if url.is_some() {
                "http"
            } else {
                "override"
            };
            let target = if let Some(cmd) = command {
                let args = value
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                if args.is_empty() {
                    cmd.to_string()
                } else {
                    format!("{cmd} {args}")
                }
            } else {
                url.unwrap_or("").to_string()
            };
            out.push(McpServer {
                name: name.clone(),
                kind: kind.to_string(),
                target,
                enabled: value
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                exposure: value
                    .get("exposure")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                description: value
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Runs `pi mcp list --json` in the given project directory.
pub fn check(cwd: Option<String>) -> Result<String, String> {
    let bin = crate::pi::resolve_bin();
    let mut cmd = Command::new(bin);
    cmd.arg("mcp").arg("list").arg("--json");
    if let Some(dir) = cwd.as_ref() {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            cmd.current_dir(p);
        }
    }
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run `pi mcp list`: {e}"))?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    let err = String::from_utf8_lossy(&output.stderr).to_string();
    if text.trim().is_empty() {
        text = err;
    }
    Ok(text)
}

// ── embedded browser (MCP server shipped with the app) ───────────────────────

pub const BROWSER_SERVER: &str = "hina-browser";

fn script_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("PI_DESKTOP_BROWSER_MCP") {
        out.push(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        // dev: <repo>/src-tauri/target/debug/pi-desktop → <repo>/scripts/browser-mcp.mjs
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        // target/debug/deps/… in tests, target/debug/… in the app: try a few levels
        for _ in 0..6 {
            if let Some(d) = dir.clone() {
                out.push(d.join("scripts/browser-mcp.mjs"));
                dir = d.parent().map(|p| p.to_path_buf());
            }
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    out.push(PathBuf::from(&home).join(".cache/pi-desktop/browser/mcp.mjs"));
    out
}

/// Copies the bundled browser MCP script into the cache dir so a packaged app
/// still has it next to the user's config.
fn write_bundled_script() -> Option<PathBuf> {
    let dir = browser_cache();
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("mcp.mjs");
    std::fs::write(&path, include_str!("../../scripts/browser-mcp.mjs")).ok()?;
    Some(path)
}

pub fn browser_script() -> Option<PathBuf> {
    if let Some(found) = script_candidates().into_iter().find(|p| p.is_file()) {
        return Some(found);
    }
    write_bundled_script()
}

#[derive(Serialize, Clone)]
pub struct BrowserPreview {
    pub enabled: bool,
    pub script: Option<String>,
    pub url: String,
    pub title: String,
    pub updated: u64,
    pub image_b64: Option<String>,
}

fn browser_cache() -> PathBuf {
    let base = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cache")
        });
    base.join("pi-desktop").join("browser")
}

pub fn browser_enabled() -> bool {
    read_config()
        .get("mcpServers")
        .and_then(|v| v.get(BROWSER_SERVER))
        .is_some()
}

pub fn browser_enable() -> Result<String, String> {
    let script = browser_script().ok_or_else(|| "scripts/browser-mcp.mjs not found".to_string())?;
    let mut config = read_config();
    if config.get("mcpServers").is_none() {
        config["mcpServers"] = serde_json::json!({});
    }
    let cwd = script
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_string_lossy().into_owned());
    let node_modules = cwd.as_ref().map(|c| format!("{c}/node_modules"));
    config["mcpServers"][BROWSER_SERVER] = serde_json::json!({
        "command": "node",
        "args": [script.to_string_lossy()],
        "cwd": cwd,
        "env": node_modules.map(|n| serde_json::json!({ "NODE_PATH": n })).unwrap_or_else(|| serde_json::json!({})),
        "description": "Agent-controlled browser; the app shows a live preview",
    });
    save_config(config)?;
    Ok(script.to_string_lossy().into_owned())
}

pub fn browser_disable() -> Result<(), String> {
    let mut config = read_config();
    if let Some(map) = config.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        map.remove(BROWSER_SERVER);
    }
    save_config(config)
}

#[cfg(test)]
mod browser_tests {
    use super::*;

    #[test]
    fn browser_script_is_found_from_the_dev_binary() {
        let found = browser_script();
        assert!(found.is_some(), "esperava achar scripts/browser-mcp.mjs");
        assert!(found.unwrap().ends_with("scripts/browser-mcp.mjs"));
    }

    #[test]
    fn preview_defaults_are_sane() {
        let p = browser_preview(false);
        assert!(p.updated == 0 || p.updated > 1_600_000_000_000);
    }
}

pub fn browser_preview(with_image: bool) -> BrowserPreview {
    let dir = browser_cache();
    let state_path = dir.join("state.json");
    let shot_path = dir.join("latest.jpg");
    let mut out = BrowserPreview {
        enabled: browser_enabled(),
        script: browser_script().map(|p| p.to_string_lossy().into_owned()),
        url: String::new(),
        title: String::new(),
        updated: 0,
        image_b64: None,
    };
    if let Ok(text) = fs::read_to_string(&state_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            out.url = json.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
            out.title = json.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
            out.updated = json.get("updated").and_then(|v| v.as_u64()).unwrap_or(0);
        }
    }
    if with_image {
        if let Ok(bytes) = fs::read(&shot_path) {
            use base64::Engine;
            out.image_b64 = Some(base64::engine::general_purpose::STANDARD.encode(bytes));
        }
    }
    out
}

#[derive(Serialize, Clone)]
pub struct McpServerStatus {
    pub name: String,
    pub enabled: bool,
    pub state: String,
    pub transport: String,
    pub tools: Vec<String>,
    pub error: Option<String>,
}

/// Runs `pi mcp list --json` and returns per-server state + tools.
pub fn status(cwd: Option<String>) -> Result<Vec<McpServerStatus>, String> {
    let raw = check(cwd)?;
    let json: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("unexpected response: {e}"))?;
    let mut out = Vec::new();
    for item in json
        .get("servers")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
    {
        out.push(McpServerStatus {
            name: item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            enabled: item.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
            state: item.get("state").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            transport: item
                .get("transport")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            tools: item
                .get("tools")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|t| {
                            t.get("name")
                                .and_then(|n| n.as_str())
                                .map(str::to_string)
                                .or_else(|| t.as_str().map(str::to_string))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            error: item
                .get("error")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string),
        });
    }
    Ok(out)
}
