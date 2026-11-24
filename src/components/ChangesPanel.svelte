<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, type Commit, type GitStatus } from "../lib/api";
  import { app, account, activeProject, activeThread, accountsOf, fail, toast } from "../lib/state.svelte";

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
      <span>⑂ <b>{status.branch || "(detached)"}</b></span>
      {#if status.ahead}<span class="muted" title="Commit yang belum di-push">↑{status.ahead}</span>{/if}
      {#if status.behind}<span class="muted" title="Commit di remote yang belum ditarik">↓{status.behind}</span>{/if}
      <span class="muted">· {files.length} file berubah</span>
    {:else if status}
      <span class="muted">Bukan repo git</span>
    {/if}
    <span class="spacer"></span>
    {#if status?.isRepo}
      <button class="ghost small" onclick={() => (branchOpen = !branchOpen)} title="Buat branch baru dari posisi sekarang">+ branch</button>
    {/if}
    <button class="ghost icon" onclick={refresh} title="Muat ulang">{loading ? "…" : "↻"}</button>
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
      <textarea rows="3" placeholder="Pesan commit… (✨ minta agent membuatkannya)" bind:value={message}></textarea>
      <div class="row">
        <button class="ghost" onclick={suggest} disabled={busy !== "" || files.length === 0} title="Buat pesan commit dari diff dengan agent (panggilan singkat tanpa tool)">
          {busy === "suggest" ? "Menulis…" : "✨ Buat pesan"}
        </button>
        <span class="spacer"></span>
        <button onclick={() => commit(false)} disabled={busy !== "" || !message.trim() || chosen.length === 0}>
          {busy === "commit" ? "…" : `Commit${allChosen ? "" : ` (${chosen.length})`}`}
        </button>
        <button class="primary" onclick={() => commit(true)} disabled={busy !== "" || !message.trim() || chosen.length === 0}>Commit &amp; Push</button>
      </div>
      <div class="row small">
        <span class="muted">{gh?.login ? `GitHub: @${gh.login}` : "GitHub: akun default gh"}</span>
        <span class="spacer"></span>
        {#if status.ahead > 0 || files.length === 0}
          <button class="ghost small" onclick={push} disabled={busy !== ""}>{busy === "push" ? "Push…" : "⇡ Push"}</button>
        {/if}
        <button class="ghost small" onclick={openPr} disabled={busy !== "" || onDefault} title={onDefault ? "Buat branch dulu: PR tidak bisa dari branch utama" : "Push lalu buat pull request"}>⤴ Buat PR</button>
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
      <label class="file all"><input type="checkbox" checked={allChosen} onchange={toggleAll} /> <span class="muted">semua file</span></label>
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
        <div class="muted small">Commit terakhir</div>
        {#each recent as c (c.hash)}
          <div class="c"><span class="mono muted">{c.hash}</span> <span>{c.subject}</span> <span class="muted">· {c.when}</span></div>
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
    gap: 6px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--line);
    font-size: 12px;
  }
  .inline {
    display: flex;
    gap: 6px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--line);
  }
  .inline input {
    flex: 1;
  }
  .commit {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--line);
  }
  .commit textarea,
  .pr textarea,
  .pr input {
    width: 100%;
    resize: vertical;
    background: var(--bg-2);
    border: 1px solid var(--line-2);
    border-radius: 6px;
    padding: 6px 8px;
    font-size: 12px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .small {
    font-size: 11px;
  }
  button.small {
    padding: 1px 8px;
  }
  .pr {
    display: flex;
    flex-direction: column;
    gap: 6px;
    border: 1px solid var(--line-2);
    border-radius: 8px;
    padding: 8px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .files {
    max-height: 35%;
    overflow: auto;
    border-bottom: 1px solid var(--line);
    flex: none;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 10px;
    cursor: pointer;
    font-size: 12px;
    white-space: nowrap;
  }
  .file input {
    margin: 0;
  }
  .file.all {
    padding-top: 4px;
  }
  .file:hover {
    background: var(--bg-2);
  }
  .file.sel {
    background: var(--bg-3);
  }
  .st {
    width: 14px;
    font-weight: 700;
    font-family: var(--mono);
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
  }
  pre {
    margin: 0;
    padding: 8px 0;
    font-size: 12px;
    line-height: 1.45;
  }
  pre span {
    display: block;
    padding: 0 10px;
    white-space: pre;
  }
  .add {
    background: rgba(74, 222, 128, 0.1);
    color: #a6f3c0;
  }
  .del {
    background: rgba(248, 113, 113, 0.1);
    color: #fbb4b4;
  }
  .hunk {
    color: var(--codex);
  }
  .meta {
    color: var(--muted);
  }
  .pad {
    padding: 10px;
    font-size: 12px;
  }
  .recent {
    padding: 10px;
    font-size: 12px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .c {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
