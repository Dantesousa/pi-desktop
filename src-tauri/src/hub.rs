//! Hubs: the official MCP registry and GitHub-hosted skill collections.
//!
//! Both are reached with `curl` / `gh` so the app needs no extra HTTP client.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn curl(url: &str) -> Result<String, String> {
    let out = Command::new("curl")
        .args(["-sS", "-L", "--max-time", "25", "-H", "Accept: application/json", url])
        .output()
        .map_err(|e| format!("curl falhou: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn gh_api(path: &str) -> Result<String, String> {
    let out = Command::new("gh")
        .args(["api", path])
        .output()
        .map_err(|e| format!("gh falhou: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    // fall back to the public API (rate limited but works without auth)
    curl(&format!("https://api.github.com/{path}"))
}

// ── MCP registry ─────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct McpCandidate {
    pub name: String,
    pub title: String,
    pub description: String,
    pub version: String,
    pub kind: String,
    pub target: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub url: Option<String>,
}

fn matches(query: &str, haystack: &str) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    let q = query.to_lowercase();
    haystack.to_lowercase().contains(&q)
}

pub fn mcp_search(query: &str, limit: u32) -> Result<Vec<McpCandidate>, String> {
    let page = limit.clamp(10, 100);
    let q: String = query
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
        .collect();
    let url = if q.trim().is_empty() {
        format!("https://registry.modelcontextprotocol.io/v0/servers?limit={page}")
    } else {
        format!(
            "https://registry.modelcontextprotocol.io/v0/servers?limit={page}&search={}",
            q.trim().replace(' ', "+")
        )
    };
    let body = curl(&url)?;
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("resposta inesperada do registro: {e}"))?;

    let mut out: Vec<McpCandidate> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let servers = json
        .get("servers")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for item in servers {
        let Some(server) = item.get("server") else { continue };
        let name = server.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if name.is_empty() || !seen.insert(name.clone()) {
            continue;
        }
        let title = server
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(&name)
            .to_string();
        let description = server
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !matches(query, &format!("{name} {title} {description}")) {
            continue;
        }
        let version = server
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // prefer remote endpoints, then npm, then pypi
        if let Some(remote) = server
            .get("remotes")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
        {
            let url = remote.get("url").and_then(|v| v.as_str()).unwrap_or("");
            if !url.is_empty() {
                out.push(McpCandidate {
                    name: name.clone(),
                    title,
                    description,
                    version,
                    kind: "http".into(),
                    target: url.to_string(),
                    command: None,
                    args: None,
                    url: Some(url.to_string()),
                });
                continue;
            }
        }

        let packages = server
            .get("packages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for pkg in packages {
            let registry = pkg.get("registryType").and_then(|v| v.as_str()).unwrap_or("");
            let identifier = pkg.get("identifier").and_then(|v| v.as_str()).unwrap_or("");
            if identifier.is_empty() {
                continue;
            }
            let (command, args) = match registry {
                "npm" => ("npx".to_string(), vec!["-y".to_string(), identifier.to_string()]),
                "pypi" => ("uvx".to_string(), vec![identifier.to_string()]),
                "oci" | "docker" => (
                    "docker".to_string(),
                    vec!["run".to_string(), "-i".to_string(), "--rm".to_string(), identifier.to_string()],
                ),
                _ => continue,
            };
            out.push(McpCandidate {
                name: name.clone(),
                title: title.clone(),
                description: description.clone(),
                version: version.clone(),
                kind: "stdio".into(),
                target: format!("{command} {}", args.join(" ")),
                command: Some(command),
                args: Some(args),
                url: None,
            });
            break;
        }
        if out.len() >= limit as usize {
            break;
        }
    }
    Ok(out)
}

// ── skills hub (GitHub) ──────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct RepoCandidate {
    pub full_name: String,
    pub description: String,
    pub stars: u64,
    pub updated: String,
    pub hits: u32,
}

#[derive(Serialize, Clone)]
pub struct SkillCandidate {
    pub name: String,
    pub dir: String,
    pub files: usize,
}

/// Known skill collections shown when the query is empty.
fn curated_repos() -> Vec<RepoCandidate> {
    [
        ("anthropics/skills", "the official Anthropic skills collection"),
        ("obra/superpowers", "workflow skills: brainstorming, plans, subagents"),
        ("affaan-m/ECC", "skills, memory and security for agents"),
        ("multica-ai/andrej-karpathy-skills", "code best practices as skills"),
    ]
    .iter()
    .map(|(name, desc)| RepoCandidate {
        full_name: name.to_string(),
        description: desc.to_string(),
        stars: 0,
        updated: String::new(),
        hits: 0,
    })
    .collect()
}

pub fn skills_search(query: &str, limit: u32) -> Result<Vec<RepoCandidate>, String> {
    if query.trim().is_empty() {
        return Ok(curated_repos());
    }
    let tokens: String = query
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("+");
    let q = if tokens.is_empty() {
        "filename:SKILL.md".to_string()
    } else {
        format!("filename:SKILL.md+{tokens}")
    };
    let body = gh_api(&format!("search/code?q={q}&per_page=100"))?;
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("resposta inesperada: {e}"))?;

    let mut repos: Vec<RepoCandidate> = Vec::new();
    for item in json.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
        let Some(repo) = item
            .get("repository")
            .and_then(|r| r.get("full_name"))
            .and_then(|v| v.as_str())
        else {
            continue;
        };
        let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("");
        match repos.iter_mut().find(|r| r.full_name == repo) {
            Some(existing) => existing.hits += 1,
            None => repos.push(RepoCandidate {
                full_name: repo.to_string(),
                description: path.to_string(),
                stars: 0,
                updated: String::new(),
                hits: 1,
            }),
        }
    }
    repos.sort_by(|a, b| b.hits.cmp(&a.hits).then(a.full_name.cmp(&b.full_name)));
    repos.truncate(limit.clamp(5, 40) as usize);
    Ok(repos)
}

fn default_branch(repo: &str) -> Result<String, String> {
    let body = gh_api(&format!("repos/{repo}"))?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(json
        .get("default_branch")
        .and_then(|v| v.as_str())
        .unwrap_or("main")
        .to_string())
}

/// Lists skill directories using GitHub code search (cheap even for huge repos).
pub fn skills_in_repo(repo: &str) -> Result<Vec<SkillCandidate>, String> {
    let body = gh_api(&format!(
        "search/code?q=filename:SKILL.md+repo:{repo}&per_page=100"
    ))?;
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("resposta inesperada: {e}"))?;

    let mut dirs: Vec<String> = Vec::new();
    for item in json.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
        if let Some(path) = item.get("path").and_then(|v| v.as_str()) {
            if path.ends_with("SKILL.md") {
                let dir = path.trim_end_matches("SKILL.md").trim_end_matches('/').to_string();
                if !dirs.contains(&dir) {
                    dirs.push(dir);
                }
            }
        }
    }
    dirs.sort();

    let mut out = Vec::new();
    for dir in dirs {
        let name = if dir.is_empty() {
            repo.rsplit('/').next().unwrap_or("skill").to_string()
        } else {
            dir.rsplit('/').next().unwrap_or("skill").to_string()
        };
        out.push(SkillCandidate { name, dir, files: 0 });
    }
    Ok(out)
}

/// Walks a directory through the contents API (bounded depth/entries).
fn collect_files(
    repo: &str,
    dir: &str,
    branch: &str,
    depth: usize,
    out: &mut Vec<(String, String)>,
) -> Result<(), String> {
    if depth > 4 || out.len() > 200 {
        return Ok(());
    }
    let body = gh_api(&format!(
        "repos/{repo}/contents/{}?ref={branch}",
        if dir.is_empty() { "".to_string() } else { dir.to_string() }
    ))?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let items = match json.as_array() {
        Some(a) => a.clone(),
        None => return Err(format!("`{dir}` is not a directory")),
    };
    for item in items {
        let kind = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if kind == "dir" {
            let _ = collect_files(repo, &path, branch, depth + 1, out);
        } else if kind == "file" {
            if let Some(url) = item.get("download_url").and_then(|v| v.as_str()) {
                out.push((path, url.to_string()));
            }
        }
    }
    Ok(())
}

fn skills_root() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".pi/agent/pi-hermes-memory/skills")
}

fn slugify(name: &str) -> String {
    let mapped: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    mapped
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Downloads a skill directory (SKILL.md plus siblings) into the skills folder.
pub fn skill_install(repo: &str, dir: &str, rename: Option<String>) -> Result<String, String> {
    let branch = default_branch(repo)?;
    let dir = dir.trim_end_matches('/').to_string();
    let mut files: Vec<(String, String)> = Vec::new();
    collect_files(repo, &dir, &branch, 0, &mut files)?;
    if !files.iter().any(|(p, _)| p.ends_with("SKILL.md")) {
        return Err("this directory has no SKILL.md".into());
    }

    let base_name = rename.unwrap_or_else(|| {
        if dir.is_empty() {
            repo.rsplit('/').next().unwrap_or("skill").to_string()
        } else {
            dir.rsplit('/').next().unwrap_or("skill").to_string()
        }
    });
    let slug = slugify(&base_name);
    let dest = skills_root().join(&slug);
    if !dest.starts_with(skills_root()) {
        return Err("invalid destination".into());
    }
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    let mut written = 0usize;
    for (path, download_url) in files {
        let rel = if dir.is_empty() {
            path.clone()
        } else {
            path.trim_start_matches(&format!("{dir}/")).to_string()
        };
        if rel.is_empty() {
            continue;
        }
        let target = dest.join(&rel);
        if !target.starts_with(&dest) {
            continue; // ignore path traversal
        }
        let body = curl(&download_url)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&target, body).map_err(|e| format!("failed to write {rel}: {e}"))?;
        written += 1;
    }
    if written == 0 {
        return Err("no file downloaded".into());
    }
    Ok(dest.to_string_lossy().into_owned())
}

pub fn skills_dir_path() -> String {
    skills_root().to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn slugify_sanitizes_names() {
        assert_eq!(slugify("My Skill!"), "my-skill");
        assert_eq!(slugify("__a__b__"), "a-b");
    }

    #[test]
    #[ignore = "rede"]
    fn mcp_registry_returns_mapped_candidates() {
        let list = mcp_search("filesystem", 10).expect("consulta ok");
        assert!(!list.is_empty(), "esperava resultados");
        let stdio = list.iter().find(|c| c.kind == "stdio");
        if let Some(c) = stdio {
            assert!(c.command.is_some());
            assert!(!c.args.as_ref().unwrap().is_empty());
        }
        assert!(list.iter().all(|c| !c.name.is_empty() && !c.target.is_empty()));
    }

    #[test]
    #[ignore = "rede"]
    fn github_skill_repos_and_dirs() {
        let repos = skills_search("", 5).expect("busca ok");
        assert!(!repos.is_empty(), "expected repositories with SKILL.md");
        assert!(repos[0].hits >= 1);
        let dirs = skills_in_repo(&repos[0].full_name).expect("code search ok");
        println!("repo {} ({} hits) → {} skills", repos[0].full_name, repos[0].hits, dirs.len());
        assert!(dirs.iter().all(|d| !d.name.is_empty()));
    }

    #[test]
    fn skills_root_is_inside_the_agent_home() {
        let root = skills_root();
        assert!(root.ends_with("skills"));
        let fake = Path::new("/etc/passwd");
        assert!(!fake.starts_with(&root));
    }
}

// ── skill provenance / updates ───────────────────────────────────────────────

fn fnv1a(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

#[derive(Serialize, Clone)]
pub struct SkillSource {
    pub slug: String,
    pub repo: String,
    pub dir: String,
    pub installed_at: String,
    pub files: usize,
}

fn source_path(slug: &str) -> PathBuf {
    skills_root().join(slug).join(".source.json")
}

fn read_source(slug: &str) -> Option<serde_json::Value> {
    fs::read_to_string(source_path(slug))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

fn write_source(slug: &str, repo: &str, dir: &str, files: &[(String, String)]) -> Result<(), String> {
    let payload = serde_json::json!({
        "repo": repo,
        "dir": dir,
        "installedAt": crate::memory::now_iso(),
        "files": files.iter().map(|(p, h)| serde_json::json!({ "path": p, "hash": h })).collect::<Vec<_>>(),
    });
    fs::write(
        source_path(slug),
        serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("failed to write .source.json: {e}"))
}

pub fn skills_sources() -> Vec<SkillSource> {
    let mut out = Vec::new();
    let Ok(dirs) = fs::read_dir(skills_root()) else {
        return out;
    };
    for entry in dirs.flatten() {
        let slug = entry.file_name().to_string_lossy().into_owned();
        let Some(src) = read_source(&slug) else { continue };
        out.push(SkillSource {
            slug,
            repo: src.get("repo").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            dir: src.get("dir").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            installed_at: src
                .get("installedAt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            files: src
                .get("files")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0),
        });
    }
    out.sort_by(|a, b| a.slug.cmp(&b.slug));
    out
}

/// Re-downloads a skill from its recorded source and reports what changed.
pub fn skill_sync(slug: &str) -> Result<String, String> {
    let src = read_source(slug).ok_or_else(|| "this skill did not come from a hub (no .source.json)".to_string())?;
    let repo = src.get("repo").and_then(|v| v.as_str()).unwrap_or("");
    let dir = src.get("dir").and_then(|v| v.as_str()).unwrap_or("");
    if repo.is_empty() {
        return Err("origem desconhecida".into());
    }
    let before: Vec<(String, String)> = src
        .get("files")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|f| {
                    Some((
                        f.get("path")?.as_str()?.to_string(),
                        f.get("hash")?.as_str()?.to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();

    // re-instala em cima e coleta os hashes atuais
    let dest = skill_install(repo, dir, Some(slug.to_string()))?;
    let mut now: Vec<(String, String)> = Vec::new();
    let dest_path = PathBuf::from(&dest);
    fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, base, out);
            } else if p.file_name().and_then(|n| n.to_str()) != Some(".source.json") {
                if let Ok(bytes) = fs::read(&p) {
                    if let Ok(rel) = p.strip_prefix(base) {
                        out.push((rel.to_string_lossy().into_owned(), fnv1a(&bytes)));
                    }
                }
            }
        }
    }
    walk(&dest_path, &dest_path, &mut now);

    let mut changed = 0;
    for (path, hash) in &now {
        match before.iter().find(|(p, _)| p == path) {
            Some((_, old)) if old == hash => {}
            _ => changed += 1,
        }
    }
    write_source(slug, repo, dir, &now)?;

    if changed == 0 {
        Ok("already up to date".to_string())
    } else {
        Ok(format!("{changed} file(s) updated"))
    }
}
