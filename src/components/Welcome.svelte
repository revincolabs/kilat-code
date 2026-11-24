<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { app, addProject, activeProject, newThread, fail, accountsOf } from "../lib/state.svelte";
  import { keyLabel } from "../lib/platform";
  import Icon from "./Icon.svelte";

  const project = $derived(activeProject());
  const claudeCount = $derived(accountsOf("claude").filter((a) => a.ready).length);
  const codexCount = $derived(accountsOf("codex").filter((a) => a.ready).length);
  const ghCount = $derived(accountsOf("github").length);

  async function pick() {
    try {
      const dir = await open({ directory: true, title: "Pilih folder project" });
      if (typeof dir === "string") await addProject(dir);
    } catch (e) {
      fail(e);
    }
  }
</script>

<div class="welcome">
  <svg class="bolt" viewBox="0 0 24 24" aria-hidden="true"><path d="M13.5 2L4 14h7l-1.5 8L20 9.5h-7.2z" /></svg>
  <h1>Kilat Code</h1>
  <p class="lead">Terminal Claude Code &amp; Codex asli. Banyak agent sekaligus, banyak akun, tanpa login ulang.</p>

  <div class="label">Mulai</div>
  <div class="actions">
    {#if project}
      <button class="act" onclick={() => newThread(project.id)}>
        <Icon name="plus" />
        <b>Thread baru</b>
        <span>di {project.name}</span>
      </button>
      <button class="act" onclick={() => newThread(project.id, true)}>
        <Icon name="branch" />
        <b>Thread di worktree</b>
        <span>agent paralel tanpa bentrok file</span>
      </button>
    {/if}
    <button class="act" onclick={pick}>
      <Icon name="folder" />
      <b>Buka folder</b>
      <span>tambah project</span>
    </button>
    <button class="act" onclick={() => { app.accountsTab = "claude"; app.accountsOpen = true; }}>
      <Icon name="user" />
      <b>Akun</b>
      <span class="mono">{claudeCount} Claude · {codexCount} ChatGPT · {ghCount} GitHub</span>
    </button>
  </div>

  <div class="label">Perlu diketahui</div>
  <ul class="tips">
    <li>Tiap thread adalah terminal <code>claude</code> / <code>codex</code> asli: slash command, <code>/model</code>, dan izin jalan seperti biasa.</li>
    <li>Kena limit? Pilih tunggu sampai reset lalu lanjut otomatis, pindah ke akun Claude lain (sesi sama), atau ke akun ChatGPT (riwayat diteruskan).</li>
    <li>Ganti akun kapan saja dari bar di atas terminal. Konteksnya tetap utuh.</li>
    <li>Akun GitHub dipilih per project di status bar, langsung berlaku di terminal dan agent yang sedang jalan.</li>
    <li><kbd>{keyLabel("`")}</kbd> membuka terminal bash, <kbd>{keyLabel("P")}</kbd> mencari file.</li>
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
    margin: 14px 0 6px;
    font-size: 30px;
    font-weight: 780;
    font-stretch: 80%;
    letter-spacing: -0.02em;
    line-height: 1.05;
  }
  .lead {
    margin: 0 0 32px;
    color: var(--fg-2);
    font-size: 14px;
  }
  .label {
    margin-bottom: 8px;
  }
  .actions {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--line);
    margin-bottom: 32px;
  }
  .act {
    display: grid;
    grid-template-columns: 18px 170px 1fr;
    gap: 12px;
    align-items: center;
    justify-content: stretch;
    text-align: left;
    min-height: 42px;
    padding: 0 10px;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--line);
    border-radius: 0;
    color: var(--fg-2);
  }
  .act:hover:not(:disabled) {
    background: var(--bg-1);
    border-color: var(--line);
    color: var(--accent);
  }
  .act b {
    color: var(--fg);
    font-weight: 600;
  }
  .act span {
    color: var(--muted);
    font-size: 12.5px;
    overflow: hidden;
    text-overflow: ellipsis;
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
    position: relative;
    padding: 7px 0 7px 16px;
    line-height: 1.55;
  }
  .tips li::before {
    content: "";
    position: absolute;
    left: 2px;
    top: 16px;
    width: 6px;
    height: 1px;
    background: var(--muted);
  }
</style>
