<script lang="ts">
  import { app, openTerminal, closeTerminal, activeProject, activeThread } from "../lib/state.svelte";
  import Terminal from "./Terminal.svelte";

  const tabs = $derived(app.terminals.filter((t) => !t.setup));

  function add() {
    const p = activeProject();
    const t = activeThread();
    openTerminal({ projectId: p?.id ?? null, cwd: t?.cwd ?? p?.path ?? null });
  }
</script>

<div class="panel">
  <div class="tabs">
    {#each tabs as t (t.key)}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="tab" class:sel={t.key === app.activeTerminal} onclick={() => (app.activeTerminal = t.key)}>
        <span class="mono">›_</span>
        <span>{t.title}</span>
        {#if t.exited}<span class="muted">(selesai)</span>{/if}
        <button class="ghost icon x" onclick={(e) => { e.stopPropagation(); closeTerminal(t.key); }}>×</button>
      </div>
    {/each}
    <button class="ghost icon" title="Terminal bash baru" onclick={add}>+</button>
    <span class="spacer"></span>
    <span class="muted hint">bash · akun GitHub mengikuti project</span>
    <button class="ghost icon" title="Sembunyikan (Ctrl+`)" onclick={() => (app.terminalOpen = false)}>▾</button>
  </div>
  <div class="body">
    {#each tabs as t (t.key)}
      <div class="slot" class:show={t.key === app.activeTerminal}>
        <Terminal
          cwd={t.cwd}
          projectId={t.projectId}
          active={app.terminalOpen && t.key === app.activeTerminal}
          onexit={() => (t.exited = true)}
        />
      </div>
    {/each}
    {#if tabs.length === 0}
      <div class="none muted">
        <button onclick={add}>+ Buka terminal bash</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .panel {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 3px 6px;
    border-bottom: 1px solid var(--line);
    background: var(--bg-1);
    flex: none;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 4px 2px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    color: var(--fg-2);
  }
  .tab:hover {
    background: var(--bg-2);
  }
  .tab.sel {
    background: var(--bg-3);
    color: var(--fg);
  }
  .tab .mono {
    color: var(--accent);
  }
  .x {
    min-width: 18px;
    padding: 0 4px;
  }
  .hint {
    font-size: 11px;
    margin-right: 6px;
  }
  .body {
    flex: 1;
    min-height: 0;
    position: relative;
  }
  .slot {
    position: absolute;
    inset: 0;
    display: none;
  }
  .slot.show {
    display: block;
  }
  .none {
    height: 100%;
    display: grid;
    place-items: center;
  }
</style>
