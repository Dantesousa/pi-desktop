<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { app, THEMES } from "../store.svelte";
  import { api } from "../api";

  let showQueue = $state(false);
  let showSettings = $state(
    typeof location !== "undefined" && new URLSearchParams(location.search).get("settings") === "1",
  );
  let piVersion = $state<any>(null);
  let updating = $state(false);

  async function checkPi() {
    try {
      piVersion = await api.piVersion();
      if (piVersion.hasUpdate) {
        app.toast(`pi ${piVersion.latest} disponível (atual ${piVersion.current})`, "warn");
      } else {
        app.toast(`pi ${piVersion.current} — já está atualizado`, "ok");
      }
    } catch (e: any) {
      app.toast(`verificação falhou: ${e?.message ?? e}`, "error");
    }
  }

  async function updatePi() {
    updating = true;
    try {
      const out = await api.piSelfUpdate();
      app.toast(`pi atualizado: ${out.split("\n").slice(-2).join(" ").slice(0, 160)}`, "ok");
      piVersion = await api.piVersion();
      await app.connect();
    } catch (e: any) {
      app.toast(`update falhou: ${e?.message ?? e}`, "error");
    } finally {
      updating = false;
    }
  }

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
      {#each app.active?.thinkingLevels ?? ["off"] as t}
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
      <button class="queue" onclick={() => (showQueue = !showQueue)} title="fila de mensagens">
        {app.pending} na fila
      </button>
    {/if}
    {#if app.active?.retrying}
      <button class="queue retry" onclick={() => app.abortRetry()} title="cancelar retry">
        retry…

        <span class="x">×</span>
      </button>
    {/if}

    <button class="ghost" title="Recarregar sessão do disco (F5)" onclick={() => app.reloadSession()}>↻</button>
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

  {#if showQueue && app.pending > 0}
    <div class="queuebox">
      {#if app.active?.queue.steering.length}
        <div class="qlabel">desvio (steer)</div>
        {#each app.active.queue.steering as q}
          <div class="qitem">{q}</div>
        {/each}
      {/if}
      {#if app.active?.queue.followUp.length}
        <div class="qlabel">depois (follow-up)</div>
        {#each app.active.queue.followUp as q}
          <div class="qitem">{q}</div>
        {/each}
      {/if}
      <div class="qfoot">
        <button class="mini" onclick={() => { showQueue = false; app.clearQueue(); }}>limpar fila</button>
      </div>
    </div>
  {/if}

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
      <div class="themes">
        <span class="lbl">tema</span>
        <div class="grid">
          {#each THEMES as t}
            <button
              class="sw"
              class:active={app.theme === t.id}
              title={t.name}
              onclick={() => app.setTheme(t.id)}
            >
              <span class="dots">
                <i style={`background:${t.colors[0]}`}></i>
                <i style={`background:${t.colors[1]}`}></i>
                <i style={`background:${t.colors[2]}`}></i>
              </span>
              <span class="nm">{t.name}</span>
            </button>
          {/each}
        </div>
      </div>
      <div class="row2">
        <label class="accent">
          <span>destaque</span>
          <input
            type="color"
            value={app.accent || "#7aa2f7"}
            oninput={(e) => app.setAccent((e.target as HTMLInputElement).value)}
          />
        </label>
        <button class="mini" onclick={() => app.setAccent("")} title="voltar à cor do tema">auto</button>
        <label class="fontsel">
          <span>texto</span>
          <select
            value={String(app.chatFont)}
            onchange={(e) => app.setChatFont(Number((e.target as HTMLSelectElement).value))}
          >
            <option value="13">13px</option>
            <option value="14">14px</option>
            <option value="15">15px</option>
            <option value="16">16px</option>
          </select>
        </label>
      </div>
      <label>
        <span>Projeto</span>
        <input value={app.cwd} readonly />
      </label>
      <label>
        <span>bin</span>
        <input value={app.bin} readonly />
      </label>
      <div class="toggles">
        <label class="cb">
          <input
            type="checkbox"
            checked={app.active?.autoCompaction ?? true}
            onchange={(e) => app.setAutoCompaction((e.target as HTMLInputElement).checked)}
          />
          compactação automática
        </label>
        <label class="cb">
          <input
            type="checkbox"
            checked={app.active?.autoRetry ?? true}
            onchange={(e) => app.setAutoRetry((e.target as HTMLInputElement).checked)}
          />
          retry automático
        </label>
      </div>
      <label>
        <span>durante a execução (steer)</span>
        <select
          value={app.active?.steeringMode ?? "all"}
          onchange={(e) => app.setSteeringMode((e.target as HTMLSelectElement).value)}
        >
          <option value="all">entregar todas</option>
          <option value="one-at-a-time">uma por vez</option>
        </select>
      </label>
      <label>
        <span>depois de terminar (follow-up)</span>
        <select
          value={app.active?.followUpMode ?? "one-at-a-time"}
          onchange={(e) => app.setFollowUpMode((e.target as HTMLSelectElement).value)}
        >
          <option value="all">entregar todas</option>
          <option value="one-at-a-time">uma por vez</option>
        </select>
      </label>
      <div class="toolrow">
        <button class="mini" onclick={() => (app.showBash = true)}>bash (Ctrl+B)</button>
        <button class="mini" onclick={() => (app.showFork = true)}>retomar de…</button>
        <button class="mini" onclick={() => (app.showTree = true)}>árvore</button>
        <button class="mini" onclick={() => app.cloneSession()}>clonar</button>
      </div>

      <div class="piver">
        <span class="faint">pi {piVersion?.current ?? "—"}{piVersion?.latest ? ` · latest ${piVersion.latest}` : ""}</span>
        <button class="mini" onclick={checkPi} disabled={updating}>verificar</button>
        <button class="mini" onclick={updatePi} disabled={updating}>
          {updating ? "atualizando…" : "atualizar"}
        </button>
      </div>
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
    min-width: 0;
    overflow: visible;
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
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
  }
  .controls button.ghost {
    color: var(--text-dim);
  }
  select {
    padding: 4px 7px;
    font-size: 11.5px;
    max-width: 200px;
    min-width: 0;
    background: var(--panel);
  }
  @media (max-width: 1280px) {
    .cost,
    .queue {
      display: none;
    }
    select {
      max-width: 165px;
    }
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
    background: linear-gradient(90deg, var(--accent), var(--accent-2));
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
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 1px 8px;
  }
  .queue.retry {
    color: var(--red);
  }
  .queuebox {
    position: absolute;
    top: 46px;
    right: 220px;
    width: 320px;
    max-height: 40vh;
    overflow: auto;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 11px;
    padding: 10px;
    z-index: 45;
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .qlabel {
    font-size: 9.5px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-faint);
  }
  .qitem {
    font-size: 11.5px;
    background: var(--bg-2);
    border: 1px solid var(--border-soft);
    border-radius: 7px;
    padding: 5px 7px;
    color: var(--text-dim);
  }
  .qfoot {
    display: flex;
    justify-content: flex-end;
  }
  .toggles {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .cb {
    flex-direction: row !important;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-dim);
  }
  .toolrow {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  button.active {
    border-color: #35507f;
    background: var(--panel-2);
  }
  .settings {
    position: absolute;
    top: 46px;
    right: 14px;
    width: 344px;
    max-height: calc(100vh - 70px);
    overflow-y: auto;
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
  .themes {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .lbl {
    font-size: 10.5px;
    color: var(--text-faint);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 4px;
  }
  .sw {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 5px 7px;
    border-radius: 8px;
    border: 1px solid var(--border);
    font-size: 11px;
    text-align: left;
    min-width: 0;
  }
  .sw:hover {
    background: var(--panel-2);
  }
  .sw.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent) inset;
  }
  .dots {
    display: flex;
    gap: 2px;
    flex: 0 0 auto;
  }
  .dots i {
    width: 8px;
    height: 14px;
    border-radius: 3px;
    display: block;
  }
  .nm {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row2 {
    display: flex;
    align-items: flex-end;
    gap: 7px;
  }
  .row2 .accent,
  .row2 .fontsel {
    flex: 1;
  }
  .row2 input[type="color"] {
    width: 100%;
    height: 26px;
    padding: 2px;
    background: var(--bg-2);
  }
  .piver {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10.5px;
    border-top: 1px solid var(--border-soft);
    padding-top: 8px;
    flex-wrap: wrap;
  }
  .piver .faint {
    flex: 1;
    font-family: var(--mono);
  }
  .mini {
    font-size: 10.5px;
    padding: 2px 7px;
  }
</style>
