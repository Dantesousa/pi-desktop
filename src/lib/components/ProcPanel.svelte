<script lang="ts">
  import { app } from "../store.svelte";
  import { t } from "../i18n.svelte";

  let cmd = $state("");
  let notify = $state(true);
  let out = $state<HTMLElement | null>(null);

  $effect(() => {
    app.procText;
    if (out) out.scrollTop = out.scrollHeight;
  });

  function elapsed(p: any) {
    const end = p.endedMs ?? Date.now();
    const s = Math.max(0, Math.round((end - p.startedMs) / 1000));
    const m = Math.floor(s / 60);
    return m > 0 ? `${m}m${String(s % 60).padStart(2, "0")}s` : `${s}s`;
  }

  const STATUS_KEY: Record<string, string> = {
    running: "proc.running",
    exited: "proc.exited",
    failed: "proc.failed",
  };

  function start() {
    if (!cmd.trim()) return;
    void app.startProc(cmd.trim(), notify);
    cmd = "";
  }
</script>

{#if app.showProcs}
  <section class="panel">
    <header>
      <span class="dot" class:live={app.procs.some((p) => p.status === "running")}></span>
      <span class="title">{t("proc.title")}</span>
      <span class="faint">{app.procs.length}</span>
      <span class="spacer"></span>
      <button class="mini" title={t("common.close")} onclick={() => app.toggleProcs(false)}>×</button>
    </header>

    <div class="list">
      {#each app.procs as p (p.id)}
        <div class="row" class:active={app.procId === p.id}>
          <button class="main" onclick={() => app.selectProc(p.id)}>
            <span class="dot" class:live={p.status === "running"} class:bad={p.status === "failed"}></span>
            <span class="cmd" title={p.cwd}>{p.command}</span>
            <span class="meta">
              {t(STATUS_KEY[p.status] ?? "proc.exited")}
              {#if p.status === "running"}· {elapsed(p)}{:else if p.exitCode != null}· exit {p.exitCode}{/if}
            </span>
          </button>
          {#if p.status === "running"}
            <button class="mini" title={t("proc.stop")} onclick={() => app.stopProc(p.id)}>■</button>
          {:else}
            <button class="mini" title={t("proc.remove")} onclick={() => app.removeProc(p.id)}>×</button>
          {/if}
        </div>
      {:else}
        <div class="empty">{t("proc.empty")}</div>
      {/each}
    </div>

    {#if app.procId}
      <pre class="out" bind:this={out}>{app.procText || t("proc.noOutput")}</pre>
    {/if}

    <footer>
      <input
        bind:value={cmd}
        placeholder={t("proc.command")}
        onkeydown={(e) => e.key === "Enter" && start()}
        spellcheck="false"
      />
      <label class="notify" title={t("proc.notify")}>
        <input type="checkbox" bind:checked={notify} /> ⚡
      </label>
      <button class="mini primary" onclick={start} disabled={!cmd.trim()}>{t("proc.start")}</button>
    </footer>
  </section>
{/if}

<style>
  .panel {
    position: fixed;
    left: 266px;
    bottom: 132px;
    width: 480px;
    height: 320px;
    min-width: 320px;
    min-height: 200px;
    resize: both;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.55);
    z-index: 49;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border-bottom: 1px solid var(--border-soft);
    font-size: 11.5px;
    flex: 0 0 auto;
  }
  .title {
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  .faint {
    color: var(--text-faint);
    font-size: 10.5px;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-faint);
    flex: 0 0 auto;
  }
  .dot.live {
    background: var(--green);
    box-shadow: 0 0 7px var(--green);
  }
  .dot.bad {
    background: var(--red);
  }
  .list {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    border-radius: 7px;
  }
  .row:hover,
  .row.active {
    background: var(--panel-2);
  }
  .main {
    display: flex;
    align-items: center;
    gap: 7px;
    flex: 1;
    min-width: 0;
    border: none;
    background: none;
    padding: 4px 6px;
    text-align: left;
    font-size: 11.5px;
  }
  .main:hover {
    background: none;
  }
  .cmd {
    font-family: var(--mono);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }
  .meta {
    color: var(--text-faint);
    font-size: 9.5px;
    flex: 0 0 auto;
  }
  .out {
    flex: 0 0 42%;
    margin: 0 6px;
    background: var(--code-bg);
    border: 1px solid var(--border-soft);
    border-radius: 8px;
    padding: 7px 9px;
    overflow: auto;
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-dim);
    white-space: pre-wrap;
  }
  .empty {
    color: var(--text-faint);
    font-size: 11.5px;
    text-align: center;
    padding: 16px;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 9px;
    flex: 0 0 auto;
  }
  footer input[type="text"],
  footer input:not([type]) {
    flex: 1;
  }
  footer input {
    flex: 1;
    font-size: 11.5px;
    padding: 5px 8px;
    font-family: var(--mono);
  }
  .notify {
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    color: var(--text-faint);
    flex: 0 0 auto;
  }
  .mini {
    font-size: 10.5px;
    padding: 2px 8px;
  }
</style>
