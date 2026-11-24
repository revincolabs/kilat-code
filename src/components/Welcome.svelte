<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { app, addProject, activeProject, newThread, fail, accountsOf } from "../lib/state.svelte";

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
  <div class="logo">⚡</div>
  <h1>Kilat Code</h1>
  <p class="muted">Terminal Claude Code &amp; Codex asli, banyak agent sekaligus, banyak akun.</p>

  <div class="cards">
    {#if project}
      <button class="card" onclick={() => newThread(project.id)}>
        <b>+ Thread baru</b>
        <span class="muted">di {project.name}</span>
      </button>
      <button class="card" onclick={() => newThread(project.id, true)}>
        <b>⑂ Thread di worktree</b>
        <span class="muted">agent paralel tanpa bentrok file</span>
      </button>
    {/if}
    <button class="card" onclick={pick}>
      <b>📁 Buka folder</b>
      <span class="muted">tambah project</span>
    </button>
    <button class="card" onclick={() => { app.accountsTab = "claude"; app.accountsOpen = true; }}>
      <b>👤 Akun</b>
      <span class="muted">{claudeCount} Claude · {codexCount} ChatGPT · {ghCount} GitHub</span>
    </button>
  </div>

  <ul class="tips muted">
    <li>Tiap thread adalah terminal <code>claude</code> / <code>codex</code> asli: slash command, /model, izin, semuanya jalan seperti biasa.</li>
    <li>Kena limit? Muncul pilihan: tunggu sampai reset lalu lanjut otomatis, pindah ke akun Claude lain (sesi sama), atau ke akun ChatGPT (riwayat diteruskan).</li>
    <li>Ganti akun kapan saja dari dropdown di atas terminal, konteks tetap utuh.</li>
    <li>Akun GitHub dipilih per project di status bar, langsung berlaku di terminal & agent yang sedang jalan.</li>
    <li><kbd>Ctrl</kbd>+<kbd>`</kbd> buka/tutup terminal bash.</li>
  </ul>
</div>

<style>
  .welcome {
    margin: auto;
    max-width: 640px;
    padding: 40px 24px;
    text-align: center;
  }
  .logo {
    font-size: 44px;
    color: var(--accent);
    filter: drop-shadow(0 0 18px rgba(255, 200, 61, 0.35));
  }
  h1 {
    margin: 6px 0 4px;
    font-size: 22px;
  }
  .cards {
    margin: 24px 0;
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
  }
  .card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 14px;
    text-align: left;
    background: var(--bg-1);
    border-radius: 10px;
  }
  .card span {
    font-size: 12px;
  }
  .tips {
    text-align: left;
    font-size: 12px;
    line-height: 1.8;
    padding-left: 18px;
  }
  kbd {
    border: 1px solid var(--line-2);
    border-radius: 4px;
    padding: 0 4px;
    font-size: 11px;
  }
</style>
