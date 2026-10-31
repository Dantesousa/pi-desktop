<script lang="ts">
  import { t } from "../i18n.svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { app, THEMES } from "../store.svelte";
  import { LOCALES, setLocale } from "../i18n.svelte";

  type Item = { id: string; label: string; hint?: string; group: string; run: () => void | Promise<void> };

  let query = $state("");
  let index = $state(0);
  let input = $state<HTMLInputElement | null>(null);

  const items = $derived.by<Item[]>(() => {
    const list: Item[] = [
      { id: "new", label: t("palette.newChat"), group: t("palette.action"), run: () => app.newSession() },
      { id: "tab", label: t("palette.newTab"), group: t("palette.action"), run: () => app.openTab() },
      {
        id: "memory",
        label: app.showMemory ? t("palette.closeMemory") : t("palette.openMemory"),
        group: t("palette.action"),
        run: () => (app.showMemory = !app.showMemory),
      },
      {
        id: "compact",
        label: t("palette.compact"),
        group: t("palette.action"),
        run: () => app.compact(),
      },
      {
        id: "reload",
        label: t("palette.reload"),
        hint: t("palette.reloadHint"),
        group: t("palette.action"),
        run: () => app.reloadSession(),
      },
      {
        id: "export",
        label: t("palette.export"),
        group: t("palette.action"),
        run: async () => {
          const p = await app.exportHtml();
          if (p) app.toast(`exportado: ${p}`, "ok");
        },
      },
      {
        id: "theme",
        label: t("palette.themeNext", { name: THEMES.find((x) => x.id === app.theme)?.name ?? app.theme }),
        group: t("palette.action"),
        run: () => {
          const i = THEMES.findIndex((t) => t.id === app.theme);
          app.setTheme(THEMES[(i + 1) % THEMES.length].id);
        },
      },
      ...LOCALES.map((loc) => ({
        id: `locale:${loc.id}`,
        label: `Language: ${loc.label}`,
        group: t("palette.action"),
        run: () => setLocale(loc.id),
      })),
      ...THEMES.map((theme) => ({
        id: `theme:${theme.id}`,
        label: t("palette.themePrefix", { name: theme.name }),
        group: t("palette.theme"),
        run: () => app.setTheme(theme.id),
      })),
      {
        id: "project",
        label: t("palette.project"),
        group: t("palette.action"),
        run: async () => {
          const dir = await open({ directory: true, multiple: false });
          if (dir) await app.setCwd(dir as string);
        },
      },
      {
        id: "sync",
        label: t("palette.sync"),
        group: t("palette.action"),
        run: () => app.syncMemoryIndex(),
      },
      { id: "palette-close", label: t("palette.close"), group: t("palette.action"), run: () => (app.showPalette = false) },
    ];

    for (const m of app.models) {
      list.push({
        id: `model:${m.provider}:${m.id}`,
        label: t("palette.modelPrefix", { name: m.name ?? m.id }),
        hint: m.provider,
        group: t("palette.model"),
        run: () => app.setModel(m.provider, m.id),
      });
    }
    for (const c of app.commands) {
      list.push({
        id: `cmd:${c.name}`,
        label: t("palette.cmdPrefix", { name: c.name }),
        hint: c.description ?? c.source,
        group: t("palette.command"),
        run: async () => {
          app.showPalette = false;
          await app.prompt(`/${c.name}`, []);
        },
      });
    }
    for (const s of app.sessions.slice(0, 10)) {
      list.push({
        id: `ses:${s.id}`,
        label: t("palette.sessionPrefix", { name: s.name ?? s.id.slice(0, 8) }),
        hint: s.cwd,
        group: t("palette.session"),
        run: () => app.switchSession(s.path),
      });
    }
    for (const s of app.skills) {
      list.push({
        id: `skill:${s.name}`,
        label: t("palette.skillPrefix", { name: s.name }),
        hint: s.description,
        group: t("palette.skill"),
        run: async () => {
          app.showMemory = true;
        },
      });
    }
    return list;
  });

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items.slice(0, 40);
    return items
      .filter((i) => i.label.toLowerCase().includes(q) || (i.hint ?? "").toLowerCase().includes(q))
      .slice(0, 40);
  });

  const groupedItems = $derived.by(() => {
    const out: { name: string; items: (Item & { idx: number })[] }[] = [];
    filtered.forEach((item, idx) => {
      let g = out.find((x) => x.name === item.group);
      if (!g) {
        g = { name: item.group, items: [] };
        out.push(g);
      }
      g.items.push({ ...item, idx });
    });
    return out;
  });

  $effect(() => {
    index = 0;
  });

  $effect(() => {
    if (app.showPalette) {
      query = "";
      index = 0;
      setTimeout(() => input?.focus(), 20);
    }
  });

  async function run(item?: Item) {
    const target = item ?? filtered[index];
    if (!target) return;
    app.showPalette = false;
    await target.run();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      app.showPalette = false;
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      index = Math.min(index + 1, filtered.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      index = Math.max(index - 1, 0);
    } else if (e.key === "Enter") {
      e.preventDefault();
      run();
    }
  }

  let overlayEl = $state<HTMLElement | null>(null);

  function closeOverlay() {
    app.showPalette = false;
  }
</script>

<svelte:window
  onclick={(e) => {
    if (app.showPalette && overlayEl && e.target === overlayEl) closeOverlay();
  }}
/>

{#if app.showPalette}
<div class="overlay" bind:this={overlayEl}>
    <div class="palette">
      <input bind:this={input} bind:value={query} placeholder={t("palette.placeholder")} onkeydown={onKey} />
      <div class="list">
        {#each groupedItems as group (group.name)}
          <div class="group">{group.name}</div>
          {#each group.items as item (item.id)}
            <button
              class="item"
              class:active={item.idx === index}
              onmouseenter={() => (index = item.idx)}
              onclick={() => run(item)}
            >
              <span class="label">{item.label}</span>
              {#if item.hint}<span class="hint">{item.hint}</span>{/if}
            </button>
          {/each}
        {:else}
          <div class="empty">{t("palette.empty")}</div>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(5, 7, 11, 0.55);
    backdrop-filter: blur(2px);
    display: flex;
    justify-content: center;
    padding-top: 12vh;
    z-index: 55;
  }
  .palette {
    width: min(620px, 92vw);
    max-height: 62vh;
    background: var(--panel);
    border: 1px solid #2b3550;
    border-radius: 14px;
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.6);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  input {
    border: none;
    border-bottom: 1px solid var(--border-soft);
    border-radius: 0;
    background: transparent;
    padding: 13px 16px;
    font-size: 13.5px;
  }
  .list {
    overflow-y: auto;
    padding: 6px;
  }
  .group {
    font-size: 9.5px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-faint);
    padding: 7px 9px 3px;
  }
  .item {
    display: flex;
    align-items: baseline;
    gap: 10px;
    width: 100%;
    border: none;
    text-align: left;
    padding: 6px 9px;
    border-radius: 7px;
    font-size: 12.5px;
  }
  .item.active {
    background: #1e2941;
  }
  .hint {
    color: var(--text-faint);
    font-size: 10.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty {
    padding: 18px;
    text-align: center;
    color: var(--text-faint);
    font-size: 12px;
  }
</style>
