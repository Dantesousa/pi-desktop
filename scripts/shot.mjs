// Usage: node scripts/shot.mjs <url> <out.png> [waitMs]
// Prints DOM measurements as JSON so layout problems can be diagnosed headlessly.
import { firefox } from "playwright-core";
import fs from "node:fs";

const url = process.argv[2] ?? "http://127.0.0.1:1420/";
const out = process.argv[3] ?? "/tmp/shot.png";
const wait = Number(process.argv[4] ?? 2500);
const exe = (fs.readdirSync(`${process.env.HOME}/.cache/ms-playwright`)
  .filter((d) => d.startsWith("firefox-"))
  .sort()
  .pop());

const executablePath = `${process.env.HOME}/.cache/ms-playwright/${exe}/firefox/firefox`;
const browser = await firefox.launch({ executablePath, headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 860 } });
const errors = [];
page.on("pageerror", (e) => errors.push(String(e).slice(0, 300)));
page.on("console", (m) => {
  if (m.type() === "error") errors.push(`console: ${m.text().slice(0, 300)}`);
});
await page.goto(url, { waitUntil: "domcontentloaded" });
await page.waitForTimeout(wait);
const info = await page.evaluate(() => {
  const pick = (sel) => {
    const el = document.querySelector(sel);
    if (!el) return null;
    const r = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    return {
      x: Math.round(r.x),
      y: Math.round(r.y),
      w: Math.round(r.width),
      h: Math.round(r.height),
      display: cs.display,
      height: cs.height,
      width: cs.width,
      flexDir: cs.flexDirection,
      align: cs.alignItems,
      overflowY: cs.overflowY,
    };
  };
  return {
    win: [innerWidth, innerHeight],
    body: document.body.scrollHeight,
    sheets: Array.from(document.styleSheets).map((s) => {
      let rules = -1;
      try {
        rules = s.cssRules ? s.cssRules.length : -1;
      } catch {
        rules = -2;
      }
      return `${(s.href || "inline").split("/").pop()}:${rules}`;
    }),
    els: {
      shell: pick(".shell"),
      aside: pick("aside"),
      main: pick("main"),
      chat: pick(".chat"),
      composer: pick(".composer"),
      box: pick(".box"),
      header: pick("header"),
      tabbar: pick(".bar"),
    },
  };
});
fs.writeFileSync(out, await page.screenshot());
console.log(JSON.stringify({ info, errors }, null, 2));
await browser.close();
