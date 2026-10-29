<script lang="ts">
  import { onMount, tick } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { app } from "$lib/store.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TabBar from "$lib/components/TabBar.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import Message from "$lib/components/Message.svelte";
  import Composer from "$lib/components/Composer.svelte";
  import MemoryPanel from "$lib/components/MemoryPanel.svelte";
  import CommandPalette from "$lib/components/CommandPalette.svelte";
  import PromptDialog from "$lib/components/PromptDialog.svelte";
  import Dialog from "$lib/components/Dialog.svelte";

  let scroller = $state<HTMLElement | null>(null);
  let stick = $state(true);

  $effect(() => {
    app.messages.length;
    app.messages[app.messages.length - 1]?.parts.map((p: any) => (p.text?.length ?? 0) + (p.output?.length ?? 0)).join(",");
    if (stick) queueMicrotask(() => scroller?.scrollTo({ top: scroller.scrollHeight }));
  });

  function onScroll() {
    if (!scroller) return;
    stick = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 90;
  }

  function onKey(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === "k") {
      e.preventDefault();
      app.showPalette = !app.showPalette;
    } else if (mod && e.key.toLowerCase() === "m") {
      e.preventDefault();
      app.showMemory = !app.showMemory;
    } else if (mod && e.shiftKey && e.key.toLowerCase() === "r") {
      // hard reload of the webview: discards stale HMR stylesheets
      e.preventDefault();
      window.location.reload();
    } else if (e.key === "F5") {
      e.preventDefault();
      app.reloadSession();
    } else if (mod && e.shiftKey && e.key.toLowerCase() === "d") {
      e.preventDefault();
      app.dumpDiagnostics();
    } else if (mod && e.key.toLowerCase() === "t") {
      e.preventDefault();
      app.openTab();
    } else if (mod && e.key.toLowerCase() === "w") {
      e.preventDefault();
      app.closeTab(app.activeId);
    } else if (e.key === "Escape" && app.showPalette) {
      app.showPalette = false;
    }
  }

  onMount(async () => {
    await app.boot();

    try {
      return await getCurrentWebview().onDragDropEvent(async (event) => {
        if (event.payload.type === "drop") await app.addPaths(event.payload.paths);
      });
    } catch {
      /* drag&drop unsupported */
    }
  });

  async function scrollToBottom() {
    await tick();
    scroller?.scrollTo({ top: scroller.scrollHeight, behavior: "smooth" });
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="shell">
  <Sidebar />

  <main>
    <TabBar />
    <TopBar />

    {#if app.error}
      <div class="banner">
        ⚠ {app.error}
        <button class="mini" onclick={() => app.connect()}>tentar de novo</button>
      </div>
    {/if}
    {#if app.active?.externalChange}
      <div class="banner info">
        🔄 esta sessão mudou no disco (outro processo escreveu nela)
        <button class="mini" onclick={() => app.reloadSession()}>recarregar (F5)</button>
      </div>
    {/if}
    {#if app.costWarn}
      <div class="banner warn">
        💸 custo desta sessão (${app.cost.toFixed(2)}) passou do limite de ${app.costLimit.toFixed(2)}
      </div>
    {/if}

    <div class="chat" bind:this={scroller} onscroll={onScroll}>
      {#if app.messages.length === 0}
        <div class="welcome">
          <div class="wlogo">◈</div>
          <h1>Hina</h1>
          <p>pi desktop · memória, skills e ferramentas do seu agente</p>
          <div class="tips">
            <span>arraste arquivos</span>
            <span>cole imagens</span>
            <span>Ctrl+K comandos</span>
            <span>Ctrl+M memória</span>
            <span>Ctrl+Alt+H janela</span>
            <span>Ctrl+Shift+R recarregar UI</span>
          </div>
        </div>
      {:else}
        {#each app.messages as msg (msg.key)}
          <Message {msg} />
        {/each}
      {/if}
    </div>

    {#if !stick}
      <button class="jump" onclick={scrollToBottom}>↓</button>
    {/if}

    <Composer focusToken={app.focusToken} />
  </main>

  {#if app.showMemory}
    <MemoryPanel />
  {/if}
</div>

<CommandPalette />
<PromptDialog />
<Dialog />

<div class="toasts">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}">{t.text}</div>
  {/each}
</div>

<style>
  .banner {
    background: #2a1a1a;
    border-bottom: 1px solid #4a2424;
    color: #ffb3ad;
    font-size: 12px;
    padding: 7px 18px;
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .banner.info {
    background: #16202e;
    border-bottom-color: #26374d;
    color: #a9c7ff;
  }
  .banner.warn {
    background: #2a2415;
    border-bottom-color: #4a3f1c;
    color: #e8d18a;
  }
  .mini {
    font-size: 10.5px;
    padding: 2px 8px;
  }
  .welcome {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    color: var(--text-dim);
    text-align: center;
  }
  .wlogo {
    width: 54px;
    height: 54px;
    border-radius: 16px;
    background: linear-gradient(135deg, var(--accent), var(--accent-2));
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 26px;
    color: var(--accent-ink, #fff);
    margin-bottom: 6px;
    box-shadow: 0 8px 40px rgba(93, 122, 224, 0.35);
  }
  h1 {
    font-size: 22px;
    margin: 0;
    color: var(--text);
    letter-spacing: -0.02em;
  }
  .welcome p {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-faint);
  }
  .tips {
    display: flex;
    gap: 8px;
    margin-top: 14px;
    flex-wrap: wrap;
    justify-content: center;
  }
  .tips span {
    font-size: 11px;
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 3px 11px;
    color: var(--text-faint);
  }
  .jump {
    position: absolute;
    bottom: 120px;
    right: 26px;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--panel-2);
    border: 1px solid var(--border);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  }
  .toasts {
    position: fixed;
    bottom: 18px;
    right: 18px;
    display: flex;
    flex-direction: column;
    gap: 7px;
    z-index: 80;
    pointer-events: none;
  }
  .toast {
    background: var(--panel);
    border: 1px solid #2b3550;
    border-radius: 9px;
    padding: 8px 12px;
    font-size: 12px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
    max-width: 380px;
    animation: slide 0.18s ease-out;
  }
  .toast.ok {
    border-color: #2f6b46;
    color: #c8f2d6;
  }
  .toast.warn {
    border-color: #6b5a2f;
    color: #f0e0b0;
  }
  .toast.error {
    border-color: #6b3030;
    color: #ffd0cc;
  }
  @keyframes slide {
    from {
      transform: translateY(8px);
      opacity: 0;
    }
  }
</style>
