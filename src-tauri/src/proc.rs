//! Background processes: long-running commands started from the app, with a
//! bounded output buffer, exit status and a notification hook so the agent can
//! react when one dies.

use serde::Serialize;
use std::collections::VecDeque;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, Runtime};

const MAX_OUTPUT: usize = 200 * 1024;

#[derive(Serialize, Clone)]
pub struct BgProc {
    pub id: String,
    pub command: String,
    pub cwd: String,
    pub status: String, // running | exited | failed
    pub exit_code: Option<i32>,
    pub started_ms: u64,
    pub ended_ms: Option<u64>,
    pub notify: bool,
    pub pid: Option<u32>,
}

struct Running {
    child: Child,
}

struct Entry {
    meta: BgProc,
    running: Option<Running>,
    /// Kept after the process exits so its tail can still be shown.
    output: Arc<Mutex<VecDeque<u8>>>,
}

#[derive(Default)]
pub struct ProcState {
    entries: Mutex<Vec<Entry>>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn push_output(buf: &Arc<Mutex<VecDeque<u8>>>, chunk: &[u8]) {
    let Ok(mut q) = buf.lock() else { return };
    for b in chunk {
        if q.len() >= MAX_OUTPUT {
            q.pop_front();
        }
        q.push_back(*b);
    }
}

fn reader_thread<R: Read + Send + 'static>(mut reader: R, buf: Arc<Mutex<VecDeque<u8>>>) {
    std::thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => push_output(&buf, &chunk[..n]),
                Err(_) => break,
            }
        }
    });
}

/// Spawns a shell command with merged, buffered output.
fn spawn(command: &str, dir: &str) -> Result<(Child, Arc<Mutex<VecDeque<u8>>>), String> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to start: {e}"))?;

    let output: Arc<Mutex<VecDeque<u8>>> = Arc::new(Mutex::new(VecDeque::new()));
    if let Some(out) = child.stdout.take() {
        reader_thread(out, output.clone());
    }
    if let Some(err) = child.stderr.take() {
        reader_thread(err, output.clone());
    }
    Ok((child, output))
}

pub fn start<R: Runtime>(
    app: &AppHandle<R>,
    state: &ProcState,
    command: &str,
    cwd: Option<String>,
    notify: bool,
) -> Result<BgProc, String> {
    let dir = cwd.unwrap_or_else(|| {
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
    });

    let (child, output) = spawn(command, &dir)?;

    let id = format!("p{}", now_ms());
    let meta = BgProc {
        id: id.clone(),
        command: command.to_string(),
        cwd: dir,
        status: "running".into(),
        exit_code: None,
        started_ms: now_ms(),
        ended_ms: None,
        notify,
        pid: Some(child.id()),
    };

    state.entries.lock().unwrap().push(Entry {
        meta: meta.clone(),
        running: Some(Running { child }),
        output: output.clone(),
    });

    // supervisor: reap and report
    let app2 = app.clone();
    let id2 = id.clone();
    let command2 = command.to_string();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(400));
        let mut outcome: Option<(Option<i32>, String)> = None;
        let mut notify_on_exit = false;
        {
            let st = app2.state::<ProcState>();
            let mut guard = st.entries.lock().unwrap();
            if let Some(entry) = guard.iter_mut().find(|e| e.meta.id == id2) {
                if entry.running.is_none() {
                    break; // stopped or already reaped elsewhere
                }
                if let Some(run) = entry.running.as_mut() {
                    match run.child.try_wait() {
                        Ok(Some(status)) => {
                            let code = status.code();
                            entry.meta.status = if code == Some(0) { "exited" } else { "failed" }.into();
                            entry.meta.exit_code = code;
                            entry.meta.ended_ms = Some(now_ms());
                            entry.running = None;
                            outcome = Some((code, entry.meta.command.clone()));
                            notify_on_exit = entry.meta.notify;
                        }
                        Ok(None) => {}
                        Err(_) => {
                            entry.meta.status = "failed".into();
                            entry.meta.ended_ms = Some(now_ms());
                            entry.running = None;
                            outcome = Some((None, entry.meta.command.clone()));
                        }
                    }
                }
            } else {
                break;
            }
        }
        if let Some((code, cmd)) = outcome {
            let tail = output_tail(&app2, &id2, 30);
            let _ = app2.emit(
                "proc:exited",
                serde_json::json!({
                    "id": id2,
                    "command": if cmd.is_empty() { command2.clone() } else { cmd },
                    "code": code,
                    "tail": tail,
                    "notify": notify_on_exit,
                }),
            );
            break;
        }
    });

    Ok(meta)
}

pub fn list(state: &ProcState) -> Vec<BgProc> {
    let mut guard = state.entries.lock().unwrap();
    // refresh status of anything still running
    for entry in guard.iter_mut() {
        if let Some(run) = entry.running.as_mut() {
            if let Ok(Some(status)) = run.child.try_wait() {
                let code = status.code();
                entry.meta.status = if code == Some(0) { "exited" } else { "failed" }.into();
                entry.meta.exit_code = code;
                entry.meta.ended_ms = Some(now_ms());
                entry.running = None;
            }
        }
    }
    guard.iter().map(|e| e.meta.clone()).collect()
}

pub fn output_tail<R: Runtime>(app: &AppHandle<R>, id: &str, lines: usize) -> String {
    let st = app.state::<ProcState>();
    let guard = st.entries.lock().unwrap();
    let Some(entry) = guard.iter().find(|e| e.meta.id == id) else {
        return String::new();
    };
    let Ok(q) = entry.output.lock() else {
        return String::new();
    };
    let bytes: Vec<u8> = q.iter().copied().collect();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

pub fn stop(state: &ProcState, id: &str) -> Result<(), String> {
    let mut guard = state.entries.lock().unwrap();
    let Some(entry) = guard.iter_mut().find(|e| e.meta.id == id) else {
        return Err("process not found".into());
    };
    if let Some(mut run) = entry.running.take() {
        let _ = run.child.kill();
        let _ = run.child.wait();
        entry.meta.status = "exited".into();
        entry.meta.ended_ms = Some(now_ms());
    }
    Ok(())
}

pub fn remove(state: &ProcState, id: &str) -> Result<(), String> {
    let mut guard = state.entries.lock().unwrap();
    if let Some(pos) = guard.iter().position(|e| e.meta.id == id) {
        if let Some(mut run) = guard[pos].running.take() {
            let _ = run.child.kill();
            let _ = run.child.wait();
        }
        guard.remove(pos);
    }
    Ok(())
}

pub fn set_notify(state: &ProcState, id: &str, notify: bool) {
    let mut guard = state.entries.lock().unwrap();
    if let Some(entry) = guard.iter_mut().find(|e| e.meta.id == id) {
        entry.meta.notify = notify;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_stdout_and_stderr_of_a_real_command() {
        let (mut child, output) = spawn("echo hello; echo oops 1>&2; sleep 0.05", "/tmp").unwrap();
        child.wait().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(150));
        let bytes: Vec<u8> = output.lock().unwrap().iter().copied().collect();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("hello"), "stdout missing: {text}");
        assert!(text.contains("oops"), "stderr missing: {text}");
    }

    #[test]
    fn tracks_a_long_running_process_and_stops_it() {
        let start = std::time::Instant::now();
        let (mut child, output) = spawn("echo booting; sleep 5", "/tmp").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(child.try_wait().unwrap().is_none(), "should still be running");

        let text: String = {
            let q = output.lock().unwrap();
            String::from_utf8_lossy(&q.iter().copied().collect::<Vec<u8>>()).into_owned()
        };
        assert!(text.contains("booting"), "output not captured: {text}");

        let _ = child.kill();
        let status = child.wait().unwrap();
        assert!(!status.success(), "killed process should not be successful");
        assert!(start.elapsed() < std::time::Duration::from_secs(4), "stop was not prompt");
    }

    #[test]
    fn full_lifecycle_with_a_real_app_handle() {
        use tauri::Manager;

        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        app.manage(ProcState::default());
        let handle = app.handle().clone();
        let state = app.state::<ProcState>();

        let started = start(&handle, &state, "echo booting; sleep 0.4; echo done", Some("/tmp".into()), true)
            .expect("start");
        assert_eq!(started.status, "running");

        // output shows up while it runs
        std::thread::sleep(std::time::Duration::from_millis(250));
        assert!(output_tail(&handle, &started.id, 10).contains("booting"));

        // supervisor flips it to exited with the code
        std::thread::sleep(std::time::Duration::from_millis(900));
        let listing = list(&state);
        let entry = listing.iter().find(|p| p.id == started.id).expect("entry");
        assert_eq!(entry.status, "exited", "status={:?}", entry.status);
        assert_eq!(entry.exit_code, Some(0));
        assert!(entry.ended_ms.is_some());

        // output is kept after exit
        let tail = output_tail(&handle, &started.id, 10);
        assert!(tail.contains("done"), "tail after exit: {tail}");

        // and it can be removed
        remove(&state, &started.id).unwrap();
        assert!(list(&state).is_empty());
    }

    #[test]
    fn output_buffer_is_bounded() {
        let buf: Arc<Mutex<VecDeque<u8>>> = Arc::new(Mutex::new(VecDeque::new()));
        let chunk = vec![b'x'; 1024];
        for _ in 0..300 {
            push_output(&buf, &chunk);
        }
        assert_eq!(buf.lock().unwrap().len(), MAX_OUTPUT);
    }
}

// ── auto-detected processes (started outside the app) ────────────────────────

#[derive(Serialize, Clone)]
pub struct ExternalProc {
    pub pid: u32,
    pub command: String,
    pub cwd: String,
    pub started_ms: u64,
    pub rss_kb: u64,
}

fn read_cmdline(pid: u32) -> String {
    std::fs::read(format!("/proc/{pid}/cmdline"))
        .map(|b| {
            b.split(|c| *c == 0)
                .filter(|s| !s.is_empty())
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn read_cwd(pid: u32) -> Option<String> {
    std::fs::read_link(format!("/proc/{pid}/cwd"))
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

/// start time (ms since epoch) from /proc/<pid>/stat field 22 (clock ticks since boot)
fn started_ms(pid: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after = stat.rsplit_once(')')?.1;
    let ticks: u64 = after.split_whitespace().nth(19)?.parse().ok()?;
    let btime = std::fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find(|l| l.starts_with("btime"))?
        .split_whitespace()
        .nth(1)?
        .parse::<u64>()
        .ok()?;
    let hz = 100u64; // USER_HZ on Linux
    Some((btime * 1000) + (ticks * 1000 / hz))
}

fn has_session_marker(pid: u32) -> bool {
    std::fs::read(format!("/proc/{pid}/environ"))
        .map(|b| {
            b.split(|c| *c == 0)
                .any(|kv| kv.starts_with(b"HINA_SESSION="))
        })
        .unwrap_or(false)
}

fn rss_kb(pid: u32) -> u64 {
    std::fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse().ok()))
        })
        .unwrap_or(0)
}

fn descendants_of(root: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return out;
    };
    for entry in entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        let Some(after) = stat.rsplit_once(')').map(|(_, a)| a) else {
            continue;
        };
        if let Some(ppid) = after.split_whitespace().nth(1).and_then(|v| v.parse::<u32>().ok()) {
            if ppid == root {
                out.push(pid);
            }
        }
    }
    out
}

/// Long-running processes living in `cwd` that the app did not start itself.
/// Lets the user watch watchers/dev-servers the agent (or a terminal) left running.
/// Processes started by an agent (they inherit `HINA_SESSION` from the sidecar).
pub fn scan(cwd: Option<String>, min_age_ms: u64) -> Vec<ExternalProc> {
    let me = std::process::id();
    let roots: Vec<u32> = {
        let mut v = vec![me];
        v.extend(descendants_of(me));
        v
    };
    let now = now_ms();
    let mut out: Vec<ExternalProc> = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return out;
    };
    for entry in entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        // never report the app or its own children (those are `proc_*` entries)
        if roots.contains(&pid) {
            continue;
        }
        let command = read_cmdline(pid);
        if command.is_empty() {
            continue;
        }
        // only processes started by one of our agents (env marker), never the
        // user's own shells or unrelated daemons that happen to share the cwd
        if !has_session_marker(pid) {
            continue;
        }
        // MCP helpers inherit the marker too; they are infrastructure, not servers
        let lower = command.to_lowercase();
        if lower.contains("mcp") || lower.contains("browser-mcp") || lower.contains("playwright") {
            continue;
        }
        // skip short-lived helpers
        let started = started_ms(pid).unwrap_or(now);
        if now.saturating_sub(started) < min_age_ms {
            continue;
        }
        let proc_cwd = read_cwd(pid).unwrap_or_default();
        let _ = &cwd; // kept for API compatibility; the env marker is authoritative
        out.push(ExternalProc {
            pid,
            command: command.chars().take(160).collect(),
            cwd: proc_cwd,
            started_ms: started,
            rss_kb: rss_kb(pid),
        });
    }
    out.sort_by(|a, b| b.started_ms.cmp(&a.started_ms));
    out.truncate(12);
    out
}

pub fn kill_external(pid: u32) -> Result<(), String> {
    // only plain SIGTERM via kill(2) through the shell, no libc dependency
    let status = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .map_err(|e| format!("failed to signal: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("kill failed (process gone?)".into())
    }
}

#[cfg(test)]
mod scan_tests {
    use super::*;

    #[test]
    fn scan_skips_our_own_children() {
        // children of the app are managed by proc_*, never reported as external
        let mut child = std::process::Command::new("sh")
            .args(["-c", "sleep 2"])
            .current_dir("/tmp")
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        let found = scan(Some("/tmp".into()), 0);
        assert!(!found.iter().any(|p| p.pid == child.id()));
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn scan_finds_an_agent_started_detached_process() {
        // setsid detaches (like `nohup … &`), the env marker is what identifies it
        let _ = std::process::Command::new("sh")
            .args(["-c", "cd /tmp && HINA_SESSION=test setsid sleep 7 >/dev/null 2>&1 &"])
            .status();
        std::thread::sleep(std::time::Duration::from_millis(600));
        let found = scan(Some("/tmp".into()), 0);
        assert!(
            found.iter().any(|p| p.command.contains("sleep 7")),
            "expected the marked sleep in {:?}",
            found.iter().map(|p| p.command.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn scan_ignores_processes_without_the_session_marker() {
        // a plain process with the marker removed: must never show up
        let mut child = std::process::Command::new("sleep")
            .arg("8")
            .current_dir("/tmp")
            .env_remove("HINA_SESSION")
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let found = scan(Some("/tmp".into()), 0);
        assert!(
            !found.iter().any(|p| p.pid == child.id()),
            "unmarked process {} should not be listed",
            child.id()
        );
        let _ = child.kill();
        let _ = child.wait();
    }
}
