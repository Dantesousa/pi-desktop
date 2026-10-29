# Hina · pi desktop

A native desktop client for the [Pi coding agent](https://github.com/earendil-works/pi), written in Rust (Tauri v2) with a Svelte 5 UI.

It does not reimplement the agent: it spawns `pi --mode rpc` as a sidecar and speaks the JSONL protocol, so you get the **same sessions, tools, memory, skills and extensions** you already use in the terminal.

```
┌────────────┐ ┌──────────────────────────────────────┐ ┌──────────────┐
│ sessions   │ │ tabs · model · thinking · context %  │ │ memory       │
│ ▸ pinned   │ │ chat (streaming text + tool cards)   │ │ user         │
│ ▸ recent   │ │ diffs · bash output · thinking       │ │ failures     │
│            │ │ composer (files, images, /commands)  │ │ sessions FTS │
└────────────┘ └──────────────────────────────────────┘ └──────────────┘
```

## Features

**Conversation**
- Live streaming: text, collapsible reasoning, and tool cards with arguments, output and duration
- Send **images** (paste with `Ctrl+V`, drag & drop, or 📎) — encoded as base64 and sent through the RPC `images` field
- Send **documents**: text files are inlined, binaries are staged on disk for the agent to open with its own tools
- **Diff viewer** for `edit` / `write` tool calls (LCS line diff, collapsed context)
- **Undo a file change** straight from the diff card: a snapshot is taken before the tool runs, so "revert" restores the previous content (or removes a newly created file)
- Send button becomes **stop** as soon as a run is active; `Esc` aborts
- Slash-command autocomplete from `get_commands` (extension commands, prompt templates, skills)

**Parallel sessions (tabs)**
- Each tab is an independent `pi` process — separate session, model, thinking level, context and cost
- Tabs are named after their project folder; double-click to rename
- `Ctrl+T` new tab · `Ctrl+W` close · click to switch

**Sessions**
- List, filter, switch, fork-ready; **pin** favourites (top of the list)
- **Rename** (writes a `session_info` entry; uses `set_session_name` for the active one)
- **Delete** — moves the `.jsonl` to the app trash (`~/.cache/pi-desktop/trash`), never `rm`
- Full-text search across your history (SQLite FTS from `pi-hermes-memory`)
- **External change detection**: warns when the session file was written by another process (e.g. a terminal `pi`) and when another process has it open; `F5` reloads it from disk

**Memory (pi-hermes-memory)**
- Read and edit `MEMORY.md`, `USER.md`, `failures.md` with usage meters against the char limits
- Add / edit / delete entries, then reconcile the SQLite index (`/memory-sync-markdown`)
- "Consolidate" hands the pruning task to the agent
- Browse and edit **skills** (`SKILL.md`) and create new ones
- **Skills hub**: searches GitHub code for `SKILL.md`, lists the skill directories of a repository (code search + contents API, so huge repos stay fast) and installs one with its sibling files into `~/.pi/agent/pi-hermes-memory/skills/<slug>`; a curated list of known collections shows up when the query is empty
- **MCP hub**: searches the official registry (`registry.modelcontextprotocol.io`), maps `http` remotes and `npm`/`pypi`/`oci` packages to a config entry, and adds the server to `mcp.json` with one click

**Control & safety**
- **Permission gate**: a bundled extension intercepts `tool_call` and asks before dangerous bash, or every write, depending on mode (`off` / `dangerous` / `all`)
- Extension UI dialogs (`select`, `confirm`, `input`, `editor`) render as native modals
- Cost limit with warning banner, context-window meter, session stats
- **MCP manager**: list / enable / disable / add servers in `~/.pi/agent/mcp.json`, and test connections
- Tray icon, close-to-tray, global quick-ask (`Ctrl+Alt+H`), and a diagnostics dump (`Ctrl+Shift+D`) that writes the full UI state to `~/.cache/pi-desktop/diagnostics/`
- Desktop notification when a long task finishes while the window is unfocused
- Command palette (`Ctrl+K`), HTML export
- **10 built-in themes** (Midnight, AMOLED, Hacker, Dracula, Nord, Gruvbox, Catppuccin, Tokyo Night, Cyberpunk, Light) plus a custom accent colour, automatic contrast ink on buttons, and a chat font-size control. Preview any theme with `?theme=<id>`, switch from the palette or the settings popover
- **Pi self-update**: check the latest version and update the sidecar from the settings popover

## Embedded browser (agent-visible)

`scripts/browser-mcp.mjs` is a stdio MCP server that gives the agent a real browser
(Playwright Firefox): `browser_navigate`, `browser_snapshot`, `browser_screenshot`,
`browser_click`, `browser_type`, `browser_press`, `browser_scroll`, `browser_eval`,
`browser_wait`, `browser_back`, `browser_close`.

It mirrors every action to `~/.cache/pi-desktop/browser/` (`state.json` + `latest.jpg`),
which the app polls to show a live preview panel (`Ctrl+Shift+W`, 🌐 in the header or
the palette). The panel can enable/disable the server (writes the `hina-browser` entry
in `~/.pi/agent/mcp.json`) and hand the agent a ready-made instruction.

MCP servers are read when a pi process starts, so restart the tab after enabling.

## Exposed RPC surface

Hina drives `pi --mode rpc` directly. Everything the protocol offers has a home in the UI:

| command | where |
|---|---|
| `prompt`, `steer`, `follow_up` | composer (Enter sends; while running it steers) |
| `abort`, `clear_queue` | stop button, queue chip (restores cleared text to the composer) |
| `new_session`, `switch_session`, `set_session_name` | sidebar / tabs |
| `fork`, `clone`, `get_fork_messages`, `get_tree`, `get_entries` | "retomar de uma mensagem", "clonar", "árvore da sessão" |
| `get_messages`, `get_last_assistant_text` | chat history, "copiar última resposta" |
| `get_state`, `get_session_stats`, `get_commands` | header, palette, slash completion |
| `get_available_models`, `set_model`, `cycle_model` | model picker, `Ctrl+Shift+M` |
| `get_available_thinking_levels`, `set_thinking_level`, `cycle_thinking_level` | thinking picker, `Ctrl+Shift+L` |
| `set_steering_mode`, `set_follow_up_mode` | settings |
| `compact`, `set_auto_compaction`, `set_auto_retry`, `abort_retry` | palette, settings, retry chip |
| `bash`, `abort_bash` | bash panel (`Ctrl+B`) with streamed output |
| `export_html` | header ⤓ |

## Requirements

- Rust 1.80+ and Cargo
- Node.js 20+ and npm
- WebKitGTK 4.1 / GTK 3 (`webkit2gtk-4.1`, `gtk3`, `libayatana-appindicator`, `librsvg`)
- [Pi](https://github.com/earendil-works/pi) installed and authenticated (`pi` on `PATH`, or `PI_BIN=/path/to/pi`)

On Arch: `sudo pacman -S webkit2gtk-4.1 gtk3 libayatana-appindicator librsvg`

## Development

```bash
npm install
npm run tauri dev
```

Rust sources live in `src-tauri/src`:

| file | responsibility |
|---|---|
| `pi.rs` | sidecar lifecycle (one process per tab), LF-only JSONL framing, event fan-out |
| `memory.rs` | memory markdown store, SQLite search, session list/rename/delete, skills |
| `mcp.rs` | `~/.pi/agent/mcp.json` read/write and `pi mcp list --json` |
| `attachments.rs` | image base64, text detection, staging big/binary files |
| `extensions/permissions.ts` | permission gate, copied to the cache dir and passed via `--extension` |

Frontend: `src/lib/store.svelte.ts` holds a `Tab` class (per-process state) and `AppState`
(which proxies the active tab so components stay simple). Components are in `src/lib/components`.

## Build a release

```bash
npm run tauri build
```

Artifacts land in `src-tauri/target/release/bundle/` (`appimage/` and `deb/`).

`npm run dist` is the same as `npm run tauri build`. Note that an AppImage built on
Ubuntu (CI) bundles Ubuntu's WebKitGTK without its ICU dependency and will only start
on Debian-derived systems; a build made on your own distro (`npm run dist`) bundles
everything and runs elsewhere — that is why the release ships both.

Pushing a `v*` tag makes `.github/workflows/release.yml` build both bundles on
Ubuntu 24.04 and attach them to a GitHub release:

```bash
git tag v0.1.2 && git push origin v0.1.2
```

## Development notes

- Global CSS lives in `static/theme.css` and is linked from `app.html`, so it is
  never injected by HMR (a stale stylesheet used to make the whole layout look
  broken). Component styles stay scoped in their `.svelte` files.
- HMR is disabled in `vite.config.js`; after changing frontend code press
  `Ctrl+Shift+R` inside the app. Vite's HMR occasionally left scoped-CSS hashes
  and the DOM out of sync, which is what "the UI broke" looked like.
- If the shell ever renders stacked, the app reloads itself once and then writes
  a diagnostic bundle.
- `node scripts/shot.mjs <url> <out.png> [waitMs]` loads the dev URL in headless
  Firefox (playwright-core + the cached build), screenshots it and prints DOM
  metrics as JSON — the way to check layout without a human looking at it.

- Use `npm run dev:clean` instead of `npm run tauri dev`: it frees a stale
  listener on port 1420 first (an orphan Vite from an earlier session otherwise
  makes `beforeDevCommand` fail and takes the app down with it).
- Rust panics are appended to `~/.cache/pi-desktop/crash.log`.

## Environment variables

| variable | default | meaning |
|---|---|---|
| `PI_BIN` | resolved from `PATH` | path to the `pi` executable |
| `PI_DESKTOP_GATE` | `dangerous` | permission gate: `off`, `dangerous`, `all` |

## Notes

- The app never writes to your session files except to append a `session_info` entry when you rename a session.
- Deleting a session moves the file to `~/.cache/pi-desktop/trash/sessions/`.
- Memory edits back up the original file to `~/.pi/agent/pi-hermes-memory/.backups/`.

## License

MIT © Dante
