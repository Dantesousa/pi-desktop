//! Reads and writes the pi-hermes-memory store that already lives under
//! `~/.pi/agent/pi-hermes-memory`.
//!
//! Markdown is authoritative (SQLite is only the FTS mirror), so this module
//! edits the `.md` files and lets `/memory-sync-markdown` reconcile the index.

use rusqlite::Connection;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DELIM: &str = "\n§\n";

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/home/dante".into()))
}

pub fn agent_root() -> PathBuf {
    home().join(".pi").join("agent")
}

pub fn memory_dir() -> PathBuf {
    agent_root().join("pi-hermes-memory")
}

fn db_path() -> PathBuf {
    memory_dir().join("sessions.db")
}

fn file_for(target: &str) -> Result<PathBuf, String> {
    let dir = memory_dir();
    match target {
        "memory" => Ok(dir.join("MEMORY.md")),
        "user" => Ok(dir.join("USER.md")),
        "failure" | "failures" => Ok(dir.join("failures.md")),
        other => Err(format!("target desconhecido: {other}")),
    }
}

#[derive(Serialize, Clone)]
pub struct MemoryEntry {
    pub index: usize,
    pub raw: String,
    pub category: Option<String>,
    pub body: String,
    pub created: Option<String>,
    pub last: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct MemoryFile {
    pub target: String,
    pub path: String,
    pub entries: Vec<MemoryEntry>,
    pub chars: usize,
    pub limit: usize,
}

#[derive(Serialize, Clone, Default)]
pub struct MemoryStats {
    pub memory_chars: usize,
    pub memory_limit: usize,
    pub user_chars: usize,
    pub user_limit: usize,
    pub failure_chars: usize,
    pub failure_limit: usize,
    pub db_entries: usize,
    pub db_sessions: usize,
    pub db_messages: usize,
    pub db_bytes: u64,
}

fn extract_meta(block: &str) -> (Option<String>, Option<String>) {
    let mut created = None;
    let mut last = None;
    for part in block.split(|c| c == ',' || c == ' ').map(str::trim) {
        if let Some(v) = part.strip_prefix("created=") {
            created = Some(v.trim_end_matches("->").to_string());
        } else if let Some(v) = part.strip_prefix("last=") {
            last = Some(v.trim_end_matches("->").to_string());
        }
    }
    (created, last)
}

fn parse_entry(index: usize, raw: &str) -> MemoryEntry {
    let raw = raw.trim().to_string();
    let mut text = raw.clone();
    let mut created = None;
    let mut last = None;

    if let Some(start) = text.rfind("<!--") {
        if text.trim_end().ends_with("-->") {
            let block = text[start..].trim();
            let (c, l) = extract_meta(block);
            created = c;
            last = l;
            text = text[..start].trim_end().to_string();
        }
    }

    let mut category = None;
    let mut body = text.clone();
    if let Some(rest) = text.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            category = Some(rest[..end].to_string());
            body = rest[end + 1..].trim_start().to_string();
        }
    }

    let mut failure_reason = None;
    if category.is_some() {
        for segment in text.split(" — ").skip(1) {
            if let Some(r) = segment.strip_prefix("Failed: ") {
                failure_reason = Some(r.trim().to_string());
                break;
            }
        }
    }

    MemoryEntry {
        index,
        raw,
        category,
        body,
        created,
        last,
        failure_reason,
    }
}

fn limit_for(target: &str, config: &serde_json::Value) -> usize {
    let memory_limit = config
        .get("memoryCharLimit")
        .and_then(|v| v.as_u64())
        .unwrap_or(9000) as usize;
    let user_limit = config
        .get("userCharLimit")
        .and_then(|v| v.as_u64())
        .unwrap_or(5000) as usize;
    match target {
        "memory" => memory_limit,
        "user" => user_limit,
        _ => memory_limit * 2,
    }
}

fn read_config() -> serde_json::Value {
    let path = agent_root().join("hermes-memory-config.json");
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

pub fn read_target(target: &str) -> Result<MemoryFile, String> {
    let path = file_for(target)?;
    let content = fs::read_to_string(&path).unwrap_or_default();
    let config = read_config();
    let entries: Vec<MemoryEntry> = content
        .split(DELIM)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .enumerate()
        .map(|(i, raw)| parse_entry(i, raw))
        .collect();
    Ok(MemoryFile {
        target: target.to_string(),
        path: path.to_string_lossy().into_owned(),
        chars: content.chars().count(),
        limit: limit_for(target, &config),
        entries,
    })
}

fn backup(path: &Path) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dir = memory_dir().join(".backups");
    fs::create_dir_all(&dir).ok()?;
    let name = path.file_name()?.to_string_lossy().into_owned();
    let dest = dir.join(format!("{name}.{ts}.bak"));
    fs::copy(path, &dest).ok()?;
    Some(dest)
}

pub fn write_target(target: &str, entries: Vec<String>) -> Result<MemoryFile, String> {
    let path = file_for(target)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    backup(&path);
    let cleaned: Vec<String> = entries
        .into_iter()
        .map(|e| e.trim().to_string())
        .filter(|e| !e.is_empty())
        .collect();
    let mut content = cleaned.join(DELIM);
    if !content.is_empty() {
        content.push_str("\n§\n");
    }
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, &content).map_err(|e| format!("falha ao escrever: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("falha ao salvar: {e}"))?;
    read_target(target)
}

fn open_db() -> Result<Connection, String> {
    let path = db_path();
    if !path.exists() {
        return Err(format!("banco não encontrado em {}", path.display()));
    }
    Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("falha ao abrir banco: {e}"))
}

fn count(conn: &Connection, table: &str) -> usize {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| {
        r.get::<_, i64>(0)
    })
    .unwrap_or(0) as usize
}

pub fn stats() -> MemoryStats {
    let config = read_config();
    let read_len = |t: &str| -> usize {
        file_for(t)
            .ok()
            .and_then(|p| fs::read_to_string(p).ok())
            .map(|s| s.chars().count())
            .unwrap_or(0)
    };
    let mut out = MemoryStats {
        memory_chars: read_len("memory"),
        memory_limit: limit_for("memory", &config),
        user_chars: read_len("user"),
        user_limit: limit_for("user", &config),
        failure_chars: read_len("failure"),
        failure_limit: limit_for("failure", &config),
        ..Default::default()
    };
    out.db_bytes = fs::metadata(db_path()).map(|m| m.len()).unwrap_or(0);
    if let Ok(conn) = open_db() {
        out.db_entries = count(&conn, "memories");
        out.db_sessions = count(&conn, "sessions");
        out.db_messages = count(&conn, "messages");
    }
    out
}

#[derive(Serialize, Clone)]
pub struct DbMemory {
    pub id: i64,
    pub target: String,
    pub project: Option<String>,
    pub category: Option<String>,
    pub content: String,
    pub failure_reason: Option<String>,
    pub created: String,
    pub last_referenced: String,
    pub hit_count: i64,
}

pub fn query_memories(target: Option<String>, query: Option<String>, limit: u32) -> Result<Vec<DbMemory>, String> {
    let conn = open_db()?;
    let mut sql = String::from(
        "SELECT id, target, project, category, content, failure_reason, created, last_referenced, hit_count FROM memories WHERE 1=1",
    );
    let mut args: Vec<rusqlite::types::Value> = Vec::new();
    if let Some(t) = target.as_ref().filter(|t| !t.is_empty()) {
        sql.push_str(" AND target = ?");
        args.push(rusqlite::types::Value::Text(t.clone()));
    }
    if let Some(q) = query.as_ref().filter(|q| !q.trim().is_empty()) {
        sql.push_str(" AND (content LIKE ? OR COALESCE(failure_reason,'') LIKE ?)");
        let like = format!("%{}%", q.trim());
        args.push(rusqlite::types::Value::Text(like.clone()));
        args.push(rusqlite::types::Value::Text(like));
    }
    sql.push_str(" ORDER BY id DESC LIMIT ?");
    args.push(rusqlite::types::Value::Integer(limit as i64));

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(args), |row| {
            Ok(DbMemory {
                id: row.get(0)?,
                target: row.get(1)?,
                project: row.get(2)?,
                category: row.get(3)?,
                content: row.get(4)?,
                failure_reason: row.get(5)?,
                created: row.get(6)?,
                last_referenced: row.get(7)?,
                hit_count: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[derive(Serialize, Clone)]
pub struct SessionHit {
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub project: String,
    pub session_id: String,
}

fn fts_query(raw: &str) -> Option<String> {
    let tokens: Vec<String> = raw
        .split_whitespace()
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '§')
                .collect::<String>()
        })
        .filter(|t| t.chars().count() > 1)
        .map(|t| format!("\"{t}\""))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" AND "))
    }
}

pub fn search_sessions(query: String, limit: u32) -> Result<Vec<SessionHit>, String> {
    let Some(fts) = fts_query(&query) else {
        return Ok(vec![]);
    };
    let conn = open_db()?;
    let mut stmt = conn
        .prepare(
            "SELECT m.role, m.content, m.timestamp, COALESCE(s.project,''), m.session_id
             FROM messages_fts f
             JOIN messages m ON m.rowid = f.rowid
             JOIN sessions s ON s.id = m.session_id
             WHERE messages_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![fts, limit as i64], |row| {
            Ok(SessionHit {
                role: row.get(0)?,
                content: row.get(1)?,
                timestamp: row.get(2)?,
                project: row.get(3)?,
                session_id: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[derive(Serialize, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub path: String,
    pub cwd: String,
    pub name: Option<String>,
    pub started_at: String,
    pub modified_ms: u64,
    pub size: u64,
    pub messages: usize,
}

fn sessions_root() -> PathBuf {
    agent_root().join("sessions")
}

fn read_session_meta(path: &Path) -> Option<SessionInfo> {
    let meta = fs::metadata(path).ok()?;
    let content = fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let header: serde_json::Value = serde_json::from_str(lines.next()?).ok()?;
    let mut name: Option<String> = None;
    let mut messages = 0usize;
    for line in content.lines() {
        if line.contains("\"role\"") {
            messages += 1;
        }
        if name.is_none() && line.contains("session_info") {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(n) = v.get("name").and_then(|n| n.as_str()) {
                    name = Some(n.to_string());
                }
            }
        }
    }
    let modified_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    Some(SessionInfo {
        id: header
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        path: path.to_string_lossy().into_owned(),
        cwd: header
            .get("cwd")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        name,
        started_at: header
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        modified_ms,
        size: meta.len(),
        messages,
    })
}

pub fn list_sessions(limit: u32) -> Vec<SessionInfo> {
    let mut out: Vec<SessionInfo> = Vec::new();
    let root = sessions_root();
    let Ok(dirs) = fs::read_dir(&root) else {
        return out;
    };
    for dir in dirs.flatten() {
        let Ok(entries) = fs::read_dir(dir.path()) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if let Some(info) = read_session_meta(&path) {
                out.push(info);
            }
        }
    }
    out.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms));
    out.truncate(limit.max(1) as usize);
    out
}

#[derive(Serialize, Clone)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub path: String,
    pub scope: String,
}

pub fn list_skills() -> Vec<SkillInfo> {
    let mut roots: Vec<(PathBuf, String)> = vec![
        (memory_dir().join("skills"), "global".into()),
        (agent_root().join("skills"), "pi".into()),
    ];
    let projects = agent_root().join("projects-memory");
    if let Ok(dirs) = fs::read_dir(&projects) {
        for d in dirs.flatten() {
            roots.push((d.path().join("skills"), "project".into()));
        }
    }

    let mut out = Vec::new();
    for (root, scope) in roots {
        let Ok(dirs) = fs::read_dir(&root) else {
            continue;
        };
        for entry in dirs.flatten() {
            let skill_md = entry.path().join("SKILL.md");
            if !skill_md.is_file() {
                continue;
            }
            let Ok(text) = fs::read_to_string(&skill_md) else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            let description = text
                .lines()
                .find(|l| l.trim_start().starts_with("description:"))
                .map(|l| {
                    l.trim_start()
                        .trim_start_matches("description:")
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'')
                        .to_string()
                })
                .unwrap_or_default();
            out.push(SkillInfo {
                name,
                description,
                path: skill_md.to_string_lossy().into_owned(),
                scope: scope.clone(),
            });
        }
    }
    out
}

fn skill_root_for_create() -> PathBuf {
    let dir = memory_dir().join("skills");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn slugify(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .replace("--", "-")
}

pub fn create_skill(name: &str, content: &str) -> Result<String, String> {
    let slug = slugify(name);
    if slug.is_empty() {
        return Err("nome inválido".into());
    }
    let dir = skill_root_for_create().join(&slug);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("SKILL.md");
    fs::write(&path, content).map_err(|e| format!("falha ao escrever skill: {e}"))?;
    Ok(path.to_string_lossy().into_owned())
}

pub fn write_skill(path: &str, content: &str) -> Result<(), String> {
    let p = PathBuf::from(path);
    let skills_root = memory_dir().join("skills");
    if !p.starts_with(&skills_root) && !p.starts_with(agent_root().join("projects-memory")) {
        return Err("só é permitido editar skills dentro do diretório de memória".into());
    }
    if p.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
        return Err("o arquivo precisa se chamar SKILL.md".into());
    }
    fs::write(&p, content).map_err(|e| format!("falha ao salvar skill: {e}"))
}

// ── session management ───────────────────────────────────────────────────────

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn iso_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.000Z",
        y,
        m,
        d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn assert_session_path(path: &Path) -> Result<(), String> {
    let root = sessions_root();
    if !path.starts_with(&root) {
        return Err("sessão fora do diretório permitido".into());
    }
    if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
        return Err("arquivo inválido".into());
    }
    Ok(())
}

/// Renames a session by appending a `session_info` entry (the format Pi uses).
pub fn rename_session(path: &str, name: &str) -> Result<(), String> {
    let p = PathBuf::from(path);
    assert_session_path(&p)?;
    let content = fs::read_to_string(&p).map_err(|e| format!("falha ao ler sessão: {e}"))?;
    let clean = name.trim();
    if clean.is_empty() {
        return Err("nome vazio".into());
    }
    let leaf = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|v| v.get("id").and_then(|i| i.as_str()).map(str::to_string))
        .last();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let entry = serde_json::json!({
        "type": "session_info",
        "id": format!("{:08x}", nanos),
        "parentId": leaf,
        "timestamp": iso_now(),
        "name": clean,
    });
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&p)
        .map_err(|e| e.to_string())?;
    use std::io::Write;
    let mut line = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
    if !content.ends_with('\n') {
        line = format!("\n{line}");
    }
    file.write_all(format!("{line}\n").as_bytes())
        .map_err(|e| format!("falha ao renomear: {e}"))?;
    Ok(())
}

/// Moves a session file to the app trash (never deletes outright).
pub fn delete_session(path: &str) -> Result<String, String> {
    let p = PathBuf::from(path);
    assert_session_path(&p)?;
    if !p.is_file() {
        return Err("sessão não encontrada".into());
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let trash = PathBuf::from(home)
        .join(".cache/pi-desktop/trash/sessions");
    fs::create_dir_all(&trash).map_err(|e| e.to_string())?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "sessao.jsonl".into());
    let dest = trash.join(format!("{ts}-{name}"));
    fs::rename(&p, &dest).map_err(|e| format!("falha ao mover sessão: {e}"))?;
    Ok(dest.to_string_lossy().into_owned())
}

// ── session holders / external change detection ──────────────────────────────

#[derive(Serialize, Clone, Debug)]
pub struct Holder {
    pub pid: i32,
    pub cmd: String,
}

#[derive(Serialize, Clone)]
pub struct SessionStat {
    pub len: u64,
    pub mtime_ms: u64,
}

pub fn session_stat(path: &str) -> SessionStat {
    let meta = fs::metadata(path).ok();
    SessionStat {
        len: meta.as_ref().map(|m| m.len()).unwrap_or(0),
        mtime_ms: meta
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    }
}

fn process_label(pid: u32) -> String {
    let cmdline = fs::read_to_string(format!("/proc/{pid}/cmdline")).unwrap_or_default();
    let joined = cmdline
        .split('\0')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if joined.is_empty() {
        fs::read_to_string(format!("/proc/{pid}/comm"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    } else {
        joined.chars().take(120).collect()
    }
}

/// Processes (other than this app and its sidecars) holding `path` open.
pub fn session_holders(path: &str, own_pids: &[u32]) -> Vec<Holder> {
    let target = fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
    let me = std::process::id();
    let mut out = Vec::new();

    let Ok(entries) = fs::read_dir("/proc") else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        if pid == me || own_pids.contains(&pid) {
            continue;
        }
        let Ok(fds) = fs::read_dir(format!("/proc/{pid}/fd")) else {
            continue;
        };
        let holds = fds.flatten().any(|fd| {
            fs::read_link(fd.path())
                .map(|p| p == target)
                .unwrap_or(false)
        });
        if holds {
            out.push(Holder {
                pid: pid as i32,
                cmd: process_label(pid),
            });
        }
    }
    out.sort_by_key(|h| h.pid);
    out.truncate(10);
    out
}

#[cfg(test)]
mod holder_tests {
    use super::*;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("pi-holders-{}-{}", std::process::id(), name))
    }

    #[test]
    fn detects_another_process_holding_the_session() {
        let path = tmp("held.jsonl");
        fs::write(&path, "{\"type\":\"session\"}\n").unwrap();

        let mut child = Command::new("tail")
            .arg("-f")
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("tail disponível");
        std::thread::sleep(Duration::from_millis(400));

        let holders = session_holders(path.to_str().unwrap(), &[]);
        let found = holders.iter().any(|h| h.pid == child.id() as i32);
        let _ = child.kill();
        let _ = child.wait();
        let _ = fs::remove_file(&path);
        assert!(found, "esperava encontrar o pid {} em {:?}", child.id(), holders);
    }

    #[test]
    fn ignores_own_pids() {
        let path = tmp("own.jsonl");
        fs::write(&path, "{}\n").unwrap();
        let mut child = Command::new("tail")
            .arg("-f")
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(400));

        let holders = session_holders(path.to_str().unwrap(), &[child.id()]);
        let found = holders.iter().any(|h| h.pid == child.id() as i32);
        let _ = child.kill();
        let _ = child.wait();
        let _ = fs::remove_file(&path);
        assert!(!found, "não deveria listar os próprios sidecars");
    }

    #[test]
    fn stat_reports_size_and_mtime() {
        let path = tmp("stat.jsonl");
        fs::write(&path, "12345").unwrap();
        let st = session_stat(path.to_str().unwrap());
        let _ = fs::remove_file(&path);
        assert_eq!(st.len, 5);
        assert!(st.mtime_ms > 0);
    }
}
