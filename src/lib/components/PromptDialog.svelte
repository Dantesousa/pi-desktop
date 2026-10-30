<script lang="ts">
  import { t } from "../i18n.svelte";
  import { app } from "../store.svelte";
  import type { UiRequest } from "../store.svelte";

  const req = $derived(app.requests[0] as UiRequest | undefined);
  let value = $state("");
  let editorText = $state("");
  let inputEl = $state<HTMLInputElement | null>(null);
  let editorEl = $state<HTMLTextAreaElement | null>(null);

  $effect(() => {
    if (req) {
      value = "";
      editorText = req.prefill ?? "";
      setTimeout(() => (inputEl ?? editorEl)?.focus(), 30);
    }
  });

  function answer(payload: Record<string, any>) {
    if (!req) return;
    app.respond(req, payload);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      answer({ cancelled: true });
    }
  }
</script>

{#if req}
  <div class="overlay" onclick={(e) => e.target === e.currentTarget && answer({ cancelled: true })}>
    <div class="modal" role="dialog" tabindex="-1" onkeydown={onKey}>
      <div class="title">{req.title ?? t("dialog.confirmation")}</div>
      {#if req.message}
        <div class="msg">{req.message}</div>
      {/if}

      {#if req.method === "select"}
        <div class="options">
          {#each req.options ?? [] as opt}
            <button
              class="opt"
              class:danger={/bloquear|negar|no|cancel/i.test(opt)}
              class:primary={/permitir|allow|yes|sim/i.test(opt)}
              onclick={() => answer({ value: opt })}
            >
              {opt}
            </button>
          {/each}
        </div>
      {:else if req.method === "confirm"}
        <div class="options row">
          <button class="opt" onclick={() => answer({ confirmed: false })}>{t("common.no")}</button>
          <button class="opt primary" onclick={() => answer({ confirmed: true })}>{t("common.yes")}</button>
        </div>
      {:else if req.method === "input"}
        <input
          bind:this={inputEl}
          bind:value={value}
          placeholder={req.placeholder ?? ""}
          onkeydown={(e) => {
            if (e.key === "Enter") answer({ value });
            if (e.key === "Escape") answer({ cancelled: true });
          }}
        />
        <div class="foot">
          <button onclick={() => answer({ cancelled: true })}>{t("common.cancel")}</button>
          <button class="primary" onclick={() => answer({ value })}>{t("composer.send")}</button>
        </div>
      {:else if req.method === "editor"}
        <textarea
          bind:this={editorEl}
          rows="10"
          bind:value={editorText}
          onkeydown={(e) => {
            if (e.key === "Escape") answer({ cancelled: true });
          }}
        ></textarea>
        <div class="foot">
          <button onclick={() => answer({ cancelled: true })}>{t("common.cancel")}</button>
          <button class="primary" onclick={() => answer({ value: editorText })}>{t("common.save")}</button>
        </div>
      {/if}

      {#if app.requests.length > 1}
        <div class="queue">{app.requests.length - 1} queued</div>
      {/if}
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
    z-index: 60;
  }
  .modal {
    width: min(560px, 90vw);
    background: var(--panel);
    border: 1px solid #2b3550;
    border-radius: 14px;
    padding: 16px 18px;
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.6);
  }
  .title {
    font-weight: 650;
    font-size: 13.5px;
    margin-bottom: 6px;
  }
  .msg {
    font-size: 12.5px;
    color: var(--text-dim);
    white-space: pre-wrap;
    font-family: var(--mono);
    background: var(--bg-2);
    border: 1px solid var(--border-soft);
    border-radius: 9px;
    padding: 9px 11px;
    max-height: 260px;
    overflow: auto;
    margin-bottom: 10px;
  }
  .options {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .options.row {
    flex-direction: row;
    justify-content: flex-end;
  }
  .opt {
    text-align: left;
    padding: 8px 12px;
    font-size: 12.5px;
  }
  .opt.primary {
    background: #245c3a;
    border-color: #2f7a4c;
    color: #d8ffe4;
  }
  .opt.danger {
    background: #4a2222;
    border-color: #6b3030;
    color: #ffd6d3;
  }
  input,
  textarea {
    width: 100%;
    font-size: 12.5px;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 10px;
  }
  .queue {
    margin-top: 10px;
    font-size: 10.5px;
    color: var(--text-faint);
  }
</style>
