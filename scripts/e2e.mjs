#!/usr/bin/env node
/**
 * Headless UI smoke test: layout, theme switching and the command palette.
 * Requires the vite dev server (npm run dev) on port 1420.
 *
 *   node scripts/e2e.mjs [url]
 */
import { firefox } from "playwright-core";
import fs from "node:fs";

const url = process.argv[2] ?? "http://127.0.0.1:1420/";
const dir = fs
  .readdirSync(`${process.env.HOME}/.cache/ms-playwright`)
  .filter((d) => d.startsWith("firefox-"))
  .sort()
  .pop();

const failures = [];
const check = (name, ok, detail = "") => {
  console.log(`${ok ? "✓" : "✗"} ${name}${detail ? ` — ${detail}` : ""}`);
  if (!ok) failures.push(name);
};

const browser = await firefox.launch({
  executablePath: `${process.env.HOME}/.cache/ms-playwright/${dir}/firefox/firefox`,
  headless: true,
});
const page = await browser.newPage({ viewport: { width: 1280, height: 860 } });
const errors = [];
page.on("pageerror", (e) => {
  const t = String(e);
  if (!t.includes("__TAURI_INTERNALS__")) errors.push(t);
});
page.on("console", (m) => {
  const t = m.text();
  // expected in a plain browser: no Tauri IPC
  if (m.type() === "error" && !t.includes("__TAURI_INTERNALS__")) errors.push(t);
});

await page.goto(url, { waitUntil: "domcontentloaded" });
await page.waitForTimeout(2200);

// 1. layout
const layout = await page.evaluate(() => {
  const r = (sel) => {
    const el = document.querySelector(sel);
    if (!el) return null;
    const b = el.getBoundingClientRect();
    return { x: Math.round(b.x), w: Math.round(b.width), h: Math.round(b.height), d: getComputedStyle(el).display, fd: getComputedStyle(el).flexDirection };
  };
  return {
    body: document.body.scrollHeight,
    win: innerWidth,
    shell: r(".shell"),
    aside: r("aside"),
    main: r("main"),
    composer: r(".composer"),
    header: r("header"),
  };
});
check("shell is a flex row", layout.shell?.d === "flex" && layout.shell?.fd === "row", JSON.stringify(layout.shell));
check("sidebar is 250px and full height", layout.aside?.w === 250 && layout.aside?.h === 860, `${layout.aside?.w}x${layout.aside?.h}`);
check("main starts after the sidebar", layout.main?.x === 250, `x=${layout.main?.x}`);
check("composer spans the main column", layout.composer?.w === layout.main?.w, `${layout.composer?.w} vs ${layout.main?.w}`);
check("no vertical overflow", layout.body === 860, `body=${layout.body}`);

// 2. theme switching
await page.click('button[title*="ettings"], button[title*="justes"]');
await page.waitForTimeout(400);
const hackerBtn = page.locator("button.sw", { hasText: "Hacker" }).first();
await hackerBtn.click();
await page.waitForTimeout(400);
const theme = await page.evaluate(() => document.documentElement.dataset.theme);
check("theme switches to hacker", theme === "hacker", `data-theme=${theme}`);
const accent = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--accent").trim());
check("accent follows the theme", accent.length > 0, accent);

// 3. palette
await page.keyboard.press("Control+k");
await page.waitForTimeout(400);
const paletteOpen = await page.locator(".palette").count();
check("palette opens with Ctrl+K", paletteOpen === 1);
await page.locator(".palette input").fill("mem");
await page.waitForTimeout(300);
const items = await page.locator(".palette .item").count();
check("palette filters", items > 0, `${items} items`);
await page.keyboard.press("Escape");
await page.waitForTimeout(300);
check("palette closes with Escape", (await page.locator(".palette").count()) === 0);

check("no console errors", errors.length === 0, errors.slice(0, 2).join(" | "));

await page.screenshot({ path: "/tmp/e2e.png" });
await browser.close();

console.log(failures.length ? `\n${failures.length} check(s) failed` : "\nall checks passed");
process.exit(failures.length ? 1 : 0);
