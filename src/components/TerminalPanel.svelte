<script lang="ts">
  import { app, openTerminal, closeTerminal, activeProject, activeThread } from "../lib/state.svelte";
  import Terminal from "./Terminal.svelte";
  import Icon from "./Icon.svelte";
  import { t as tr } from "../lib/i18n/index.svelte";
  import { keyLabel } from "../lib/platform";

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
        <Icon name="terminal" size={13} />
        <span>{t.title}</span>
        {#if t.exited}<span class="muted">{tr().terminal.exited}</span>{/if}
        <button class="ghost icon x" aria-label={tr().terminal.close} onclick={(e) => { e.stopPropagation(); closeTerminal(t.key); }}><Icon name="x" size={12} /></button>
      </div>
    {/each}
    <button class="ghost icon" title={tr().terminal.new} aria-label={tr().terminal.new} onclick={add}><Icon name="plus" size={14} /></button>
    <span class="spacer"></span>
    <span class="hint">{tr().terminal.hint}</span>
    <button class="ghost icon" title={tr().terminal.hide(keyLabel("`"))} aria-label={tr().terminal.hideLabel} onclick={() => (app.terminalOpen = false)}><Icon name="hide" size={14} /></button>
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
        <button onclick={add}><Icon name="terminal" size={13} /> {tr().terminal.open}</button>
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
    height: 32px;
    padding: 0 6px 0 4px;
    border-bottom: 1px solid var(--line);
    background: var(--bg-1);
    flex: none;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 24px;
    padding: 0 2px 0 10px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 12px;
    color: var(--muted);
  }
  .tab:hover {
    color: var(--fg);
  }
  .tab.sel {
    color: var(--fg);
    background: var(--bg-3);
  }
  .x {
    width: 20px;
    min-width: 20px;
    min-height: 20px;
  }
  .hint {
    font: 400 11px/1 var(--mono);
    color: var(--muted);
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
