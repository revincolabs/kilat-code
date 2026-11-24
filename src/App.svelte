<script lang="ts">
  import { onMount } from "svelte";
  import { app, init, activeThread, activeProject, threadMeta, fail } from "./lib/state.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import ThreadView from "./components/ThreadView.svelte";
  import Welcome from "./components/Welcome.svelte";
  import EditorPanel from "./components/EditorPanel.svelte";
  import ChangesPanel from "./components/ChangesPanel.svelte";
  import TerminalPanel from "./components/TerminalPanel.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import AccountsModal from "./components/AccountsModal.svelte";
  import Toasts from "./components/Toasts.svelte";
  import TopBar from "./components/TopBar.svelte";
  import QuickOpen from "./components/QuickOpen.svelte";
  import { modKey } from "./lib/platform";
  import { startUpdateChecks } from "./lib/updater.svelte";

  let rightWidth = $state(520);
  let termHeight = $state(260);

  onMount(() => {
    init().catch(fail);
    if (!import.meta.env.DEV) startUpdateChecks();
    const onKey = (e: KeyboardEvent) => {
      if (modKey(e) && e.key === "`") {
        e.preventDefault();
        app.terminalOpen = !app.terminalOpen;
      }
      if (modKey(e) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "p") {
        e.preventDefault();
        app.quickOpen = !app.quickOpen;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  function drag(e: PointerEvent, axis: "x" | "y") {
    e.preventDefault();
    const start = axis === "x" ? e.clientX : e.clientY;
    const startVal = axis === "x" ? rightWidth : termHeight;
    const move = (ev: PointerEvent) => {
      const d = start - (axis === "x" ? ev.clientX : ev.clientY);
      if (axis === "x") rightWidth = Math.max(300, Math.min(window.innerWidth - 520, startVal + d));
      else termHeight = Math.max(120, Math.min(window.innerHeight - 200, startVal + d));
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  const thread = $derived(activeThread());
  const project = $derived(activeProject());
  const panelRoot = $derived(thread?.cwd ?? project?.path ?? null);
  /** Thread views stay mounted while hidden so their terminals keep scrollback. */
  const views = $derived(app.mounted.map((id) => threadMeta(id)).filter((t) => !!t));
</script>

{#if !app.ready}
  <div class="boot"><span class="spin"></span></div>
{:else}
  <div class="app">
    <Sidebar />
    <main>
      <TopBar />
      <div class="center">
        <section class="chat">
          {#each views as t (t.id)}
            <div class="view" class:show={t.id === thread?.id}>
              <ThreadView thread={t} active={t.id === thread?.id} />
            </div>
          {/each}
          {#if !thread}
            <Welcome />
          {/if}
        </section>
        {#if app.rightPanel !== "none" && panelRoot}
          <div class="handle-x" role="separator" aria-orientation="vertical" onpointerdown={(e) => drag(e, "x")}></div>
          <aside class="right" style:width="{rightWidth}px">
            {#if app.rightPanel === "editor"}
              {#key panelRoot}
                <EditorPanel root={panelRoot} />
              {/key}
            {:else}
              <ChangesPanel cwd={panelRoot} />
            {/if}
          </aside>
        {/if}
      </div>
      <div class="handle-y" role="separator" class:hidden={!app.terminalOpen} onpointerdown={(e) => drag(e, "y")}></div>
      <div class="term" class:hidden={!app.terminalOpen} style:height="{termHeight}px">
        <TerminalPanel />
      </div>
      <StatusBar />
    </main>
  </div>
  {#if app.accountsOpen}
    <AccountsModal />
  {/if}
{/if}
<QuickOpen />
<Toasts />

<style>
  .boot {
    height: 100%;
    display: grid;
    place-items: center;
  }
  .app {
    display: flex;
    height: 100%;
  }
  main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .center {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .chat {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .view {
    display: none;
    flex: 1;
    min-height: 0;
    flex-direction: column;
  }
  .view.show {
    display: flex;
  }
  .right {
    flex: none;
    min-width: 0;
    border-left: 1px solid var(--line);
    background: var(--bg-1);
    display: flex;
    flex-direction: column;
  }
  .handle-x {
    width: 4px;
    margin-right: -4px;
    cursor: col-resize;
    z-index: 2;
  }
  .handle-y {
    height: 4px;
    margin-bottom: -4px;
    cursor: row-resize;
    z-index: 2;
  }
  .handle-x:hover,
  .handle-y:hover {
    background: var(--line-2);
  }
  .term {
    flex: none;
    border-top: 1px solid var(--line);
    background: var(--bg);
    min-height: 0;
  }
  .hidden {
    display: none;
  }
</style>
