//! Spawns and talks to `pi --mode rpc` over JSONL stdin/stdout.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub struct PiState {
    pub procs: Mutex<std::collections::HashMap<String, Running>>,
}

impl Default for PiState {
    fn default() -> Self {
        Self {
            procs: Mutex::new(std::collections::HashMap::new()),
        }
    }
}

pub struct Running {
    pub child: Child,
    pub stdin: Arc<Mutex<ChildStdin>>,
    pub pid: u32,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StartOptions {
    pub cwd: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub thinking: Option<String>,
    pub session: Option<String>,
    pub no_session: Option<bool>,
    pub extra_args: Option<Vec<String>>,
}

#[derive(Serialize, Clone)]
pub struct PiStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub bin: String,
}

fn cache_dir() -> PathBuf {
    let base = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cache")
        });
    base.join("pi-desktop")
}

/// Writes the bundled permission-gate extension to the cache and returns its path.
pub fn permission_extension() -> Option<String> {
    let mode = std::env::var("PI_DESKTOP_GATE").unwrap_or_else(|_| "dangerous".into());
    if mode.eq_ignore_ascii_case("off") {
        return None;
    }
    let dir = cache_dir().join("ext").join("v1");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("permissions.ts");
    std::fs::write(&path, include_str!("../extensions/permissions.ts")).ok()?;
    Some(path.to_string_lossy().into_owned())
}

pub fn resolve_bin() -> String {
    if let Ok(p) = std::env::var("PI_BIN") {
        if Path::new(&p).is_file() {
            return p;
        }
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let candidate = Path::new(dir).join("pi");
            if candidate.is_file() {
                return candidate.to_string_lossy().into_owned();
            }
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{home}/.pi/agent/bin/pi"),
        "/usr/local/bin/pi".to_string(),
        "/usr/bin/pi".to_string(),
    ];
    for c in candidates {
        if Path::new(&c).is_file() {
            return c;
        }
    }
    "pi".to_string()
}

fn emit(app: &AppHandle, event: &str, payload: serde_json::Value) {
    let _ = app.emit(event, payload);
}

fn emit_tab(app: &AppHandle, tab: &str, event: &str, payload: serde_json::Value) {
    let _ = app.emit(event, serde_json::json!({ "tab": tab, "payload": payload }));
}

pub fn start(app: &AppHandle, state: &PiState, tab: &str, opts: StartOptions) -> Result<PiStatus, String> {
    stop(state, tab)?;

    let bin = resolve_bin();
    let mut cmd = Command::new(&bin);
    cmd.arg("--mode").arg("rpc");
    if let (Some(provider), Some(model)) = (opts.provider.as_ref(), opts.model.as_ref()) {
        if !provider.is_empty() && !model.is_empty() {
            cmd.arg("--provider").arg(provider);
            cmd.arg("--model").arg(model);
        }
    }
    if let Some(thinking) = opts.thinking.as_ref() {
        if !thinking.is_empty() {
            cmd.arg("--thinking").arg(thinking);
        }
    }
    if let Some(session) = opts.session.as_ref() {
        if !session.is_empty() {
            cmd.arg("--session").arg(session);
        }
    }
    if opts.no_session.unwrap_or(false) {
        cmd.arg("--no-session");
    }
    if let Some(extra) = opts.extra_args.as_ref() {
        cmd.args(extra);
    }
    if let Some(ext) = permission_extension() {
        cmd.arg("--extension").arg(ext);
    }
    if let Some(cwd) = opts.cwd.as_ref() {
        let p = PathBuf::from(cwd);
        if p.is_dir() {
            cmd.current_dir(p);
        }
    }
    cmd.env("FORCE_COLOR", "0");
    cmd.env("NO_COLOR", "1");
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("falha ao iniciar `{bin}`: {e}"))?;

    let pid = child.id();
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "sem stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "sem stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "sem stderr".to_string())?;

    // ── stdout: strict LF framing, never readline ────────────────────────
    let app_out = app.clone();
    let tab_out = tab.to_string();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut buf: Vec<u8> = Vec::with_capacity(8192);
        loop {
            buf.clear();
            match reader.read_until(b'\n', &mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    let mut end = buf.len();
                    while end > 0 && (buf[end - 1] == b'\n' || buf[end - 1] == b'\r') {
                        end -= 1;
                    }
                    if end == 0 {
                        continue;
                    }
                    match serde_json::from_slice::<serde_json::Value>(&buf[..end]) {
                        Ok(v) => emit_tab(&app_out, &tab_out, "pi:record", v),
                        Err(_) => {
                            let text = String::from_utf8_lossy(&buf[..end]).to_string();
                            emit_tab(
                                &app_out,
                                &tab_out,
                                "pi:stderr",
                                serde_json::json!({ "line": text, "kind": "non-json" }),
                            );
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // ── stderr: diagnostics only, never protocol ─────────────────────────
    let app_err = app.clone();
    let tab_err = tab.to_string();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut buf: Vec<u8> = Vec::with_capacity(4096);
        loop {
            buf.clear();
            match reader.read_until(b'\n', &mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    let text = String::from_utf8_lossy(&buf).trim_end().to_string();
                    if !text.is_empty() {
                        emit_tab(
                            &app_err,
                            &tab_err,
                            "pi:stderr",
                            serde_json::json!({ "line": text, "kind": "stderr" }),
                        );
                    }
                }
                Err(_) => break,
            }
        }
    });

    let stdin = Arc::new(Mutex::new(stdin));
    state
        .procs
        .lock()
        .unwrap()
        .insert(tab.to_string(), Running { child, stdin, pid });

    // ── supervisor: reap the child, report exit ──────────────────────────
    let app_mon = app.clone();
    let tab_mon = tab.to_string();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(250));
        let mut exited: Option<Option<i32>> = None;
        {
            let st = app_mon.state::<PiState>();
            let mut guard = st.procs.lock().unwrap();
            if let Some(running) = guard.get_mut(&tab_mon) {
                match running.child.try_wait() {
                    Ok(Some(status)) => {
                        exited = Some(status.code());
                        guard.remove(&tab_mon);
                    }
                    Ok(None) => {}
                    Err(_) => {
                        exited = Some(None);
                        guard.remove(&tab_mon);
                    }
                }
            } else {
                break;
            }
        }
        match exited {
            Some(code) => {
                emit_tab(&app_mon, &tab_mon, "pi:exit", serde_json::json!({ "code": code }));
                break;
            }
            None => continue,
        }
    });

    emit_tab(app, tab, "pi:spawned", serde_json::json!({ "pid": pid, "bin": bin }));
    Ok(PiStatus {
        running: true,
        pid: Some(pid),
        bin,
    })
}

pub fn send(state: &PiState, tab: &str, command: serde_json::Value) -> Result<(), String> {
    let guard = state.procs.lock().unwrap();
    let running = guard
        .get(tab)
        .ok_or_else(|| "pi não está rodando".to_string())?;
    let line = serde_json::to_string(&command).map_err(|e| e.to_string())?;
    let mut stdin = running.stdin.lock().unwrap();
    stdin
        .write_all(line.as_bytes())
        .and_then(|_| stdin.write_all(b"\n"))
        .and_then(|_| stdin.flush())
        .map_err(|e| format!("falha ao escrever no pi: {e}"))
}

pub fn stop(state: &PiState, tab: &str) -> Result<(), String> {
    let mut guard = state.procs.lock().unwrap();
    if let Some(mut running) = guard.remove(tab) {
        if let Ok(mut stdin) = running.stdin.lock() {
            let _ = stdin.flush();
        }
        let _ = running.child.kill();
        let _ = running.child.wait();
    }
    Ok(())
}

pub fn stop_all(state: &PiState) -> Result<(), String> {
    let mut guard = state.procs.lock().unwrap();
    let keys: Vec<String> = guard.keys().cloned().collect();
    for key in keys {
        if let Some(mut running) = guard.remove(&key) {
            let _ = running.child.kill();
            let _ = running.child.wait();
        }
    }
    Ok(())
}

pub fn status(state: &PiState, tab: &str) -> PiStatus {
    let guard = state.procs.lock().unwrap();
    match guard.get(tab) {
        Some(r) => PiStatus {
            running: true,
            pid: Some(r.pid),
            bin: resolve_bin(),
        },
        None => PiStatus {
            running: false,
            pid: None,
            bin: resolve_bin(),
        },
    }
}
