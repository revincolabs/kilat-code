<script lang="ts">
  import { app, activeProject, activeThread, listFiles } from "../lib/state.svelte";
  import { fuzzy } from "../lib/fuzzy";

  const root = $derived(activeThread()?.cwd ?? activeProject()?.path ?? null);
  let query = $state("");
  let files = $state<string[]>([]);
  let sel = $state(0);
  let loading = $state(false);
  let input = $state<HTMLInputElement>();

  $effect(() => {
    if (!app.quickOpen || !root) return;
    query = "";
    sel = 0;
    loading = true;
    listFiles(root)
      .then((f) => (files = f))
      .finally(() => (loading = false));
    requestAnimationFrame(() => input?.focus());
  });

  const matches = $derived(fuzzy(files, query, 60));

  $effect(() => {
    void query;
    sel = 0;
  });

  function close() {
    app.quickOpen = false;
  }

  function choose(rel: string) {
    if (!root) return;
    const sep = root.includes("\\") ? "\\" : "/";
    app.openFileReq = root.replace(/[\\/]+$/, "") + sep + rel.split("/").join(sep);
    app.rightPanel = "editor";
    close();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") return close();
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const n = matches.length || 1;
      sel = (sel + (e.key === "ArrowDown" ? 1 : n - 1)) % n;
      document.querySelector(".qo-item.sel")?.scrollIntoView({ block: "nearest" });
    }
    if (e.key === "Enter" && matches[sel]) choose(matches[sel]);
  }

  function split(p: string) {
    const i = p.lastIndexOf("/");
    return { name: p.slice(i + 1), dir: i > 0 ? p.slice(0, i) : "" };
  }
</script>

{#if app.quickOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={close}>
    <div class="qo" role="dialog" tabindex="-1" aria-label="Buka file" onclick={(e) => e.stopPropagation()}>
      {#if !root}
        <p class="muted pad">Buka project dulu.</p>
      {:else}
        <input bind:this={input} type="text" placeholder="Ketik nama file…" bind:value={query} onkeydown={onKey} />
        <div class="list">
          {#each matches as m, i (m)}
            {@const s = split(m)}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="qo-item" class:sel={i === sel} onclick={() => choose(m)} onmouseenter={() => (sel = i)}>
              <span class="name">{s.name}</span>
              <span class="dir muted">{s.dir}</span>
            </div>
          {/each}
          {#if loading}
            <p class="muted pad">Memuat daftar file…</p>
          {:else if matches.length === 0}
            <p class="muted pad">Tidak ada file yang cocok.</p>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 95;
    background: rgba(10, 10, 9, 0.45);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
  }
  .qo {
    width: min(620px, 92vw);
    background: var(--bg-1);
    border: 1px solid var(--line-2);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    overflow: hidden;
  }
  input {
    width: 100%;
    border: none;
    border-bottom: 1px solid var(--line);
    border-radius: 0;
    background: transparent;
    padding: 12px 14px;
    font-size: 14px;
    outline: none;
  }
  input:focus {
    border-color: var(--line);
  }
  .list {
    max-height: 50vh;
    overflow: auto;
    padding: 4px;
  }
  .qo-item {
    display: flex;
    gap: 12px;
    align-items: baseline;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    white-space: nowrap;
  }
  .qo-item.sel {
    background: var(--bg-3);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .name {
    font-family: var(--mono);
    font-size: 12.5px;
  }
  .dir {
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .pad {
    padding: 10px 12px;
    margin: 0;
    font-size: 12px;
  }
</style>
