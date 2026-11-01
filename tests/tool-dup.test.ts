import { describe, expect, it, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({ invoke: () => {} }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));
vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: async () => true,
  requestPermission: async () => "granted",
  sendNotification: async () => {},
}));
import { app } from "../src/lib/store.svelte";

describe("tool cards do not duplicate", () => {
  it("findToolMsg returns the message that owns the tool call", () => {
    const tab: any = (app as any).newTab("/home/dante");
    // mensagem que streamou texto + toolcall
    (app as any).hydrate(tab, [
      {
        role: "assistant",
        timestamp: 1,
        content: [
          { type: "thinking", thinking: "pensando" },
          { type: "text", text: "Resposta com texto." },
          { type: "toolCall", id: "call_1", name: "bash", arguments: { command: "ls" } },
        ],
      },
    ]);
    (tab as any).currentKey = null; // message_end zera o ponteiro
    const before = tab.messages.length;
    const owner = (tab as any).findToolMsg("call_1");
    const dump = { before, ownerFound: !!owner, sameMsg: owner === tab.messages[0] };
    expect(dump.ownerFound).toBe(true);
    expect(dump.sameMsg).toBe(true);
    expect((tab as any).findToolMsg("nao_existe")).toBe(null);
  });
});
