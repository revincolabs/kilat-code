<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { app, addProject, activeProject, newThread, fail, accountsOf } from "../lib/state.svelte";
  import { keyLabel } from "../lib/platform";
  import Icon from "./Icon.svelte";
  import { t } from "../lib/i18n/index.svelte";

  const project = $derived(activeProject());
  const claudeCount = $derived(accountsOf("claude").filter((a) => a.ready).length);
  const codexCount = $derived(accountsOf("codex").filter((a) => a.ready).length);
  const ghCount = $derived(accountsOf("github").length);

  async function pick() {
    try {
      const dir = await open({ directory: true, title: t().state.pickFolder });
      if (typeof dir === "string") await addProject(dir);
    } catch (e) {
      fail(e);
    }
  }
</script>

<div class="welcome">
  <svg class="bolt" viewBox="0 0 24 24" aria-hidden="true"><path d="M13.5 2L4 14h7l-1.5 8L20 9.5h-7.2z" /></svg>
  <h1>Kilat Code</h1>
  <p class="lead">{t().welcome.lead}</p>

  <div class="label bolt">{t().welcome.start}</div>
  <div class="actions">
    {#if project}
      <button class="act" onclick={() => newThread(project.id)}>
        <Icon name="plus" />
        <b>{t().welcome.newThread}</b>
        <span>{t().welcome.inProject(project.name)}</span>
      </button>
      <button class="act" onclick={() => newThread(project.id, true)}>
        <Icon name="branch" />
        <b>{t().welcome.worktree}</b>
        <span>{t().welcome.worktreeSub}</span>
      </button>
    {/if}
    <button class="act" onclick={pick}>
      <Icon name="folder" />
      <b>{t().welcome.openFolder}</b>
      <span>{t().welcome.openFolderSub}</span>
    </button>
    <button class="act" onclick={() => { app.accountsTab = "claude"; app.accountsOpen = true; }}>
      <Icon name="user" />
      <b>{t().welcome.accounts}</b>
      <span class="mono">{t().welcome.counts(claudeCount, codexCount, ghCount)}</span>
    </button>
  </div>

  <div class="label bolt">{t().welcome.know}</div>
  <ul class="tips">
    {#each t().welcome.tips as tip}<li>{@html tip}</li>{/each}
    <li>{@html t().welcome.keys(keyLabel("`"), keyLabel("P"))}</li>
  </ul>
</div>

<style>
  .welcome {
    margin: auto;
    width: 100%;
    max-width: 600px;
    padding: 48px 28px;
  }
  .bolt {
    width: 26px;
    height: 26px;
    fill: var(--accent);
  }
  h1 {
    margin: 14px 0 8px;
    font-size: 34px;
    font-weight: 600;
    letter-spacing: -0.035em;
    line-height: 1;
  }
  .lead {
    margin: 0 0 36px;
    color: var(--fg-2);
    font-size: 14.5px;
    line-height: 1.5;
  }
  .label {
    margin-bottom: 12px;
  }
  .actions {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin-bottom: 36px;
  }
  .act {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    grid-template-rows: auto auto;
    gap: 3px 10px;
    align-items: center;
    justify-content: stretch;
    text-align: left;
    min-height: 0;
    padding: 12px 14px;
    background: var(--bg-1);
    border-radius: var(--radius);
    color: var(--fg-2);
    white-space: normal;
  }
  .act:hover:not(:disabled) {
    background: var(--bg-2);
  }
  .act :global(svg) {
    color: var(--accent);
  }
  .act b {
    color: var(--fg);
    font-weight: 600;
  }
  .act span {
    grid-column: 2;
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .act span.mono {
    font-size: 11.5px;
  }
  .tips {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12.5px;
    color: var(--fg-2);
  }
  .tips li {
    padding: 9px 0;
    border-top: 1px solid var(--line);
    line-height: 1.55;
  }
  .tips li:last-child {
    border-bottom: 1px solid var(--line);
  }
</style>
