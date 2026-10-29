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
    fs::write(&tmp, pretty).map_err(|e| format!("falha ao escrever mcp.json: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("falha ao salvar mcp.json: {e}"))?;
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
        .map_err(|e| format!("falha ao rodar `pi mcp list`: {e}"))?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    let err = String::from_utf8_lossy(&output.stderr).to_string();
    if text.trim().is_empty() {
        text = err;
    }
    Ok(text)
}
