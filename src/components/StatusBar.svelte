<script lang="ts">
  import { api } from "../lib/api";
  import { app, activeProject, accountsOf, busyCount, fail, toast } from "../lib/state.svelte";
  import { updater, installUpdate } from "../lib/updater.svelte";

  const project = $derived(activeProject());
  const ghAccounts = $derived(accountsOf("github"));
  const missing = $derived(
    [
      !app.tools.bash && "bash",
      !app.tools.claude && "claude",
      !app.tools.codex && "codex",
      !app.tools.gh && "gh",
    ].filter(Boolean) as string[],
  );

  async function setGithub(e: Event) {
    const v = (e.target as HTMLSelectElement).value;
    if (!project) return;
    if (v === "__add") {
      (e.target as HTMLSelectElement).value = project.githubAccountId ?? "";
      app.accountsTab = "github";
      app.accountsOpen = true;
      return;
    }
    try {
      const p = await api.setProjectGithub(project.id, v || null);
      project.githubAccountId = p.githubAccountId;
      const acc = ghAccounts.find((a) => a.id === v);
      toast(acc ? `GitHub ${project.name} → @${acc.login} (langsung aktif di terminal & agent)` : `GitHub ${project.name}: pakai akun default gh`, "ok");
    } catch (err) {
      fail(err);
    }
  }
</script>

<footer>
  {#if project}
    <label class="gh" title="Akun GitHub untuk git push/pull & gh di project ini">
      <span class="muted">GitHub</span>
      <select value={project.githubAccountId ?? ""} onchange={setGithub}>
        <option value="">default gh</option>
        {#each ghAccounts as a (a.id)}
          <option value={a.id}>@{a.login}{a.ready ? "" : " (logout)"}</option>
        {/each}
        <option value="__add">+ Tambah akun GitHub…</option>
      </select>
    </label>
  {/if}
  <span class="spacer"></span>
  {#if missing.length}
    <button class="ghost warn" onclick={() => { app.accountsTab = "settings"; app.accountsOpen = true; }}>
      ⚠ tidak ditemukan: {missing.join(", ")}
    </button>
  {/if}
  {#if busyCount() > 0}
    <span class="run"><span class="dot run"></span> {busyCount()} agent bekerja</span>
  {/if}
  {#if updater.progress != null}
    <span class="upd">⬇ Mengunduh update{updater.progress ? ` ${Math.round(updater.progress * 100)}%` : "…"}</span>
  {:else if updater.version}
    <button class="ghost upd" title={updater.notes || "Unduh, pasang, lalu mulai ulang Kilat"} onclick={installUpdate}>
      ⬆ Update ke v{updater.version}
    </button>
  {/if}
  <span class="muted">Kilat {app.version}</span>
</footer>

<style>
  footer {
    height: 26px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 10px;
    border-top: 1px solid var(--line);
    background: var(--bg-1);
    font-size: 11px;
  }
  .gh {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .gh select {
    font-size: 11px;
    padding: 0 4px;
    height: 20px;
    background: var(--bg-2);
  }
  .run {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--accent);
  }
  .upd {
    color: var(--accent);
    font-size: 11px;
    padding: 0 6px;
    font-weight: 600;
  }
  .warn {
    color: var(--warn);
    font-size: 11px;
    padding: 0 6px;
  }
</style>
