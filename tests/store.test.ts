import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (cmd: string, args?: any) => invokeMock(cmd, args),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async () => () => {},
}));
vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: async () => true,
  requestPermission: async () => "granted",
  sendNotification: async () => {},
}));

import { app, basename, type Tab } from "../src/lib/store.svelte";

const uid = () => Math.random().toString(36).slice(2);

function freshTab(cwd = "/home/dante/Documents/pi-desktop"): Tab {
  app.tabs = [];
  const tab = app.newTab(cwd);
  (app as any).activeId = tab.id;
  return tab;
}

/** Feeds a raw RPC record into the store, like the Tauri event listener does. */
function record(tab: Tab, rec: any) {
  (app as any).onRecord(tab, rec);
}

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockImplementation(async (cmd: string, args: any = {}) => {
    if (cmd === "file_snapshot")
      return { path: args.path, exists: true, content: "conteudo-antigo", size: 15, canRestore: true };
    if (cmd === "file_restore") return null;
    if (cmd === "pi_stop" || cmd === "pi_stop_all") return null;
    if (cmd === "sessions_list") return [];
    if (cmd === "skills_list") return [];
    return { running: true, pid: 1, bin: "pi" };
  });
});

describe("streaming", () => {
  it("accumulates text deltas and replaces them with the final message", () => {
    const tab = freshTab();
    record(tab, {
      type: "message_start",
      message: { role: "assistant", content: [], timestamp: 1 },
    });
    record(tab, {
      type: "message_update",
      assistantMessageEvent: { type: "text_start", contentIndex: 0 },
    });
    for (const delta of ["Ol", "á, ", "Dante"]) {
      record(tab, {
        type: "message_update",
        assistantMessageEvent: { type: "text_delta", contentIndex: 0, delta },
      });
    }

    const live = tab.messages[tab.messages.length - 1];
    expect(live.role).toBe("assistant");
    expect((live.parts[0] as any).text).toBe("Olá, Dante");

    record(tab, {
      type: "message_end",
      message: {
        role: "assistant",
        content: [{ type: "text", text: "Olá, Dante (final)" }],
        stopReason: "stop",
      },
    });

    expect((tab.messages[tab.messages.length - 1].parts[0] as any).text).toBe("Olá, Dante (final)");
    expect(tab.currentKey).toBeNull();
  });

  it("streams thinking blocks separately from text", () => {
    const tab = freshTab();
    record(tab, { type: "message_start", message: { role: "assistant", content: [] } });
    record(tab, {
      type: "message_update",
      assistantMessageEvent: { type: "thinking_delta", contentIndex: 0, delta: "hmm" },
    });
    record(tab, {
      type: "message_update",
      assistantMessageEvent: { type: "text_delta", contentIndex: 1, delta: "resposta" },
    });
    const parts: any[] = tab.messages[tab.messages.length - 1].parts;
    expect(parts.find((p) => p.kind === "thinking").text).toBe("hmm");
    expect(parts.find((p) => p.kind === "text").text).toBe("resposta");
  });
});

describe("tool calls", () => {
  it("creates a running tool part with args and duration", () => {
    const tab = freshTab();
    record(tab, { type: "message_start", message: { role: "assistant", content: [] } });
    record(tab, {
      type: "message_update",
      assistantMessageEvent: {
        type: "toolcall_end",
        contentIndex: 2,
        toolCall: { id: "call-1", name: "bash", arguments: { command: "ls" } },
      },
    });
    record(tab, {
      type: "tool_execution_start",
      toolCallId: "call-1",
      toolName: "bash",
      args: { command: "ls" },
    });

    let part: any = tab.messages[tab.messages.length - 1].parts.find(
      (p) => p.kind === "tool" && p.id === "call-1",
    );
    expect(part.running).toBe(true);
    expect(part.args.command).toBe("ls");

    record(tab, {
      type: "tool_execution_end",
      toolCallId: "call-1",
      toolName: "bash",
      result: { content: [{ type: "text", text: "a.txt\nb.txt" }] },
      isError: false,
      durationMs: 42,
    });

    part = tab.messages[tab.messages.length - 1].parts.find(
      (p) => p.kind === "tool" && p.id === "call-1",
    );
    expect(part.running).toBe(false);
    expect(part.output).toBe("a.txt\nb.txt");
    expect(part.durationMs).toBe(42);
  });

  it("snapshots the target file before an edit and can revert it", async () => {
    const tab = freshTab();
    record(tab, { type: "message_start", message: { role: "assistant", content: [] } });
    record(tab, {
      type: "message_update",
      assistantMessageEvent: {
        type: "toolcall_end",
        contentIndex: 0,
        toolCall: {
          id: "call-2",
          name: "edit",
          arguments: { path: "/tmp/proj/a.txt", edits: [] },
        },
      },
    });

    await vi.waitFor(() => {
      const part: any = tab.messages[0].parts.find((p) => p.kind === "tool");
      expect(part.snap?.canRestore).toBe(true);
    });

    const part: any = tab.messages[0].parts.find((p) => p.kind === "tool");
    expect(invokeMock).toHaveBeenCalledWith("file_snapshot", { path: "/tmp/proj/a.txt" });

    await app.revertPart(part);
    expect(invokeMock).toHaveBeenCalledWith("file_restore", {
      path: "/tmp/proj/a.txt",
      content: "conteudo-antigo",
    });
    expect(part.reverted).toBe(true);
  });

  it("never snapshots read-only tools", () => {
    const tab = freshTab();
    record(tab, { type: "message_start", message: { role: "assistant", content: [] } });
    record(tab, {
      type: "message_update",
      assistantMessageEvent: {
        type: "toolcall_end",
        contentIndex: 0,
        toolCall: { id: "call-3", name: "read", arguments: { path: "/etc/hosts" } },
      },
    });
    expect(invokeMock).not.toHaveBeenCalledWith("file_snapshot", expect.anything());
  });
});

describe("tabs", () => {
  it("names a tab after its project folder and renames on cwd change", async () => {
    const tab = freshTab("/home/dante/Documents/pi-desktop");
    expect(tab.label).toBe("pi-desktop");
    expect(tab.title).toBe("pi-desktop");

    // keep the test offline: setCwd restarts the sidecar, which needs a live RPC
    vi.spyOn(app as any, "connect").mockResolvedValue(undefined);
    await app.setCwd("/home/dante/Documents/outro-projeto");
    expect(app.active.label).toBe("outro-projeto");
  });

  it("keeps an explicit name when the project changes", () => {
    const tab = freshTab("/home/dante/Documents/pi-desktop");
    app.renameTab(tab.id, "minha aba");
    expect(tab.title).toBe("minha aba");
  });

  it("basename handles trailing slashes and root", () => {
    expect(basename("/home/dante/proj/")).toBe("proj");
    expect(basename("/")).toBe("/");
  });
});

describe("in-app dialogs", () => {
  it("askConfirm resolves true on accept and false on cancel", async () => {
    const accepted = app.askConfirm({ title: "apagar?", danger: true });
    expect(app.dialog?.kind).toBe("confirm");
    expect(app.dialog?.danger).toBe(true);
    app.resolveDialog(true);
    await expect(accepted).resolves.toBe(true);

    const cancelled = app.askConfirm({ title: "apagar?" });
    app.resolveDialog(null);
    await expect(cancelled).resolves.toBe(false);
    expect(app.dialog).toBeNull();
  });

  it("askPrompt returns text or null", async () => {
    const typed = app.askPrompt({ title: "nome", value: "atual" });
    expect(app.dialog?.value).toBe("atual");
    app.resolveDialog("novo nome");
    await expect(typed).resolves.toBe("novo nome");

    const cancelled = app.askPrompt({ title: "nome" });
    app.resolveDialog(null);
    await expect(cancelled).resolves.toBeNull();
  });

  it("a second dialog replaces the first only after it resolves", async () => {
    const first = app.askConfirm({ title: "primeiro" });
    app.resolveDialog(false);
    await expect(first).resolves.toBe(false);
    const second = app.askConfirm({ title: "segundo" });
    app.resolveDialog(true);
    await expect(second).resolves.toBe(true);
  });
});

describe("sidebar navigation", () => {
  function stubRpc() {
    vi.spyOn(app as any, "connect").mockResolvedValue(undefined as any);
    vi.spyOn(app as any, "send").mockResolvedValue({ messages: [] } as any);
    vi.spyOn(app as any, "syncSessionMeta").mockResolvedValue(undefined as any);
    vi.spyOn(app as any, "loadSessionInto").mockResolvedValue(undefined as any);
  }

  it("opens a busy tab's session in a new tab, spawning it with --session", async () => {
    const tab = freshTab();
    tab.working = true;
    stubRpc();
    const before = app.tabs.length;
    await app.switchSession("/home/dante/.pi/agent/sessions/a.jsonl");
    expect(app.tabs.length).toBe(before + 1);
    expect(app.tabs[0].working).toBe(true);
    // o novo tab já nasce apontando para a sessão (sem criar arquivo órfão)
    expect((app as any).connect).toHaveBeenCalledWith(
      expect.objectContaining({ session: "/home/dante/.pi/agent/sessions/a.jsonl" }),
    );
  });

  it("reuses the tab when it is idle", async () => {
    const tab = freshTab();
    tab.working = false;
    stubRpc();
    const before = app.tabs.length;
    await app.switchSession("/home/dante/.x.jsonl");
    expect(app.tabs.length).toBe(before);
  });

  it("focuses the existing tab when the session is already open", async () => {
    const tab = freshTab();
    stubRpc();
    tab.sessionFile = "/home/dante/.pi/agent/sessions/b.jsonl";
    const other = app.newTab("/home/dante");
    app.activeId = other.id;
    const before = app.tabs.length;
    await app.switchSession(tab.sessionFile!);
    expect(app.tabs.length).toBe(before);
    expect(app.activeId).toBe(tab.id);
  });
});

describe("local slash commands", () => {
  it("/yolo turns the gate off and /safe restores it", async () => {
    freshTab();
    const spy = vi.spyOn(app as any, "setGate").mockResolvedValue(undefined as any);
    expect(await app.runLocalCommand("/yolo")).toBe(true);
    expect(spy).toHaveBeenCalledWith("off");
    expect(await app.runLocalCommand("/safe")).toBe(true);
    expect(spy).toHaveBeenCalledWith("dangerous");
    expect(await app.runLocalCommand("/not-a-command")).toBe(false);
  });

  it("lists app commands for the slash menu", () => {
    const names = app.localCommands.map((c) => c.name);
    expect(names).toContain("yolo");
    expect(names).toContain("safe");
  });
});

describe("themes", () => {
  it("applies the preset and a custom accent on the root element", () => {
    app.setTheme("hacker");
    expect(document.documentElement.dataset.theme).toBe("hacker");

    app.setAccent("#ff2e97");
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#ff2e97");
    expect(document.documentElement.style.getPropertyValue("--accent-2")).toMatch(/^#[0-9a-f]{6}$/i);

    app.setAccent("");
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("");
  });

  it("controls the chat font size", () => {
    app.setChatFont(16);
    expect(document.documentElement.style.getPropertyValue("--chat-font")).toBe("16px");
    app.setChatFont(14);
  });

  it("derives a readable ink for light and dark accents", async () => {
    const { accentVariant } = await import("../src/lib/store.svelte");
    expect(accentVariant("#39ff14")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(accentVariant("not-a-color")).toBe("not-a-color");
  });
});

describe("extended RPC surface", () => {
  function respond(tab: Tab, data: any) {
    const call = invokeMock.mock.calls.at(-1)!;
    const command = (call[1] as any).command;
    record(tab, { type: "response", id: command.id, command: command.type, success: true, data });
  }

  it("cycles to the next model and thinking level", async () => {
    const tab = freshTab();
    const p = app.cycleModel();
    respond(tab, { model: { id: "m2", provider: "p", name: "M2" }, thinkingLevel: "high" });
    await p;
    expect(app.active.model?.id).toBe("m2");
    expect(app.active.thinking).toBe("high");
  });

  it("applies steering / follow-up modes and auto toggles", async () => {
    freshTab();
    const a = app.setSteeringMode("one-at-a-time");
    respond(app.active, {});
    await a;
    expect(app.active.steeringMode).toBe("one-at-a-time");

    const b = app.setAutoCompaction(false);
    respond(app.active, {});
    await b;
    expect(app.active.autoCompaction).toBe(false);
  });

  it("clear_queue returns the texts to the composer draft", async () => {
    const tab = freshTab();
    const p = app.clearQueue();
    respond(tab, { steering: ["mude de plano"], followUp: ["depois resuma"] });
    await p;
    expect(app.draftRestore).toContain("mude de plano");
    expect(app.active.pending).toBe(0);
    app.draftRestore = null;
  });

  it("streams bash output into the tab", () => {
    const tab = freshTab();
    record(tab, { type: "bash_execution_update", id: "b1", delta: "line1\n" });
    record(tab, { type: "bash_execution_update", id: "b1", delta: "line2\n" });
    expect(tab.bashLog).toBe("line1\nline2\n");
  });

  it("tracks retry state", () => {
    const tab = freshTab();
    record(tab, { type: "auto_retry_start", attempt: 1, maxAttempts: 3, delayMs: 10 });
    expect(tab.retrying).toBe(true);
    record(tab, { type: "auto_retry_end", success: true, attempt: 1 });
    expect(tab.retrying).toBe(false);
  });
});

describe("run lifecycle", () => {
  it("flips working off when the agent settles", () => {
    const tab = freshTab();
    record(tab, { type: "agent_start" });
    expect(tab.working).toBe(true);
    record(tab, { type: "agent_settled", aborted: false });
    expect(tab.working).toBe(false);
    expect(tab.streaming).toBe(false);
    expect(tab.activity).toBeNull();
  });

  it("routes command responses by tab and id", async () => {
    const tab = freshTab();
    const pending = app.send(tab.id, { type: "get_state" });
    const id = (invokeMock.mock.calls.at(-1)![1] as any).command.id;
    record(tab, { type: "response", id, command: "get_state", success: true, data: { sessionId: "abc" } });
    await expect(pending).resolves.toEqual({ sessionId: "abc" });
  });
});
