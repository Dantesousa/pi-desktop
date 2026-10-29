<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { app } from "../store.svelte";

  const THINKING = ["off", "minimal", "low", "medium", "high", "xhigh", "max"];
  let showSettings = $state(false);

  const grouped = $derived.by(() => {
    const map = new Map<string, any[]>();
    for (const m of app.models) {
      if (!map.has(m.provider)) map.set(m.provider, []);
      map.get(m.provider)!.push(m);
    }
    return [...map.entries()];
  });

  let modelValue = $state("");
  $effect(() => {
    modelValue = app.model ? `${app.model.provider}::${app.model.id}` : "";
  });

  async function onModel(e: Event) {
    const v = (e.target as HTMLSelectElement).value;
    if (!v) return;
    const [provider, modelId] = v.split("::");
    await app.setModel(provider, modelId);
  }

  async function onThinking(e: Event) {
    await app.setThinking((e.target as HTMLSelectElement).value);
  }

  async function pickProject() {
    const dir = await open({ directory: true, multiple: false });
    if (dir) await app.setCwd(dir as string);
  }

  async function exportHtml() {
    try {
      const path = await app.exportHtml();
      if (path) app.toast(`exportado: ${path}`, "ok");
    } catch (e: any) {
      app.toast(`export falhou: ${e?.message ?? e}`, "error");
    }
  }
</script>

<header>
  <div class="session">
    <span class="dot" class:live={app.conn === "ready"}></span>
    <span class="title">{app.sessionName ?? app.sessionId?.slice(0, 8) ?? "nova sessão"}</span>
    <button class="proj" title="trocar projeto" onclick={pickProject}>
      📁 {app.cwd.split("/").pop() || "/"}
    </button>
  </div>

  <div class="controls">
    <select value={modelValue} onchange={onModel} title="modelo">
      {#if !app.models.length}
        <option value="">{app.modelLabel}</option>
      {/if}
      {#each grouped as [provider, models]}
        <optgroup label={provider}>
          {#each models as m}
            <option value={`${m.provider}::${m.id}`}>{m.name ?? m.id}</option>
          {/each}
        </optgroup>
      {/each}
    </select>

    <select value={app.thinking} onchange={onThinking} title="thinking level">
      {#each THINKING as t}
        <option value={t}>{t}</option>
      {/each}
    </select>

    <div class="meter" title={`contexto: ${app.contextTokens}`}>
      <div class="bar">
        <div class="fill" style={`width:${app.contextPercent}%`} class:hot={app.contextPercent > 80}></div>
      </div>
      <span class="pct">{app.contextPercent}%</span>
    </div>

    {#if app.cost > 0}
      <span class="cost" class:warn={app.costWarn}>${app.cost.toFixed(3)}</span>
    {/if}
    {#if app.pending > 0}
      <span class="queue">{app.pending} na fila</span>
    {/if}

    <button class="ghost" title="Exportar HTML" onclick={exportHtml}>⤓</button>
    <button
      class="ghost"
      class:active={app.showMemory}
      title="Memória (Ctrl+M)"
      onclick={() => (app.showMemory = !app.showMemory)}
    >
      🧠
    </button>
    <button class="ghost" class:active={showSettings} title="Ajustes" onclick={() => (showSettings = !showSettings)}>
      ⚙
    </button>
  </div>

  {#if showSettings}
    <div class="settings">
      <label>
        <span>Permissões</span>
        <select value={app.gateMode} onchange={(e) => app.setGate((e.target as HTMLSelectElement).value)}>
          <option value="off">desligado</option>
          <option value="dangerous">só perigosos</option>
          <option value="all">aprovar tudo</option>
        </select>
      </label>
      <label>
        <span>Limite de custo (US$)</span>
        <input
          type="number"
          min="0"
          step="0.5"
          value={app.costLimit}
          onchange={(e) => app.setCostLimit(Number((e.target as HTMLInputElement).value))}
        />
      </label>
      <label>
        <span>Tema</span>
        <select value={app.theme} onchange={(e) => app.setTheme((e.target as HTMLSelectElement).value)}>
          <option value="dark">escuro</option>
          <option value="light">claro</option>
        </select>
      </label>
      <label>
        <span>Projeto</span>
        <input value={app.cwd} readonly />
      </label>
      <label>
        <span>bin</span>
        <input value={app.bin} readonly />
      </label>
      <div class="foot">
        <button onclick={() => app.connect()}>reconectar</button>
        <button onclick={() => (showSettings = false)}>fechar</button>
      </div>
    </div>
  {/if}
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 9px 18px;
    border-bottom: 1px solid var(--border-soft);
    background: var(--bg-2);
    position: relative;
  }
  .session {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 1;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--red);
    flex: 0 0 auto;
  }
  .dot.live {
    background: var(--green);
    box-shadow: 0 0 8px rgba(126, 231, 135, 0.55);
  }
  .title {
    font-weight: 600;
    font-size: 12.5px;
    white-space: nowrap;
  }
  .proj {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-faint);
    border: 1px solid transparent;
    padding: 2px 7px;
    border-radius: 6px;
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .proj:hover {
    border-color: var(--border);
    color: var(--text-dim);
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  select {
    padding: 4px 7px;
    font-size: 11.5px;
    max-width: 220px;
    background: var(--panel);
  }
  .meter {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .bar {
    width: 64px;
    height: 5px;
    background: #1c2333;
    border-radius: 4px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: linear-gradient(90deg, #4f8cf7, #7aa2f7);
    transition: width 0.3s;
  }
  .fill.hot {
    background: linear-gradient(90deg, #e3b341, #ff7b72);
  }
  .pct {
    font-size: 10.5px;
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
    width: 28px;
  }
  .cost {
    font-size: 10.5px;
    color: var(--text-faint);
    font-family: var(--mono);
  }
  .cost.warn {
    color: var(--yellow);
  }
  .queue {
    font-size: 10.5px;
    color: var(--yellow);
  }
  button.active {
    border-color: #35507f;
    background: var(--panel-2);
  }
  .settings {
    position: absolute;
    top: 46px;
    right: 14px;
    width: 320px;
    background: var(--panel);
    border: 1px solid #2b3550;
    border-radius: 12px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 9px;
    z-index: 40;
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.5);
  }
  .settings label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 10.5px;
    color: var(--text-faint);
  }
  .settings input,
  .settings select {
    max-width: none;
    font-size: 11.5px;
  }
  .foot {
    display: flex;
    gap: 7px;
    justify-content: flex-end;
    margin-top: 2px;
  }
</style>
