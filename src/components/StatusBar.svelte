<script lang="ts">
  import { api } from "../lib/api";
  import { app, activeProject, accountsOf, busyCount, fail, toast } from "../lib/state.svelte";
  import { updater, installUpdate } from "../lib/updater.svelte";
  import Icon from "./Icon.svelte";
  import { t } from "../lib/i18n/index.svelte";

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
      toast(acc ? t().status.ghSet(project.name, acc.login ?? "") : t().status.ghCleared(project.name), "ok");
    } catch (err) {
      fail(err);
    }
  }
</script>

<footer>
  {#if project}
    <label class="gh" title={t().status.ghTitle}>
      <Icon name="github" size={12} />
      <select value={project.githubAccountId ?? ""} onchange={setGithub} aria-label={t().status.ghLabel}>
        <option value="">{t().status.ghDefault}</option>
        {#each ghAccounts as a (a.id)}
          <option value={a.id}>@{a.login}{a.ready ? "" : t().status.loggedOut}</option>
        {/each}
        <option value="__add">{t().status.addGithub}</option>
      </select>
    </label>
  {/if}
  <span class="spacer"></span>
  {#if missing.length}
    <button class="ghost warn" onclick={() => { app.accountsTab = "settings"; app.accountsOpen = true; }}>
      <Icon name="alert" size={12} /> {t().status.missing(missing.join(", "))}
    </button>
  {/if}
  {#if busyCount() > 0}
    <span class="run"><span class="dot run"></span>{t().status.busy(busyCount())}</span>
  {/if}
  {#if updater.progress != null}
    <span class="upd"><Icon name="download" size={12} /> {t().status.downloading(updater.progress ? ` ${Math.round(updater.progress * 100)}%` : "…")}</span>
  {:else if updater.version}
    <button class="upd-btn" title={updater.notes || t().status.updateTitle} onclick={installUpdate}>
      <Icon name="update" size={12} /> {t().status.updateTo(updater.version)}
    </button>
  {/if}
  <span class="ver">Kilat {app.version}</span>
</footer>

<style>
  footer {
    height: 26px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 0 6px 0 10px;
    border-top: 1px solid var(--line);
    background: var(--bg-1);
    font: 400 11px/1 var(--mono);
    color: var(--muted);
  }
  .gh {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .gh select {
    font: inherit;
    height: 20px;
    padding: 0 20px 0 4px;
    border-color: transparent;
    background-color: transparent;
    background-position: right 5px center;
    color: var(--fg-2);
  }
  .gh select:hover {
    border-color: var(--line-2);
  }
  footer button {
    min-height: 20px;
    font: inherit;
    padding: 0 6px;
    gap: 5px;
  }
  .run {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--accent);
  }
  .upd {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--accent);
  }
  footer .upd-btn {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
    font-weight: 600;
  }
  footer .upd-btn:hover:not(:disabled) {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }
  .warn {
    color: var(--warn);
  }
  .ver {
    padding-right: 4px;
  }
</style>
