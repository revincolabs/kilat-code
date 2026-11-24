<script lang="ts">
  import { app, activeThread, activeProject, openTerminal } from "../lib/state.svelte";
  import { api } from "../lib/api";
  import { keyLabel } from "../lib/platform";
  import Icon from "./Icon.svelte";

  const thread = $derived(activeThread());
  const project = $derived(activeProject());
  let editing = $state(false);
  let title = $state("");

  function toggle(panel: "editor" | "changes") {
    app.rightPanel = app.rightPanel === panel ? "none" : panel;
  }

  function newTerminal() {
    if (!app.terminals.some((t) => !t.setup)) {
      openTerminal({ projectId: project?.id ?? null, cwd: thread?.cwd ?? project?.path ?? null });
    } else {
      app.terminalOpen = !app.terminalOpen;
    }
  }

  async function saveTitle() {
    editing = false;
    if (thread && title.trim() && title !== thread.title) {
      thread.title = title.trim();
      await api.renameThread(thread.id, thread.title);
    }
  }
</script>

<header>
  {#if thread}
    {#if editing}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="text"
        class="title-input"
        bind:value={title}
        autofocus
        onblur={saveTitle}
        onkeydown={(e) => e.key === "Enter" && saveTitle()}
      />
    {:else}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <span class="title" ondblclick={() => { title = thread.title; editing = true; }} title="Klik dua kali untuk ganti nama">{thread.title}</span>
    {/if}
    <span class="path" title={thread.cwd}>
      {project?.name}
      {#if thread.worktreeBranch}<span class="wt"><Icon name="branch" size={11} />{thread.worktreeBranch}</span>{/if}
    </span>
  {:else if project}
    <span class="title">{project.name}</span>
    <span class="path">{project.path}</span>
  {/if}
  <span class="spacer"></span>
  <div class="seg" role="group" aria-label="Panel">
    <button class="ghost" class:on={app.rightPanel === "editor"} aria-pressed={app.rightPanel === "editor"} onclick={() => toggle("editor")} disabled={!project}>
      <Icon name="code" size={14} />Editor
    </button>
    <button class="ghost" class:on={app.rightPanel === "changes"} aria-pressed={app.rightPanel === "changes"} onclick={() => toggle("changes")} disabled={!project}>
      <Icon name="diff" size={14} />Perubahan
    </button>
    <button class="ghost" class:on={app.terminalOpen} aria-pressed={app.terminalOpen} onclick={newTerminal} title={keyLabel("`")}>
      <Icon name="terminal" size={14} />Terminal
    </button>
  </div>
</header>

<style>
  header {
    height: 40px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 8px 0 14px;
    border-bottom: 1px solid var(--line);
    background: var(--bg-1);
  }
  .title {
    font-weight: 600;
    font-size: 13.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 40%;
  }
  .title-input {
    width: 320px;
    height: 26px;
    padding: 0 6px;
  }
  .path {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font: 400 11.5px/1 var(--mono);
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .wt {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .seg {
    display: flex;
    gap: 2px;
  }
  .seg button {
    min-height: 28px;
    padding: 0 10px;
    color: var(--fg-2);
  }
  .seg button.on {
    background: var(--bg-3);
    color: var(--fg);
    box-shadow: inset 0 -2px 0 var(--accent);
  }
</style>
