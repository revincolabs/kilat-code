<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, type ThreadMeta } from "../lib/api";
  import Icon from "./Icon.svelte";
  import {
    app,
    addProject,
    selectProject,
    projectThreads,
    newThread,
    openThread,
    deleteThread,
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
    <svg class="bolt" viewBox="0 0 24 24" aria-hidden="true"><path d="M13.5 2L4 14h7l-1.5 8L20 9.5h-7.2z" /></svg>
    <b>Kilat Code</b>
  </div>

  <div class="actions">
    <button class="primary wide" disabled={!app.activeProjectId} onclick={() => app.activeProjectId && newThread(app.activeProjectId)}>
      <Icon name="plus" size={14} /> Thread baru
    </button>
    <button class="icon box" title="Buka folder project" aria-label="Buka folder project" onclick={pickFolder}><Icon name="folder" /></button>
  </div>

  <label class="searchbox">
    <Icon name="search" size={13} />
    <input type="search" placeholder="Cari thread" aria-label="Cari thread" bind:value={query} onkeydown={onSearchKey} />
  </label>

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
        <span class="ttitle">{t.title}</span>
        {#if t.status === "limited"}<span class="flag warn" title={statusTitle(t)}><Icon name="clock" size={12} /></span>{/if}
        {#if t.pinned}<span class="flag" title="Disematkan"><Icon name="pin" size={12} /></span>{/if}
        {#if t.worktreeBranch}<span class="flag" title="Worktree: {t.worktreeBranch}"><Icon name="branch" size={12} /></span>{/if}
        <span class="prov {t.provider}" title={t.provider === "claude" ? "Claude" : "ChatGPT / Codex"}>{t.provider === "claude" ? "C" : "G"}</span>
        <span class="when">{ago(t.updatedAt)}</span>
        <span class="tacts">
          {#if !t.archived}
            <button class="ghost icon" title={t.pinned ? "Lepas sematan" : "Sematkan di atas"} onclick={(e) => { e.stopPropagation(); setThreadFlags(t.id, { pinned: !t.pinned }); }}><Icon name="pin" size={13} /></button>
          {/if}
          <button class="ghost icon" title={t.archived ? "Keluarkan dari arsip" : "Arsipkan"} onclick={(e) => { e.stopPropagation(); setThreadFlags(t.id, { archived: !t.archived }); }}><Icon name={t.archived ? "unarchive" : "archive"} size={13} /></button>
          <button
            class="ghost icon"
            title="Hapus thread"
            onclick={(e) => {
              e.stopPropagation();
              if (confirm(`Hapus thread "${t.title}"? Agent-nya dihentikan. Sesi aslinya tetap ada di ~/.claude / ~/.codex.`)) deleteThread(t.id).catch(fail);
            }}><Icon name="trash" size={13} /></button
          >
        </span>
      </div>
      {#if showProject}
        <div class="sub">{projectName(t.projectId)}{t.archived ? " · arsip" : ""}{snippet ? ` · ${snippet}` : ""}</div>
      {/if}
    </div>
  {/snippet}

  <div class="list">
    {#if query.trim()}
      {#each results as r (r.t.id)}
        {@render threadRow(r.t, r.snippet, true)}
      {/each}
      {#if results.length === 0}
        <p class="empty">Tidak ada thread yang cocok.</p>
      {/if}
    {:else if app.data.projects.length === 0}
      <p class="empty">Belum ada project. Buka folder dengan tombol folder di atas.</p>
    {/if}
    {#each app.data.projects as p (p.id)}
      <div class="project" class:active={p.id === app.activeProjectId}>
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="prow" onclick={() => selectProject(p.id)} title={p.path}>
          <button
            class="ghost icon caret"
            aria-label={collapsed[p.id] ? "Buka" : "Tutup"}
            onclick={(e) => {
              e.stopPropagation();
              collapsed[p.id] = !collapsed[p.id];
            }}><Icon name={collapsed[p.id] ? "chevronRight" : "chevronDown"} size={12} /></button
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
            }}><Icon name="plus" size={13} /></button
          >
          <button
            class="ghost icon more"
            title="Opsi"
            onclick={(e) => {
              e.stopPropagation();
              menuFor = menuFor === p.id ? null : p.id;
            }}><Icon name="more" size={13} /></button
          >
          {#if menuFor === p.id}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="menu" onclick={(e) => e.stopPropagation()}>
              <button class="ghost" onclick={() => { menuFor = null; newThread(p.id, true); }}><Icon name="branch" size={13} /> Thread baru di worktree terpisah</button>
              <button class="ghost danger" onclick={() => removeProject(p.id)}><Icon name="x" size={13} /> Hapus dari daftar</button>
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
              <Icon name={showArchived[p.id] ? "chevronDown" : "chevronRight"} size={11} /> Arsip <span class="mono">{archived.length}</span>
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
    <button class="ghost acct" onclick={() => { app.accountsTab = "claude"; app.accountsOpen = true; }}>
      <Icon name="user" size={14} />
      <span>Akun &amp; Pengaturan</span>
      <span class="spacer"></span>
      <span class="count">{app.data.accounts.length}</span>
    </button>
  </div>
</nav>

<style>
  nav {
    width: 252px;
    flex: none;
    background: var(--bg-1);
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .brand {
    height: 40px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 14px;
    border-bottom: 1px solid var(--line);
  }
  .brand b {
    font-size: 14.5px;
    font-weight: 650;
    letter-spacing: -0.02em;
  }
  .bolt {
    width: 15px;
    height: 15px;
    fill: var(--accent);
  }
  .actions {
    display: flex;
    gap: 6px;
    padding: 12px 10px 8px;
  }
  .actions button {
    min-height: 30px;
  }
  .actions .box {
    width: 30px;
  }
  .wide {
    flex: 1;
  }
  .searchbox {
    position: relative;
    display: block;
    margin: 0 10px 10px;
    color: var(--muted);
  }
  .searchbox :global(svg) {
    position: absolute;
    left: 9px;
    top: 50%;
    transform: translateY(-50%);
    pointer-events: none;
  }
  .searchbox input {
    width: 100%;
    height: 28px;
    padding: 0 8px 0 28px;
    font-size: 12.5px;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 0 6px 10px;
  }
  .empty {
    padding: 8px 10px;
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .project {
    margin-top: 6px;
  }
  .prow {
    position: relative;
    display: flex;
    align-items: center;
    gap: 2px;
    height: 26px;
    padding: 0 2px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    color: var(--muted);
    font: 500 11.5px/1 var(--mono);
  }
  .prow:hover,
  .project.active .prow {
    color: var(--fg-2);
  }
  .caret {
    width: 18px;
    min-width: 18px;
    min-height: 20px;
    color: inherit;
  }
  .pname {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .gh {
    font-size: 11px;
    color: var(--muted);
    max-width: 90px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .more {
    width: 22px;
    min-width: 22px;
    min-height: 22px;
    opacity: 0;
  }
  .prow:hover .more,
  .more:focus-visible {
    opacity: 1;
  }
  .menu {
    position: absolute;
    right: 2px;
    top: 26px;
    z-index: 10;
    background: var(--bg-2);
    border: 1px solid var(--line-2);
    border-radius: var(--radius);
    padding: 4px;
    display: flex;
    flex-direction: column;
    min-width: 236px;
    box-shadow: var(--shadow-pop);
    font-family: var(--sans);
  }
  .menu button {
    justify-content: flex-start;
    font-size: 12.5px;
    min-height: 28px;
    color: var(--fg);
  }
  .menu button.danger {
    color: var(--err);
  }
  .thread {
    position: relative;
    padding: 0 6px 0 22px;
    border-radius: 8px;
    cursor: pointer;
    color: var(--fg-2);
  }
  .thread.result {
    padding-left: 10px;
  }
  .tline {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    height: 28px;
  }
  .thread:hover {
    background: var(--bg-2);
    color: var(--fg);
  }
  .thread.sel {
    background: var(--bg-3);
    color: var(--fg);
  }
  .thread.sel::before {
    content: "";
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 2px;
    background: var(--accent);
  }
  .ttitle {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    font-size: 11px;
    color: var(--muted);
    padding: 0 0 6px 14px;
    margin-top: -4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .flag {
    color: var(--muted);
  }
  .flag.warn {
    color: var(--warn);
  }
  .prov {
    font: 600 10px/1 var(--mono);
    width: 12px;
    text-align: center;
  }
  .prov.claude {
    color: var(--claude);
  }
  .prov.codex {
    color: var(--codex);
  }
  .when {
    font: 400 10.5px/1 var(--mono);
    color: var(--muted);
    min-width: 24px;
    text-align: right;
  }
  .tacts {
    display: none;
  }
  .tacts button {
    width: 22px;
    min-width: 22px;
    min-height: 22px;
  }
  .thread:hover .tacts {
    display: inline-flex;
  }
  .thread:hover .when,
  .thread:hover .prov {
    display: none;
  }
  .archive-toggle {
    width: 100%;
    justify-content: flex-start;
    gap: 5px;
    min-height: 24px;
    font-size: 11.5px;
    color: var(--muted);
    padding: 0 6px 0 22px;
  }
  .foot {
    border-top: 1px solid var(--line);
    padding: 8px;
  }
  .acct {
    width: 100%;
    justify-content: flex-start;
    gap: 8px;
    min-height: 30px;
    padding: 0 8px;
    color: var(--fg-2);
  }
  .acct:hover:not(:disabled) {
    color: var(--fg);
  }
  .count {
    font: 500 10.5px/1 var(--mono);
    color: var(--muted);
    background: var(--bg-3);
    border-radius: 5px;
    padding: 3px 6px;
  }
</style>
