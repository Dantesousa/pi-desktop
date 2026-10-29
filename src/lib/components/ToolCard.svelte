<script lang="ts">
  import { lineDiff, addedOnly } from "../diff";

  let { part }: { part: any } = $props();
  let open = $state(false);
  let outputEl = $state<HTMLElement | null>(null);
  let showAll = $state(false);

  const ICONS: Record<string, string> = {
    bash: "▸_",
    read: "📄",
    write: "✎",
    edit: "⇄",
    memory_add: "🧠",
    memory_replace: "🧠",
    memory_remove: "🧠",
    memory_search: "🔎",
    session_search: "🗂",
    skill_manage: "⚙",
    web_fetch: "🌐",
    browser: "🌐",
  };

  const icon = $derived(ICONS[part.name] ?? "◆");

  const summary = $derived.by(() => {
    const a = part.args ?? {};
    const text = a.path ?? a.command ?? a.pattern ?? a.query ?? a.target ?? a.message ?? a.url ?? "";
    const s = String(text).replace(/\s+/g, " ").trim();
    return s.length > 90 ? s.slice(0, 90) + "…" : s;
  });

  const output = $derived(part.output ?? "");
  const preview = $derived(
    showAll || output.length < 1600 ? output : output.slice(0, 1600) + "\n… (truncado)",
  );

  const diffs = $derived.by(() => {
    const a = part.args ?? {};
    if (part.name === "edit" && Array.isArray(a.edits)) {
      return a.edits.map((e: any) => ({
        path: a.path,
        lines: lineDiff(String(e.oldText ?? ""), String(e.newText ?? "")),
      }));
    }
    if (part.name === "write" && typeof a.content === "string") {
      return [{ path: a.path, lines: addedOnly(a.content) }];
    }
    return [];
  });

  const argsJson = $derived.by(() => {
    try {
      return JSON.stringify(part.args ?? {}, null, 2);
    } catch {
      return String(part.args);
    }
  });

  let expandedByDefault = $state(false);
  $effect(() => {
    if (diffs.length && !expandedByDefault) {
      expandedByDefault = true;
      open = true;
    }
  });
</script>

<div class="tool" class:error={part.isError} class:running={part.running}>
  <button class="head" onclick={() => (open = !open)}>
    <span class="icon">{icon}</span>
    <span class="name">{part.name || "tool"}</span>
    {#if summary}<span class="summary">{summary}</span>{/if}
    <span class="spacer"></span>
    {#if diffs.length}
      <span class="badge">{diffs.reduce((n, d) => n + d.lines.filter((l) => l.type !== "ctx").length, 0)} linhas</span>
    {/if}
    {#if part.running}
      <span class="spin"></span>
    {:else if part.durationMs != null}
      <span class="dur">{(part.durationMs / 1000).toFixed(1)}s</span>
    {/if}
    <span class="chev">{open ? "▾" : "▸"}</span>
  </button>

  {#if open}
    <div class="body">
      {#each diffs as d}
        <div class="difflabel">{d.path ?? "diff"}</div>
        <div class="diff">
          {#each d.lines as line}
            <div class="dl" class:add={line.type === "add"} class:del={line.type === "del"} class:ctx={line.type === "ctx"}
                 class:skip={line.type === "ctx" && line.text === "⋯"}>
              <span class="ln">{line.type === "add" ? line.b : line.a ?? ""}</span>
              <span class="sign">{line.type === "add" ? "+" : line.type === "del" ? "−" : " "}</span>
              <span class="txt">{line.text}</span>
            </div>
          {/each}
        </div>
      {/each}

      {#if !diffs.length}
        <div class="label">argumentos</div>
        <pre class="args">{argsJson}</pre>
      {/if}

      {#if output}
        <div class="label">
          resultado
          {#if output.length >= 1600}
            <button class="mini" onclick={() => (showAll = !showAll)}>
              {showAll ? "recolher" : "tudo"}
            </button>
          {/if}
        </div>
        <div class="out" bind:this={outputEl}><pre>{preview}</pre></div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .tool {
    border: 1px solid var(--border-soft);
    background: var(--bg-2);
    border-radius: 10px;
    margin: 6px 0;
    overflow: hidden;
  }
  .tool.error {
    border-color: #4a2a2a;
  }
  .tool.running {
    border-color: #2a3a5c;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    border: none;
    border-radius: 0;
    padding: 7px 11px;
    background: transparent;
    text-align: left;
    font-size: 12.5px;
  }
  .head:hover {
    background: #131926;
  }
  .icon {
    opacity: 0.85;
    font-size: 12px;
    width: 16px;
    text-align: center;
  }
  .name {
    font-family: var(--mono);
    font-weight: 600;
    color: #9fc0ff;
  }
  .summary {
    font-family: var(--mono);
    color: var(--text-faint);
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .badge {
    font-size: 10px;
    color: var(--text-faint);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 7px;
  }
  .dur {
    color: var(--text-faint);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .chev {
    color: var(--text-faint);
    font-size: 10px;
  }
  .spin {
    width: 9px;
    height: 9px;
    border: 1.5px solid #35507f;
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .body {
    padding: 2px 11px 10px;
    border-top: 1px solid var(--border-soft);
  }
  .label,
  .difflabel {
    font-size: 10.5px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-faint);
    margin: 8px 0 3px;
    display: flex;
    gap: 8px;
    align-items: center;
    font-family: var(--mono);
  }
  .args,
  .out pre {
    margin: 0;
    background: #0c1017;
    border: 1px solid var(--border-soft);
    border-radius: 8px;
    padding: 8px 10px;
    max-height: 320px;
    overflow: auto;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    color: #b7c3d9;
    font-size: 11.5px;
    line-height: 1.45;
  }
  .error .out pre {
    color: #ffb3ad;
  }
  .mini {
    padding: 1px 6px;
    font-size: 10px;
    border-radius: 5px;
  }
  .diff {
    background: #0c1017;
    border: 1px solid var(--border-soft);
    border-radius: 8px;
    overflow: auto;
    max-height: 340px;
    font-family: var(--mono);
    font-size: 11.5px;
    line-height: 1.5;
  }
  .dl {
    display: flex;
    gap: 0;
    white-space: pre;
    padding: 0 6px 0 0;
  }
  .ln {
    width: 34px;
    text-align: right;
    padding-right: 7px;
    color: #46536e;
    flex: 0 0 auto;
    user-select: none;
    font-size: 10.5px;
  }
  .sign {
    width: 14px;
    flex: 0 0 auto;
    color: #5a6885;
    user-select: none;
  }
  .txt {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    color: #b7c3d9;
  }
  .add {
    background: rgba(46, 160, 67, 0.13);
  }
  .add .sign,
  .add .txt {
    color: #a6e3b0;
  }
  .del {
    background: rgba(248, 81, 73, 0.12);
  }
  .del .sign,
  .del .txt {
    color: #f0a5a0;
  }
  .skip {
    color: #46536e;
    opacity: 0.7;
  }
</style>
