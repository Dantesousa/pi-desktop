import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { api } from "./api";
import { extractImages } from "./media";
import type { Chip, Msg, ModelInfo, Part, SessionInfo, SkillInfo } from "./types";

const uid = () => Math.random().toString(36).slice(2, 10);

export type UiRequest = {
  id: string;
  method: "select" | "confirm" | "input" | "editor";
  title?: string;
  message?: string;
  options?: string[];
  placeholder?: string;
  prefill?: string;
};

export type Toast = { id: string; text: string; kind: "info" | "warn" | "error" | "ok" };

export type ThemePreset = { id: string; name: string; colors: [string, string, string] };

export const THEMES: ThemePreset[] = [
  { id: "midnight", name: "Meia-noite", colors: ["#0b0d12", "#7aa2f7", "#7ee787"] },
  { id: "amoled", name: "AMOLED", colors: ["#000000", "#8ab4ff", "#7ee787"] },
  { id: "hacker", name: "Hacker", colors: ["#03070a", "#39ff14", "#00ffa3"] },
  { id: "dracula", name: "Dracula", colors: ["#282a36", "#bd93f9", "#50fa7b"] },
  { id: "nord", name: "Nord", colors: ["#2e3440", "#88c0d0", "#a3be8c"] },
  { id: "gruvbox", name: "Gruvbox", colors: ["#282828", "#fabd2f", "#b8bb26"] },
  { id: "catppuccin", name: "Catppuccin", colors: ["#1e1e2e", "#89b4fa", "#a6e3a1"] },
  { id: "tokyo", name: "Tokyo Night", colors: ["#1a1b26", "#7aa2f7", "#bb9af7"] },
  { id: "cyberpunk", name: "Cyberpunk", colors: ["#0a0512", "#ff2e97", "#00e5ff"] },
  { id: "light", name: "Claro", colors: ["#ffffff", "#2f62d8", "#1a7f37"] },
];

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l * 100];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h = 0;
  if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
  else if (max === g) h = ((b - r) / d + 2) / 6;
  else h = ((r - g) / d + 4) / 6;
  return [h * 360, s * 100, l * 100];
}

function hslToHex(h: number, s: number, l: number): string {
  const c = (1 - Math.abs(2 * (l / 100) - 1)) * (s / 100);
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l / 100 - c / 2;
  const seg = Math.floor(((h % 360) + 360) % 360 / 60);
  const rgb = [
    [c, x, 0],
    [x, c, 0],
    [0, c, x],
    [0, x, c],
    [x, 0, c],
    [c, 0, x],
  ][seg % 6];
  const hex = rgb
    .map((v) => Math.round((v + m) * 255).toString(16).padStart(2, "0"))
    .join("");
  return `#${hex}`;
}

/** Derives the secondary accent (hue shift + a bit lighter). */
function contrastInk(hex: string): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return "#ffffff";
  const n = parseInt(m[1], 16);
  const toLin = (v: number) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  };
  const lum =
    0.2126 * toLin((n >> 16) & 255) + 0.7152 * toLin((n >> 8) & 255) + 0.0722 * toLin(n & 255);
  return lum > 0.45 ? "#0b0d12" : "#ffffff";
}

export function accentVariant(hex: string): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return hex;
  const n = parseInt(m[1], 16);
  const [h, s, l] = rgbToHsl((n >> 16) & 255, (n >> 8) & 255, n & 255);
  return hslToHex((h + 28) % 360, Math.min(100, s * 0.9 + 12), Math.min(80, l + 12));
}

export type DialogState = {
  kind: "confirm" | "prompt";
  title: string;
  message?: string;
  value?: string;
  placeholder?: string;
  confirmLabel?: string;
  danger?: boolean;
};

function newMsg(init: any): Msg {
  const m = $state(init) as Msg;
  return m;
}
function newPart(init: any): Part {
  const p = $state(init) as Part;
  return p;
}

function extractText(result: any): string {
  if (!result) return "";
  if (typeof result === "string") return result;
  const content = result.content ?? result;
  if (Array.isArray(content)) {
    return content
      .map((c: any) => (c?.type === "text" ? c.text : c?.type === "image" ? "[imagem]" : ""))
      .filter(Boolean)
      .join("\n");
  }
  if (typeof content === "string") return content;
  try {
    return JSON.stringify(result, null, 2);
  } catch {
    return String(result);
  }
}

/** One independent pi process + conversation. */
export class Tab {
  id = $state("");
  label = $state("nova");
  cwd = $state("/home/dante");
  conn = $state<"idle" | "starting" | "ready" | "dead">("idle");
  streaming = $state(false);
  working = $state(false);
  activity = $state<string | null>(null);
  messages = $state<Msg[]>([]);
  models = $state<ModelInfo[]>([]);
  commands = $state<any[]>([]);
  model = $state<ModelInfo | null>(null);
  stats = $state<any>(null);
  sessionId = $state<string | null>(null);
  sessionName = $state<string | null>(null);
  sessionFile = $state<string | null>(null);
  externalChange = $state(false);
  holders = $state<{ pid: number; cmd: string }[]>([]);
  lastLen = $state<number | null>(null);
  pending = $state(0);
  queue = $state<{ steering: string[]; followUp: string[] }>({ steering: [], followUp: [] });
  steeringMode = $state("all");
  followUpMode = $state("one-at-a-time");
  autoCompaction = $state(true);
  autoRetry = $state(true);
  retrying = $state(false);
  thinkingLevels = $state<string[]>(["off", "minimal", "low", "medium", "high", "xhigh", "max"]);
  bashLog = $state("");
  bashRunning = $state(false);
  currentKey = $state<string | null>(null);
  taskStartedAt = $state<number | null>(null);
  thinking = $state("medium");

  constructor(init: { id: string; label?: string; cwd: string; thinking?: string }) {
    this.id = init.id;
    this.label = init.label ?? basename(init.cwd);
    this.cwd = init.cwd;
    if (init.thinking) this.thinking = init.thinking;
  }

  get modelLabel() {
    if (!this.model) return "—";
    return `${this.model.provider}/${this.model.id}`;
  }

  get contextPercent(): number {
    const p = this.stats?.contextUsage?.percent;
    return typeof p === "number" ? Math.min(100, Math.round(p)) : 0;
  }

  get contextTokens(): string {
    const t = this.stats?.contextUsage;
    if (!t || t.tokens == null) return "—";
    const k = (n: number) => (n >= 1000 ? `${(n / 1000).toFixed(1)}k` : `${n}`);
    return `${k(t.tokens)} / ${k(t.contextWindow)}`;
  }

  get cost(): number {
    return Number(this.stats?.cost ?? 0);
  }

  get title(): string {
    if (this.sessionName) return this.sessionName;
    if (this.label && this.label !== "nova") return this.label;
    return this.sessionId?.slice(0, 8) ?? "nova";
  }

  get current(): Msg | null {
    if (!this.currentKey) return null;
    return this.messages.find((m) => m.key === this.currentKey) ?? null;
  }

  ensureCurrent(): Msg {
    const existing = this.current;
    if (existing) return existing;
    const key = uid();
    this.messages.push(
      newMsg({
        key,
        role: "assistant",
        parts: [],
        ts: Date.now(),
        streaming: true,
        model: this.modelLabel,
      }),
    );
    this.currentKey = key;
    return this.current!;
  }

  part(msg: Msg, ci: number, kind: Part["kind"]): Part {
    let p = msg.parts.find((x) => x.ci === ci && x.kind === kind);
    if (!p) {
      p =
        kind === "tool"
          ? newPart({ ci, kind: "tool", id: "", name: "", args: {}, running: true })
          : newPart({ ci, kind: kind as "text" | "thinking", text: "" });
      msg.parts.push(p);
      msg.parts.sort((a, b) => a.ci - b.ci);
      p = msg.parts.find((x) => x.ci === ci && x.kind === kind)!;
    }
    return p;
  }
}

class AppState {
  tabs = $state<Tab[]>([]);
  activeId = $state("");
  chips = $state<Chip[]>([]);
  dialog = $state<DialogState | null>(null);
  private dialogResolve: ((v: any) => void) | null = null;
  toasts = $state<Toast[]>([]);
  requests = $state<UiRequest[]>([]);
  stderr = $state<string[]>([]);
  error = $state<string | null>(null);
  status = $state<string | null>(null);
  skills = $state<SkillInfo[]>([]);
  sessions = $state<SessionInfo[]>([]);
  pinned = $state<string[]>(JSON.parse(localStorage.getItem("pi.pinned") || "[]"));
  showMemory = $state(false);
  showPalette = $state(false);
  showBash = $state(false);
  showFork = $state(false);
  showTree = $state(false);
  draftRestore = $state<string | null>(null);
  focusToken = $state(0);
  bin = $state("pi");
  appVersion = $state("0.1.1");
  gateMode = $state(localStorage.getItem("pi.gate") || "dangerous");
  costLimit = $state(Number(localStorage.getItem("pi.costLimit") || "0"));
  theme = $state(localStorage.getItem("pi.theme") || "midnight");
  accent = $state(localStorage.getItem("pi.accent") || "");
  chatFont = $state(Number(localStorage.getItem("pi.chatFont") || "14"));
  thinking = $state(localStorage.getItem("pi.thinking") || "medium");

  private listeners: UnlistenFn[] = [];
  private pollTimer: number | null = null;
  private waiting = new Map<string, { resolve: (v: any) => void; reject: (e: any) => void; timer: number }>();
  private booted = false;
  private notificationsAllowed: boolean | null = null;

  get active(): Tab {
    return this.tabs.find((t) => t.id === this.activeId) ?? this.tabs[0];
  }

  // ── proxies to the active tab (keeps components simple) ────────────────
  get messages() {
    return this.active?.messages ?? [];
  }
  get models() {
    return this.active?.models ?? [];
  }
  get commands() {
    return this.active?.commands ?? [];
  }
  get model() {
    return this.active?.model ?? null;
  }
  get stats() {
    return this.active?.stats ?? null;
  }
  get conn() {
    return this.active?.conn ?? "idle";
  }
  get streaming() {
    return this.active?.streaming ?? false;
  }
  get working() {
    return this.active?.working ?? false;
  }
  get activity() {
    return this.active?.activity ?? null;
  }
  get pending() {
    return this.active?.pending ?? 0;
  }
  get sessionId() {
    return this.active?.sessionId ?? null;
  }
  get sessionName() {
    return this.active?.sessionName ?? null;
  }
  get cwd() {
    return this.active?.cwd ?? "/home/dante";
  }
  get modelLabel() {
    return this.active?.modelLabel ?? "—";
  }
  get contextPercent() {
    return this.active?.contextPercent ?? 0;
  }
  get contextTokens() {
    return this.active?.contextTokens ?? "—";
  }
  get cost() {
    return this.active?.cost ?? 0;
  }
  get costWarn() {
    return this.costLimit > 0 && this.cost >= this.costLimit;
  }

  // ── tabs ──────────────────────────────────────────────────────────────
  newTab(cwd = localStorage.getItem("pi.cwd") || "/home/dante", label?: string) {
    const tab = new Tab({ id: uid(), cwd, label, thinking: this.thinking });
    this.tabs.push(tab);
    this.activeId = tab.id;
    return tab;
  }

  async openTab(cwd?: string) {
    const tab = this.newTab(cwd ?? undefined);
    await this.connect({ tab });
    return tab;
  }

  activate(id: string) {
    if (this.tabs.some((t) => t.id === id)) this.activeId = id;
  }

  async closeTab(id: string) {
    const idx = this.tabs.findIndex((t) => t.id === id);
    if (idx < 0) return;
    try {
      await api.stop(id);
    } catch {
      /* ignore */
    }
    this.tabs.splice(idx, 1);
    if (this.activeId === id) {
      const next = this.tabs[Math.min(idx, this.tabs.length - 1)];
      if (next) this.activeId = next.id;
      else {
        this.newTab();
        await this.connect();
      }
    }
  }

  // ── lifecycle ─────────────────────────────────────────────────────────
  async boot() {
    if (this.booted) return;
    this.booted = true;
    const urlTheme = new URLSearchParams(location.search).get("theme");
    if (urlTheme && THEMES.some((t) => t.id === urlTheme)) {
      this.theme = urlTheme;
      localStorage.setItem("pi.theme", urlTheme);
    }
    this.applyTheme();
    const panel = new URLSearchParams(location.search).get("panel");
    if (panel === "bash") this.showBash = true;
    if (panel === "fork") this.showFork = true;
    if (panel === "tree") this.showTree = true;
    this.bin = await api.resolveBin();
    try {
      await api.stopAll();
    } catch {
      /* ignore */
    }
    this.listeners.push(
      await listen<any>("pi:record", (e) => {
        const tab = this.tabs.find((t) => t.id === e.payload?.tab);
        if (tab) this.onRecord(tab, e.payload?.payload);
      }),
      await listen<any>("pi:spawned", (e) => {
        const tab = this.tabs.find((t) => t.id === e.payload?.tab);
        if (tab) {
          tab.conn = "ready";
          this.bin = e.payload?.payload?.bin ?? this.bin;
        }
      }),
      await listen<any>("pi:stderr", (e) => {
        const line = e.payload?.payload?.line;
        if (!line) return;
        this.stderr.push(line);
        if (this.stderr.length > 500) this.stderr.splice(0, this.stderr.length - 500);
      }),
      await listen<any>("pi:exit", (e) => {
        const tab = this.tabs.find((t) => t.id === e.payload?.tab);
        if (tab) {
          tab.conn = "dead";
          tab.streaming = false;
          tab.working = false;
          tab.activity = null;
        }
        for (const [key, w] of this.waiting) {
          if (key.startsWith(`${e.payload?.tab}:`)) {
            clearTimeout(w.timer);
            w.reject(new Error("processo pi encerrou"));
            this.waiting.delete(key);
          }
        }
      }),
      await listen<any>("quick-ask", () => {
        this.focusToken++;
      }),
      await listen<any>("quick-new", () => {
        this.newSession();
      }),
    );

    if (!this.tabs.length) this.newTab();
    await this.connect();
    this.skills = await api.skillsList();
    await this.loadSessions();
    this.syncSessionMeta(this.active);
    this.startPolling();
  }

  async connect(opts: { session?: string; cwd?: string; tab?: Tab } = {}) {
    const tab = opts.tab ?? this.active;
    if (!tab) return;
    tab.conn = "starting";
    this.error = null;
    try {
      await api.start(tab.id, {
        cwd: opts.cwd ?? tab.cwd,
        provider: tab.model?.provider,
        model: tab.model?.id,
        thinking: tab.thinking,
        session: opts.session,
        noSession: false,
      });
      tab.conn = "ready";
      const st = await this.procStatus(tab.id);
      if (st?.running) tab.conn = "ready";
      await this.refreshModels(tab);
      await this.refreshState(tab);
      await this.refreshStats(tab);
      tab.commands = (await this.send(tab.id, { type: "get_commands" }))?.commands ?? [];
      void this.loadThinkingLevels(tab);
    } catch (e: any) {
      tab.conn = "dead";
      this.error = String(e?.message ?? e);
    }
  }

  async procStatus(tabId: string) {
    try {
      return await api.status(tabId);
    } catch {
      return null;
    }
  }

  // ── external change detection ─────────────────────────────────────────
  private startPolling() {
    if (this.pollTimer != null) return;
    this.pollTimer = window.setInterval(() => void this.pollSessions(), 8000);
  }

  private async pollSessions() {
    for (const tab of this.tabs) {
      const path = tab.sessionFile;
      if (!path) continue;
      try {
        const st = await api.sessionStat(path);
        if (tab.lastLen == null) {
          tab.lastLen = st.len;
          continue;
        }
        if (st.len !== tab.lastLen) {
          tab.lastLen = st.len;
          // writes while a run is in flight are our own
          if (!tab.working && !tab.streaming) tab.externalChange = true;
        }
      } catch {
        /* ignore */
      }
    }
  }

  /** Records the current file size as "ours" and checks who else holds the file. */
  async syncSessionMeta(tab = this.active, checkHolders = true) {
    if (!tab?.sessionFile) return;
    try {
      const st = await api.sessionStat(tab.sessionFile);
      tab.lastLen = st.len;
      tab.externalChange = false;
      if (checkHolders) {
        const holders = await api.sessionHolders(tab.sessionFile);
        tab.holders = holders;
        if (holders.length) {
          const who = holders.map((h) => `pid ${h.pid}`).join(", ");
          this.toast(`esta sessão também está aberta em outro processo (${who})`, "warn");
        }
      }
    } catch {
      /* ignore */
    }
  }

  async setCwd(path: string) {
    const tab = this.active;
    if (tab.label === basename(tab.cwd)) tab.label = basename(path);
    tab.cwd = path;
    localStorage.setItem("pi.cwd", path);
    await api.stop(tab.id);
    tab.messages = [];
    tab.currentKey = null;
    await this.connect({ cwd: path, tab });
    await this.loadSessions();
  }

  async setGate(mode: string) {
    localStorage.setItem("pi.gate", mode);
    this.gateMode = mode;
    for (const tab of this.tabs) {
      await api.stop(tab.id);
    }
    await this.connect();
  }

  setCostLimit(value: number) {
    this.costLimit = value;
    localStorage.setItem("pi.costLimit", String(value));
  }

  setTheme(theme: string) {
    this.theme = theme;
    localStorage.setItem("pi.theme", theme);
    this.applyTheme();
    const panel = new URLSearchParams(location.search).get("panel");
    if (panel === "bash") this.showBash = true;
    if (panel === "fork") this.showFork = true;
    if (panel === "tree") this.showTree = true;
  }

  private applyTheme() {
    const root = document.documentElement;
    root.dataset.theme = this.theme;
    const accent = this.accent.trim();
    if (accent) {
      root.style.setProperty("--accent", accent);
      root.style.setProperty("--accent-2", accentVariant(accent));
    } else {
      root.style.removeProperty("--accent");
      root.style.removeProperty("--accent-2");
    }
    root.style.setProperty("--chat-font", `${this.chatFont}px`);
    // readable text on top of the accent (buttons, avatar, logo)
    const effective =
      accent || getComputedStyle(root).getPropertyValue("--accent").trim() || "#7aa2f7";
    root.style.setProperty("--accent-ink", contrastInk(effective));
  }

  setAccent(hex: string) {
    this.accent = hex;
    localStorage.setItem("pi.accent", hex);
    this.applyTheme();
    const panel = new URLSearchParams(location.search).get("panel");
    if (panel === "bash") this.showBash = true;
    if (panel === "fork") this.showFork = true;
    if (panel === "tree") this.showTree = true;
  }

  setChatFont(size: number) {
    this.chatFont = size;
    localStorage.setItem("pi.chatFont", String(size));
    this.applyTheme();
    const panel = new URLSearchParams(location.search).get("panel");
    if (panel === "bash") this.showBash = true;
    if (panel === "fork") this.showFork = true;
    if (panel === "tree") this.showTree = true;
  }

  // ── rpc plumbing ──────────────────────────────────────────────────────
  async send(tabId: string, command: any, timeoutMs = 30000): Promise<any> {
    const id = command.id ?? uid();
    const payload = { id, ...command };
    const key = `${tabId}:${id}`;
    return new Promise(async (resolve, reject) => {
      const timer = window.setTimeout(() => {
        this.waiting.delete(key);
        reject(new Error(`timeout em ${command.type}`));
      }, timeoutMs);
      this.waiting.set(key, { resolve, reject, timer });
      try {
        await api.send(tabId, payload);
      } catch (e) {
        clearTimeout(timer);
        this.waiting.delete(key);
        reject(e);
      }
    });
  }

  private reply(tabId: string, id: string, ok: boolean, data: any, error?: string) {
    const key = `${tabId}:${id}`;
    const waiter = this.waiting.get(key);
    if (!waiter) return;
    clearTimeout(waiter.timer);
    this.waiting.delete(key);
    if (ok) waiter.resolve(data);
    else waiter.reject(new Error(error ?? "comando falhou"));
  }

  // ── prompt ────────────────────────────────────────────────────────────
  async prompt(text: string, chips: Chip[] = []) {
    const tab = this.active;
    if (!tab) return;
    if (!text.trim() && chips.length === 0) return;

    const images = chips
      .filter((c) => c.kind === "image" && c.data)
      .map((c) => ({ type: "image", data: c.data, mimeType: c.mime }));

    let message = text.trim();
    const inline: string[] = [];
    for (const chip of chips) {
      if (chip.kind === "text" && chip.text) {
        const fence = chip.text.includes("~~~") ? "````" : "~~~";
        inline.push(`${fence}${langFor(chip.name)}\n// ${chip.name}\n${chip.text}\n${fence}`);
      } else if (chip.kind === "binary") {
        inline.push(
          `[anexo] ${chip.name} (${chip.mime}, ${fmtSize(chip.size)}) em ${chip.stagedPath ?? chip.name} — abra com suas ferramentas se precisar.`,
        );
      } else if (chip.kind === "image") {
        inline.push(`[imagem anexada] ${chip.name}`);
      }
    }
    if (inline.length) message = `${message}\n\n${inline.join("\n\n")}`;

    const userText = text.trim() || chips.map((c) => c.name).join(", ");
    tab.messages.push(
      newMsg({
        key: uid(),
        role: "user",
        parts: [{ ci: 0, kind: "text", text: userText }],
        ts: Date.now(),
        chips,
      }),
    );
    if (tab.label === "nova" || !tab.label) {
      tab.label = userText.replace(/\s+/g, " ").slice(0, 28);
    }

    const command: any = { type: "prompt", message: message || "(anexo)" };
    if (images.length) command.images = images;
    if (tab.streaming || tab.working) command.streamingBehavior = "steer";
    tab.streaming = true;
    tab.working = true;
    tab.activity = "pensando…";
    tab.taskStartedAt = Date.now();
    try {
      await this.send(tab.id, command);
    } catch (e: any) {
      tab.streaming = false;
      tab.working = false;
      tab.activity = null;
      tab.messages.push(
        newMsg({
          key: uid(),
          role: "system",
          parts: [{ ci: 0, kind: "text", text: `Falha ao enviar: ${e?.message ?? e}` }],
          ts: Date.now(),
          error: String(e),
        }),
      );
    }
  }

  async sendActive(command: any) {
    return this.send(this.active.id, command);
  }

  async compact() {
    await this.sendActive({ type: "compact" });
    await this.refreshStats(this.active);
  }

  // ── attachments ───────────────────────────────────────────────────────
  async addPaths(paths: string[]) {
    for (const path of paths) {
      try {
        const a: any = await api.attachmentRead(path);
        this.chips.push({
          name: a.name,
          kind: a.kind,
          mime: a.mime,
          size: a.size,
          preview: a.previewUrl ?? undefined,
          data: a.data ?? undefined,
          text: a.text ?? undefined,
          stagedPath: a.stagedPath ?? undefined,
        });
      } catch (e: any) {
        this.toast(`anexo falhou: ${e?.message ?? e}`, "error");
      }
    }
  }

  async addPastedImage(file: File) {
    const buf = await file.arrayBuffer();
    const bytes = new Uint8Array(buf);
    let bin = "";
    for (let i = 0; i < bytes.length; i += 0x8000) {
      bin += String.fromCharCode.apply(null, Array.from(bytes.subarray(i, i + 0x8000)) as any);
    }
    const b64 = btoa(bin);
    const res: any = await api.attachmentSaveB64(file.name || "", file.type || "image/png", b64);
    this.chips.push({
      name: res.name,
      kind: res.kind,
      mime: res.mime,
      size: res.size,
      preview: res.previewUrl ?? undefined,
      data: res.data ?? undefined,
    });
  }

  removeChip(index: number) {
    this.chips.splice(index, 1);
  }

  clearChips() {
    this.chips = [];
  }

  // ── session ops ───────────────────────────────────────────────────────
  async abort() {
    const tab = this.active;
    try {
      await this.send(tab.id, { type: "abort" });
    } catch {
      /* ignore */
    }
    tab.streaming = false;
    tab.working = false;
    tab.activity = null;
    tab.currentKey = null;
  }

  async newSession() {
    const tab = this.active;
    try {
      await this.send(tab.id, { type: "new_session" });
    } catch {
      /* ignore */
    }
    tab.messages = [];
    tab.currentKey = null;
    tab.streaming = false;
    tab.working = false;
    tab.label = "nova";
    tab.sessionName = null;
    await this.refreshState(tab);
    await this.refreshStats(tab);
    await this.loadSessions();
  }

  /** Re-reads the session file from disk, picking up messages written elsewhere
   *  (e.g. the same session used from the terminal). */
  async reloadSession() {
    const tab = this.active;
    const path = tab.sessionFile;
    if (!path) {
      this.toast("sessão ainda não tem arquivo em disco", "warn");
      return;
    }
    try {
      await this.send(tab.id, { type: "switch_session", sessionPath: path });
      tab.messages = [];
      tab.currentKey = null;
      await this.refreshState(tab);
      const history = await this.send(tab.id, { type: "get_messages" });
      this.hydrate(tab, history?.messages ?? []);
      await this.refreshStats(tab);
      await this.syncSessionMeta(tab, false);
      this.toast("sessão recarregada do disco", "ok");
    } catch (e: any) {
      this.toast(`recarregar falhou: ${e?.message ?? e}`, "error");
    }
  }

  async switchSession(path: string) {
    const tab = this.active;
    await this.send(tab.id, { type: "switch_session", sessionPath: path });
    tab.messages = [];
    tab.currentKey = null;
    await this.refreshState(tab);
    const history = await this.send(tab.id, { type: "get_messages" });
    this.hydrate(tab, history?.messages ?? []);
    await this.refreshStats(tab);
    await this.syncSessionMeta(tab);
  }

  private hydrate(tab: Tab, messages: any[]) {
    for (const m of messages) {
      if (m.role === "system") continue;
      if (m.role === "toolResult" || m.role === "tool") continue;
      const parts: Part[] = [];
      const content =
        typeof m.content === "string" ? [{ type: "text", text: m.content }] : m.content ?? [];
      content.forEach((c: any, i: number) => {
        if (c.type === "text") parts.push(newPart({ ci: i, kind: "text", text: c.text }));
        else if (c.type === "thinking")
          parts.push(newPart({ ci: i, kind: "thinking", text: c.thinking }));
        else if (c.type === "toolCall")
          parts.push(
            newPart({
              ci: i,
              kind: "tool",
              id: c.id,
              name: c.name,
              args: c.arguments,
              running: false,
            }),
          );
      });
      if (!parts.length) continue;
      tab.messages.push(newMsg({ key: uid(), role: m.role, parts, ts: m.timestamp ?? Date.now() }));
    }
  }

  // ── models / thinking / stats ─────────────────────────────────────────
  async refreshModels(tab = this.active) {
    try {
      const data = await this.send(tab.id, { type: "get_available_models" });
      tab.models = data?.models ?? [];
    } catch {
      /* ignore */
    }
  }

  async refreshState(tab = this.active) {
    try {
      const st = await this.send(tab.id, { type: "get_state" });
      if (st?.model) tab.model = st.model;
      if (st?.thinkingLevel) tab.thinking = st.thinkingLevel;
      if (st?.steeringMode) tab.steeringMode = st.steeringMode;
      if (st?.followUpMode) tab.followUpMode = st.followUpMode;
      if (typeof st?.autoCompactionEnabled === "boolean") tab.autoCompaction = st.autoCompactionEnabled;
      tab.sessionId = st?.sessionId ?? null;
      tab.sessionName = st?.sessionName ?? null;
      if (st?.sessionFile) tab.sessionFile = st.sessionFile;
    } catch {
      /* ignore */
    }
  }

  async setModel(provider: string, modelId: string) {
    const tab = this.active;
    const m = await this.send(tab.id, { type: "set_model", provider, modelId });
    if (m) tab.model = m;
  }

  async setThinking(level: string) {
    this.thinking = level;
    localStorage.setItem("pi.thinking", level);
    const tab = this.active;
    tab.thinking = level;
    await this.send(tab.id, { type: "set_thinking_level", level });
  }

  async refreshStats(tab = this.active) {
    try {
      tab.stats = await this.send(tab.id, { type: "get_session_stats" });
    } catch {
      /* ignore */
    }
  }

  async loadSessions() {
    this.sessions = await api.sessionsList(80);
  }

  renameTab(id: string, name: string) {
    const tab = this.tabs.find((t) => t.id === id);
    if (tab) tab.label = name.trim() || basename(tab.cwd);
  }

  // ── revert edit/write ─────────────────────────────────────────────────
  async captureSnapshot(part: any) {
    if (!part || part.snap || part.snapLoading) return;
    if (part.name !== "edit" && part.name !== "write") return;
    const path = part.args?.path;
    if (!path) return;
    part.snapLoading = true;
    try {
      part.snap = await api.fileSnapshot(path);
    } catch {
      /* ignore */
    } finally {
      part.snapLoading = false;
    }
  }

  async revertPart(part: any) {
    const snap = part.snap;
    if (!snap?.canRestore) {
      this.toast("sem snapshot deste arquivo — não dá pra reverter", "warn");
      return;
    }
    try {
      await api.fileRestore(snap.path, snap.exists ? snap.content : null);
      part.reverted = true;
      this.toast(`revertido: ${snap.path}`, "ok");
    } catch (e: any) {
      this.toast(`reverter falhou: ${e?.message ?? e}`, "error");
    }
  }

  togglePin(path: string) {
    if (this.pinned.includes(path)) this.pinned = this.pinned.filter((p) => p !== path);
    else this.pinned = [...this.pinned, path];
    localStorage.setItem("pi.pinned", JSON.stringify(this.pinned));
  }

  isPinned(path: string) {
    return this.pinned.includes(path);
  }

  async renameSession(session: SessionInfo, name: string) {
    const clean = name.trim();
    if (!clean) return;
    try {
      if (this.active?.sessionId === session.id) {
        await this.send(this.active.id, { type: "set_session_name", name: clean });
        this.active.sessionName = clean;
      } else {
        await api.sessionRename(session.path, clean);
      }
      await this.loadSessions();
      this.toast("sessão renomeada", "ok");
    } catch (e: any) {
      this.toast(`renomear falhou: ${e?.message ?? e}`, "error");
    }
  }

  async deleteSession(session: SessionInfo) {
    try {
      if (this.active?.sessionId === session.id) await this.newSession();
      const dest = await api.sessionDelete(session.path);
      this.pinned = this.pinned.filter((p) => p !== session.path);
      localStorage.setItem("pi.pinned", JSON.stringify(this.pinned));
      await this.loadSessions();
      this.toast(`sessão movida para a lixeira: ${dest}`, "ok");
    } catch (e: any) {
      this.toast(`apagar falhou: ${e?.message ?? e}`, "error");
    }
  }

  async loadSkills() {
    this.skills = await api.skillsList();
  }

  /** Dumps everything an agent needs to debug the UI without a screenshot. */
  async dumpDiagnostics() {
    const rect = (sel: string) => {
      const el = document.querySelector(sel) as HTMLElement | null;
      if (!el) return null;
      const r = el.getBoundingClientRect();
      const cs = getComputedStyle(el);
      return {
        x: Math.round(r.x),
        y: Math.round(r.y),
        w: Math.round(r.width),
        h: Math.round(r.height),
        display: cs.display,
        overflowY: cs.overflowY,
      };
    };
    const tab = this.active;
    const payload = {
      at: new Date().toISOString(),
      appVersion: this.appVersion,
      conn: tab?.conn,
      streaming: tab?.streaming,
      working: tab?.working,
      activity: tab?.activity,
      model: tab?.modelLabel,
      thinking: tab?.thinking,
      sessionId: tab?.sessionId,
      sessionName: tab?.sessionName,
      cwd: tab?.cwd,
      gateMode: this.gateMode,
      cost: tab?.cost,
      context: tab?.contextTokens,
      stats: tab?.stats,
      tabs: this.tabs.map((t) => ({
        title: t.title,
        cwd: t.cwd,
        conn: t.conn,
        working: t.working,
        messages: t.messages.length,
      })),
      messages: (tab?.messages ?? []).slice(-6).map((m) => ({
        role: m.role,
        parts: m.parts.map((part: any) => ({
          kind: part.kind,
          name: part.name,
          chars: part.text?.length ?? part.output?.length ?? 0,
          running: part.running,
          error: part.isError,
        })),
        error: m.error ?? null,
      })),
      layout: {
        window: { w: window.innerWidth, h: window.innerHeight },
        bodyScroll: document.body.scrollHeight,
        shell: rect(".shell"),
        sidebar: rect("aside"),
        main: rect("main"),
        tabs: rect(".bar"),
        header: rect("header"),
        controls: rect(".controls"),
        memoryTabs: rect(".tabs"),
        footer: rect("aside .foot"),
        chat: rect(".chat"),
        composer: rect(".composer"),
        composerBox: rect(".box"),
        composerTextarea: rect(".composer textarea"),
        actions: rect(".actions"),
        lastMessage: rect(".chat > div:last-child"),
        memory: rect("section"),
      },
      stderr: this.stderr.slice(-20),
      toasts: this.toasts.map((t) => t.text),
      requests: this.requests.map((r) => r.title),
    };
    try {
      const path = await api.writeDiagnostics(JSON.stringify(payload, null, 2));
      this.toast(`diagnóstico salvo em ${path}`, "ok");
      return path;
    } catch (e: any) {
      this.toast(`falha ao salvar diagnóstico: ${e?.message ?? e}`, "error");
      return null;
    }
  }

  // ── queue / retry / modes ──────────────────────────────────────────────
  async clearQueue() {
    const tab = this.active;
    try {
      const data = await this.send(tab.id, { type: "clear_queue" });
      const texts = [...(data?.steering ?? []), ...(data?.followUp ?? [])];
      tab.queue = { steering: [], followUp: [] };
      tab.pending = 0;
      if (texts.length) {
        this.draftRestore = texts.join("\n\n");
        this.toast(`fila limpa (${texts.length}) — texto devolvido ao campo`, "ok");
      } else {
        this.toast("fila vazia", "info");
      }
    } catch (e: any) {
      this.toast(`limpar fila falhou: ${e?.message ?? e}`, "error");
    }
  }

  async abortRetry() {
    try {
      await this.sendActive({ type: "abort_retry" });
      this.active.retrying = false;
      this.toast("retry cancelado", "ok");
    } catch (e: any) {
      this.toast(`cancelar retry falhou: ${e?.message ?? e}`, "error");
    }
  }

  async setSteeringMode(mode: string) {
    const tab = this.active;
    await this.send(tab.id, { type: "set_steering_mode", mode });
    tab.steeringMode = mode;
  }

  async setFollowUpMode(mode: string) {
    const tab = this.active;
    await this.send(tab.id, { type: "set_follow_up_mode", mode });
    tab.followUpMode = mode;
  }

  async setAutoCompaction(enabled: boolean) {
    const tab = this.active;
    await this.send(tab.id, { type: "set_auto_compaction", enabled });
    tab.autoCompaction = enabled;
  }

  async setAutoRetry(enabled: boolean) {
    const tab = this.active;
    await this.send(tab.id, { type: "set_auto_retry", enabled });
    tab.autoRetry = enabled;
  }

  // ── model / thinking cycling ───────────────────────────────────────────
  async cycleModel() {
    const tab = this.active;
    try {
      const data = await this.send(tab.id, { type: "cycle_model" });
      if (!data?.model) {
        this.toast("só há um modelo disponível", "info");
        return;
      }
      tab.model = data.model;
      if (data.thinkingLevel) tab.thinking = data.thinkingLevel;
      void this.loadThinkingLevels(tab);
      this.toast(`modelo: ${data.model.name ?? data.model.id}`, "ok");
    } catch (e: any) {
      this.toast(`trocar modelo falhou: ${e?.message ?? e}`, "error");
    }
  }

  async cycleThinking() {
    const tab = this.active;
    try {
      const data = await this.send(tab.id, { type: "cycle_thinking_level" });
      if (!data?.level) {
        this.toast("este modelo não suporta raciocínio", "info");
        return;
      }
      tab.thinking = data.level;
      this.toast(`thinking: ${data.level}`, "ok");
    } catch (e: any) {
      this.toast(`trocar thinking falhou: ${e?.message ?? e}`, "error");
    }
  }

  async loadThinkingLevels(tab = this.active) {
    try {
      const data = await this.send(tab.id, { type: "get_available_thinking_levels" });
      if (Array.isArray(data?.levels) && data.levels.length) tab.thinkingLevels = data.levels;
    } catch {
      /* ignore */
    }
  }

  // ── bash runner ────────────────────────────────────────────────────────
  async runBash(command: string, excludeFromContext = false) {
    const tab = this.active;
    tab.bashLog = "";
    tab.bashRunning = true;
    try {
      const data = await this.send(
        tab.id,
        { type: "bash", command, excludeFromContext },
        15 * 60 * 1000,
      );
      return data;
    } finally {
      tab.bashRunning = false;
    }
  }

  async abortBash() {
    try {
      await this.sendActive({ type: "abort_bash" });
    } catch {
      /* ignore */
    }
  }

  // ── fork / clone / tree ────────────────────────────────────────────────
  async forkMessages(): Promise<{ entryId: string; text: string }[]> {
    try {
      const data = await this.sendActive({ type: "get_fork_messages" });
      return data?.messages ?? [];
    } catch (e: any) {
      this.toast(`listar pontos de fork falhou: ${e?.message ?? e}`, "error");
      return [];
    }
  }

  async forkFrom(entryId: string) {
    const tab = this.active;
    try {
      const data = await this.send(tab.id, { type: "fork", entryId });
      if (data?.cancelled) {
        this.toast("fork cancelado por uma extensão", "warn");
        return;
      }
      await this.reloadAfterSessionChange(tab);
      this.toast("fork criado", "ok");
    } catch (e: any) {
      this.toast(`fork falhou: ${e?.message ?? e}`, "error");
    }
  }

  async cloneSession() {
    const tab = this.active;
    try {
      const data = await this.send(tab.id, { type: "clone" });
      if (data?.cancelled) {
        this.toast("clone cancelado", "warn");
        return;
      }
      await this.reloadAfterSessionChange(tab);
      this.toast("sessão clonada", "ok");
    } catch (e: any) {
      this.toast(`clone falhou: ${e?.message ?? e}`, "error");
    }
  }

  async sessionTree(): Promise<{ tree: any[]; leafId: string | null } | null> {
    try {
      const data = await this.sendActive({ type: "get_tree" });
      return { tree: data?.tree ?? [], leafId: data?.leafId ?? null };
    } catch (e: any) {
      this.toast(`árvore falhou: ${e?.message ?? e}`, "error");
      return null;
    }
  }

  private async reloadAfterSessionChange(tab: Tab) {
    tab.messages = [];
    tab.currentKey = null;
    await this.refreshState(tab);
    const history = await this.send(tab.id, { type: "get_messages" });
    this.hydrate(tab, history?.messages ?? []);
    await this.refreshStats(tab);
    await this.loadSessions();
  }

  // ── clipboard ──────────────────────────────────────────────────────────
  async copyLastResponse() {
    try {
      const data = await this.sendActive({ type: "get_last_assistant_text" });
      const text = data?.text;
      if (!text) {
        this.toast("nenhuma resposta de texto ainda", "info");
        return;
      }
      await copyText(text);
      this.toast("última resposta copiada", "ok");
    } catch (e: any) {
      this.toast(`copiar falhou: ${e?.message ?? e}`, "error");
    }
  }

  async exportHtml() {
    const res = await this.sendActive({ type: "export_html" });
    return res?.path as string | undefined;
  }

  async syncMemoryIndex() {
    try {
      await this.sendActive({ type: "prompt", message: "/memory-sync-markdown" });
      this.toast("índice de memória sincronizado", "ok");
    } catch (e: any) {
      this.toast(`sync falhou: ${e?.message ?? e}`, "error");
    }
  }

  // ── ui helpers ────────────────────────────────────────────────────────
  /** In-app confirm (webview's window.confirm is unreliable in WebKitGTK). */
  askConfirm(opts: {
    title: string;
    message?: string;
    confirmLabel?: string;
    danger?: boolean;
  }): Promise<boolean> {
    this.dialog = { kind: "confirm", ...opts };
    return new Promise((resolve) => {
      this.dialogResolve = (v) => resolve(v === true);
    });
  }

  /** In-app text prompt returning null when cancelled. */
  askPrompt(opts: {
    title: string;
    message?: string;
    value?: string;
    placeholder?: string;
    confirmLabel?: string;
  }): Promise<string | null> {
    this.dialog = { kind: "prompt", ...opts };
    return new Promise((resolve) => {
      this.dialogResolve = (v) => resolve(typeof v === "string" ? v : null);
    });
  }

  resolveDialog(value: any) {
    const resolve = this.dialogResolve;
    this.dialog = null;
    this.dialogResolve = null;
    resolve?.(value);
  }

  toast(text: string, kind: Toast["kind"] = "info") {
    const id = uid();
    this.toasts.push({ id, text, kind });
    setTimeout(
      () => {
        this.toasts = this.toasts.filter((t) => t.id !== id);
      },
      kind === "error" ? 7000 : 3800,
    );
  }

  async respond(req: UiRequest, payload: Record<string, any>) {
    this.requests = this.requests.filter((r) => r.id !== req.id);
    try {
      await this.sendActive({ type: "extension_ui_response", id: req.id, ...payload });
    } catch (e: any) {
      this.toast(`resposta falhou: ${e?.message ?? e}`, "error");
    }
  }

  private async notifyDone(text: string) {
    if (this.notificationsAllowed === null) {
      let granted = await isPermissionGranted();
      if (!granted) granted = (await requestPermission()) === "granted";
      this.notificationsAllowed = granted;
    }
    if (!this.notificationsAllowed) return;
    try {
      await sendNotification({ title: "Hina · tarefa concluída", body: text.slice(0, 160) });
    } catch {
      /* ignore */
    }
  }

  private handleUiRequest(rec: any) {
    if (rec.method === "notify") {
      this.toast(rec.message ?? "", rec.notifyType === "warning" ? "warn" : "info");
      return;
    }
    if (rec.method === "setStatus") {
      this.status = rec.status ?? null;
      return;
    }
    if (rec.method === "setTitle") {
      document.title = `Hina · ${rec.title ?? ""}`;
      return;
    }
    if (["select", "confirm", "input", "editor"].includes(rec.method)) {
      this.requests.push({
        id: rec.id,
        method: rec.method,
        title: rec.title,
        message: rec.message,
        options: rec.options,
        placeholder: rec.placeholder,
        prefill: rec.prefill,
      });
    }
  }

  // ── event handling ────────────────────────────────────────────────────
  private onRecord(tab: Tab, rec: any) {
    if (rec?.type === "response") {
      this.reply(tab.id, rec.id, rec.success !== false, rec.data, rec.error);
      return;
    }
    if (rec?.type === "extension_ui_request") {
      this.handleUiRequest(rec);
      return;
    }

    switch (rec.type) {
      case "agent_start":
        tab.streaming = true;
        tab.working = true;
        tab.activity = "pensando…";
        break;

      case "message_start": {
        const m = rec.message;
        if (m?.role === "assistant") {
          const key = uid();
          tab.messages.push(
            newMsg({
              key,
              role: "assistant",
              parts: [],
              ts: m.timestamp ?? Date.now(),
              streaming: true,
              model: tab.modelLabel,
            }),
          );
          tab.currentKey = key;
        }
        break;
      }

      case "message_update": {
        const ev = rec.assistantMessageEvent;
        if (!ev) break;
        const msg = tab.ensureCurrent();
        const ci = ev.contentIndex ?? 0;
        if (ev.type === "text_delta") {
          (tab.part(msg, ci, "text") as any).text += ev.delta ?? "";
          tab.activity = "escrevendo…";
        } else if (ev.type === "text_end") {
          (tab.part(msg, ci, "text") as any).text = ev.content ?? "";
        } else if (ev.type === "thinking_delta") {
          (tab.part(msg, ci, "thinking") as any).text += ev.delta ?? "";
          tab.activity = "pensando…";
        } else if (ev.type === "thinking_end") {
          (tab.part(msg, ci, "thinking") as any).text = ev.content ?? "";
        } else if (ev.type === "toolcall_start") {
          const p: any = tab.part(msg, ci, "tool");
          p.id = ev.id ?? p.id;
          p.name = ev.toolName ?? p.name;
          tab.activity = `preparando ${ev.toolName}…`;
        } else if (ev.type === "toolcall_end" && ev.toolCall) {
          const p: any = tab.part(msg, ci, "tool");
          p.id = ev.toolCall.id;
          p.name = ev.toolCall.name;
          p.args = ev.toolCall.arguments;
          this.captureSnapshot(p);
        } else if (ev.type === "error") {
          msg.error = String(ev.error ?? ev.reason ?? "erro no stream");
        }
        break;
      }

      case "message_end": {
        const m = rec.message;
        if (m?.role !== "assistant") break;
        const msg = tab.current ?? tab.ensureCurrent();
        const content =
          typeof m.content === "string" ? [{ type: "text", text: m.content }] : m.content ?? [];
        const rebuilt: Part[] = [];
        content.forEach((c: any, i: number) => {
          if (c.type === "text" && c.text) rebuilt.push(newPart({ ci: i, kind: "text", text: c.text }));
          else if (c.type === "thinking" && c.thinking)
            rebuilt.push(newPart({ ci: i, kind: "thinking", text: c.thinking }));
          else if (c.type === "toolCall") {
            const existing = msg.parts.find((p: any) => p.kind === "tool" && p.id === c.id) as any;
            rebuilt.push(
              newPart({
                ci: i,
                kind: "tool",
                id: c.id,
                name: c.name,
                args: c.arguments,
                output: existing?.output,
                isError: existing?.isError,
                durationMs: existing?.durationMs,
                running: existing ? existing.running : false,
              }),
            );
          }
        });
        if (rebuilt.length) msg.parts = rebuilt;
        msg.streaming = false;
        if (m.stopReason === "error") msg.error = m.errorMessage ?? "erro";
        tab.currentKey = null;
        break;
      }

      case "tool_execution_start": {
        const msg = tab.ensureCurrent();
        const existing = msg.parts.find((p: any) => p.kind === "tool" && p.id === rec.toolCallId) as any;
        if (existing) {
          existing.name = rec.toolName;
          existing.args = rec.args;
          existing.running = true;
          this.captureSnapshot(existing);
        } else {
          const created = newPart({
            ci: 1000 + msg.parts.length,
            kind: "tool",
            id: rec.toolCallId,
            name: rec.toolName,
            args: rec.args,
            running: true,
          }) as any;
          msg.parts.push(created);
          this.captureSnapshot(created);
        }
        tab.activity = `executando ${rec.toolName}…`;
        break;
      }

      case "tool_execution_update": {
        const msg = tab.current;
        const p = msg?.parts.find((x: any) => x.kind === "tool" && x.id === rec.toolCallId) as any;
        if (p) p.output = extractText(rec.partialResult);
        break;
      }

      case "tool_execution_end": {
        const msg = tab.current;
        const p = msg?.parts.find((x: any) => x.kind === "tool" && x.id === rec.toolCallId) as any;
        if (p) {
          p.running = false;
          p.output = extractText(rec.result);
          p.isError = !!rec.isError;
          p.durationMs = rec.durationMs;
          const imgs = extractImages(rec.result);
          if (imgs.length) p.images = imgs;
        }
        tab.activity = "pensando…";
        break;
      }

      case "turn_end":
        tab.currentKey = null;
        break;

      case "agent_settled": {
        const wasWorking = tab.working;
        tab.streaming = false;
        tab.working = false;
        tab.activity = null;
        for (const m of tab.messages) if (m.streaming) m.streaming = false;
        tab.currentKey = null;
        this.refreshStats(tab);
        this.loadSessions();
        this.syncSessionMeta(tab, false);
        const elapsed = tab.taskStartedAt ? Date.now() - tab.taskStartedAt : 0;
        tab.taskStartedAt = null;
        if (wasWorking && elapsed > 12000 && !document.hasFocus()) {
          const last = [...tab.messages].reverse().find((m) => m.role === "assistant");
          const text = last?.parts.map((p: any) => p.text ?? "").join(" ") ?? "concluído";
          this.notifyDone(text.replace(/\s+/g, " ").trim() || "concluído");
        }
        break;
      }

      case "thinking_level_changed":
        tab.thinking = rec.level;
        break;

      case "session_info_changed":
        tab.sessionName = rec.name ?? null;
        break;

      case "queue_update":
        tab.queue = { steering: rec.steering ?? [], followUp: rec.followUp ?? [] };
        tab.pending = tab.queue.steering.length + tab.queue.followUp.length;
        break;

      case "bash_execution_update":
        tab.bashLog += rec.delta ?? "";
        break;

      case "compaction_start":
        tab.messages.push(
          newMsg({
            key: uid(),
            role: "system",
            parts: [{ ci: 0, kind: "text", text: `compactando contexto (${rec.reason})…` }],
            ts: Date.now(),
          }),
        );
        break;

      case "auto_retry_end":
        tab.retrying = false;
        break;

      case "auto_retry_start":
        tab.retrying = true;
        tab.messages.push(
          newMsg({
            key: uid(),
            role: "system",
            parts: [
              {
                ci: 0,
                kind: "text",
                text: `retry ${rec.attempt}/${rec.maxAttempts}: ${rec.errorMessage ?? ""}`,
              },
            ],
            ts: Date.now(),
          }),
        );
        break;

      case "extension_error":
        this.stderr.push(`[extension] ${rec.extensionPath}: ${rec.error}`);
        this.toast(`extensão: ${rec.error}`, "error");
        break;

      default:
        break;
    }
  }
}

export function langFor(name: string): string {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  const map: Record<string, string> = {
    rs: "rust",
    ts: "typescript",
    tsx: "tsx",
    js: "javascript",
    jsx: "jsx",
    py: "python",
    cs: "csharp",
    json: "json",
    md: "markdown",
    html: "html",
    css: "css",
    xml: "xml",
    yaml: "yaml",
    yml: "yaml",
    toml: "toml",
    sh: "bash",
    sql: "sql",
    cpp: "cpp",
    h: "c",
    c: "c",
  };
  return map[ext] ?? "";
}

export async function copyText(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    return;
  } catch {
    /* fall through to the textarea trick (WebKitGTK) */
  }
  const ta = document.createElement("textarea");
  ta.value = text;
  ta.style.position = "fixed";
  ta.style.opacity = "0";
  document.body.appendChild(ta);
  ta.select();
  const ok = document.execCommand("copy");
  document.body.removeChild(ta);
  if (!ok) throw new Error("clipboard indisponível");
}

export function basename(path: string): string {
  const clean = (path ?? "").replace(/\/+$/, "");
  const parts = clean.split("/");
  return parts[parts.length - 1] || "/";
}

export function fmtSize(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

export const app = new AppState();
