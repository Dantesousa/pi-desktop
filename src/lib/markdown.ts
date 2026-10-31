import { marked } from "marked";
import DOMPurify from "dompurify";
import hljs from "highlight.js/lib/common";
import { t as tr, locale } from "./i18n.svelte";

marked.setOptions({ gfm: true, breaks: true });

export function renderMarkdown(text: string): string {
  const html = marked.parse(text ?? "", { async: false }) as string;
  return DOMPurify.sanitize(html, {
    ADD_ATTR: ["target", "rel"],
    FORBID_TAGS: ["style", "iframe", "form", "input"],
  });
}

export function highlightAll(root: HTMLElement | null) {
  if (!root) return;
  root.querySelectorAll<HTMLElement>("pre code").forEach((el) => {
    if (el.dataset.hl === "1") return;
    try {
      hljs.highlightElement(el);
      el.dataset.hl = "1";
    } catch {
      /* ignore */
    }
  });
}

export function relTime(iso: string | number | null | undefined): string {
  if (!iso) return "";
  const t = typeof iso === "number" ? iso : Date.parse(iso);
  if (!t || Number.isNaN(t)) return "";
  const diff = Date.now() - t;
  const min = Math.floor(diff / 60000);
  if (min < 1) return tr("time.now");
  if (min < 60) return `${min}m`;
  const h = Math.floor(min / 60);
  if (h < 24) return `${h}h`;
  const d = Math.floor(h / 24);
  if (d < 30) return `${d}d`;
  return new Date(t).toLocaleDateString(locale());
}
