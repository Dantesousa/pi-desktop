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

// ── custom provider model sync ───────────────────────────────────────────────

/// Re-reads `/models` from every OpenAI-compatible provider in models.json and
/// rewrites its model list (catalogs rotate; this keeps the picker honest).
pub fn sync_custom_models() -> Result<String, String> {
    let path = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".pi/agent/models.json");
    let text = fs::read_to_string(&path).map_err(|e| format!("models.json: {e}"))?;
    let mut config: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("models.json inválido: {e}"))?;

    let auth_path = PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".pi/agent/auth.json");
    let auth: serde_json::Value = fs::read_to_string(&auth_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let Some(providers) = config.get_mut("providers").and_then(|v| v.as_object_mut()) else {
        return Err("models.json has no providers".into());
    };

    let mut updated = Vec::new();
    for (name, provider) in providers.iter_mut() {
        let api = provider.get("api").and_then(|v| v.as_str()).unwrap_or("");
        let base = provider
            .get("baseUrl")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim_end_matches('/')
            .to_string();
        if api != "openai-completions" || base.is_empty() {
            continue;
        }
        let key = provider
            .get("apiKey")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| {
                auth.get(name)
                    .and_then(|c| c.get("key"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            });
        let Some(key) = key else { continue };

        let body = fetch_with_key(&format!("{base}/models"), &key)?;
        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("{name}: resposta inesperada: {e}"))?;
        let entries = json
            .get("data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if entries.is_empty() {
            continue;
        }
        let mut models: Vec<serde_json::Value> = Vec::new();
        for m in entries {
            let Some(id) = m.get("id").and_then(|v| v.as_str()) else { continue };
            let locked = m.get("requires_training_optin").and_then(|v| v.as_bool()).unwrap_or(false)
                && !m.get("premium_unlocked").and_then(|v| v.as_bool()).unwrap_or(false);
            let base_name = m
                .get("display_name")
                .or_else(|| m.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or(id)
                .chars()
                .take(56)
                .collect::<String>();
            let mods: Vec<String> = m
                .get("architecture")
                .and_then(|a| a.get("input_modalities"))
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let mut e = serde_json::json!({
                "id": id,
                "name": if locked { format!("{base_name} (premium)") } else { base_name },
                "contextWindow": m.get("context_length").and_then(|v| v.as_u64()).unwrap_or(128000),
                "maxTokens": 16384,
                "input": if mods.iter().any(|x| x == "image") { serde_json::json!(["text","image"]) } else { serde_json::json!(["text"]) },
            });
            if m.get("supported_parameters")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().any(|p| ["reasoning","include_reasoning","thinking","reasoning_effort"].contains(&p.as_str().unwrap_or(""))))
                .unwrap_or(false)
            {
                e["reasoning"] = serde_json::json!(true);
            }
            models.push(e);
        }
        models.sort_by(|a, b| {
            let pa = a["name"].as_str().unwrap_or("").contains("(premium)");
            let pb = b["name"].as_str().unwrap_or("").contains("(premium)");
            pa.cmp(&pb)
        });
        provider["models"] = serde_json::Value::Array(models.clone());
        updated.push(format!("{name}: {}", models.len()));
    }

    if updated.is_empty() {
        return Err("no openai-compatible provider with an api key".into());
    }
    fs::write(&path, serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?)
        .map_err(|e| format!("failed to write models.json: {e}"))?;
    Ok(updated.join(", "))
}

fn fetch_with_key(url: &str, key: &str) -> Result<String, String> {
    let out = std::process::Command::new("curl")
        .args(["-sS", "-L", "--max-time", "25", "-H", "Accept: application/json"])
        .arg("-H")
        .arg(format!("Authorization: Bearer {key}"))
        .arg(url)
        .output()
        .map_err(|e| format!("curl failed: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

// ── local OpenAI-compatible servers ──────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct LocalServer {
    pub label: String,
    pub base_url: String,
    pub models: Vec<String>,
    pub running: bool,
    pub hint: Option<String>,
}

struct Probe {
    label: &'static str,
    slug: &'static str,
    url: &'static str,
    installed: &'static [&'static str],
}

const PROBES: &[Probe] = &[
    Probe { label: "LM Studio", slug: "lmstudio", url: "http://127.0.0.1:1234/v1", installed: &["~/.lmstudio"] },
    Probe { label: "Ollama", slug: "ollama", url: "http://127.0.0.1:11434/v1", installed: &["~/.ollama", "/usr/share/ollama"] },
    Probe { label: "llama.cpp", slug: "llamacpp", url: "http://127.0.0.1:8080/v1", installed: &[] },
    Probe { label: "vLLM", slug: "vllm", url: "http://127.0.0.1:8000/v1", installed: &[] },
    Probe { label: "text-generation-webui", slug: "textgen", url: "http://127.0.0.1:5000/v1", installed: &[] },
    Probe { label: "SGLang", slug: "sglang", url: "http://127.0.0.1:30000/v1", installed: &[] },
    Probe { label: "KoboldCpp", slug: "koboldcpp", url: "http://127.0.0.1:5001/v1", installed: &[] },
    Probe { label: "Jan", slug: "jan", url: "http://127.0.0.1:1337/v1", installed: &["~/.jan"] },
];

fn home_expand(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(rest)
    } else {
        PathBuf::from(p)
    }
}

fn probe_models(url: &str) -> Option<Vec<String>> {
    let out = std::process::Command::new("curl")
        .args(["-sS", "--max-time", "1.5", "-H", "Accept: application/json"])
        .arg(format!("{url}/models"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let ids: Vec<String> = json
        .get("data")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if ids.is_empty() {
        None
    } else {
        Some(ids)
    }
}

/// Looks for OpenAI-compatible servers on the usual local ports.
pub fn scan_local() -> Vec<LocalServer> {
    let mut out = Vec::new();
    for probe in PROBES {
        let models = probe_models(probe.url);
        let installed_dir = probe
            .installed
            .iter()
            .map(|p| home_expand(p))
            .find(|p| p.exists());
        match models {
            Some(models) => out.push(LocalServer {
                label: probe.label.to_string(),
                base_url: probe.url.to_string(),
                models,
                running: true,
                hint: None,
            }),
            None => {
                if let Some(dir) = installed_dir {
                    out.push(LocalServer {
                        label: probe.label.to_string(),
                        base_url: probe.url.to_string(),
                        models: vec![],
                        running: false,
                        hint: Some(format!(
                            "installed at {} — start its server and scan again",
                            dir.to_string_lossy()
                        )),
                    });
                }
            }
        }
    }
    out
}

/// Adds (or replaces) a provider in models.json for a detected local server.
pub fn add_local_provider(label: &str, base_url: &str, models: Vec<String>) -> Result<String, String> {
    if models.is_empty() {
        return Err("no models to add".into());
    }
    let slug: String = label
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let slug = if slug.is_empty() { "local".to_string() } else { slug };

    let path = PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".pi/agent/models.json");
    let mut config: serde_json::Value = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({ "providers": {} }));
    if config.get("providers").is_none() {
        config["providers"] = serde_json::json!({});
    }

    let entries: Vec<serde_json::Value> = models
        .iter()
        .map(|id| {
            serde_json::json!({
                "id": id,
                "name": id,
                "contextWindow": 16384,
                "maxTokens": 4096,
                "input": ["text"],
            })
        })
        .collect();

    config["providers"][&slug] = serde_json::json!({
        "name": label,
        "baseUrl": base_url,
        "api": "openai-completions",
        "apiKey": "local",
        "models": entries,
    });

    fs::write(&path, serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?)
        .map_err(|e| format!("failed to write models.json: {e}"))?;
    Ok(format!("{slug}: {} model(s)", models.len()))
}

#[cfg(test)]
mod local_tests {
    use super::*;

    #[test]
    fn probe_reads_model_ids_from_a_local_server() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let body = r#"{"object":"list","data":[{"id":"qwen-test"},{"id":"llama-test"}]}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });
        let models = probe_models(&format!("http://127.0.0.1:{port}/v1")).expect("should answer");
        assert_eq!(models, vec!["qwen-test".to_string(), "llama-test".to_string()]);
    }

    #[test]
    fn probe_returns_none_when_nothing_listens() {
        assert!(probe_models("http://127.0.0.1:9/v1").is_none());
    }
}

#[cfg(test)]
mod scan_local_tests {
    use super::*;

    #[test]
    #[ignore = "needs a fake OpenAI server on 127.0.0.1:1234"]
    fn scan_local_detects_the_lmstudio_port() {
        let found = scan_local();
        let hit = found.iter().find(|s| s.base_url.contains(":1234"));
        let hit = hit.expect("expected a server on 1234");
        assert!(hit.running);
        assert!(!hit.models.is_empty(), "models should be listed");
        println!("detected: {} → {:?}", hit.label, hit.models);
    }
}

#[cfg(test)]
mod add_local_tests {
    use super::*;

    #[test]
    fn add_local_provider_writes_models_json() {
        let tmp = std::env::temp_dir().join(format!("hina-home-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join(".pi/agent")).unwrap();
        let old = std::env::var("HOME").ok();
        std::env::set_var("HOME", &tmp);

        let out = add_local_provider(
            "LM Studio",
            "http://127.0.0.1:1234/v1",
            vec!["qwen-test".into(), "llama-test".into()],
        );
        let written = std::fs::read_to_string(tmp.join(".pi/agent/models.json")).unwrap();

        match old {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
        let _ = std::fs::remove_dir_all(&tmp);

        assert!(out.unwrap().contains("2 model(s)"));
        let json: serde_json::Value = serde_json::from_str(&written).unwrap();
        let provider = &json["providers"]["lm-studio"];
        assert_eq!(provider["api"], "openai-completions");
        assert_eq!(provider["baseUrl"], "http://127.0.0.1:1234/v1");
        assert_eq!(provider["models"][0]["id"], "qwen-test");
    }
}
