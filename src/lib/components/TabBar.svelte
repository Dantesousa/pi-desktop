<script lang="ts">
  import { t } from "../i18n.svelte";
  import { app } from "../store.svelte";
</script>

<div class="bar">
  {#each app.tabs as tab (tab.id)}
    <button class="tab" class:active={tab.id === app.activeId} onclick={() => app.activate(tab.id)} title={tab.cwd}>
      <span class="dot" class:live={tab.conn === "ready"} class:work={tab.working}></span>
      <span
        class="lbl"
        role="button"
        tabindex="-1"
        title={t("tabs.renameHint")}
        ondblclick={async (e) => {
          e.stopPropagation();
          const name = await app.askPrompt({
            title: t("tabs.renameTitle"),
            value: tab.title,
            placeholder: t("tabs.renamePlaceholder"),
            confirmLabel: t("common.rename"),
          });
          if (name != null) app.renameTab(tab.id, name);
        }}
        onkeydown={() => {}}
        >{tab.title}</span
      >
      {#if tab.working}<span class="spin"></span>{/if}
      <span
        class="x"
        role="button"
        tabindex="0"
        onclick={(e) => {
          e.stopPropagation();
          app.closeTab(tab.id);
        }}
        onkeydown={(e) => e.key === "Enter" && app.closeTab(tab.id)}
        >×</span
      >
    </button>
  {/each}
  <button class="add" title={t("tabs.new")} onclick={() => app.openTab()}>+</button>
</div>

<style>
  .bar {
    display: flex;
    align-items: stretch;
    gap: 2px;
    padding: 5px 10px 0;
    background: var(--bg);
    border-bottom: 1px solid var(--border-soft);
    overflow-x: auto;
    flex: 0 0 auto;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    border: 1px solid transparent;
    border-bottom: none;
    border-radius: 8px 8px 0 0;
    padding: 5px 9px;
    font-size: 11.5px;
    color: var(--text-faint);
    max-width: 220px;
    background: transparent;
  }
  .tab:hover {
    background: var(--panel);
  }
  .tab.active {
    background: var(--bg-2);
    border-color: var(--border-soft);
    color: var(--text);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #3a4358;
    flex: 0 0 auto;
  }
  .dot.live {
    background: var(--green);
  }
  .dot.work {
    background: var(--accent);
    animation: pulse 1s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 0.35; }
    50% { opacity: 1; }
  }
  .lbl {
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .spin {
    width: 8px;
    height: 8px;
    border: 1.5px solid #35507f;
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  .x {
    color: var(--text-faint);
    padding: 0 2px;
    border-radius: 4px;
    font-size: 13px;
    line-height: 1;
  }
  .x:hover {
    background: var(--bubble);
    color: var(--red);
  }
  .add {
    border: 1px solid transparent;
    border-radius: 8px 8px 0 0;
    padding: 3px 10px;
    color: var(--text-faint);
    font-size: 14px;
  }
  .add:hover {
    background: var(--panel);
    color: var(--text);
  }
</style>
