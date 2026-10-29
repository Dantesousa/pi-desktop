<script lang="ts">
  import { app } from "../store.svelte";

  let command = $state("");
  let exclude = $state(false);
  let out = $state<HTMLElement | null>(null);
  let input = $state<HTMLTextAreaElement | null>(null);

  $effect(() => {
    if (app.showBash) setTimeout(() => input?.focus(), 30);
  });

  $effect(() => {
    app.active?.bashLog;
    if (out) out.scrollTop = out.scrollHeight;
  });

  async function run() {
    if (!command.trim() || app.active?.bashRunning) return;
    try {
      await app.runBash(command, exclude);
    } catch (e: any) {
      app.toast(`bash falhou: ${e?.message ?? e}`, "error");
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      app.showBash = false;
    } else if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      run();
    }
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape" && app.showBash) app.showBash = false;
  }}
/>

{#if app.showBash}
  <div class="overlay" onclick={(e) => e.target === e.currentTarget && (app.showBash = false)}>
    <div class="box" role="dialog" aria-modal="true" tabindex="-1">
      <div class="head">
        <strong>bash</strong>
        <span class="cwd">{app.cwd}</span>
        <span class="spacer"></span>
        <label class="ctx">
          <input type="checkbox" bind:checked={exclude} />
          fora do contexto
        </label>
        <button class="mini" onclick={() => (app.showBash = false)}>×</button>
      </div>

      <textarea
        bind:this={input}
        bind:value={command}
        rows="2"
        placeholder="comando… (Enter roda, Shift+Enter quebra linha)"
        onkeydown={onKey}
        spellcheck="false"
      ></textarea>

      <div class="actions">
        {#if app.active?.bashRunning}
          <button class="danger" onclick={() => app.abortBash()}>■ abortar</button>
          <span class="running">executando…</span>
        {:else}
          <button class="primary" onclick={run} disabled={!command.trim()}>executar</button>
        {/if}
        <span class="spacer"></span>
        <button class="mini" onclick={() => (app.active!.bashLog = "")}>limpar saída</button>
      </div>

      <pre class="out" bind:this={out}>{app.active?.bashLog || "— sem saída ainda —"}</pre>
      <div class="hint">
        A saída entra no contexto do modelo na próxima mensagem (a menos que "fora do contexto"
        esteja marcado).
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
    z-index: 65;
  }
  .box {
    width: min(760px, 94vw);
    max-height: 82vh;
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 13px;
    padding: 14px;
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.6);
    outline: none;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12.5px;
  }
  .cwd {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ctx {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 10.5px;
    color: var(--text-faint);
  }
  .spacer {
    flex: 1;
  }
  textarea {
    font-family: var(--mono);
    font-size: 12.5px;
    resize: vertical;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .running {
    font-size: 11px;
    color: var(--accent);
  }
  .out {
    flex: 1;
    min-height: 180px;
    max-height: 46vh;
    overflow: auto;
    margin: 0;
    background: var(--code-bg);
    border: 1px solid var(--border-soft);
    border-radius: 9px;
    padding: 9px 11px;
    font-family: var(--mono);
    font-size: 11.5px;
    color: var(--text-dim);
    white-space: pre-wrap;
  }
  .hint {
    font-size: 10.5px;
    color: var(--text-faint);
  }
  .mini {
    font-size: 10.5px;
    padding: 2px 8px;
  }
</style>
