<script lang="ts">
  import { t } from "../i18n.svelte";
  import { app } from "../store.svelte";
  import TreeNode from "./TreeNode.svelte";

  let forkmessages = $state<{ entryId: string; text: string }[]>([]);
  let tree = $state<{ tree: any[]; leafId: string | null } | null>(null);
  let busy = $state(false);

  $effect(() => {
    if (app.showFork) {
      busy = true;
      app
        .forkMessages()
        .then((m) => (forkmessages = m))
        .finally(() => (busy = false));
    }
    if (app.showTree) {
      busy = true;
      app
        .sessionTree()
        .then((t) => (tree = t))
        .finally(() => (busy = false));
    }
  });

  async function pick(entryId: string) {
    app.showFork = false;
    await app.forkFrom(entryId);
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape") {
      app.showFork = false;
      app.showTree = false;
    }
  }}
/>

{#if app.showFork}
  <div class="overlay" onclick={(e) => e.target === e.currentTarget && (app.showFork = false)}>
    <div class="box" role="dialog" aria-modal="true" tabindex="-1">
      <div class="head">
        <strong>{t("tree.forkTitle")}</strong>
        <span class="faint">{t("tree.forkHint")}</span>
        <span class="spacer"></span>
        <button class="mini" onclick={() => (app.showFork = false)}>×</button>
      </div>
      <div class="list">
        {#each forkmessages as m (m.entryId)}
          <button class="item" onclick={() => pick(m.entryId)}>
            <span class="txt">{m.text.replace(/\\s+/g, " ").slice(0, 200)}</span>
            <span class="eid">{m.entryId}</span>
          </button>
        {:else}
          <div class="empty">{busy ? t("common.loading") : t("tree.noMessages")}</div>
        {/each}
      </div>
    </div>
  </div>
{/if}

{#if app.showTree}
  <div class="overlay" onclick={(e) => e.target === e.currentTarget && (app.showTree = false)}>
    <div class="box" role="dialog" aria-modal="true" tabindex="-1">
      <div class="head">
        <strong>{t("tree.title")}</strong>
        <span class="faint">{tree?.leafId ? `leaf ${tree.leafId}` : ""}</span>
        <span class="spacer"></span>
        <button class="mini" onclick={() => (app.showTree = false)}>×</button>
      </div>
      <div class="list tree">
        {#each tree?.tree ?? [] as node}
          <TreeNode {node} leafId={tree?.leafId ?? null} />
        {:else}
          <div class="empty">{busy ? t("common.loading") : t("tree.empty")}</div>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(5, 7, 11, 0.6);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 66;
  }
  .box {
    width: min(720px, 94vw);
    max-height: 78vh;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 13px;
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.6);
    overflow: hidden;
    outline: none;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-soft);
    font-size: 12.5px;
  }
  .faint {
    color: var(--text-faint);
    font-size: 10.5px;
    font-family: var(--mono);
  }
  .spacer {
    flex: 1;
  }
  .list {
    overflow: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .list.tree {
    gap: 0;
  }
  .item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    text-align: left;
    border: 1px solid var(--border-soft);
    background: var(--bg-2);
    border-radius: 8px;
    padding: 7px 9px;
  }
  .item:hover {
    background: var(--panel-2);
    border-color: var(--border);
  }
  .txt {
    font-size: 12px;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .eid {
    font-family: var(--mono);
    font-size: 9.5px;
    color: var(--text-faint);
  }
  .empty {
    padding: 18px;
    text-align: center;
    color: var(--text-faint);
    font-size: 12px;
  }
  .mini {
    font-size: 10.5px;
    padding: 2px 8px;
  }
</style>
