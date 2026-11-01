//! Pi Desktop — native shell around the Pi coding agent.

mod attachments;
mod files;
mod hub;
mod mcp;
mod memory;
mod pi;
mod proc;

use pi::{PiState, StartOptions};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

// ── agent process ────────────────────────────────────────────────────────────

#[tauri::command]
fn pi_start(
    app: tauri::AppHandle,
    state: tauri::State<PiState>,
    tab: String,
    opts: StartOptions,
) -> Result<pi::PiStatus, String> {
    pi::start(&app, &state, &tab, opts)
}

#[tauri::command]
fn pi_send(
    state: tauri::State<PiState>,
    tab: String,
    command: serde_json::Value,
) -> Result<(), String> {
    pi::send(&state, &tab, command)
}

#[tauri::command]
fn pi_stop(state: tauri::State<PiState>, tab: String) -> Result<(), String> {
    pi::stop(&state, &tab)
}

#[tauri::command]
fn pi_status(state: tauri::State<PiState>, tab: String) -> pi::PiStatus {
    pi::status(&state, &tab)
}

#[tauri::command]
fn pi_stop_all(state: tauri::State<PiState>) -> Result<(), String> {
    pi::stop_all(&state)
}

#[tauri::command]
fn pi_version() -> pi::PiVersion {
    pi::check_update()
}

#[tauri::command]
fn pi_self_update() -> Result<String, String> {
    pi::self_update()
}

#[derive(serde::Serialize)]
pub struct BuildInfo {
    pub channel: String,
    pub version: String,
    pub commit: String,
    pub tag: String,
}

fn channel() -> &'static str {
    option_env!("PI_CHANNEL").unwrap_or("dev")
}

#[tauri::command]
fn proc_start(
    app: tauri::AppHandle,
    state: tauri::State<proc::ProcState>,
    command: String,
    cwd: Option<String>,
    notify: Option<bool>,
) -> Result<proc::BgProc, String> {
    proc::start(&app, &state, &command, cwd, notify.unwrap_or(true))
}

#[tauri::command]
fn proc_list(state: tauri::State<proc::ProcState>) -> Vec<proc::BgProc> {
    proc::list(&state)
}

#[tauri::command]
fn proc_output(app: tauri::AppHandle, id: String, lines: Option<usize>) -> String {
    proc::output_tail(&app, &id, lines.unwrap_or(120))
}

#[tauri::command]
fn proc_stop(state: tauri::State<proc::ProcState>, id: String) -> Result<(), String> {
    proc::stop(&state, &id)
}

#[tauri::command]
fn proc_remove(state: tauri::State<proc::ProcState>, id: String) -> Result<(), String> {
    proc::remove(&state, &id)
}

#[tauri::command]
fn proc_set_notify(state: tauri::State<proc::ProcState>, id: String, notify: bool) {
    proc::set_notify(&state, &id, notify)
}

#[tauri::command]
fn build_info() -> BuildInfo {
    BuildInfo {
        channel: channel().to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: option_env!("PI_COMMIT").unwrap_or("").to_string(),
        tag: option_env!("PI_TAG").unwrap_or("").to_string(),
    }
}

#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    files::open_external(&url)
}

#[tauri::command]
fn file_snapshot(path: String) -> Result<files::FileSnapshot, String> {
    files::snapshot(&path)
}

#[tauri::command]
fn file_restore(path: String, content: Option<String>) -> Result<(), String> {
    files::restore(&path, content)
}

#[tauri::command]
fn write_diagnostics(payload: String) -> Result<String, String> {
    files::write_diagnostics(&payload)
}

#[tauri::command]
fn pi_resolve_bin() -> String {
    pi::resolve_bin()
}

#[tauri::command]
fn pi_permission_extension() -> Option<String> {
    pi::permission_extension()
}

// ── memory store ─────────────────────────────────────────────────────────────

#[tauri::command]
fn memory_read(target: String) -> Result<memory::MemoryFile, String> {
    memory::read_target(&target)
}

#[tauri::command]
fn memory_write(target: String, entries: Vec<String>) -> Result<memory::MemoryFile, String> {
    memory::write_target(&target, entries)
}

#[tauri::command]
fn memory_stats() -> memory::MemoryStats {
    memory::stats()
}

#[tauri::command]
fn memory_query(
    target: Option<String>,
    query: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<memory::DbMemory>, String> {
    memory::query_memories(target, query, limit.unwrap_or(200))
}

#[tauri::command]
fn session_search(query: String, limit: Option<u32>) -> Result<Vec<memory::SessionHit>, String> {
    memory::search_sessions(query, limit.unwrap_or(30))
}

#[tauri::command]
fn sessions_list(limit: Option<u32>) -> Vec<memory::SessionInfo> {
    memory::list_sessions(limit.unwrap_or(60))
}

#[tauri::command]
fn session_rename(path: String, name: String) -> Result<(), String> {
    memory::rename_session(&path, &name)
}

#[tauri::command]
fn session_delete(path: String) -> Result<String, String> {
    memory::delete_session(&path)
}

#[tauri::command]
fn session_holders(state: tauri::State<PiState>, path: String) -> Vec<memory::Holder> {
    let own: Vec<u32> = state
        .procs
        .lock()
        .map(|m| m.values().map(|r| r.pid).collect())
        .unwrap_or_default();
    memory::session_holders(&path, &own)
}

#[tauri::command]
fn session_stat(path: String) -> memory::SessionStat {
    memory::session_stat(&path)
}

#[tauri::command]
fn skills_list() -> Vec<memory::SkillInfo> {
    memory::list_skills()
}

#[tauri::command]
fn skill_read(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("failed to read {path}: {e}"))
}

#[tauri::command]
fn skill_write(path: String, content: String) -> Result<(), String> {
    memory::write_skill(&path, &content)
}

#[tauri::command]
fn skill_create(name: String, content: String) -> Result<String, String> {
    memory::create_skill(&name, &content)
}

// ── mcp ──────────────────────────────────────────────────────────────────────

#[tauri::command]
fn mcp_config() -> serde_json::Value {
    mcp::read_config()
}

#[tauri::command]
fn mcp_servers() -> Vec<mcp::McpServer> {
    mcp::list_servers()
}

#[tauri::command]
fn browser_enable() -> Result<String, String> {
    mcp::browser_enable()
}

#[tauri::command]
fn browser_disable() -> Result<(), String> {
    mcp::browser_disable()
}

#[tauri::command]
fn browser_preview(with_image: Option<bool>) -> mcp::BrowserPreview {
    mcp::browser_preview(with_image.unwrap_or(true))
}

#[tauri::command]
fn hub_mcp_search(query: String, limit: Option<u32>) -> Result<Vec<hub::McpCandidate>, String> {
    hub::mcp_search(&query, limit.unwrap_or(30))
}

#[tauri::command]
fn hub_skills_search(query: String, limit: Option<u32>) -> Result<Vec<hub::RepoCandidate>, String> {
    hub::skills_search(&query, limit.unwrap_or(15))
}

#[tauri::command]
fn hub_skills_in_repo(repo: String) -> Result<Vec<hub::SkillCandidate>, String> {
    hub::skills_in_repo(&repo)
}

#[tauri::command]
fn hub_skill_install(
    repo: String,
    dir: String,
    rename: Option<String>,
) -> Result<String, String> {
    hub::skill_install(&repo, &dir, rename)
}

#[tauri::command]
fn hub_skills_sources() -> Vec<hub::SkillSource> {
    hub::skills_sources()
}

#[tauri::command]
fn hub_skill_sync(slug: String) -> Result<String, String> {
    hub::skill_sync(&slug)
}

#[tauri::command]
fn hub_skills_dir() -> String {
    hub::skills_dir_path()
}

#[tauri::command]
fn mcp_save(config: serde_json::Value) -> Result<(), String> {
    mcp::save_config(config)
}

#[tauri::command]
fn mcp_status(cwd: Option<String>) -> Result<Vec<mcp::McpServerStatus>, String> {
    mcp::status(cwd)
}

#[tauri::command]
fn mcp_check(cwd: Option<String>) -> Result<String, String> {
    mcp::check(cwd)
}

#[tauri::command]
fn memory_dir_path() -> String {
    memory::memory_dir().to_string_lossy().into_owned()
}

// ── attachments ──────────────────────────────────────────────────────────────

#[tauri::command]
fn attachment_read(path: String) -> Result<attachments::Attachment, String> {
    attachments::read(path)
}

#[tauri::command]
fn attachment_stage(path: String) -> Result<String, String> {
    attachments::stage(std::path::Path::new(&path))
}

#[tauri::command]
fn attachment_save_b64(
    name: String,
    mime: String,
    data: String,
) -> Result<attachments::Attachment, String> {
    attachments::save_b64(name, mime, data)
}

// ── window helpers ───────────────────────────────────────────────────────────

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let visible = window.is_visible().unwrap_or(false);
        let focused = window.is_focused().unwrap_or(false);
        if visible && focused {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
            let _ = window.emit("quick-ask", ());
        }
    }
}

#[tauri::command]
fn window_toggle(app: tauri::AppHandle) {
    toggle_window(&app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn install_panic_log() {
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        let base = std::env::var("XDG_CACHE_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cache")
            })
            .join("pi-desktop");
        let _ = std::fs::create_dir_all(&base);
        let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(base.join("crash.log"))
        else {
            return;
        };
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let thread = std::thread::current();
        let _ = writeln!(
            f,
            "\n=== panic {stamp} | {} {} ({}) | thread {}",
            option_env!("PI_CHANNEL").unwrap_or("dev"),
            env!("CARGO_PKG_VERSION"),
            option_env!("PI_COMMIT").unwrap_or("?"),
            thread.name().unwrap_or("unnamed")
        );
        let _ = writeln!(f, "{info}");
        if std::env::var("RUST_BACKTRACE").is_ok() {
            let bt = std::backtrace::Backtrace::force_capture();
            let _ = writeln!(f, "{bt}");
        } else {
            let _ = writeln!(f, "(set RUST_BACKTRACE=1 for a backtrace)");
        }
    }));
}

pub fn run() {
    install_panic_log();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        toggle_window(app);
                    }
                })
                .build(),
        )
        .manage(PiState::default())
        .manage(proc::ProcState::default())
        .invoke_handler(tauri::generate_handler![
            pi_start,
            pi_send,
            pi_stop,
            pi_status,
            pi_resolve_bin,
            write_diagnostics,
            pi_version,
            pi_self_update,
            open_external,
            build_info,
            proc_start,
            proc_list,
            proc_output,
            proc_stop,
            proc_remove,
            proc_set_notify,
            file_snapshot,
            file_restore,
            pi_stop_all,
            pi_permission_extension,
            memory_read,
            memory_write,
            memory_stats,
            memory_query,
            session_search,
            sessions_list,
            session_rename,
            session_delete,
            session_holders,
            session_stat,
            skills_list,
            skill_read,
            skill_write,
            skill_create,
            memory_dir_path,
            mcp_config,
            mcp_servers,
            mcp_save,
            mcp_check,
            mcp_status,
            browser_enable,
            browser_disable,
            browser_preview,
            hub_mcp_search,
            hub_skills_search,
            hub_skills_in_repo,
            hub_skill_install,
            hub_skills_dir,
            hub_skills_sources,
            hub_skill_sync,
            attachment_read,
            attachment_stage,
            attachment_save_b64,
            window_toggle,
        ])
        .setup(|app| {
            use tauri_plugin_global_shortcut::GlobalShortcutExt;

            // window icon (fallback for X11; Wayland uses the .desktop file)
            let dev = channel() == "dev";
            let icon_bytes: &[u8] = if dev {
                include_bytes!("../icons/dev/128x128@2x.png")
            } else {
                include_bytes!("../icons/128x128@2x.png")
            };
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title(if dev { "Hina · DEV" } else { "Hina · pi desktop" });
                if let Ok(icon) = tauri::image::Image::from_bytes(icon_bytes) {
                    let _ = window.set_icon(icon);
                }
            }

            // tray icon
            let show = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
            let new_chat = MenuItem::with_id(app, "new", "New chat", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &new_chat, &quit])?;

            let tray_image = tauri::image::Image::from_bytes(icon_bytes).ok()
                .or_else(|| app.default_window_icon().cloned());
            let _ = TrayIconBuilder::with_id("main-tray")
                .icon(tray_image.unwrap())
                .tooltip("Hina · pi desktop")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "new" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                            let _ = w.emit("quick-new", ());
                        }
                    }
                    "quit" => {
                        // collect the tab ids and drop the lock before stopping:
                        // pi::stop() takes the same (non-reentrant) mutex.
                        let tabs = app
                            .try_state::<PiState>()
                            .and_then(|st| {
                                st.procs
                                    .lock()
                                    .ok()
                                    .map(|m| m.keys().cloned().collect::<Vec<_>>())
                            })
                            .unwrap_or_default();
                        if let Some(st) = app.try_state::<PiState>() {
                            for tab in tabs {
                                let _ = pi::stop(&st, &tab);
                            }
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window(tray.app_handle());
                    }
                })
                .build(app);

            // global quick-ask shortcut
            let _ = app.global_shortcut().register("Ctrl+Alt+H");

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            tauri::WindowEvent::Destroyed => {
                let tabs = window
                    .try_state::<PiState>()
                    .and_then(|st| {
                        st.procs
                            .lock()
                            .ok()
                            .map(|m| m.keys().cloned().collect::<Vec<_>>())
                    })
                    .unwrap_or_default();
                if let Some(st) = window.try_state::<PiState>() {
                    for tab in tabs {
                        let _ = pi::stop(&st, &tab);
                    }
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
