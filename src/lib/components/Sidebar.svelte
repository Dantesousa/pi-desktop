<script lang="ts">
  import { t } from "../i18n.svelte";
  import { app } from "../store.svelte";
  import { relTime } from "../markdown";

  let filter = $state("");

  const visible = $derived.by(() => {
    const q = filter.trim().toLowerCase();
    const list = q
      ? app.sessions.filter(
          (s) =>
            (s.name ?? "").toLowerCase().includes(q) ||
            s.cwd.toLowerCase().includes(q) ||
            s.id.includes(q),
        )
      : app.sessions;
    return [...list]
      .sort((a, b) => {
        const pa = app.isPinned(a.path) ? 1 : 0;
        const pb = app.isPinned(b.path) ? 1 : 0;
        if (pa !== pb) return pb - pa;
        return b.modifiedMs - a.modifiedMs;
      })
      .slice(0, 50);
  });

  async function openSession(path: string) {
    try {
      await app.switchSession(path);
    } catch (e: any) {
      app.stderr.push(`switch falhou: ${e?.message ?? e}`);
    }
  }

  async function rename(s: any) {
    const name = await app.askPrompt({
      title: t("sidebar.renameTitle"),
      message: s.path.split("/").pop(),
      value: s.name ?? "",
      placeholder: t("sidebar.renamePlaceholder"),
      confirmLabel: t("common.rename"),
    });
    if (name == null) return;
    await app.renameSession(s, name);
  }

  async function remove(s: any) {
    const ok = await app.askConfirm({
      title: t("sidebar.deleteTitle"),
      message: t("sidebar.deleteBody", { name: s.name ?? s.id.slice(0, 8) }),
      confirmLabel: t("sidebar.trashAction"),
      danger: true,
    });
    if (!ok) return;
    await app.deleteSession(s);
  }
</script>

<aside>
  <div class="brand">
    <div class="logo">◈</div>
    <div>
      <div class="name">
        Hina
        {#if app.build?.channel === "dev"}
          <span class="devchip" title={`dev build ${app.build.commit}`}>DEV</span>
        {/if}
      </div>
      <div class="sub">pi desktop</div>
    </div>
  </div>

  <button class="primary new" onclick={() => app.newSession()}>{t("sidebar.newChat")}</button>

  <input class="filter" placeholder={t("sidebar.filter")} bind:value={filter} />

  <div class="sessions">
    {#each visible as s (s.path)}
      <div class="row" class:active={s.id === app.sessionId} class:pinned={app.isPinned(s.path)}>
        <button class="item" onclick={() => openSession(s.path)} title={`${s.cwd}\n${s.messages} mensagens`}>
          <span class="nm">
            {#if app.isPinned(s.path)}<span class="pin">📌</span>{/if}
            {s.name ?? s.id.slice(0, 8)}
          </span>
          <span class="meta">{s.cwd.split("/").pop() || "/"} · {relTime(s.modifiedMs)}</span>
        </button>
        <div class="acts">
          <button class="act" title={t("sidebar.pin")} onclick={() => app.togglePin(s.path)}>
            {app.isPinned(s.path) ? "📌" : "📍"}
          </button>
          <button class="act" title={t("sidebar.rename")} onclick={() => rename(s)}>✎</button>
          <button class="act danger" title={t("sidebar.delete")} onclick={() => remove(s)}>🗑</button>
        </div>
      </div>
    {:else}
      <div class="empty">{t("sidebar.noSessions")}</div>
    {/each}
  </div>

  <div class="foot">
    <button class="ghost" class:active={app.showMemory} onclick={() => (app.showMemory = !app.showMemory)}>
      🧠 {t("sidebar.memory")}
    </button>
    <button class="ghost" title={t("sidebar.paletteHint")} onclick={() => (app.showPalette = true)}>
      {t("sidebar.palette")}
    </button>
    <button class="ghost" title={t("sidebar.refresh")} onclick={() => app.loadSessions()}>⟳</button>
  </div>

  {#if app.conn !== "ready"}
    <div class="conn">
      {app.conn === "starting" ? "starting pi…" : app.conn === "dead" ? "pi stopped" : "—"}
      <button class="mini" onclick={() => app.connect()}>{t("app.reconnect")}</button>
    </div>
  {/if}
</aside>

<style>
  aside {
    width: 250px;
    flex: 0 0 auto;
    border-right: 1px solid var(--border-soft);
    background: var(--bg-2);
    display: flex;
    flex-direction: column;
    padding: 12px 8px 10px;
    gap: 9px;
    height: 100%;
    min-height: 0;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 2px 4px 6px;
  }
  .logo {
    width: 30px;
    height: 30px;
    border-radius: 9px;
    background: linear-gradient(135deg, var(--accent), var(--accent-2));
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 15px;
    color: var(--accent-ink, #fff);
  }
  .name {
    font-weight: 700;
    font-size: 13.5px;
    letter-spacing: 0.01em;
  }
  .devchip {
    font-family: var(--mono);
    font-size: 8.5px;
    letter-spacing: 0.08em;
    padding: 1px 5px;
    border-radius: 999px;
    background: rgba(245, 158, 11, 0.18);
    border: 1px solid rgba(245, 158, 11, 0.55);
    color: #fbbf24;
    vertical-align: middle;
    margin-left: 2px;
  }
  .sub {
    font-size: 10px;
    color: var(--text-faint);
  }
  .new {
    width: 100%;
    padding: 7px;
    font-size: 12px;
    margin: 0 2px;
  }
  .filter {
    color: var(--text);
    font-size: 11.5px;
    padding: 5px 8px;
    background: var(--bg);
    margin: 0 2px;
  }
  .sessions {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
    margin: 0 -2px;
    padding: 0 2px;
  }
  .row {
    display: flex;
    align-items: stretch;
    border-radius: 8px;
  }
  .row:hover {
    background: var(--panel);
  }
  .row.active {
    background: var(--bubble);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .row.pinned .nm {
    color: #ffe6a8;
  }
  .item {
    display: flex;
    flex-direction: column;
    gap: 1px;
    border: none;
    background: transparent;
    border-radius: 8px;
    padding: 6px 8px;
    text-align: left;
    flex: 1;
    min-width: 0;
  }
  .item:hover {
    background: transparent;
  }
  .nm {
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .pin {
    font-size: 9px;
  }
  .meta {
    font-size: 10px;
    color: var(--text-faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .acts {
    display: none;
    align-items: center;
    gap: 1px;
    padding-right: 3px;
  }
  .row:hover .acts {
    display: flex;
  }
  .act {
    border: none;
    padding: 1px 3px;
    font-size: 10px;
    border-radius: 5px;
    opacity: 0.75;
  }
  .act:hover {
    background: #24304a;
    opacity: 1;
  }
  .act.danger:hover {
    background: #3a2028;
  }
  .empty {
    color: var(--text-faint);
    font-size: 11.5px;
    padding: 10px;
    text-align: center;
  }
  .foot {
    display: flex;
    gap: 5px;
    border-top: 1px solid var(--border-soft);
    padding-top: 8px;
    margin: 0 2px;
    flex: 0 0 auto;
  }
  .foot button {
    flex: 1 1 0;
    min-width: 0;
    color: var(--text-dim);
    font-size: 11.5px;
    padding: 5px 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .foot button:first-child {
    flex: 1.7 1 0;
  }
  .foot button:nth-child(2) {
    flex: 1.1 1 0;
  }
  .foot button:last-child {
    flex: 0 0 30px;
  }
  button.active {
    border-color: #35507f;
    background: var(--panel-2);
  }
  .conn {
    flex: 0 0 auto;
    padding-bottom: 2px;
    font-size: 11px;
    color: var(--text-faint);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    margin: 0 2px;
  }
  .mini {
    font-size: 10.5px;
    padding: 2px 7px;
  }
</style>
