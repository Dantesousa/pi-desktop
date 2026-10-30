import { api } from "./api";
import { t } from "./i18n.svelte";

/** True when a markdown `img src` points to a local file rather than a URL. */
export function isLocalImageSrc(src: string): boolean {
  if (!src) return false;
  return !/^(https?:|data:|blob:|mailto:)/i.test(src);
}

/** Resolves a local image reference (absolute, file://, or relative to cwd). */
export function resolveLocalPath(src: string, cwd: string): string {
  let path = src.trim();
  if (path.startsWith("file://")) path = decodeURIComponent(path.slice("file://".length));
  if (path.startsWith("/")) return path;
  const base = cwd.replace(/\/+$/, "");
  return `${base}/${path.replace(/^\.\//, "")}`;
}

/**
 * Replaces local image references in rendered markdown with data URLs so the
 * webview can display files from disk (CSP blocks file:// loads).
 */
export async function hydrateImages(root: HTMLElement | null, cwd: string) {
  if (!root) return;
  const images = Array.from(root.querySelectorAll("img"));
  for (const img of images) {
    const src = img.getAttribute("src") ?? "";
    if (!isLocalImageSrc(src)) continue;
    if (img.dataset.hydrated === "1") continue;
    img.dataset.hydrated = "1";

    const path = resolveLocalPath(src, cwd);
    try {
      const att: any = await api.attachmentRead(path);
      if (att.previewUrl) {
        img.src = att.previewUrl;
        img.title = path;
      } else {
        img.replaceWith(brokenBadge(path, t("media.notAnImage")));
      }
    } catch (e: any) {
      img.replaceWith(brokenBadge(path, String(e?.message ?? e).slice(0, 80)));
    }
  }
}

function brokenBadge(path: string, reason: string): HTMLElement {
  const span = document.createElement("span");
  span.className = "img-missing";
  span.textContent = `🖼 ${path} — ${reason}`;
  return span;
}

/** Extracts inline images from a tool result (`{content:[{type:"image",...}]}`). */
export function extractImages(result: any): { data: string; mimeType: string }[] {
  const content = result?.content ?? result;
  if (!Array.isArray(content)) return [];
  return content
    .filter((c: any) => c?.type === "image" && c.data)
    .map((c: any) => ({ data: c.data, mimeType: c.mimeType ?? "image/png" }));
}
