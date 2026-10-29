<script lang="ts">
  import { renderMarkdown, highlightAll } from "../markdown";
  import { hydrateImages } from "../media";
  import { app } from "../store.svelte";
  import ToolCard from "./ToolCard.svelte";
  import type { Msg } from "../types";

  let { msg }: { msg: Msg } = $props();
  let root = $state<HTMLElement | null>(null);
  let thinkingOpen = $state(false);

  $effect(() => {
    // re-run when text changes
    msg.parts.map((p: any) => p.text?.length ?? 0).join(",");
    highlightAll(root);
    hydrateImages(root, app.cwd);
  });
</script>

{#if msg.role === "system"}
  <div class="sys">{msg.parts.map((p: any) => p.text).join(" ")}</div>
{:else if msg.role === "user"}
  <div class="row user">
    <div class="bubble">
      {#if msg.chips?.length}
        <div class="chips">
          {#each msg.chips as chip}
            {#if chip.kind === "image" && chip.preview}
              <img class="thumb" src={chip.preview} alt={chip.name} />
            {:else}
              <span class="chip">📎 {chip.name}</span>
            {/if}
          {/each}
        </div>
      {/if}
      <div class="markdown" bind:this={root}>
        {@html renderMarkdown(msg.parts.map((p: any) => p.text).join(""))}
      </div>
    </div>
  </div>
{:else}
  <div class="row" bind:this={root}>
    <div class="avatar">◈</div>
    <div class="content">
      <div class="meta">
        <span class="who">Hina</span>
        {#if msg.model}<span class="model">{msg.model}</span>{/if}
        {#if msg.streaming}<span class="typing"><i></i><i></i><i></i></span>{/if}
      </div>
      {#each msg.parts as part (part.ci + "-" + part.kind)}
        {#if part.kind === "text"}
          {#if part.text}
            <div class="markdown">{@html renderMarkdown(part.text)}</div>
          {/if}
        {:else if part.kind === "thinking"}
          <div class="thinking">
            <button class="th-head" onclick={() => (thinkingOpen = !thinkingOpen)}>
              💭 raciocínio {thinkingOpen ? "▾" : "▸"}
            </button>
            {#if thinkingOpen}
              <div class="th-body">{part.text}</div>
            {/if}
          </div>
        {:else if part.kind === "tool"}
          <ToolCard {part} />
        {/if}
      {/each}
      {#if msg.error}
        <div class="err">⚠ {msg.error}</div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .row {
    display: flex;
    gap: 10px;
    padding: 9px 0;
    align-items: flex-start;
  }
  .row.user {
    justify-content: flex-end;
  }
  .avatar {
    width: 24px;
    height: 24px;
    border-radius: 8px;
    background: linear-gradient(135deg, #3d6fd6, #8b5cf6);
    color: #fff;
    font-size: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
    margin-top: 2px;
  }
  .content {
    flex: 1;
    min-width: 0;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 3px;
    font-size: 11.5px;
  }
  .who {
    font-weight: 650;
    color: #cdd9ef;
  }
  .model {
    color: var(--text-faint);
    font-family: var(--mono);
    font-size: 10.5px;
  }
  .bubble {
    max-width: min(72ch, 82%);
    background: #1b2333;
    border: 1px solid #263149;
    border-radius: 14px 14px 4px 14px;
    padding: 8px 13px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 6px;
  }
  .chip {
    font-size: 11px;
    background: #131a27;
    border: 1px solid #2a3550;
    border-radius: 6px;
    padding: 2px 7px;
    color: #9db2d4;
  }
  .thumb {
    max-height: 150px;
    max-width: 100%;
    border-radius: 8px;
    border: 1px solid #2a3550;
  }
  .thinking {
    border-left: 2px solid #2b3550;
    margin: 5px 0;
    padding-left: 9px;
  }
  .th-head {
    border: none;
    background: none;
    padding: 0;
    color: var(--text-faint);
    font-size: 11.5px;
  }
  .th-head:hover {
    background: none;
    color: var(--text-dim);
  }
  .th-body {
    white-space: pre-wrap;
    color: var(--text-faint);
    font-size: 12px;
    margin-top: 4px;
    max-height: 300px;
    overflow: auto;
  }
  .sys {
    text-align: center;
    color: var(--text-faint);
    font-size: 11.5px;
    padding: 6px 0;
    font-style: italic;
  }
  .err {
    color: #ffada6;
    font-size: 12px;
    background: #2a1616;
    border: 1px solid #4a2424;
    border-radius: 8px;
    padding: 5px 9px;
    margin-top: 5px;
  }
  .typing i {
    display: inline-block;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--accent);
    margin-right: 2px;
    animation: blink 1.2s infinite;
  }
  .typing i:nth-child(2) {
    animation-delay: 0.2s;
  }
  .typing i:nth-child(3) {
    animation-delay: 0.4s;
  }
  @keyframes blink {
    0%,
    60%,
    100% {
      opacity: 0.25;
    }
    30% {
      opacity: 1;
    }
  }
</style>
