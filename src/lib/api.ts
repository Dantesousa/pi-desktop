import { invoke } from "@tauri-apps/api/core";
import type {
  MemoryFile,
  MemoryStats,
  SessionInfo,
  SkillInfo,
  DbMemory,
  SessionHit,
} from "./types";

const camel = (o: any): any => {
  if (Array.isArray(o)) return o.map(camel);
  if (o && typeof o === "object") {
    const out: Record<string, any> = {};
    for (const [k, v] of Object.entries(o)) {
      out[k.replace(/_([a-z])/g, (_, c) => c.toUpperCase())] = camel(v);
    }
    return out;
  }
  return o;
};

export const api = {
  // agent process (one per tab)
  start: (tab: string, opts: Record<string, any>) => invoke<any>("pi_start", { tab, opts }),
  send: (tab: string, command: any) => invoke<void>("pi_send", { tab, command }),
  stop: (tab: string) => invoke<void>("pi_stop", { tab }),
  stopAll: () => invoke<void>("pi_stop_all"),
  status: (tab: string) => invoke<any>("pi_status", { tab }),
  resolveBin: () => invoke<string>("pi_resolve_bin"),
  openExternal: (url: string) => invoke<void>("open_external", { url }),
  permissionExtension: () => invoke<string | null>("pi_permission_extension"),
  writeDiagnostics: (payload: string) => invoke<string>("write_diagnostics", { payload }),
  piVersion: async () => camel(await invoke<any>("pi_version")),
  piSelfUpdate: () => invoke<string>("pi_self_update"),
  fileSnapshot: async (path: string) => camel(await invoke<any>("file_snapshot", { path })),
  fileRestore: (path: string, content: string | null) =>
    invoke<void>("file_restore", { path, content }),

  // memory
  memoryRead: async (target: string) => camel(await invoke<MemoryFile>("memory_read", { target })),
  memoryWrite: async (target: string, entries: string[]) =>
    camel(await invoke<MemoryFile>("memory_write", { target, entries })),
  memoryStats: async () => camel(await invoke<MemoryStats>("memory_stats")),
  memoryQuery: async (target: string | null, query: string | null, limit = 200) =>
    camel(await invoke<DbMemory[]>("memory_query", { target, query, limit })),
  sessionSearch: async (query: string, limit = 30) =>
    camel(await invoke<SessionHit[]>("session_search", { query, limit })),
  sessionsList: async (limit = 60) => camel(await invoke<SessionInfo[]>("sessions_list", { limit })),
  sessionRename: (path: string, name: string) => invoke<void>("session_rename", { path, name }),
  sessionDelete: (path: string) => invoke<string>("session_delete", { path }),
  sessionHolders: async (path: string) => camel(await invoke<any[]>("session_holders", { path })),
  sessionStat: async (path: string) => camel(await invoke<any>("session_stat", { path })),
  skillsList: async () => camel(await invoke<SkillInfo[]>("skills_list")),
  skillRead: (path: string) => invoke<string>("skill_read", { path }),
  skillWrite: (path: string, content: string) => invoke<void>("skill_write", { path, content }),
  skillCreate: (name: string, content: string) => invoke<string>("skill_create", { name, content }),
  memoryDir: () => invoke<string>("memory_dir_path"),

  // mcp
  mcpConfig: async () => camel(await invoke<any>("mcp_config")),
  mcpServers: async () => camel(await invoke<any[]>("mcp_servers")),
  mcpSave: (config: any) => invoke<void>("mcp_save", { config }),
  mcpCheck: (cwd: string | null) => invoke<string>("mcp_check", { cwd }),

  // hubs
  browserEnable: () => invoke<string>("browser_enable"),
  browserDisable: () => invoke<void>("browser_disable"),
  browserPreview: async (withImage = true) =>
    camel(await invoke<any>("browser_preview", { withImage })),

  hubMcpSearch: async (query: string, limit = 30) =>
    camel(await invoke<any[]>("hub_mcp_search", { query, limit })),
  hubSkillsSearch: async (query: string, limit = 15) =>
    camel(await invoke<any[]>("hub_skills_search", { query, limit })),
  hubSkillsInRepo: async (repo: string) => camel(await invoke<any[]>("hub_skills_in_repo", { repo })),
  hubSkillInstall: (repo: string, dir: string, rename?: string) =>
    invoke<string>("hub_skill_install", { repo, dir, rename: rename ?? null }),
  hubSkillsDir: () => invoke<string>("hub_skills_dir"),

  // attachments
  attachmentRead: async (path: string) => camel(await invoke<any>("attachment_read", { path })),
  attachmentStage: (path: string) => invoke<string>("attachment_stage", { path }),
  attachmentSaveB64: async (name: string, mime: string, data: string) =>
    camel(await invoke<any>("attachment_save_b64", { name, mime, data })),
};
