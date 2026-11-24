<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, type Commit, type GitStatus } from "../lib/api";
  import { app, account, activeProject, activeThread, accountsOf, fail, toast } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";

  let { cwd }: { cwd: string } = $props();

  let status = $state<GitStatus | null>(null);
  let selected = $state<string | null>(null);
  let diff = $state("");
  let loading = $state(false);
  /** files left out of the next commit */
  let excluded = $state<Record<string, boolean>>({});
  let message = $state("");
  let busy = $state<"" | "suggest" | "commit" | "push" | "pr" | "branch">("");
  let recent = $state<Commit[]>([]);
  let branchOpen = $state(false);
  let branchName = $state("");
  let prOpen = $state(false);
  let prTitle = $state("");
  let prBody = $state("");
  let prDraft = $state(false);

  const thread = $derived(activeThread());
  const project = $derived(activeProject());
  const projectId = $derived(thread?.projectId ?? project?.id ?? "");
  const gh = $derived(account(project?.githubAccountId));
  const files = $derived(status?.files ?? []);
  const chosen = $derived(files.filter((f) => !excluded[f.path]));
  const allChosen = $derived(chosen.length === files.length);

  async function refresh() {
    loading = true;
    try {
      status = await api.gitStatus(cwd);
      recent = status.isRepo ? await api.gitRecent(cwd) : [];
      if (selected && !status.files.some((f) => f.path === selected)) selected = null;
      if (selected) await show(selected);
    } catch (e) {
      fail(e);
    } finally {
      loading = false;
    }
  }

  async function show(path: string) {
    selected = path;
    try {
      diff = await api.gitDiff(cwd, path);
    } catch (e) {
      diff = String(e);
    }
  }

  $effect(() => {
    void app.fsTick;
    void cwd;
    refresh();
  });

  function toggleAll() {
    const v = allChosen;
    excluded = Object.fromEntries(files.map((f) => [f.path, v]));
  }

  /** Agent used for the commit message: the thread's choice, else any ready Claude account. */
  function writer(): { provider: "claude" | "codex"; accountId: string } {
    if (thread) return { provider: thread.provider, accountId: thread.accountId };
    const a = accountsOf("claude").find((x) => x.ready) ?? accountsOf("codex").find((x) => x.ready);
    return { provider: (a?.provider as "claude" | "codex") ?? "claude", accountId: a?.id ?? "claude-default" };
  }

  async function run<T>(what: typeof busy, job: () => Promise<T>): Promise<T | undefined> {
    busy = what;
    try {
      return await job();
    } catch (e) {
      fail(e);
    } finally {
      busy = "";
    }
  }

  async function suggest() {
    const w = writer();
    const m = await run("suggest", () => api.gitSuggestMessage(projectId, cwd, w.provider, w.accountId));
    if (m) message = m;
  }

  async function commit(andPush: boolean) {
    const paths = allChosen ? [] : chosen.map((f) => f.path);
    const done = await run("commit", () => api.gitCommit(projectId, cwd, message, paths));
    if (!done) return;
    toast(`Commit: ${done}`, "ok");
    message = "";
    excluded = {};
    await refresh();
    if (andPush) await push();
  }

  async function push() {
    const branch = await run("push", () => api.gitPush(projectId, cwd));
    if (branch !== undefined) {
      toast(`Push ${branch} berhasil${gh?.login ? ` sebagai @${gh.login}` : ""}`, "ok");
      await refresh();
    }
  }

  async function createBranch() {
    const name = branchName.trim();
    if (!name) return;
    const ok = await run("branch", () => api.gitBranch(projectId, cwd, name).then(() => true));
    if (ok) {
      toast(`Pindah ke branch baru ${name}`, "ok");
      branchOpen = false;
      branchName = "";
      await refresh();
    }
  }

  function openPr() {
    const [subject, ...rest] = (recent[0]?.subject ?? "").split("\n");
    prTitle = subject;
    prBody = rest.join("\n").trim();
    prOpen = true;
  }

  async function createPr() {
    const url = await run("pr", () => api.gitPr(projectId, cwd, prTitle, prBody, prDraft));
    if (!url) return;
    prOpen = false;
    toast("Pull request dibuat", "ok");
    if (url.startsWith("https://")) openUrl(url).catch(() => {});
    await refresh();
  }

  function cls(line: string) {
    if (line.startsWith("+++") || line.startsWith("---")) return "meta";
    if (line.startsWith("+")) return "add";
    if (line.startsWith("-")) return "del";
    if (line.startsWith("@@")) return "hunk";
    if (line.startsWith("diff ") || line.startsWith("index ")) return "meta";
    return "";
  }

  const label: Record<string, string> = { M: "M", A: "A", D: "D", R: "R", "??": "U", AM: "A", MM: "M" };
  const onDefault = $derived(["main", "master", "trunk", "develop"].includes(status?.branch ?? ""));
</script>

<div class="changes">
  <div class="head">
    {#if status?.isRepo}
      <span class="br"><Icon name="branch" size={13} /><b>{status.branch || "(detached)"}</b></span>
      {#if status.ahead}<span class="num" title="Commit yang belum di-push">↑{status.ahead}</span>{/if}
      {#if status.behind}<span class="num" title="Commit di remote yang belum ditarik">↓{status.behind}</span>{/if}
      <span class="num">{files.length} file berubah</span>
    {:else if status}
      <span class="muted">Bukan repo git</span>
    {/if}
    <span class="spacer"></span>
    {#if status?.isRepo}
      <button class="ghost small" onclick={() => (branchOpen = !branchOpen)} title="Buat branch baru dari posisi sekarang"><Icon name="plus" size={12} />branch</button>
    {/if}
    <button class="ghost icon" onclick={refresh} title="Muat ulang" aria-label="Muat ulang">{#if loading}<span class="spin"></span>{:else}<Icon name="refresh" size={14} />{/if}</button>
  </div>

  {#if status?.isRepo}
    {#if branchOpen}
      <div class="inline">
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" placeholder="nama-branch-baru" bind:value={branchName} autofocus onkeydown={(e) => e.key === "Enter" && createBranch()} />
        <button onclick={createBranch} disabled={!branchName.trim() || busy !== ""}>{busy === "branch" ? "…" : "Buat & pindah"}</button>
      </div>
    {/if}

    <div class="commit">
      <textarea rows="3" placeholder="Pesan commit, atau minta agent membuatkannya" bind:value={message}></textarea>
      <div class="row">
        <button class="ghost" onclick={suggest} disabled={busy !== "" || files.length === 0} title="Buat pesan commit dari diff dengan agent (panggilan singkat tanpa tool)">
          <Icon name="sparkle" size={13} />{busy === "suggest" ? "Menulis…" : "Buat pesan"}
        </button>
        <span class="spacer"></span>
        <button onclick={() => commit(false)} disabled={busy !== "" || !message.trim() || chosen.length === 0}>
          {busy === "commit" ? "…" : `Commit${allChosen ? "" : ` (${chosen.length})`}`}
        </button>
        <button class="primary" onclick={() => commit(true)} disabled={busy !== "" || !message.trim() || chosen.length === 0}>Commit &amp; Push</button>
      </div>
      <div class="row small">
        <span class="who"><Icon name="github" size={12} />{gh?.login ? `@${gh.login}` : "akun default gh"}</span>
        <span class="spacer"></span>
        {#if status.ahead > 0 || files.length === 0}
          <button class="ghost small" onclick={push} disabled={busy !== ""}><Icon name="push" size={12} />{busy === "push" ? "Push…" : "Push"}</button>
        {/if}
        <button class="ghost small" onclick={openPr} disabled={busy !== "" || onDefault} title={onDefault ? "Buat branch dulu: PR tidak bisa dari branch utama" : "Push lalu buat pull request"}><Icon name="pr" size={12} />Buat PR</button>
      </div>
      {#if prOpen}
        <div class="pr">
          <input type="text" placeholder="Judul PR" bind:value={prTitle} />
          <textarea rows="4" placeholder="Deskripsi (opsional)" bind:value={prBody}></textarea>
          <div class="row small">
            <label class="check"><input type="checkbox" bind:checked={prDraft} /> draft</label>
            <span class="spacer"></span>
            <button class="ghost small" onclick={() => (prOpen = false)}>Batal</button>
            <button class="primary small" onclick={createPr} disabled={busy !== "" || !prTitle.trim()}>{busy === "pr" ? "Membuat…" : "Push & buat PR"}</button>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <div class="files">
    {#if files.length}
      <label class="file all"><input type="checkbox" checked={allChosen} onchange={toggleAll} /> <span>semua file</span></label>
    {/if}
    {#each files as f (f.path)}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="file" class:sel={f.path === selected} onclick={() => show(f.path)}>
        <input
          type="checkbox"
          checked={!excluded[f.path]}
          onclick={(e) => e.stopPropagation()}
          onchange={(e) => (excluded[f.path] = !(e.target as HTMLInputElement).checked)}
          title="Ikutkan di commit"
        />
        <span class="st st-{label[f.status] ?? f.status}">{label[f.status] ?? f.status}</span>
        <span class="mono">{f.path}</span>
      </div>
    {/each}
    {#if status?.isRepo && files.length === 0}
      <div class="muted pad">Tidak ada perubahan.</div>
    {/if}
  </div>
  <div class="diff">
    {#if selected}
      <pre>{#each diff.split("\n") as line}<span class={cls(line)}>{line}
</span>{/each}</pre>
    {:else if recent.length}
      <div class="recent">
        <div class="label">Commit terakhir</div>
        {#each recent as c (c.hash)}
          <div class="c"><span class="hash">{c.hash}</span><span class="subj">{c.subject}</span><span class="when">{c.when}</span></div>
        {/each}
      </div>
    {:else}
      <div class="muted pad">Pilih file untuk melihat diff.</div>
    {/if}
  </div>
</div>

<style>
  .changes {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 38px;
    flex: none;
    padding: 0 6px 0 12px;
    border-bottom: 1px solid var(--line);
    font-size: 12.5px;
  }
  .br {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .br b {
    font: 600 12px/1 var(--mono);
  }
  .num {
    font: 400 11px/1 var(--mono);
    color: var(--muted);
  }
  .inline {
    display: flex;
    gap: 6px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--line);
  }
  .inline input {
    flex: 1;
  }
  .commit {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border-bottom: 1px solid var(--line);
  }
  .commit textarea,
  .pr textarea,
  .pr input {
    width: 100%;
    resize: vertical;
    padding: 7px 9px;
    font-size: 12.5px;
    line-height: 1.5;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .row.small {
    font-size: 11.5px;
  }
  .who {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-family: var(--mono);
    color: var(--muted);
  }
  .pr {
    display: flex;
    flex-direction: column;
    gap: 6px;
    border-radius: var(--radius);
    padding: 12px;
    background: var(--bg-2);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .files {
    max-height: 35%;
    overflow: auto;
    border-bottom: 1px solid var(--line);
    flex: none;
    padding: 4px 6px;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 9px;
    height: 26px;
    padding: 0 8px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    white-space: nowrap;
    color: var(--fg-2);
  }
  .file.all {
    color: var(--muted);
  }
  .file:hover {
    background: var(--bg-2);
    color: var(--fg);
  }
  .file.sel {
    background: var(--bg-3);
    color: var(--fg);
  }
  .file .mono {
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .st {
    width: 12px;
    font: 600 11px/1 var(--mono);
    flex: none;
  }
  .st-M {
    color: var(--warn);
  }
  .st-A,
  .st-U {
    color: var(--ok);
  }
  .st-D {
    color: var(--err);
  }
  .diff {
    flex: 1;
    overflow: auto;
    min-height: 0;
    background: var(--bg);
  }
  pre {
    margin: 0;
    padding: 8px 0;
    font-size: 12px;
    line-height: 1.5;
  }
  pre span {
    display: block;
    padding: 0 12px;
    white-space: pre;
  }
  .add {
    background: rgba(111, 201, 139, 0.09);
    color: #a9e3bb;
  }
  .del {
    background: rgba(242, 104, 93, 0.09);
    color: #f4b1ab;
  }
  .hunk {
    color: var(--codex);
  }
  .meta {
    color: var(--muted);
  }
  .pad {
    padding: 12px;
    font-size: 12px;
  }
  .recent {
    padding: 14px 12px;
    font-size: 12px;
    display: flex;
    flex-direction: column;
  }
  .recent .label {
    margin-bottom: 8px;
  }
  .c {
    display: flex;
    gap: 10px;
    align-items: baseline;
    padding: 5px 0;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
    min-width: 0;
  }
  .hash {
    font: 400 11px/1 var(--mono);
    color: var(--accent);
    flex: none;
  }
  .subj {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .when {
    font: 400 11px/1 var(--mono);
    color: var(--muted);
    flex: none;
  }
</style>
