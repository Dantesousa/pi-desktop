<script lang="ts">
  import { t } from "../i18n.svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { app, fmtSize } from "../store.svelte";

  let { focusToken = 0 }: { focusToken?: number } = $props();
  let input = $state<HTMLTextAreaElement | null>(null);
  let text = $state("");
  let showCommands = $state(false);

  const filteredCommands = $derived.by(() => {
    if (!text.startsWith("/")) return [];
    const q = text.slice(1).toLowerCase();
    if (q.includes(" ")) return [];
    return app.commands
      .filter((c) => c.name.toLowerCase().includes(q))
      .slice(0, 8);
  });

  $effect(() => {
    showCommands = filteredCommands.length > 0;
  });

  function autosize() {
    if (!input) return;
    input.style.height = "auto";
    input.style.height = Math.min(input.scrollHeight, 220) + "px";
  }

  $effect(() => {
    text;
    autosize();
  });

  $effect(() => {
    focusToken;
    if (focusToken > 0) input?.focus();
  });

  $effect(() => {
    if (app.draftRestore) {
      text = app.draftRestore;
      app.draftRestore = null;
      setTimeout(() => input?.focus(), 20);
    }
  });

  async function pickFiles() {
    const picked = await open({ multiple: true, directory: false });
    if (!picked) return;
    const list = Array.isArray(picked) ? picked : [picked];
    await app.addPaths(list as string[]);
  }

  function pickCommand(name: string) {
    text = `/${name} `;
    showCommands = false;
    input?.focus();
  }

  async function send() {
    if (!text.trim() && app.chips.length === 0) return;
    const cmd = text.trim().toLowerCase();
    if (cmd === "/yolo" || cmd === "/yolo on") {
      text = "";
      await app.setYolo(true);
      return;
    }
    if (cmd === "/safe" || cmd === "/yolo off") {
      text = "";
      await app.setYolo(false);
      return;
    }
    const payload = text;
    const chips = [...app.chips];
    text = "";
    app.clearChips();
    await app.prompt(payload, chips);
    input?.focus();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      if (showCommands) {
        e.preventDefault();
        pickCommand(filteredCommands[0].name);
        return;
      }
      e.preventDefault();
      send();
    }
    if (e.key === "Escape" && app.streaming) {
      e.preventDefault();
      app.abort();
    }
  }

  async function onPaste(e: ClipboardEvent) {
    const files = Array.from(e.clipboardData?.files ?? []);
    if (!files.length) return;
    e.preventDefault();
    for (const f of files) {
      if (f.type.startsWith("image/")) await app.addPastedImage(f);
    }
  }
</script>

<div class="composer">
  {#if showCommands}
    <div class="cmds">
      {#each filteredCommands as cmd}
        <button class="cmd" onclick={() => pickCommand(cmd.name)}>
          <span class="cname">/{cmd.name}</span>
          <span class="cdesc">{cmd.description ?? cmd.source}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if app.chips.length}
    <div class="chips">
      {#each app.chips as chip, i}
        {#if chip.kind === "image" && chip.preview}
          <div class="chip img">
            <img src={chip.preview} alt={chip.name} />
            <button class="x" onclick={() => app.removeChip(i)}>×</button>
          </div>
        {:else}
          <div class="chip">
            <span class="cico">{chip.kind === "text" ? "📄" : "📦"}</span>
            <span class="cname2">{chip.name}</span>
            <span class="csize">{fmtSize(chip.size)}</span>
            <button class="x" onclick={() => app.removeChip(i)}>×</button>
          </div>
        {/if}
      {/each}
    </div>
  {/if}

  <div class="box">
    <textarea
      bind:this={input}
      bind:value={text}
      placeholder={t("composer.placeholder")}
      rows="1"
      onkeydown={onKeydown}
      onpaste={onPaste}
    ></textarea>
    <div class="actions">
      <button class="ghost" title={t("composer.attach")} onclick={pickFiles}>📎</button>
      {#if app.working}
        <select
          class="behavior"
          bind:value={app.sendBehavior}
          title={t("composer.behavior")}
        >
          <option value="steer">{t("composer.steer")}</option>
          <option value="followUp">{t("composer.followUp")}</option>
        </select>
      {/if}
      <span class="hint">
        {#if app.working}
          <span class="work"><span class="pulse"></span>{app.activity ?? "trabalhando…"}</span>
        {:else}
          {t("composer.hint")}
        {/if}
      </span>
      {#if app.working}
        <button class="stop" onclick={() => app.abort()}>{t("composer.stop")}</button>
      {:else}
        <button class="primary" onclick={send} disabled={!text.trim() && !app.chips.length}>
          {t("composer.send")}
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .composer {
    width: 100%;
    box-sizing: border-box;
    flex: 0 0 auto;
    padding: 10px 22px 16px;
    border-top: 1px solid var(--border-soft);
    background: linear-gradient(180deg, rgba(11, 13, 18, 0.4), var(--bg));
  }
  .box {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid var(--border);
    background: var(--panel);
    border-radius: 14px;
    padding: 8px 10px 7px;
  }
  .box:focus-within {
    border-color: #35507f;
  }
  textarea {
    width: 100%;
    border: none;
    background: transparent;
    resize: none;
    padding: 4px 4px 2px;
    max-height: 220px;
    line-height: 1.5;
  }
  textarea:focus {
    border: none;
  }
  .actions {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 3px;
  }
  .hint {
    flex: 1;
    font-size: 10.5px;
    color: var(--text-faint);
  }
  .work {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: #9fc0ff;
  }
  .pulse {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.1s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 0.25; transform: scale(0.8); }
    50% { opacity: 1; transform: scale(1.15); }
  }
  .behavior {
    font-size: 10.5px;
    padding: 2px 5px;
    background: var(--panel-2);
    max-width: 92px;
  }
  .stop {
    background: #4a2222;
    border-color: #6b3030;
    color: #ffd6d3;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
    margin-bottom: 7px;
  }
  .chip {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 9px;
    padding: 4px 7px;
    font-size: 11.5px;
  }
  .chip.img {
    padding: 3px;
    position: relative;
  }
  .chip.img img {
    height: 54px;
    border-radius: 7px;
    display: block;
  }
  .cname2 {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .csize {
    color: var(--text-faint);
    font-size: 10.5px;
  }
  .x {
    border: none;
    padding: 0 3px;
    color: var(--text-faint);
    font-size: 14px;
    line-height: 1;
  }
  .chip.img .x {
    position: absolute;
    top: 2px;
    right: 2px;
    background: rgba(10, 12, 18, 0.85);
    border-radius: 6px;
    padding: 0 4px;
  }
  .cmds {
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 5px;
    margin-bottom: 7px;
  }
  .cmd {
    display: flex;
    gap: 10px;
    align-items: baseline;
    border: none;
    text-align: left;
    padding: 5px 8px;
    border-radius: 6px;
  }
  .cmd:hover {
    background: var(--panel-2);
  }
  .cname {
    font-family: var(--mono);
    color: #9fc0ff;
    font-size: 12px;
  }
  .cdesc {
    color: var(--text-faint);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
