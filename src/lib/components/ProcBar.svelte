<script lang="ts">
  import { app } from "../store.svelte";
  import { t } from "../i18n.svelte";

  function elapsed(p: any) {
    const end = p.endedMs ?? Date.now();
    const s = Math.max(0, Math.round((end - p.startedMs) / 1000));
    const m = Math.floor(s / 60);
    return m > 0 ? `${m}m${String(s % 60).padStart(2, "0")}s` : `${s}s`;
  }
</script>

{#if app.procs.length}
  <div class="procbar">
    <span class="label">{t("proc.title")}</span>
    {#each app.procs as p (p.id)}
      <button
        class="chip"
        class:bad={p.status === "failed"}
        class:done={p.status !== "running"}
        title={`${p.command}\n${p.cwd}`}
        onclick={() => {
          app.toggleProcs(true);
          app.selectProc(p.id);
        }}
      >
        <span class="dot" class:live={p.status === "running"} class:bad={p.status === "failed"}></span>
        <span class="cmd">{p.command}</span>
        <span class="time">
          {#if p.status === "running"}{elapsed(p)}{:else if p.exitCode != null}exit {p.exitCode}{/if}
        </span>
      </button>
    {/each}
    <span class="spacer"></span>
    <button class="mini" title={t("proc.panel")} onclick={() => app.toggleProcs()}>⌃</button>
  </div>
{/if}

<style>
  .procbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 14px;
    border-top: 1px solid var(--border-soft);
    background: var(--bg-2);
    overflow-x: auto;
    flex: 0 0 auto;
    scrollbar-width: none;
  }
  .procbar::-webkit-scrollbar {
    height: 0;
  }
  .label {
    font-size: 9.5px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-faint);
    flex: 0 0 auto;
  }
  .chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 9px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--panel);
    font-size: 11px;
    max-width: 260px;
    flex: 0 0 auto;
  }
  .chip:hover {
    background: var(--panel-2);
  }
  .chip.done {
    opacity: 0.6;
  }
  .chip.bad {
    border-color: #6b3030;
    color: #ffb4ad;
  }
  .cmd {
    font-family: var(--mono);
    font-size: 10.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .time {
    font-size: 9.5px;
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-faint);
    flex: 0 0 auto;
  }
  .dot.live {
    background: var(--green);
    box-shadow: 0 0 6px var(--green);
  }
  .dot.bad {
    background: var(--red);
  }
  .spacer {
    flex: 1;
  }
  .mini {
    font-size: 10px;
    padding: 1px 7px;
    flex: 0 0 auto;
  }
</style>
