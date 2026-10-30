<script lang="ts">
  import { t } from "../i18n.svelte";
  import { app } from "../store.svelte";
  import { relTime } from "../markdown";

  let busy = $state(false);
  let cmd = $state("");

  async function enable() {
    busy = true;
    await app.enableBrowser();
    busy = false;
  }

  function ask() {
    if (!cmd.trim()) return;
    app.askAgentToBrowse(cmd.trim());
    app.showBrowser = false;
    cmd = "";
  }
</script>

{#if app.showBrowser}
  <section class="panel">
    <header>
      <span class="dot" class:live={app.browser.updated && Date.now() - app.browser.updated < 5000}></span>
      <span class="title" title={app.browser.url}>{app.browser.title || t("browser.title")}</span>
      <span class="spacer"></span>
      {#if app.browser.updated}
        <span class="faint">{relTime(app.browser.updated)}</span>
      {/if}
      <button class="mini" title={t("common.close")} onclick={() => app.toggleBrowser(false)}>×</button>
    </header>

    <div class="stage">
      {#if app.browser.img}
        <img src={app.browser.img} alt={t("browser.preview")} />
      {:else if app.browser.enabled}
        <div class="empty">
          <p>{t("browser.nothing")}</p>
          <p class="faint">{t("browser.hint")}</p>
        </div>
      {:else}
        <div class="empty">
          <p>{@html t("browser.off")}</p>
          <p class="faint">
            Ao ativar, um servidor MCP local (<code>hina-browser</code>) é registrado e o agente ganha
            ferramentas para navegar, clicar, digitar e tirar screenshots — que aparecem aqui.
          </p>
          <button class="primary" onclick={enable} disabled={busy}>
            {busy ? t("browser.enabling") : t("browser.enable")}
          </button>
        </div>
      {/if}
    </div>

    <footer>
      <input
        bind:value={cmd}
        placeholder={t("browser.ask")}
        onkeydown={(e) => e.key === "Enter" && ask()}
      />
      <button class="mini" onclick={ask} disabled={!cmd.trim()}>{t("browser.send")}</button>
    </footer>
    <div class="url">{app.browser.url || "—"}</div>
  </section>
{/if}

<style>
  .panel {
    position: fixed;
    right: 16px;
    bottom: 132px;
    width: 520px;
    height: 400px;
    min-width: 320px;
    min-height: 220px;
    resize: both;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.55);
    z-index: 50;
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
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 60%;
  }
  .spacer {
    flex: 1;
  }
  .faint {
    color: var(--text-faint);
    font-size: 10.5px;
  }
  .stage {
    flex: 1;
    min-height: 0;
    background: var(--code-bg);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }
  .stage img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    display: block;
  }
  .empty {
    padding: 18px;
    text-align: center;
    font-size: 12px;
    color: var(--text-dim);
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: center;
  }
  .empty p {
    margin: 0;
  }
  footer {
    display: flex;
    gap: 6px;
    padding: 7px 9px 2px;
    flex: 0 0 auto;
  }
  footer input {
    flex: 1;
    font-size: 11.5px;
    padding: 5px 8px;
  }
  .url {
    padding: 0 10px 7px;
    font-family: var(--mono);
    font-size: 10px;
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mini {
    font-size: 10.5px;
    padding: 2px 8px;
  }
</style>
