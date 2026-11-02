/**
 * Extracts a string field from a *partially streamed* JSON object.
 * Tool arguments arrive as text deltas, so the JSON is incomplete until the end.
 */
export function partialStringField(raw: string, field: string): string | null {
  if (!raw) return null;
  const key = `"${field}"`;
  const at = raw.indexOf(key);
  if (at < 0) return null;
  const colon = raw.indexOf(":", at + key.length);
  if (colon < 0) return null;
  let i = colon + 1;
  while (i < raw.length && /\s/.test(raw[i])) i++;
  if (raw[i] !== '"') return null;
  i++;

  let out = "";
  const esc: Record<string, string> = {
    n: "\n",
    t: "\t",
    r: "\r",
    b: "\b",
    f: "\f",
    '"': '"',
    "\\": "\\",
    "/": "/",
  };
  while (i < raw.length) {
    const c = raw[i];
    if (c === "\\") {
      const n = raw[i + 1];
      if (n === undefined) break; // escape split across deltas
      if (n === "u") {
        const hex = raw.slice(i + 2, i + 6);
        if (hex.length < 4) break;
        out += String.fromCharCode(parseInt(hex, 16));
        i += 6;
        continue;
      }
      out += esc[n] ?? n;
      i += 2;
      continue;
    }
    if (c === '"') break; // field finished
    out += c;
    i++;
  }
  return out;
}

/** Text being written by a `write` tool call, or the first `newText` of an `edit`. */
export function streamedContent(name: string, raw: string): string | null {
  if (name === "write") return partialStringField(raw, "content");
  if (name === "edit") return partialStringField(raw, "newText");
  return null;
}
