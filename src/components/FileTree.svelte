<script lang="ts">
  import { onMount } from "svelte";
  import FileTree from "./FileTree.svelte";
  import { api, type Entry } from "../lib/api";
  import { app } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";

  let { path, depth, onopen }: { path: string; depth: number; onopen: (p: string) => void } = $props();

  let entries = $state<Entry[]>([]);
  let open = $state<Record<string, boolean>>({});
  let error = $state("");

  async function load() {
    try {
      entries = await api.listDir(path);
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  onMount(load);

  // Refresh after an agent turn created/removed files.
  let lastTick = app.fsTick;
  $effect(() => {
    if (app.fsTick !== lastTick) {
      lastTick = app.fsTick;
      load();
    }
  });
</script>

{#if error}
  <div class="err" style:padding-left="{depth * 12 + 10}px">{error}</div>
{/if}
{#each entries as e (e.path)}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div
    class="node"
    style:padding-left="{depth * 12 + 8}px"
    onclick={() => (e.isDir ? (open[e.path] = !open[e.path]) : onopen(e.path))}
    title={e.path}
  >
    <span class="ic">{#if e.isDir}<Icon name={open[e.path] ? "chevronDown" : "chevronRight"} size={11} />{/if}</span>
    <span class:dir={e.isDir}>{e.name}</span>
  </div>
  {#if e.isDir && open[e.path]}
    <FileTree path={e.path} depth={depth + 1} {onopen} />
  {/if}
{/each}

<style>
  .node {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 23px;
    padding-right: 8px;
    cursor: pointer;
    white-space: nowrap;
    font-size: 12.5px;
    color: var(--fg-2);
  }
  .node:hover {
    background: var(--bg-2);
    color: var(--fg);
  }
  .ic {
    width: 12px;
    color: var(--muted);
    flex: none;
  }
  .dir {
    color: var(--fg);
  }
  .err {
    color: var(--err);
    font-size: 11px;
  }
</style>
