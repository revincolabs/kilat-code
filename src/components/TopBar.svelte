<script lang="ts">
  import { app, activeThread, activeProject, openTerminal } from "../lib/state.svelte";
  import { api } from "../lib/api";
  import { keyLabel } from "../lib/platform";

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
    <span class="muted path" title={thread.cwd}>{project?.name}{thread.worktreeBranch ? ` · ⑂ ${thread.worktreeBranch}` : ""}</span>
  {:else if project}
    <span class="title">{project.name}</span>
    <span class="muted path">{project.path}</span>
  {/if}
  <span class="spacer"></span>
  <button class="ghost" class:on={app.rightPanel === "editor"} onclick={() => toggle("editor")} disabled={!project}>Editor</button>
  <button class="ghost" class:on={app.rightPanel === "changes"} onclick={() => toggle("changes")} disabled={!project}>Perubahan</button>
  <button class="ghost" class:on={app.terminalOpen} onclick={newTerminal} title={keyLabel("`")}>Terminal</button>
</header>

<style>
  header {
    height: 42px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px;
    border-bottom: 1px solid var(--line);
  }
  .title {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 40%;
  }
  .title-input {
    width: 320px;
    padding: 2px 6px;
  }
  .path {
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .on {
    background: var(--bg-3);
    color: var(--accent);
  }
</style>
