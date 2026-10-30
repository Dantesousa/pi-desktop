import { firefox } from "playwright-core";
import fs from "node:fs";
const dir = fs.readdirSync(`${process.env.HOME}/.cache/ms-playwright`).filter(d=>d.startsWith("firefox-")).sort().pop();
const browser = await firefox.launch({ executablePath: `${process.env.HOME}/.cache/ms-playwright/${dir}/firefox/firefox`, headless: true });
const themes = ["midnight","amoled","hacker","dracula","nord","gruvbox","catppuccin","tokyo","cyberpunk","light"];
const page = await browser.newPage();
const out = [];
for (const t of themes) {
  await page.goto("file:///tmp/fixture.html");
  await page.evaluate((theme) => { document.documentElement.dataset.theme = theme; }, t);
  await page.waitForTimeout(120);
  const r = await page.evaluate(() => {
    const cs = (sel) => { const el = document.querySelector(sel); const c = getComputedStyle(el); return { color: c.color, bg: c.backgroundColor }; };
    const body = getComputedStyle(document.body).backgroundColor;
    return { pageBg: body, link: cs("#lnk"), code: cs("#cd"), block: cs("#block") };
  });
  const parse = (s) => (s.match(/\d+/g) || [0,0,0]).slice(0,3).map(Number);
  const lum = ([r,g,b]) => { const f = (v)=>{v/=255; return v<=0.03928? v/12.92 : Math.pow((v+0.055)/1.055,2.4);}; return 0.2126*f(r)+0.7152*f(g)+0.0722*f(b); };
  const ratio = (a,b) => { const l1=lum(parse(a)), l2=lum(parse(b)); return ((Math.max(l1,l2)+0.05)/(Math.min(l1,l2)+0.05)).toFixed(2); };
  out.push({ theme: t, link: `${r.link.color} on ${r.pageBg} = ${ratio(r.link.color, r.pageBg)}`, code: `${r.code.color} on ${r.code.bg} = ${ratio(r.code.color, r.code.bg)}` });
}
console.table(out);
await browser.close();
