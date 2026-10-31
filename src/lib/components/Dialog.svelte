<script lang="ts">
  import { t } from "../i18n.svelte";
  import { app } from "../store.svelte";

  let inputEl = $state<HTMLInputElement | null>(null);
  const d = $derived(app.dialog);

  $effect(() => {
    if (d?.kind === "prompt") {
      setTimeout(() => {
        inputEl?.focus();
        inputEl?.select();
      }, 30);
    }
  });

  function accept() {
    if (!d) return;
    app.resolveDialog(d.kind === "prompt" ? (d.value ?? "") : true);
  }

  function cancel() {
    app.resolveDialog(null);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      cancel();
    } else if (e.key === "Enter" && d?.kind === "confirm") {
      e.preventDefault();
      accept();
    }
  }
  let overlayEl = $state<HTMLElement | null>(null);

  function closeOverlay() {
    cancel();
  }
</script>

<svelte:window
  onkeydown={onKey}
  onclick={(e) => {
    if (e.target === overlayEl) closeOverlay();
  }}
/>

{#if d}
<div class="overlay" bind:this={overlayEl}>
    <div class="box" role="dialog" aria-modal="true" tabindex="-1">
      <div class="title">{d.title}</div>
      {#if d.message}
        <div class="msg">{d.message}</div>
      {/if}
      {#if d.kind === "prompt"}
        <input
          bind:this={inputEl}
          bind:value={d.value}
          placeholder={d.placeholder ?? ""}
          onkeydown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              accept();
            }
          }}
        />
      {/if}
      <div class="row">
        <button onclick={cancel}>{t("common.cancel")}</button>
        <button class="primary" class:danger={d.danger} onclick={accept}>
          {d.confirmLabel ?? (d.kind === "prompt" ? t("common.save") : t("common.confirm"))}
        </button>
      </div>
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
    z-index: 70;
  }
  .box {
    width: min(460px, 92vw);
    background: var(--panel);
    border: 1px solid #2b3550;
    border-radius: 13px;
    padding: 16px 18px 13px;
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.6);
    outline: none;
  }
  .title {
    font-weight: 650;
    font-size: 13.5px;
    margin-bottom: 5px;
  }
  .msg {
    font-size: 12.5px;
    color: var(--text-dim);
    margin-bottom: 10px;
    overflow-wrap: anywhere;
  }
  input {
    width: 100%;
    font-size: 12.5px;
    margin-bottom: 4px;
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 12px;
  }
  .primary.danger {
    background: #6a2626;
    border-color: #8a3636;
    color: #ffdedb;
  }
  .primary.danger:hover {
    background: #7d2c2c;
  }
</style>
