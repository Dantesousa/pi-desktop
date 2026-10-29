export type DiffLine = { type: "ctx" | "add" | "del"; text: string; a?: number; b?: number };

const MAX_LINES = 900;

/** Line diff via LCS; falls back to a coarse replace when too large. */
export function lineDiff(oldText: string, newText: string, context = 4): DiffLine[] {
  const a = oldText.split("\n");
  const b = newText.split("\n");

  if (a.length > MAX_LINES || b.length > MAX_LINES) {
    return [
      ...a.map((t, i) => ({ type: "del" as const, text: t, a: i + 1 })),
      ...b.map((t, i) => ({ type: "add" as const, text: t, b: i + 1 })),
    ];
  }

  const n = a.length;
  const m = b.length;
  const dp: Uint32Array[] = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      dp[i][j] = a[i] === b[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
    }
  }

  const raw: DiffLine[] = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      raw.push({ type: "ctx", text: a[i], a: i + 1, b: j + 1 });
      i++;
      j++;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      raw.push({ type: "del", text: a[i], a: i + 1 });
      i++;
    } else {
      raw.push({ type: "add", text: b[j], b: j + 1 });
      j++;
    }
  }
  while (i < n) raw.push({ type: "del", text: a[i], a: i++ + 1 });
  while (j < m) raw.push({ type: "add", text: b[j], b: j++ + 1 });

  // collapse long context runs
  const keep = new Array(raw.length).fill(false);
  raw.forEach((l, idx) => {
    if (l.type !== "ctx") {
      for (let k = Math.max(0, idx - context); k <= Math.min(raw.length - 1, idx + context); k++) keep[k] = true;
    }
  });
  const out: DiffLine[] = [];
  let skipping = false;
  raw.forEach((l, idx) => {
    if (keep[idx]) {
      if (skipping) {
        out.push({ type: "ctx", text: "⋯" });
        skipping = false;
      }
      out.push(l);
    } else {
      skipping = true;
    }
  });
  return out;
}

export function addedOnly(text: string, cap = 200): DiffLine[] {
  return text
    .split("\n")
    .slice(0, cap)
    .map((t, i) => ({ type: "add" as const, text: t, b: i + 1 }));
}
