<script lang="ts">
  import { t } from "../i18n.svelte";
  import { app } from "../store.svelte";
  import { api } from "../api";
  import type { MemoryFile, SkillInfo, SessionHit } from "../types";
  import { relTime } from "../markdown";

  type Tab = "memory" | "user" | "failure" | "sessions" | "skills" | "mcp";

  const qs = typeof location !== "undefined" ? new URLSearchParams(location.search) : null;
  const initialTab = qs?.get("tab") as Tab | null;
  let tab = $state<Tab>(
    initialTab && ["memory", "user", "failure", "sessions", "skills", "mcp"].includes(initialTab)
      ? initialTab
      : "memory",
  );
  const initialView = qs?.get("view");
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
  let mcpView = $state<"list" | "hub">(initialView === "hub" ? "hub" : "list");
  let hubQuery = $state("");
  let hubResults = $state<any[]>([]);
  let hubBusy = $state(false);
  let hubError = $state<string | null>(null);
  let skillView = $state<"list" | "hub">(initialView === "hub" ? "hub" : "list");
  let repoResults = $state<any[]>([]);
  let repoSkills = $state<any[]>([]);
  let activeRepo = $state<string | null>(null);
  let mcpStatuses = $state<any[]>([]);
  let skillSources = $state<any[]>([]);
  let openTools = $state<string | null>(null);
  let newName = $state("");
  let newCommand = $state("");
  let newUrl = $state("");

  const tabs = $derived<{ id: Tab; label: string }[]>([
    { id: "memory", label: t("mem.tab.memory") },
    { id: "user", label: t("mem.tab.user") },
    { id: "failure", label: t("mem.tab.failure") },
    { id: "sessions", label: t("mem.tab.sessions") },
    { id: "skills", label: t("mem.tab.skills") },
    { id: "mcp", label: t("mem.tab.mcp") },
  ]);

  async function load() {
    if (tab === "sessions") return;
    loading = true;
    err = null;
    try {
      if (tab === "skills") {
        file = null;
        skills = await api.skillsList();
        skillSources = await api.hubSkillsSources();
        return;
      }
      if (tab === "mcp") {
        file = null;
        mcpRaw = await api.mcpConfig();
        mcpServers = await api.mcpServers();
        await refreshStatuses();
        return;
      }
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
    void load().catch(() => {});
  });

  $effect(() => {
    // curated list as soon as the skills hub opens
    if (tab === "skills" && skillView === "hub" && !repoResults.length && !activeRepo) {
      void searchSkillsHub();
    }
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
    const ok = await app.askConfirm({
      title: t("mem.deleteTitle"),
      message: (file.entries[i].body ?? "").slice(0, 200),
      confirmLabel: t("mem.deleteAction"),
      danger: true,
    });
    if (!ok) return;
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
      app.toast(t("mem.skillSaved"), "ok");
    } catch (e: any) {
      err = String(e?.message ?? e);
    }
  }

  async function newSkill() {
    const name = await app.askPrompt({
      title: t("mem.skillNewTitle"),
      message: t("mem.skillNewHint"),
      placeholder: t("mem.skillNewPlaceholder"),
      confirmLabel: t("mem.skillCreate"),
    });
    if (!name) return;
    const body = t("mem.skillTemplate", { name });
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
    await app.prompt(t("mem.consolidatePrompt"), []);
    app.toast(t("mem.consolidated"), "info");
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
      app.toast(t("mem.mcpSaved"), "ok");
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
    const ok = await app.askConfirm({
      title: t("mem.mcpRemoveTitle"),
      message: t("mem.mcpRemoveBody", { name }),
      confirmLabel: t("mem.mcpRemove"),
      danger: true,
    });
    if (!ok) return;
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
      err = t("mem.mcpCommand");
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

  async function refreshStatuses() {
    try {
      mcpStatuses = await api.mcpStatus(app.cwd);
    } catch {
      mcpStatuses = [];
    }
  }

  function statusFor(name: string) {
    return mcpStatuses.find((s) => s.name === name);
  }

  async function checkMcp() {
    mcpBusy = true;
    mcpOutput = null;
    try {
      await refreshStatuses();
      mcpOutput = await api.mcpCheck(app.cwd);
    } catch (e: any) {
      mcpOutput = String(e?.message ?? e);
    } finally {
      mcpBusy = false;
    }
  }

  function sourceFor(skill: any) {
    return skillSources.find((s) => s.slug === skill.name);
  }

  async function syncSkill(skill: any) {
    hubBusy = true;
    try {
      const msg = await api.hubSkillSync(skill.name);
      app.toast(`${skill.name}: ${msg}`, "ok");
      skills = await api.skillsList();
      skillSources = await api.hubSkillsSources();
    } catch (e: any) {
      app.toast(t("toast.syncFailed", { error: e?.message ?? e }), "error");
    } finally {
      hubBusy = false;
    }
  }

  function safeName(raw: string): string {
    return raw
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .split("-")
      .filter(Boolean)
      .slice(-2)
      .join("-");
  }

  async function searchMcpHub() {
    hubBusy = true;
    hubError = null;
    try {
      hubResults = await api.hubMcpSearch(hubQuery, 30);
      if (!hubResults.length) hubError = t("mem.mcpNothing");
    } catch (e: any) {
      hubError = String(e?.message ?? e);
    } finally {
      hubBusy = false;
    }
  }

  async function addCandidate(c: any) {
    const name = safeName(c.name) || safeName(c.title) || "server";
    await saveMcp((cfg) => {
      cfg.mcpServers[name] = c.url
        ? { url: c.url, description: c.description }
        : { command: c.command, args: c.args ?? [], description: c.description };
    });
    app.toast(t("toast.added", { name }), "ok");
  }

  async function searchSkillsHub() {
    hubBusy = true;
    hubError = null;
    repoSkills = [];
    activeRepo = null;
    try {
      repoResults = await api.hubSkillsSearch(hubQuery, 15);
      if (!repoResults.length) hubError = t("mem.noRepo");
    } catch (e: any) {
      hubError = String(e?.message ?? e);
    } finally {
      hubBusy = false;
    }
  }

  async function openRepo(repo: string) {
    hubBusy = true;
    hubError = null;
    activeRepo = repo;
    try {
      repoSkills = await api.hubSkillsInRepo(repo);
      if (!repoSkills.length) hubError = t("mem.noSkillFile");
    } catch (e: any) {
      hubError = String(e?.message ?? e);
    } finally {
      hubBusy = false;
    }
  }

  async function installSkill(s: any) {
    if (!activeRepo) return;
    hubBusy = true;
    try {
      const dest = await api.hubSkillInstall(activeRepo, s.dir);
      await app.loadSkills();
      skills = app.skills;
      app.toast(t("toast.skillInstalled", { path: dest }), "ok");
    } catch (e: any) {
      hubError = String(e?.message ?? e);
    } finally {
      hubBusy = false;
    }
  }

  const pct = $derived(file ? Math.min(100, (file.chars / file.limit) * 100) : 0);
</script>

<section>
  <div class="tabbar">
    <div class="tabs">
      {#each tabs as t}
        <button class="tab" class:active={tab === t.id} onclick={() => (tab = t.id)}>{t.label}</button>
      {/each}
    </div>
    <button class="ghost mini close" title={t("common.close")} onclick={() => (app.showMemory = false)}>×</button>
  </div>

  {#if err}
    <div class="err">{err}</div>
  {/if}

  {#if tab !== "sessions" && tab !== "skills" && tab !== "mcp"}
    {#if file}
      <div class="usage">
        <div class="ubar"><div class="ufill" style={`width:${pct}%`}></div></div>
        <span>{t("mem.chars", { used: file.chars.toLocaleString(), limit: file.limit.toLocaleString() })}</span>
        <span class="faint">{t("mem.entries", { n: file.entries.length })}</span>
      </div>
      <div class="toolbar">
        <button class="mini" onclick={() => { adding = true; draft = ""; editing = null; }}>{t("mem.addEntry")}</button>
        <button class="mini" onclick={syncIndex} title={t("mem.syncHint")}>
          {t("mem.sync")}
        </button>
        <button class="mini" onclick={consolidate} title={t("mem.consolidateHint")}>
          {t("mem.consolidate")}
        </button>
        <button class="mini" onclick={load}>↻</button>
      </div>

      {#if adding}
        <div class="editor">
          <textarea bind:value={draft} placeholder={t("mem.newEntry")} rows="4"></textarea>
          <div class="row">
            <button class="mini" onclick={() => (adding = false)}>{t("common.cancel")}</button>
            <button class="mini primary" onclick={addEntry}>{t("common.save")}</button>
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
                <button class="mini" onclick={() => startEdit(i, entry.raw)}>{t("mem.edit")}</button>
                <button class="mini danger" onclick={() => remove(i)}>×</button>
              {/if}
            </div>
            {#if editing === i}
              <textarea bind:value={draft} rows="5"></textarea>
              <div class="row">
                <button class="mini" onclick={() => (editing = null)}>{t("common.cancel")}</button>
                <button class="mini primary" onclick={() => saveEdit(i)}>{t("common.save")}</button>
              </div>
            {:else}
              <div class="body">{entry.body}</div>
              {#if entry.failure_reason}
                <div class="reason">⚠ {entry.failure_reason}</div>
              {/if}
            {/if}
          </div>
        {:else}
          <div class="empty">{loading ? t("common.loading") : t("common.empty")}</div>
        {/each}
      </div>
    {/if}
  {/if}

  {#if tab === "sessions"}
    <div class="searchbox">
      <input
        bind:value={query}
        placeholder={t("mem.searchHistory")}
        onkeydown={(e) => e.key === "Enter" && search()}
      />
      <button class="mini" onclick={search}>{t("common.search")}</button>
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
        <div class="empty">{query ? t("mem.nothingFound") : t("mem.typeToSearch")}</div>
      {/each}
    </div>
  {/if}

  {#if tab === "mcp"}
    <div class="toolbar">
      <button class="mini" class:sel={mcpView === "list"} onclick={() => (mcpView = "list")}>
        {t("mem.mcpConfigured")}
      </button>
      <button class="mini" class:sel={mcpView === "hub"} onclick={() => (mcpView = "hub")}>
        {t("mem.hub")}
      </button>
      <span class="spacer"></span>
      <button class="mini" onclick={load}>↻</button>
      <button class="mini" onclick={checkMcp} disabled={mcpBusy}>
        {mcpBusy ? t("mem.mcpTesting") : t("mem.mcpTest")}
      </button>
      <span class="faint small">{t("mem.mcpServers", { n: mcpServers.length })}</span>
    </div>
    {#if mcpView === "hub"}
      <div class="searchbox">
        <input
          bind:value={hubQuery}
          placeholder={t("mem.mcpSearch")}
          onkeydown={(e) => e.key === "Enter" && searchMcpHub()}
        />
        <button class="mini" onclick={searchMcpHub} disabled={hubBusy}>{t("common.search")}</button>
      </div>
      {#if hubError}<div class="err">{hubError}</div>{/if}
      <div class="entries">
        {#each hubResults as c}
          <div class="entry">
            <div class="ehead">
              <span class="cat">{c.kind}</span>
              <span class="faint small">{c.title || c.name}</span>
              <span class="spacer"></span>
              <button
                class="mini"
                title={t("mem.askAgentConfig")}
                onclick={() =>
                  app.askAgent(
                    t("mem.mcpConfigPrompt", {
                      name: c.name,
                      target: c.target,
                      desc: c.description ? t("mem.mcpDescLine", { description: c.description }) : "",
                    }),
                  )}
              >
                🤖
              </button>
              <button class="mini primary" onclick={() => addCandidate(c)}>{t("common.add")}</button>
            </div>
            <div class="faint small">{c.description}</div>
            <div class="faint small mono">{c.target}</div>
          </div>
        {:else}
          <div class="empty">{hubBusy ? t("common.loading") : t("mem.mcpSearchEmpty")}</div>
        {/each}
      </div>
    {:else}
    <div class="entries">
      {#each mcpServers as srv}
        {@const st = statusFor(srv.name)}
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
          {#if st}
            <div class="toolsrow">
              {#if st.error}
                <span class="err small">{st.error.slice(0, 90)}</span>
              {:else}
                <button class="mini" onclick={() => (openTools = openTools === srv.name ? null : srv.name)}>
                  {st.tools.length} tools {openTools === srv.name ? "▾" : "▸"}
                </button>
                <span class="faint small">{st.state}</span>
              {/if}
            </div>
            {#if openTools === srv.name && st.tools.length}
              <div class="toollist">
                {#each st.tools as tool}
                  <code>{tool}</code>
                {/each}
              </div>
            {/if}
          {/if}
          {#if srv.description}<div class="faint small">{srv.description}</div>{/if}
        </div>
      {:else}
        <div class="empty">{t("mem.mcpNone")}</div>
      {/each}

      <div class="entry addbox">
        <div class="faint small">{t("mem.mcpAddServer")}</div>
        <input bind:value={newName} placeholder={t("mem.mcpName")} />
        <input bind:value={newCommand} placeholder={t("mem.mcpCommand")} />
        <input bind:value={newUrl} placeholder={t("mem.mcpUrl")} />
        <div class="row">
          <button class="mini primary" onclick={addServer}>{t("common.add")}</button>
        </div>
      </div>

      {#if mcpOutput}
        <pre class="mcpsetup">{mcpOutput}</pre>
      {/if}
    </div>
    {/if}
  {/if}

  {#if tab === "skills"}
    {#if skillBody !== null}
      <div class="toolbar">
        <button class="mini" onclick={() => { skillBody = null; skillDirty = false; }}>{t("mem.back")}</button>
        <span class="faint">{skillName}</span>
        <span class="spacer"></span>
        <button class="mini primary" onclick={saveSkill} disabled={!skillDirty}>{t("common.save")}</button>
      </div>
      <textarea
        class="skilledit"
        bind:value={skillBody}
        oninput={() => (skillDirty = true)}
        spellcheck="false"
      ></textarea>
    {:else if skillView === "hub"}
      <div class="toolbar">
        <button class="mini" onclick={() => { skillView = "list"; repoSkills = []; activeRepo = null; }}>{t("mem.installed")}</button>
        <span class="spacer"></span>
        {#if activeRepo}
          <button class="mini" onclick={() => { activeRepo = null; repoSkills = []; }}>{t("mem.repos")}</button>
        {/if}
      </div>
      {#if !activeRepo}
        <div class="searchbox">
          <input
            bind:value={hubQuery}
            placeholder={t("mem.skillsSearch")}
            onkeydown={(e) => e.key === "Enter" && searchSkillsHub()}
          />
          <button class="mini" onclick={searchSkillsHub} disabled={hubBusy}>{t("common.search")}</button>
        </div>
        {#if hubError}<div class="err">{hubError}</div>{/if}
        <div class="entries">
          {#each repoResults as r}
            <button class="entry" onclick={() => openRepo(r.fullName)}>
              <div class="ehead">
                <span class="cat">{r.hits ? `${r.hits} SKILL.md` : "curada"}</span>
                <span class="spacer"></span>
              </div>
              <div class="body"><strong>{r.fullName}</strong></div>
              <div class="faint small">{r.description}</div>
            </button>
          {:else}
            <div class="empty">{hubBusy ? t("common.loading") : t("mem.skillsSearchEmpty")}</div>
          {/each}
        </div>
      {:else}
        <div class="toolbar"><span class="faint small mono">{activeRepo}</span></div>
        {#if hubError}<div class="err">{hubError}</div>{/if}
        <div class="entries">
          {#each repoSkills as s}
            <div class="entry">
              <div class="ehead">
                <span class="spacer"></span>
                <button
                  class="mini"
                  title={t("mem.askAgentInstall")}
                  onclick={() =>
                    app.askAgent(
                      t("mem.skillInstallPrompt", {
                        name: s.name ?? "",
                        repo: activeRepo ?? "",
                        dir: s.dir || "/",
                      }),
                    )}
                >
                  🤖
                </button>
                <button class="mini primary" onclick={() => installSkill(s)} disabled={hubBusy}>{t("mem.install")}</button>
              </div>
              <div class="body"><strong>{s.name}</strong></div>
              <div class="faint small">{s.dir || "/"} · {t("mem.files", { n: s.files })}</div>
            </div>
          {:else}
            <div class="empty">{hubBusy ? t("common.loading") : t("mem.skillsNone")}</div>
          {/each}
        </div>
      {/if}
    {:else}
      <div class="toolbar">
        <button class="mini" onclick={newSkill}>{t("mem.skillNew")}</button>
        <button class="mini" onclick={() => { skillView = "hub"; searchSkillsHub(); }}>
          {t("mem.hub")}
        </button>
        <button class="mini" onclick={async () => { await app.loadSkills(); skills = app.skills; }}>↻</button>
      </div>
      <div class="entries">
        {#each skills as s}
          {@const src = sourceFor(s)}
          <div class="entry skill">
            <div class="ehead">
              <span class="cat">{s.scope}</span>
              {#if src}<span class="faint small mono">{src.repo}</span>{/if}
              <span class="spacer"></span>
              {#if src}
                <button class="mini" title={t("mem.syncSkill")} onclick={() => syncSkill(s)} disabled={hubBusy}>
                  ⟳
                </button>
              {/if}
            </div>
            <button class="linkish" onclick={() => openSkill(s)}>
              <div class="body"><strong>{s.name}</strong></div>
              <div class="faint small">{s.description}</div>
            </button>
          </div>
        {:else}
          <div class="empty">{t("mem.skillsNone")}</div>
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
  .tabbar {
    display: flex;
    align-items: stretch;
    border-bottom: 1px solid var(--border-soft);
    flex: 0 0 auto;
    min-width: 0;
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 8px 4px 0 8px;
    overflow-x: auto;
    scrollbar-width: none;
    flex: 1 1 auto;
    min-width: 0;
  }
  .tabs::-webkit-scrollbar {
    height: 0;
  }
  .close {
    flex: 0 0 auto;
    align-self: flex-start;
    margin: 6px 6px 0 2px;
    padding: 0 6px;
  }
  .tab {
    white-space: nowrap;
    flex: 0 0 auto;
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
    flex-wrap: wrap;
    padding: 4px 11px 8px;
  }
  .mini {
    font-size: 10.5px;
    padding: 3px 8px;
  }
  .linkish {
    border: none;
    background: none;
    padding: 0;
    text-align: left;
    width: 100%;
    border-radius: 6px;
  }
  .linkish:hover {
    background: none;
  }
  .toolsrow {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
  }
  .toollist {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 5px;
  }
  .toollist code {
    font-size: 9.5px;
    background: var(--code-bg-2);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 0 5px;
    color: var(--text-dim);
  }
  .mini.sel {
    background: var(--panel-2);
    border-color: var(--accent);
    color: var(--text);
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
