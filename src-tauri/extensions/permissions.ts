/**
 * Pi Desktop — permission gate.
 *
 * Asks the desktop UI before running dangerous tools. The UI answers through
 * the RPC extension-UI subprotocol (`extension_ui_response`).
 *
 * Modes (env PI_DESKTOP_GATE):
 *   off        — never ask
 *   dangerous  — bash with risky patterns, writes/deletes outside the workspace
 *   all        — every bash/write/edit call
 */

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import path from "node:path";

const MODE = (process.env.PI_DESKTOP_GATE ?? "dangerous").toLowerCase();

const DANGEROUS = [
  /\brm\s+(-[a-z]*[rf][a-z]*|--recursive|--force)/i,
  /\bmkfs\b/i,
  /\bdd\s+if=/i,
  /\bsudo\b/i,
  /\b(chmod|chown)\b.*(-R|777)/i,
  /\bgit\s+push\b.*(--force|-f)\b/i,
  /\bgit\s+reset\s+--hard\b/i,
  /\b(shutdown|reboot|poweroff)\b/i,
  /:\s*\(\s*\)\s*\{/, // fork bomb
  /\bcurl\b[^|]*\|\s*(ba)?sh/i,
  /\btruncate\s+-s\s*0\b/i,
];

const WRITE_TOOLS = new Set(["write", "edit"]);
const SHELL_TOOLS = new Set(["bash"]);

function outsideWorkspace(target: string, cwd: string): boolean {
  if (!target) return false;
  const abs = path.isAbsolute(target) ? target : path.resolve(cwd, target);
  const roots = [cwd, "/tmp", path.join(process.env.HOME ?? "", ".cache/pi-desktop")];
  return !roots.some((r) => abs === r || abs.startsWith(r + path.sep));
}

export default function (pi: ExtensionAPI) {
  if (MODE === "off") return;

  pi.on("tool_call", async (event, ctx) => {
    const name = event.toolName;
    const input = (event.input ?? {}) as Record<string, any>;
    let reason = "";
    let detail = "";

    if (SHELL_TOOLS.has(name)) {
      const command = String(input.command ?? "");
      const hit = DANGEROUS.find((p) => p.test(command));
      if (hit) {
        reason = "potentially destructive command";
        detail = command;
      } else if (MODE === "all") {
        reason = "bash (mode: approve all)";
        detail = command;
      }
    }

    if (!reason && WRITE_TOOLS.has(name) && MODE !== "all") {
      const target = String(input.path ?? input.file_path ?? "");
      if (target && outsideWorkspace(target, ctx.cwd ?? process.cwd())) {
        reason = "write outside the workspace";
        detail = target;
      }
    }

    if (!reason && WRITE_TOOLS.has(name) && MODE === "all") {
      reason = "write (mode: approve all)";
      detail = String(input.path ?? "");
    }

    if (!reason) return undefined;

    if (!ctx.hasUI) {
      return { block: true, reason: `blocked without UI: ${reason}` };
    }

    const command = detail.length > 400 ? detail.slice(0, 400) + "…" : detail;
    const choice = await ctx.ui.select(
      `Allow ${name}?\n\n${reason}\n\n${command}`,
      ["Allow", "Allow and stop asking", "Block"],
    );

    if (choice === "Allow") return undefined;
    if (choice === "Allow and stop asking") {
      // remember for this process
      allowAll = true;
      return undefined;
    }
    return { block: true, reason: "blocked by the user" };
  });

  pi.on("tool_call", async () => {
    if (allowAll) return undefined;
    return undefined;
  });
}

let allowAll = false;
