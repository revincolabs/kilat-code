<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, type ThreadMeta } from "../lib/api";
  import {
    app,
    addProject,
    selectProject,
    projectThreads,
    newThread,
    openThread,
    deleteThread,
    busyCount,
    fail,
    account,
    setThreadFlags,
  } from "../lib/state.svelte";

  let collapsed = $state<Record<string, boolean>>({});
  let menuFor = $state<string | null>(null);
  let showArchived = $state<Record<string, boolean>>({});

  // ---- search by title
  let query = $state("");

  const results = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return [];
    return app.data.threads
      .filter((t) => t.title.toLowerCase().includes(q))
      .map((t) => ({ t, snippet: "" }));
  });

  function statusTitle(t: ThreadMeta) {
    if (t.status === "running") return "Agent bekerja";
    if (t.status === "attention") return "Agent butuh izin";
    if (t.status === "limited") return t.waitUntil ? "Menunggu limit reset" : "Akun kena limit";
    if (t.status === "ready") return "Agent siap, menunggu kamu";
    return "Agent tidak berjalan";
  }

  function projectName(id: string) {
    return app.data.projects.find((p) => p.id === id)?.name ?? "";
  }

  /** pinned first, archived hidden */
  function visibleThreads(pid: string): ThreadMeta[] {
    const all = projectThreads(pid).filter((t) => !t.archived);
    return [...all.filter((t) => t.pinned), ...all.filter((t) => !t.pinned)];
  }

  function archivedThreads(pid: string): ThreadMeta[] {
    return projectThreads(pid).filter((t) => t.archived);
  }

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === "Escape") query = "";
    if (e.key === "Enter" && results[0]) {
      openThread(results[0].t.id);
      query = "";
    }
  }

  async function pickFolder() {
    try {
      const dir = await open({ directory: true, multiple: false, title: "Pilih folder project" });
      if (typeof dir === "string") await addProject(dir);
    } catch (e) {
      fail(e);
    }
  }

  async function removeProject(id: string) {
    menuFor = null;
    try {
      await api.removeProject(id);
      app.data.projects = app.data.projects.filter((p) => p.id !== id);
      app.data.threads = app.data.threads.filter((t) => t.projectId !== id);
      if (app.activeProjectId === id) app.activeProjectId = app.data.projects[0]?.id ?? null;
    } catch (e) {
      fail(e);
    }
  }

  function ago(ms: number) {
    const s = (Date.now() - ms) / 1000;
    if (s < 60) return "baru";
    if (s < 3600) return `${Math.floor(s / 60)}m`;
    if (s < 86400) return `${Math.floor(s / 3600)}j`;
    return `${Math.floor(s / 86400)}h`;
  }
</script>

<svelte:window onclick={() => (menuFor = null)} />

<nav>
  <div class="brand">
    <span class="bolt">⚡</span>
    <b>Kilat</b><span class="muted">Code</span>
  </div>

  <div class="actions">
    <button class="primary wide" disabled={!app.activeProjectId} onclick={() => app.activeProjectId && newThread(app.activeProjectId)}>
      + Thread baru
    </button>
    <button class="icon" title="Buka folder project" onclick={pickFolder}>📁</button>
  </div>

  <div class="searchbox">
    <input type="search" placeholder="🔍 Cari thread…" bind:value={query} onkeydown={onSearchKey} />
  </div>

  {#snippet threadRow(t: ThreadMeta, snippet: string, showProject: boolean)}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="thread" class:sel={t.id === app.activeThreadId} class:result={showProject} onclick={() => openThread(t.id)}>
      <div class="tline">
        <span
          class="dot"
          title={statusTitle(t)}
          class:run={t.status === "running"}
          class:warn={t.status === "attention" || t.status === "limited"}
          class:ok={t.status === "ready"}
        ></span>
        {#if t.status === "limited"}<span class="lim" title={statusTitle(t)}>⏳</span>{/if}
        {#if t.pinned}<span class="pin" title="Disematkan">📌</span>{/if}
        <span class="ttitle">{t.title}</span>
        {#if t.worktreeBranch}<span class="wt" title={t.worktreeBranch}>⑂</span>{/if}
        <span class="prov {t.provider}">{t.provider === "claude" ? "C" : "G"}</span>
        <span class="muted when">{ago(t.updatedAt)}</span>
        <span class="tacts">
          {#if !t.archived}
            <button class="ghost icon" title={t.pinned ? "Lepas sematan" : "Sematkan di atas"} onclick={(e) => { e.stopPropagation(); setThreadFlags(t.id, { pinned: !t.pinned }); }}>{t.pinned ? "◌" : "📌"}</button>
          {/if}
          <button class="ghost icon" title={t.archived ? "Keluarkan dari arsip" : "Arsipkan"} onclick={(e) => { e.stopPropagation(); setThreadFlags(t.id, { archived: !t.archived }); }}>{t.archived ? "↩" : "🗄"}</button>
          <button
            class="ghost icon"
            title="Hapus thread"
            onclick={(e) => {
              e.stopPropagation();
              if (confirm(`Hapus thread "${t.title}"? Agent-nya dihentikan. Sesi aslinya tetap ada di ~/.claude / ~/.codex.`)) deleteThread(t.id).catch(fail);
            }}>×</button
          >
        </span>
      </div>
      {#if showProject}
        <div class="sub muted">{projectName(t.projectId)}{t.archived ? " · arsip" : ""}{snippet ? ` · ${snippet}` : ""}</div>
      {/if}
    </div>
  {/snippet}

  <div class="list">
    {#if query.trim()}
      {#each results as r (r.t.id)}
        {@render threadRow(r.t, r.snippet, true)}
      {/each}
      {#if results.length === 0}
        <p class="muted empty">Tidak ada thread yang cocok.</p>
      {/if}
    {:else if app.data.projects.length === 0}
      <p class="muted empty">Belum ada project. Klik 📁 untuk membuka folder.</p>
    {/if}
    {#each app.data.projects as p (p.id)}
      <div class="project" class:active={p.id === app.activeProjectId}>
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="prow" onclick={() => selectProject(p.id)} title={p.path}>
          <button
            class="ghost icon caret"
            onclick={(e) => {
              e.stopPropagation();
              collapsed[p.id] = !collapsed[p.id];
            }}>{collapsed[p.id] ? "▸" : "▾"}</button
          >
          <span class="pname">{p.name}</span>
          {#if p.githubAccountId}
            <span class="gh" title="GitHub: {account(p.githubAccountId)?.label}">@{account(p.githubAccountId)?.login}</span>
          {/if}
          <button
            class="ghost icon more"
            title="Thread baru di project ini"
            onclick={(e) => {
              e.stopPropagation();
              newThread(p.id);
            }}>+</button
          >
          <button
            class="ghost icon more"
            title="Opsi"
            onclick={(e) => {
              e.stopPropagation();
              menuFor = menuFor === p.id ? null : p.id;
            }}>⋯</button
          >
          {#if menuFor === p.id}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="menu" onclick={(e) => e.stopPropagation()}>
              <button class="ghost" onclick={() => { menuFor = null; newThread(p.id, true); }}>Thread baru (worktree terpisah)</button>
              <button class="ghost danger" onclick={() => removeProject(p.id)}>Hapus dari daftar</button>
            </div>
          {/if}
        </div>
        {#if !collapsed[p.id]}
          {#each visibleThreads(p.id) as t (t.id)}
            {@render threadRow(t, "", false)}
          {/each}
          {@const archived = archivedThreads(p.id)}
          {#if archived.length}
            <button class="ghost archive-toggle" onclick={() => (showArchived[p.id] = !showArchived[p.id])}>
              {showArchived[p.id] ? "▾" : "▸"} 🗄 Arsip ({archived.length})
            </button>
            {#if showArchived[p.id]}
              {#each archived as t (t.id)}
                {@render threadRow(t, "", false)}
              {/each}
            {/if}
          {/if}
        {/if}
      </div>
    {/each}
  </div>

  <div class="foot">
    {#if busyCount() > 0}
      <div class="running"><span class="dot run"></span> {busyCount()} agent bekerja</div>
    {/if}
    <button class="ghost wide left" onclick={() => { app.accountsTab = "claude"; app.accountsOpen = true; }}>
      👤 Akun &amp; Pengaturan
      <span class="muted">({app.data.accounts.length})</span>
    </button>
  </div>
</nav>

<style>
  nav {
    width: 260px;
    flex: none;
    background: var(--bg-1);
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 12px 14px 8px;
    font-size: 15px;
  }
  .bolt {
    color: var(--accent);
  }
  .actions {
    display: flex;
    gap: 6px;
    padding: 4px 10px 10px;
  }
  .wide {
    flex: 1;
  }
  .left {
    text-align: left;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 0 6px 8px;
  }
  .empty {
    padding: 8px;
    font-size: 12px;
  }
  .project {
    margin-bottom: 4px;
  }
  .prow {
    position: relative;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 3px 4px;
    border-radius: 6px;
    cursor: pointer;
    color: var(--fg-2);
    font-weight: 600;
    font-size: 12px;
  }
  .prow:hover {
    background: var(--bg-2);
  }
  .project.active .prow {
    color: var(--fg);
  }
  .caret {
    font-size: 10px;
    min-width: 18px;
    padding: 0 2px;
  }
  .pname {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .gh {
    font-weight: 400;
    font-size: 11px;
    color: var(--muted);
    max-width: 90px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .more {
    opacity: 0;
    min-width: 22px;
  }
  .prow:hover .more {
    opacity: 1;
  }
  .menu {
    position: absolute;
    right: 4px;
    top: 26px;
    z-index: 10;
    background: var(--bg-2);
    border: 1px solid var(--line-2);
    border-radius: 8px;
    padding: 4px;
    display: flex;
    flex-direction: column;
    min-width: 220px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  }
  .menu button {
    text-align: left;
    font-weight: 400;
  }
  .searchbox {
    padding: 0 10px 8px;
  }
  .searchbox input {
    width: 100%;
    font-size: 12px;
    padding: 4px 8px;
    background: var(--bg-2);
    border: 1px solid var(--line-2);
    border-radius: 6px;
  }
  .thread {
    padding: 4px 6px 4px 22px;
    border-radius: 6px;
    cursor: pointer;
    color: var(--fg-2);
  }
  .lim {
    font-size: 10px;
  }
  .thread.result {
    padding-left: 8px;
  }
  .tline {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }
  .sub {
    font-size: 11px;
    padding-left: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pin {
    font-size: 10px;
  }
  .tacts {
    display: none;
    gap: 0;
  }
  .tacts button {
    min-width: 18px;
    padding: 0 3px;
    font-size: 11px;
  }
  .thread:hover .tacts {
    display: inline-flex;
  }
  .archive-toggle {
    display: block;
    width: 100%;
    text-align: left;
    font-size: 11px;
    color: var(--muted);
    padding: 3px 6px 3px 22px;
    font-weight: 400;
  }
  .thread:hover {
    background: var(--bg-2);
  }
  .thread.sel {
    background: var(--bg-3);
    color: var(--fg);
  }
  .ttitle {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .prov {
    font-size: 10px;
    font-weight: 700;
    width: 14px;
    text-align: center;
  }
  .prov.claude {
    color: var(--claude);
  }
  .prov.codex {
    color: var(--codex);
  }
  .wt {
    color: var(--muted);
    font-size: 11px;
  }
  .when {
    font-size: 11px;
    min-width: 22px;
    text-align: right;
  }
  .thread:hover .when {
    display: none;
  }
  .foot {
    border-top: 1px solid var(--line);
    padding: 8px;
  }
  .running {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    padding: 0 6px 6px;
    color: var(--accent);
  }
</style>
