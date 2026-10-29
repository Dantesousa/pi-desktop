<script lang="ts">
  import { app } from "../store.svelte";
  import { api } from "../api";
  import type { MemoryFile, SkillInfo, SessionHit } from "../types";
  import { relTime } from "../markdown";

  type Tab = "memory" | "user" | "failure" | "sessions" | "skills" | "mcp";

  let tab = $state<Tab>("memory");
  let file = $state<MemoryFile | null>(null);
  let loading = $state(false);
  let err = $state<string | null>(null);
  let editing = $state<number | null>(null);
  let draft = $state("");
  let adding = $state(false);
  let hits = $state<SessionHit[]>([]);
  let query = $state("");
  let skills = $state<SkillInfo[]>([]);
  let skillBody = $state<string | null>(null);
  let skillName = $state("");
  let skillPath = $state("");
  let skillDirty = $state(false);
  let mcpServers = $state<any[]>([]);
  let mcpRaw = $state<any>(null);
  let mcpOutput = $state<string | null>(null);
  let mcpBusy = $state(false);
  let newName = $state("");
  let newCommand = $state("");
  let newUrl = $state("");

  const tabs: { id: Tab; label: string }[] = [
    { id: "memory", label: "memória" },
    { id: "user", label: "usuário" },
    { id: "failure", label: "falhas" },
    { id: "sessions", label: "sessões" },
    { id: "skills", label: "skills" },
    { id: "mcp", label: "mcp" },
  ];

  async function load() {
    if (tab === "sessions") return;
    if (tab === "skills") {
      skills = await api.skillsList();
      return;
    }
    if (tab === "mcp") {
      mcpRaw = await api.mcpConfig();
      mcpServers = await api.mcpServers();
      return;
    }
    loading = true;
    err = null;
    try {
      file = await api.memoryRead(tab);
      editing = null;
      adding = false;
    } catch (e: any) {
      err = String(e?.message ?? e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    tab;
    load();
  });

  async function saveAll(raws: string[]) {
    if (!file) return;
    try {
      file = await api.memoryWrite(file.target, raws);
    } catch (e: any) {
      err = String(e?.message ?? e);
    }
  }

  async function saveEdit(i: number) {
    if (!file) return;
    const raws = file.entries.map((e) => e.raw);
    raws[i] = draft.trim();
    await saveAll(raws);
    editing = null;
  }

  async function remove(i: number) {
    if (!file) return;
    if (!confirm("Apagar esta entrada da memória?")) return;
    const raws = file.entries.map((e) => e.raw).filter((_, idx) => idx !== i);
    await saveAll(raws);
  }

  async function addEntry() {
    if (!file || !draft.trim() || !file.entries) return;
    const raws = file.entries.map((e) => e.raw);
    raws.push(draft.trim());
    await saveAll(raws);
    adding = false;
    draft = "";
  }

  function startEdit(i: number, raw: string) {
    editing = i;
    draft = raw;
  }

  async function search() {
    if (!query.trim()) return;
    try {
      hits = await api.sessionSearch(query, 40);
    } catch (e: any) {
      err = String(e?.message ?? e);
    }
  }

  async function openSkill(s: SkillInfo) {
    skillName = s.name;
    skillPath = s.path;
    skillDirty = false;
    skillBody = await api.skillRead(s.path);
  }

  async function saveSkill() {
    if (!skillBody || !skillPath) return;
    try {
      await api.skillWrite(skillPath, skillBody);
      skillDirty = false;
      await app.loadSkills();
      skills = app.skills;
      app.toast("skill salva", "ok");
    } catch (e: any) {
      err = String(e?.message ?? e);
    }
  }

  async function newSkill() {
    const name = prompt("nome da skill (slug):");
    if (!name) return;
    const body = `---\nname: ${name}\ndescription: descreva quando usar esta skill (o gatilho que o usuário digita)\n---\n\n## When to Use\n\n- \n\n## Procedure\n\n1. \n\n## Pitfalls\n\n- \n\n## Verification\n\n- \n`;
    try {
      const path = await api.skillCreate(name, body);
      await app.loadSkills();
      skills = app.skills;
      skillName = name;
      skillPath = path;
      skillBody = body;
      skillDirty = false;
    } catch (e: any) {
      err = String(e?.message ?? e);
    }
  }

  async function consolidate() {
    await app.prompt(
      `Revise a memória persistente em ~/.pi/agent/pi-hermes-memory/ (MEMORY.md, USER.md, failures.md) e consolide entradas redundantes ou obsoletas usando as ferramentas memory_* / edit, respeitando os limites de caracteres. Não apague informação sobre preferências do usuário. Ao final, rode /memory-sync-markdown.`,
      [],
    );
    app.toast("consolidação enviada ao agente", "info");
  }

  async function syncIndex() {
    await app.syncMemoryIndex();
  }

  async function saveMcp(mutate: (cfg: any) => void) {
    if (!mcpRaw) return;
    const cfg = JSON.parse(JSON.stringify(mcpRaw));
    cfg.mcpServers = cfg.mcpServers ?? {};
    mutate(cfg);
    try {
      await api.mcpSave(cfg);
      mcpRaw = cfg;
      mcpServers = await api.mcpServers();
      app.toast("mcp.json salvo — reinicie a aba para aplicar", "ok");
    } catch (e: any) {
      err = String(e?.message ?? e);
    }
  }

  async function toggleServer(name: string, enabled: boolean) {
    await saveMcp((cfg) => {
      cfg.mcpServers[name] = { ...(cfg.mcpServers[name] ?? {}), enabled };
    });
  }

  async function removeServer(name: string) {
    if (!confirm(`Remover o servidor MCP "${name}"?`)) return;
    await saveMcp((cfg) => {
      delete cfg.mcpServers[name];
    });
  }

  async function addServer() {
    if (!newName.trim()) return;
    const name = newName.trim();
    const url = newUrl.trim();
    const cmd = newCommand.trim();
    if (!url && !cmd) {
      err = "informe comando ou URL";
      return;
    }
    await saveMcp((cfg) => {
      if (url) {
        cfg.mcpServers[name] = { url };
      } else {
        const [command, ...args] = cmd.split(/\s+/);
        cfg.mcpServers[name] = { command, args };
      }
    });
    newName = "";
    newCommand = "";
    newUrl = "";
  }

  async function checkMcp() {
    mcpBusy = true;
    mcpOutput = null;
    try {
      mcpOutput = await api.mcpCheck(app.cwd);
    } catch (e: any) {
      mcpOutput = String(e?.message ?? e);
    } finally {
      mcpBusy = false;
    }
  }

  const pct = $derived(file ? Math.min(100, (file.chars / file.limit) * 100) : 0);
</script>

<section>
  <div class="tabs">
    {#each tabs as t}
      <button class="tab" class:active={tab === t.id} onclick={() => (tab = t.id)}>{t.label}</button>
    {/each}
    <span class="spacer"></span>
    <button class="ghost mini" title="fechar" onclick={() => (app.showMemory = false)}>×</button>
  </div>

  {#if err}
    <div class="err">{err}</div>
  {/if}

  {#if tab !== "sessions" && tab !== "skills"}
    {#if file}
      <div class="usage">
        <div class="ubar"><div class="ufill" style={`width:${pct}%`}></div></div>
        <span>{file.chars.toLocaleString("pt-BR")} / {file.limit.toLocaleString("pt-BR")} chars</span>
        <span class="faint">· {file.entries.length} entradas</span>
      </div>
      <div class="toolbar">
        <button class="mini" onclick={() => { adding = true; draft = ""; editing = null; }}>+ entrada</button>
        <button class="mini" onclick={syncIndex} title="reconciliar o índice SQLite com os .md">
          ⟳ sync
        </button>
        <button class="mini" onclick={consolidate} title="pedir ao agente para consolidar a memória">
          ✦ consolidar
        </button>
        <button class="mini" onclick={load}>↻</button>
      </div>

      {#if adding}
        <div class="editor">
          <textarea bind:value={draft} placeholder="nova entrada…" rows="4"></textarea>
          <div class="row">
            <button class="mini" onclick={() => (adding = false)}>cancelar</button>
            <button class="mini primary" onclick={addEntry}>salvar</button>
          </div>
        </div>
      {/if}

      <div class="entries">
        {#each file.entries as entry, i}
          <div class="entry">
            <div class="ehead">
              {#if entry.category}<span class="cat">{entry.category}</span>{/if}
              <span class="faint">{entry.created ?? ""}</span>
              <span class="spacer"></span>
              {#if editing !== i}
                <button class="mini" onclick={() => startEdit(i, entry.raw)}>editar</button>
                <button class="mini danger" onclick={() => remove(i)}>×</button>
              {/if}
            </div>
            {#if editing === i}
              <textarea bind:value={draft} rows="5"></textarea>
              <div class="row">
                <button class="mini" onclick={() => (editing = null)}>cancelar</button>
                <button class="mini primary" onclick={() => saveEdit(i)}>salvar</button>
              </div>
            {:else}
              <div class="body">{entry.body}</div>
              {#if entry.failure_reason}
                <div class="reason">⚠ {entry.failure_reason}</div>
              {/if}
            {/if}
          </div>
        {:else}
          <div class="empty">{loading ? "carregando…" : "vazio"}</div>
        {/each}
      </div>
    {/if}
  {/if}

  {#if tab === "sessions"}
    <div class="searchbox">
      <input
        bind:value={query}
        placeholder="buscar no histórico (FTS)…"
        onkeydown={(e) => e.key === "Enter" && search()}
      />
      <button class="mini" onclick={search}>buscar</button>
    </div>
    <div class="hits">
      {#each hits as h}
        <div class="hit">
          <div class="hmeta">
            <span class="role" class:user={h.role === "user"}>{h.role}</span>
            <span class="faint">{relTime(h.timestamp)}</span>
          </div>
          <div class="hbody">{h.content.slice(0, 500)}</div>
        </div>
      {:else}
        <div class="empty">{query ? "nada encontrado" : "digite pra buscar"}</div>
      {/each}
    </div>
  {/if}

  {#if tab === "mcp"}
    <div class="toolbar">
      <button class="mini" onclick={load}>↻</button>
      <button class="mini" onclick={checkMcp} disabled={mcpBusy}>
        {mcpBusy ? "testando…" : "testar conexão"}
      </button>
      <span class="faint small">{mcpServers.length} servidor(es)</span>
    </div>
    <div class="entries">
      {#each mcpServers as srv}
        <div class="entry">
          <div class="ehead">
            <span class="cat">{srv.kind}</span>
            <span class="faint small">{srv.name}</span>
            <span class="spacer"></span>
            <button class="mini" onclick={() => toggleServer(srv.name, !srv.enabled)}>
              {srv.enabled ? "on" : "off"}
            </button>
            <button class="mini danger" onclick={() => removeServer(srv.name)}>×</button>
          </div>
          <div class="faint small mono">{srv.target}</div>
          {#if srv.description}<div class="faint small">{srv.description}</div>{/if}
        </div>
      {:else}
        <div class="empty">nenhum servidor MCP configurado</div>
      {/each}

      <div class="entry addbox">
        <div class="faint small">adicionar servidor</div>
        <input bind:value={newName} placeholder="nome" />
        <input bind:value={newCommand} placeholder="comando + args (stdio)" />
        <input bind:value={newUrl} placeholder="ou URL (http)" />
        <div class="row">
          <button class="mini primary" onclick={addServer}>adicionar</button>
        </div>
      </div>

      {#if mcpOutput}
        <pre class="mcpsetup">{mcpOutput}</pre>
      {/if}
    </div>
  {/if}

  {#if tab === "skills"}
    {#if skillBody !== null}
      <div class="toolbar">
        <button class="mini" onclick={() => { skillBody = null; skillDirty = false; }}>← voltar</button>
        <span class="faint">{skillName}</span>
        <span class="spacer"></span>
        <button class="mini primary" onclick={saveSkill} disabled={!skillDirty}>salvar</button>
      </div>
      <textarea
        class="skilledit"
        bind:value={skillBody}
        oninput={() => (skillDirty = true)}
        spellcheck="false"
      ></textarea>
    {:else}
      <div class="toolbar">
        <button class="mini" onclick={newSkill}>+ nova skill</button>
        <button class="mini" onclick={async () => { await app.loadSkills(); skills = app.skills; }}>↻</button>
      </div>
      <div class="entries">
        {#each skills as s}
          <button class="entry skill" onclick={() => openSkill(s)}>
            <div class="ehead">
              <span class="cat">{s.scope}</span>
              <span class="spacer"></span>
            </div>
            <div class="body"><strong>{s.name}</strong></div>
            <div class="faint small">{s.description}</div>
          </button>
        {:else}
          <div class="empty">nenhuma skill</div>
        {/each}
      </div>
    {/if}
  {/if}
</section>

<style>
  section {
    width: 380px;
    flex: 0 0 auto;
    border-left: 1px solid var(--border-soft);
    background: var(--bg-2);
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 8px 8px 0;
    border-bottom: 1px solid var(--border-soft);
  }
  .tab {
    border: none;
    border-radius: 7px 7px 0 0;
    padding: 5px 9px;
    font-size: 11.5px;
    color: var(--text-faint);
  }
  .tab.active {
    background: var(--panel);
    color: var(--text);
    box-shadow: inset 0 -2px 0 var(--accent);
  }
  .spacer {
    flex: 1;
  }
  .usage {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 8px 11px 4px;
    font-size: 10.5px;
    color: var(--text-dim);
  }
  .ubar {
    width: 70px;
    height: 5px;
    background: #1c2333;
    border-radius: 4px;
    overflow: hidden;
  }
  .ufill {
    height: 100%;
    background: linear-gradient(90deg, #4f8cf7, #b48ef7);
  }
  .faint {
    color: var(--text-faint);
  }
  .toolbar {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 4px 11px 8px;
  }
  .mini {
    font-size: 10.5px;
    padding: 3px 8px;
  }
  .mini.primary {
    background: #2f4f8f;
    border-color: #3a5c9e;
  }
  .entries {
    flex: 1;
    overflow-y: auto;
    padding: 0 11px 14px;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .entry {
    background: var(--panel);
    border: 1px solid var(--border-soft);
    border-radius: 9px;
    padding: 7px 9px;
    display: block;
    text-align: left;
    width: 100%;
  }
  .ehead {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    margin-bottom: 3px;
  }
  .cat {
    background: #22304a;
    color: #9fc0ff;
    border-radius: 5px;
    padding: 0 5px;
    font-family: var(--mono);
    font-size: 9.5px;
  }
  .body {
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    color: #c6d2e8;
  }
  .small {
    font-size: 11px;
    margin-top: 3px;
  }
  .reason {
    margin-top: 4px;
    font-size: 11px;
    color: #d9a7a3;
  }
  .row {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
    margin-top: 5px;
  }
  textarea {
    width: 100%;
    font-size: 12px;
    margin-top: 4px;
  }
  .err {
    margin: 8px 11px;
    color: #ffada6;
    font-size: 11.5px;
    background: #2a1616;
    border: 1px solid #4a2424;
    border-radius: 8px;
    padding: 5px 8px;
  }
  .empty {
    color: var(--text-faint);
    text-align: center;
    font-size: 11.5px;
    padding: 18px;
  }
  .searchbox {
    display: flex;
    gap: 6px;
    padding: 9px 11px;
  }
  .searchbox input {
    flex: 1;
    font-size: 12px;
    padding: 5px 8px;
  }
  .hits {
    flex: 1;
    overflow-y: auto;
    padding: 0 11px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .hit {
    background: var(--panel);
    border: 1px solid var(--border-soft);
    border-radius: 8px;
    padding: 6px 9px;
  }
  .hmeta {
    display: flex;
    gap: 7px;
    font-size: 10px;
    margin-bottom: 2px;
  }
  .role {
    color: #9fc0ff;
    font-family: var(--mono);
  }
  .role.user {
    color: #b9a4ff;
  }
  .hbody {
    font-size: 11.5px;
    color: #b8c4da;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 130px;
    overflow: hidden;
  }
  .mono {
    font-family: var(--mono);
  }
  .addbox {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .addbox input {
    font-size: 11.5px;
    padding: 4px 7px;
  }
  .mcpsetup {
    background: var(--panel);
    border: 1px solid var(--border-soft);
    border-radius: 9px;
    padding: 9px;
    font-size: 10.5px;
    white-space: pre-wrap;
    max-height: 260px;
    overflow: auto;
    color: #b8c4da;
    margin: 6px 0 0;
  }
  .skillbody,
  .skilledit {
    flex: 1;
    overflow: auto;
    margin: 0 11px 12px;
    background: var(--panel);
    border: 1px solid var(--border-soft);
    border-radius: 9px;
    padding: 10px;
    font-family: var(--mono);
    font-size: 11px;
    line-height: 1.55;
    white-space: pre-wrap;
    color: #b8c4da;
    resize: none;
  }
</style>
