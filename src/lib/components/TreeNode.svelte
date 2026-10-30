<script lang="ts">
  import { t } from "../i18n.svelte";
  import Self from "./TreeNode.svelte";

  let { node, leafId, depth = 0 }: { node: any; leafId: string | null; depth?: number } = $props();
  let open = $state(true);

  const entry = $derived(node.entry ?? {});
  const isLeaf = $derived(entry.id === leafId);

  const label = $derived.by(() => {
    const e: any = entry;
    if (e.type === "message") {
      const m = e.message ?? {};
      const text =
        typeof m.content === "string"
          ? m.content
          : (m.content ?? [])
              .map((c: any) => (c.type === "text" ? c.text : c.type === "toolCall" ? `<${c.name}>` : ""))
              .filter(Boolean)
              .join(" ");
      return `${m.role ?? "?"}: ${text.replace(/\s+/g, " ").slice(0, 80)}`;
    }
    if (e.type === "session_info") return `nome: ${e.name}`;
    if (e.type === "model_change") return `modelo: ${e.modelId}`;
    if (e.type === "thinking_level_change") return `thinking: ${e.thinkingLevel}`;
    return e.type ?? "?";
  });
</script>

<div class="node" style={`padding-left:${depth * 14}px`}>
  <div class="row" class:leaf={isLeaf}>
    {#if node.children?.length}
      <button class="tw" onclick={() => (open = !open)}>{open ? "▾" : "▸"}</button>
    {:else}
      <span class="tw">·</span>
    {/if}
    <span class="kind">{entry.type}</span>
    <span class="label" title={label}>{label}</span>
    {#if isLeaf}<span class="badge">{t("tree.current")}</span>{/if}
  </div>
  {#if open}
    {#each node.children ?? [] as child}
      <Self node={child} {leafId} depth={depth + 1} />
    {/each}
  {/if}
</div>

<style>
  .node {
    font-size: 11.5px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 4px;
    border-radius: 6px;
    min-width: 0;
  }
  .row:hover {
    background: var(--panel-2);
  }
  .row.leaf {
    background: var(--bubble);
  }
  .tw {
    border: none;
    background: none;
    padding: 0 2px;
    color: var(--text-faint);
    font-size: 10px;
    width: 14px;
    flex: 0 0 auto;
  }
  .tw:hover {
    background: none;
    color: var(--text);
  }
  .kind {
    font-family: var(--mono);
    font-size: 9.5px;
    color: var(--text-faint);
    flex: 0 0 auto;
  }
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-dim);
  }
  .badge {
    font-size: 9px;
    background: var(--accent);
    color: var(--accent-ink, #fff);
    border-radius: 999px;
    padding: 0 6px;
    flex: 0 0 auto;
  }
</style>
