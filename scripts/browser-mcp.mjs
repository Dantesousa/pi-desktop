#!/usr/bin/env node
/**
 * Hina browser MCP server.
 *
 * Gives the agent a real browser (Playwright Firefox) over stdio MCP, while
 * mirroring every frame to ~/.cache/pi-desktop/browser/latest.jpg so the desktop
 * app can show what the agent is doing.
 */
import { firefox } from "playwright-core";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const CACHE = path.join(
  process.env.XDG_CACHE_HOME || path.join(os.homedir(), ".cache"),
  "pi-desktop",
  "browser",
);
const STATE = path.join(CACHE, "state.json");
const SHOT = path.join(CACHE, "latest.jpg");
fs.mkdirSync(CACHE, { recursive: true });

let browser = null;
let page = null;
let launchError = null;

function pickFirefox() {
  const root = path.join(os.homedir(), ".cache", "ms-playwright");
  try {
    const dirs = fs
      .readdirSync(root)
      .filter((d) => d.startsWith("firefox-"))
      .sort();
    if (!dirs.length) return undefined;
    const exe = path.join(root, dirs[dirs.length - 1], "firefox", "firefox");
    return fs.existsSync(exe) ? exe : undefined;
  } catch {
    return undefined;
  }
}

async function ensurePage() {
  if (page && !page.isClosed()) return page;
  if (!browser) {
    const executablePath = process.env.PLAYWRIGHT_FIREFOX_EXECUTABLE || pickFirefox();
    browser = await firefox.launch({ executablePath, headless: true });
  }
  const context = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    locale: "en-US",
  });
  page = await context.newPage();
  return page;
}

async function publish(extra = {}) {
  try {
    if (!page || page.isClosed()) return;
    const info = await page.evaluate(() => ({
      title: document.title,
      url: location.href,
      text: (document.body?.innerText ?? "").slice(0, 4000),
      links: Array.from(document.querySelectorAll("a[href]"))
        .slice(0, 60)
        .map((a) => ({ text: (a.textContent ?? "").trim().slice(0, 80), href: a.href })),
      forms: Array.from(document.querySelectorAll("input,textarea,select,button"))
        .slice(0, 40)
        .map((el) => ({
          tag: el.tagName.toLowerCase(),
          type: el.getAttribute("type") ?? "",
          name: el.getAttribute("name") ?? "",
          id: el.id ?? "",
          placeholder: el.getAttribute("placeholder") ?? "",
        })),
    }));
    const buf = await page.screenshot({ type: "jpeg", quality: 70, fullPage: false });
    fs.writeFileSync(SHOT, buf);
    fs.writeFileSync(
      STATE,
      JSON.stringify(
        { ...info, updated: Date.now(), viewport: { w: 1280, h: 800 }, ...extra },
        null,
        2,
      ),
    );
  } catch {
    /* ignore publishing errors */
  }
}

const tools = [
  {
    name: "browser_navigate",
    description: "Opens a URL in the embedded browser and waits for it to load.",
    inputSchema: {
      type: "object",
      properties: { url: { type: "string" }, waitMs: { type: "number" } },
      required: ["url"],
    },
  },
  {
    name: "browser_snapshot",
    description:
      "Reads the current page: title, URL, visible text, links and form fields. Use before clicking.",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "browser_screenshot",
    description: "Takes a screenshot of the page (the user also sees it in the app).",
    inputSchema: {
      type: "object",
      properties: { fullPage: { type: "boolean" } },
    },
  },
  {
    name: "browser_click",
    description:
      "Clicks by CSS selector, by visible text, or at the centre of an element found by the snapshot.",
    inputSchema: {
      type: "object",
      properties: {
        selector: { type: "string" },
        text: { type: "string" },
        index: { type: "number" },
      },
    },
  },
  {
    name: "browser_type",
    description: "Fills a field (selector or label/placeholder) and optionally submits with Enter.",
    inputSchema: {
      type: "object",
      properties: {
        selector: { type: "string" },
        text: { type: "string" },
        value: { type: "string" },
        submit: { type: "boolean" },
      },
      required: ["value"],
    },
  },
  {
    name: "browser_press",
    description: "Presses a key (Enter, Escape, PageDown, Tab...).",
    inputSchema: {
      type: "object",
      properties: { key: { type: "string" } },
      required: ["key"],
    },
  },
  {
    name: "browser_scroll",
    description: "Scrolls the page in pixels (positive = down).",
    inputSchema: {
      type: "object",
      properties: { dy: { type: "number" } },
      required: ["dy"],
    },
  },
  {
    name: "browser_eval",
    description: "Runs JavaScript on the page and returns the result (JSON).",
    inputSchema: {
      type: "object",
      properties: { js: { type: "string" } },
      required: ["js"],
    },
  },
  {
    name: "browser_wait",
    description: "Waits for some time (ms) or for a selector to appear.",
    inputSchema: {
      type: "object",
      properties: { ms: { type: "number" }, selector: { type: "string" } },
    },
  },
  {
    name: "browser_back",
    description: "Goes back one page in the history.",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "browser_close",
    description: "Closes the browser and frees the memory.",
    inputSchema: { type: "object", properties: {} },
  },
];

async function handleTool(name, args) {
  const p = await ensurePage();
  switch (name) {
    case "browser_navigate": {
      let url = String(args.url ?? "").trim();
      if (!/^https?:\/\//i.test(url)) url = `https://${url}`;
      await p.goto(url, { waitUntil: "domcontentloaded", timeout: 45000 });
      if (args.waitMs) await p.waitForTimeout(Number(args.waitMs));
      await publish();
      return `navigated to ${p.url()}\ntitle: ${await p.title()}`;
    }
    case "browser_snapshot": {
      await publish();
      const info = JSON.parse(fs.readFileSync(STATE, "utf8"));
      const links = info.links
        .map((l, i) => `  [${i}] ${l.text || "(no text)"} → ${l.href}`)
        .join("\n");
      const forms = info.forms
        .map((f) => `  <${f.tag}${f.type ? ` type="${f.type}"` : ""}${f.id ? ` id="${f.id}"` : ""}${f.name ? ` name="${f.name}"` : ""}${f.placeholder ? ` placeholder="${f.placeholder}"` : ""}>`)
        .join("\n");
      return `URL: ${info.url}\nTITLE: ${info.title}\n\nTEXT:\n${info.text}\n\nLINKS:\n${links || "  (none)"}\n\nFIELDS:\n${forms || "  (none)"}`;
    }
    case "browser_screenshot": {
      const buf = await p.screenshot({
        type: "jpeg",
        quality: 75,
        fullPage: !!args.fullPage,
      });
      fs.writeFileSync(SHOT, buf);
      await publish();
      return {
        content: [
          { type: "image", data: buf.toString("base64"), mimeType: "image/jpeg" },
          { type: "text", text: `screenshot of ${p.url()}` },
        ],
      };
    }
    case "browser_click": {
      let target = null;
      if (args.selector) target = p.locator(String(args.selector)).nth(args.index ?? 0);
      else if (args.text) target = p.getByText(String(args.text), { exact: false }).nth(args.index ?? 0);
      else throw new Error("provide selector or text");
      await target.waitFor({ state: "visible", timeout: 15000 });
      await target.click({ timeout: 15000 });
      await p.waitForLoadState("domcontentloaded").catch(() => {});
      await publish();
      return `clicked ${args.text ?? args.selector} → ${p.url()}`;
    }
    case "browser_type": {
      const sel =
        args.selector ||
        (args.text ? `:is(input[placeholder*="${args.text}" i], input[name*="${args.text}" i], textarea[placeholder*="${args.text}" i])` : "");
      if (!sel) throw new Error("provide selector (or text as a label)");
      const field = p.locator(sel).first();
      await field.waitFor({ state: "visible", timeout: 15000 });
      await field.fill(String(args.value ?? ""));
      if (args.submit) await field.press("Enter");
      await publish();
      return `filled ${sel}${args.submit ? " and submitted" : ""}`;
    }
    case "browser_press": {
      await p.keyboard.press(String(args.key ?? "Enter"));
      await p.waitForLoadState("domcontentloaded").catch(() => {});
      await publish();
      return `pressed ${args.key}`;
    }
    case "browser_scroll": {
      await p.mouse.wheel(0, Number(args.dy ?? 600));
      await p.waitForTimeout(300);
      await publish();
      return `scrolled ${args.dy}px`;
    }
    case "browser_eval": {
      const result = await p.evaluate(String(args.js ?? "null"));
      await publish();
      return JSON.stringify(result, null, 2).slice(0, 8000);
    }
    case "browser_wait": {
      if (args.selector) await p.waitForSelector(String(args.selector), { timeout: 30000 });
      else await p.waitForTimeout(Number(args.ms ?? 1000));
      await publish();
      return "ok";
    }
    case "browser_back": {
      await p.goBack({ waitUntil: "domcontentloaded", timeout: 30000 }).catch(() => {});
      await publish();
      return `went back to ${p.url()}`;
    }
    case "browser_close": {
      if (browser) await browser.close().catch(() => {});
      browser = null;
      page = null;
      return "browser closed";
    }
    default:
      throw new Error(`unknown tool: ${name}`);
  }
}

// ── minimal MCP (jsonrpc over stdio) ─────────────────────────────────────────
let buffer = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", async (chunk) => {
  buffer += chunk;
  let idx;
  while ((idx = buffer.indexOf("\n")) >= 0) {
    const line = buffer.slice(0, idx).trim();
    buffer = buffer.slice(idx + 1);
    if (!line) continue;
    let msg;
    try {
      msg = JSON.parse(line);
    } catch {
      continue;
    }
    if (msg.id === undefined) continue; // notification
    const reply = (result) =>
      process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id: msg.id, result }) + "\n");
    const fail = (error) =>
      process.stdout.write(
        JSON.stringify({ jsonrpc: "2.0", id: msg.id, error: { code: -32000, message: String(error) } }) + "\n",
      );

    try {
      if (msg.method === "initialize") {
        reply({
          protocolVersion: msg.params?.protocolVersion ?? "2024-11-05",
          capabilities: { tools: {} },
          serverInfo: { name: "hina-browser", version: "0.1.0" },
        });
      } else if (msg.method === "tools/list") {
        reply({ tools });
      } else if (msg.method === "tools/call") {
        const name = msg.params?.name;
        const args = msg.params?.arguments ?? {};
        const out = await handleTool(name, args);
        if (out && typeof out === "object" && out.content) reply(out);
        else reply({ content: [{ type: "text", text: String(out) }] });
      } else if (msg.method === "ping") {
        reply({});
      } else if (msg.method === "resources/list") {
        reply({ resources: [] });
      } else if (msg.method === "prompts/list") {
        reply({ prompts: [] });
      } else {
        fail(`unsupported method: ${msg.method}`);
      }
    } catch (err) {
      launchError = String(err);
      fail(err?.message ?? err);
    }
  }
});

process.stdin.on("end", async () => {
  if (browser) await browser.close().catch(() => {});
  process.exit(0);
});
