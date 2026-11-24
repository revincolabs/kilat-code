<script lang="ts">
  import { onDestroy } from "svelte";
  import { api, type Account, type Settings } from "../lib/api";
  import { app, accountsOf, isLimited, accountIdentity, fail, toast, reloadData } from "../lib/state.svelte";
  import Terminal from "./Terminal.svelte";
  import Icon from "./Icon.svelte";
  import { chime } from "../lib/sound";
  import { fmtReset } from "../lib/format";
  import { updater, checkForUpdate, installUpdate } from "../lib/updater.svelte";
  import { t as tr, applyLang } from "../lib/i18n/index.svelte";
  import { id as idText } from "../lib/i18n/id";
  import { en as enText } from "../lib/i18n/en";

  type Tab = "claude" | "codex" | "github" | "settings";
  const tabs: { id: Tab; label: string }[] = $derived([
    { id: "claude", label: "Claude" },
    { id: "codex", label: "ChatGPT / Codex" },
    { id: "github", label: "GitHub" },
    { id: "settings", label: tr().accounts.settingsTab },
  ]);

  let newLabel = $state("");
  let apiKey = $state("");
  let showKeyFor = $state<"claude" | "codex" | null>(null);
  /**
   * Active one-time setup terminal. Claude subscriptions take two steps:
   * `claude-token:` (1-year token for the agent) then `claude-identity:`
   * (a sign-in that is only read for the email, then deleted).
   * `run` restarts the same command.
   */
  let setup = $state<{ accountId: string | null; command: string; provider: string; run?: number } | null>(null);
  let pasteToken = $state("");
  /** the email step ended without an email: offer to try again */
  let identityFailed = $state(false);
  let poll: ReturnType<typeof setInterval> | null = null;
  let settings = $state<Settings>(structuredClone($state.snapshot(app.data.settings)));
  // Show the built-in follow-up message in the current language.
  if ([idText.settings.defaultContinue, enText.settings.defaultContinue].includes(settings.continuePrompt)) {
    settings.continuePrompt = tr().settings.defaultContinue;
  }
  settings.language ||= "auto";
  let modelsText = $state((app.data.settings.claudeModels ?? []).join(", "));
  let busy = $state(false);

  const list = $derived(app.accountsTab === "settings" ? [] : accountsOf(app.accountsTab));
  /** Name is required and must be unique so accounts are never confused. */
  const nameError = $derived.by(() => {
    const n = newLabel.trim();
    if (!n) return tr().accounts.nameRequired;
    if (list.some((a) => a.label.toLowerCase() === n.toLowerCase())) return tr().accounts.nameTaken(n);
    return "";
  });
  const setupAccount = $derived(setup?.accountId ? app.data.accounts.find((a) => a.id === setup!.accountId) : undefined);

  const identityStep = $derived(!!setup?.command.startsWith("claude-identity:"));

  // Once the account becomes ready: Claude subscriptions without an email go
  // on to the email step, everything else closes the setup terminal.
  $effect(() => {
    if (!setup || setup.provider === "github" || identityStep || !setupAccount?.ready) return;
    if (setupAccount.provider === "claude" && setupAccount.kind === "oauth" && !setupAccount.email) {
      toast(tr().accounts.tokenSaved(setupAccount.label), "ok");
      pasteToken = "";
      setup = { ...setup, command: `claude-identity:${setupAccount.id}` };
      return;
    }
    toast(tr().accounts.ready(setupAccount.label), "ok");
    endSetup();
  });

  function endSetup() {
    setup = null;
    pasteToken = "";
    identityFailed = false;
    if (poll) clearInterval(poll);
    poll = null;
  }

  function putAccount(a: Account) {
    const i = app.data.accounts.findIndex((x) => x.id === a.id);
    if (i >= 0) app.data.accounts[i] = a;
  }

  function detectEmail(a: Account) {
    identityFailed = false;
    setup = { accountId: a.id, provider: "claude", command: `claude-identity:${a.id}` };
  }

  function retryIdentity() {
    if (!setup) return;
    identityFailed = false;
    setup = { ...setup, run: (setup.run ?? 0) + 1 };
  }

  /** The email-step sign-in ended (successfully or not): read who it was. */
  async function identityExit(_code: number | null, cmd?: string) {
    // Ignore terminals that were replaced or closed (they report exit too).
    const id = setup?.accountId;
    if (!id || !cmd || cmd !== setup?.command) return;
    try {
      const r = await api.claudeIdentity(id);
      putAccount(r.account);
      toast(tr().accounts.readyAs(r.account.label, accountIdentity(r.account)), "ok");
      if (r.warning) toast(r.warning, "error");
      endSetup();
    } catch (e) {
      identityFailed = true;
      fail(e);
    }
  }
  onDestroy(endSetup);

  function close() {
    app.accountsOpen = false;
  }

  async function startLogin(provider: "claude" | "codex") {
    if (nameError) return toast(nameError, "warn");
    try {
      const acc = await api.addAccount(provider, provider === "claude" ? "oauth" : "chatgpt", newLabel);
      app.data.accounts.push(acc);
      newLabel = "";
      setup = {
        accountId: acc.id,
        provider,
        command: provider === "claude" ? `claude-token:${acc.id}` : `codex-login:${acc.id}`,
      };
      if (provider === "codex") {
        poll = setInterval(async () => {
          try {
            putAccount(await api.refreshAccount(acc.id));
          } catch {}
        }, 2000);
      }
    } catch (e) {
      fail(e);
    }
  }

  function resetup(a: Account) {
    identityFailed = false;
    setup = {
      accountId: a.id,
      provider: a.provider,
      command: a.provider === "claude" ? `claude-token:${a.id}` : `codex-login:${a.id}`,
    };
  }

  async function addWithKey(provider: "claude" | "codex") {
    if (!apiKey.trim()) return;
    if (nameError) return toast(nameError, "warn");
    busy = true;
    let acc: Account | null = null;
    try {
      acc = await api.addAccount(provider, "apikey", newLabel);
      const ready = await api.setSecret(acc.id, apiKey);
      app.data.accounts.push(ready);
      toast(tr().accounts.saved(ready.label), "ok");
      apiKey = "";
      newLabel = "";
      showKeyFor = null;
    } catch (e) {
      if (acc) await api.removeAccount(acc.id).catch(() => {});
      fail(e);
    } finally {
      busy = false;
    }
  }

  async function savePasted() {
    if (!setup?.accountId || !pasteToken.trim()) return;
    try {
      putAccount(await api.setSecret(setup.accountId, pasteToken));
    } catch (e) {
      fail(e);
    }
  }

  async function rename(a: Account, input: HTMLInputElement) {
    const label = input.value.trim();
    if (label === a.label) return;
    try {
      Object.assign(a, await api.updateAccount(a.id, { label }));
      toast(tr().accounts.renamed(a.label), "ok");
    } catch (e) {
      input.value = a.label;
      fail(e);
    }
  }

  async function clearLimit(a: Account) {
    Object.assign(a, await api.updateAccount(a.id, { clearLimit: true }));
  }

  async function remove(a: Account) {
    if (!confirm(tr().accounts.confirmRemove(a.label))) return;
    try {
      await api.removeAccount(a.id);
      app.data.accounts = app.data.accounts.filter((x) => x.id !== a.id);
      if (setup?.accountId === a.id) endSetup();
    } catch (e) {
      fail(e);
    }
  }

  async function ghImport() {
    busy = true;
    try {
      await api.githubImport();
      await reloadData();
      toast(tr().accounts.ghImported, "ok");
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }

  async function saveGit(a: Account) {
    try {
      Object.assign(
        a,
        await api.updateAccount(a.id, { gitName: a.gitName ?? "", gitEmail: a.gitEmail ?? "", sshKey: a.sshKey ?? "" }),
      );
      toast(tr().accounts.gitSaved(a.login ?? ""), "ok");
    } catch (e) {
      fail(e);
    }
  }

  async function saveSettings() {
    settings.claudeModels = modelsText.split(",").map((s) => s.trim()).filter(Boolean);
    try {
      app.tools = await api.saveSettings($state.snapshot(settings) as Settings);
      app.data.settings = structuredClone($state.snapshot(settings)) as Settings;
      applyLang(settings.language);
      toast(tr().accounts.settingsSaved, "ok");
    } catch (e) {
      fail(e);
    }
  }

  function kindLabel(a: Account) {
    const k = tr().accounts.kind;
    if (a.kind === "default") return k.default;
    if (a.kind === "apikey") return k.apikey;
    if (a.provider === "claude") return k.claude;
    if (a.provider === "codex") return k.codex;
    return k.gh;
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" onclick={close}>
  <div class="modal" role="dialog" tabindex="-1" aria-label={tr().accounts.title} onclick={(e) => e.stopPropagation()}>
    <div class="top">
      <b>{tr().accounts.title}</b>
      <span class="spacer"></span>
      <button class="ghost icon" aria-label={tr().common.close} onclick={close}><Icon name="x" size={15} /></button>
    </div>
    <div class="tabs" role="tablist">
      {#each tabs as t}
        <button class="ghost" role="tab" aria-selected={app.accountsTab === t.id} class:sel={app.accountsTab === t.id} onclick={() => { app.accountsTab = t.id; endSetup(); }}>
          {t.label}
          {#if t.id !== "settings"}<span class="n">{accountsOf(t.id).length}</span>{/if}
        </button>
      {/each}
    </div>

    <div class="content">
      {#if app.accountsTab === "claude" || app.accountsTab === "codex"}
        {@const provider = app.accountsTab}
        <p class="intro">
          {@html provider === "claude" ? tr().accounts.introClaude : tr().accounts.introCodex}
        </p>

        <div class="list">
          {#each list as a (a.id)}
            <div class="acc">
              <span class="dot" class:ok={a.ready && !isLimited(a)} class:warn={isLimited(a)} class:err={!a.ready}></span>
              <div class="who">
                <input
                  type="text"
                  class="name-input"
                  value={a.label}
                  aria-label={tr().accounts.name}
                  title={tr().accounts.renameTitle}
                  onchange={(e) => rename(a, e.target as HTMLInputElement)}
                />
                <span class="email">
                  {accountIdentity(a) || (a.ready ? tr().accounts.noEmail : tr().accounts.notSetUp)}
                  {#if a.displayName && a.email}· {a.displayName}{/if}
                </span>
                {#if a.usage?.windows?.length || isLimited(a)}
                  <span class="usage">
                    {#if isLimited(a)}<span class="lim"><Icon name="clock" size={11} /> {tr().accounts.limitedUntil(fmtReset(a.limitedUntil))}</span>{/if}
                    {#each a.usage?.windows ?? [] as w (w.label)}
                      <span class="uw" class:hot={w.usedPercent >= 80} title={tr().accounts.updated(fmtReset(a.usage?.updatedAt))}>
                        <span class="ubar"><span style:width="{Math.min(100, w.usedPercent)}%"></span></span>
                        {tr().accounts.windowLine(tr().fmt.window(w.label), Math.round(w.usedPercent), fmtReset(w.resetsAt))}
                      </span>
                    {/each}
                  </span>
                {/if}
              </div>
              <span class="spacer"></span>
              <span class="badge">{kindLabel(a)}</span>
              {#if isLimited(a)}
                <button class="ghost small" onclick={() => clearLimit(a)} title={tr().accounts.clearLimitTitle}>{tr().accounts.clearLimit}</button>
              {/if}
              {#if !a.ready && a.kind !== "apikey" && a.kind !== "default"}
                <button class="small" onclick={() => resetup(a)}>{tr().accounts.setup}</button>
              {:else if a.provider === "claude" && a.kind === "oauth" && !a.email}
                <button class="small" onclick={() => detectEmail(a)} title={tr().accounts.detectEmailTitle}>
                  {tr().accounts.detectEmail}
                </button>
              {/if}
              {#if a.kind !== "default"}
                <button class="ghost icon del" title={tr().accounts.remove} aria-label={tr().accounts.remove} onclick={() => remove(a)}><Icon name="trash" size={14} /></button>
              {/if}
            </div>
          {/each}
        </div>

        {#if setup && setup.provider === provider}
          <div class="setup">
            <div class="row">
              <b>{tr().accounts.setupOf(setupAccount?.label ?? "")}</b>
              <span class="spacer"></span>
              <button class="ghost small" onclick={endSetup}>{tr().common.close}</button>
            </div>
            <p class="small">
              {@html provider === "claude" && identityStep ? tr().accounts.step2 : provider === "claude" ? tr().accounts.step1 : tr().accounts.codexSetup}
            </p>
            <div class="setup-term">
              {#key `${setup.command}#${setup.run ?? 0}`}
                <Terminal setup={setup.command} onexit={identityStep ? identityExit : undefined} />
              {/key}
            </div>
            {#if provider === "claude" && identityStep}
              {#if identityFailed}
                <div class="row">
                  <span class="small grow">{tr().accounts.noEmailYet}</span>
                  <button onclick={retryIdentity}>{tr().accounts.retry}</button>
                  <button class="ghost" onclick={endSetup}>{tr().accounts.skip}</button>
                </div>
              {/if}
            {:else if provider === "claude"}
              <div class="row">
                <input type="password" class="grow" placeholder={tr().accounts.pastePlaceholder} bind:value={pasteToken} />
                <button onclick={savePasted} disabled={!pasteToken.trim()}>{tr().accounts.saveToken}</button>
              </div>
            {/if}
          </div>
        {:else}
          <div class="label">{tr().accounts.add}</div>
          <div class="add">
            <input
              type="text"
              class:invalid={newLabel.trim() !== "" && !!nameError}
              placeholder={provider === "claude" ? tr().accounts.namePlaceholderClaude : tr().accounts.namePlaceholderCodex}
              aria-label={tr().accounts.newName}
              bind:value={newLabel}
            />
            <button class="primary" disabled={!!nameError} title={nameError} onclick={() => startLogin(provider)}>
              <Icon name="plus" size={13} />
              {provider === "claude" ? tr().accounts.loginClaude : tr().accounts.loginChatgpt}
            </button>
            <button disabled={!!nameError} title={nameError} onclick={() => (showKeyFor = showKeyFor === provider ? null : provider)}>
              <Icon name="key" size={13} /> {tr().accounts.useApiKey}
            </button>
          </div>
          {#if newLabel.trim() && nameError}<p class="name-err">{nameError}</p>{/if}
          {#if showKeyFor === provider}
            <div class="add">
              <input
                type="password"
                class="grow"
                placeholder={provider === "claude" ? "sk-ant-api…" : "sk-…"}
                aria-label="API key"
                bind:value={apiKey}
                onkeydown={(e) => e.key === "Enter" && addWithKey(provider)}
              />
              <button class="primary" disabled={busy || !apiKey.trim()} onclick={() => addWithKey(provider)}>{tr().common.save}</button>
            </div>
          {/if}
        {/if}
      {:else if app.accountsTab === "github"}
        <p class="intro">{@html tr().accounts.introGithub}</p>
        <div class="list">
          {#each list as a (a.id)}
            <div class="gh-acc">
              <div class="row">
                <span class="dot" class:ok={a.ready} class:err={!a.ready}></span>
                <b>@{a.login}</b>
                {#if !a.ready}<span class="muted small">{tr().accounts.notInGh}</span>{/if}
                <span class="spacer"></span>
                <button class="ghost icon del" title={tr().accounts.removeFromKilat} aria-label={tr().accounts.removeFromKilat} onclick={() => remove(a)}><Icon name="trash" size={14} /></button>
              </div>
              <div class="grid">
                <label>{tr().accounts.commitName} <input type="text" bind:value={a.gitName} /></label>
                <label>{tr().accounts.commitEmail} <input type="text" bind:value={a.gitEmail} /></label>
                <label class="full">{tr().accounts.sshKey} <input type="text" placeholder="~/.ssh/id_ed25519_kerja" bind:value={a.sshKey} /></label>
              </div>
              <div class="row"><span class="spacer"></span><button class="small" onclick={() => saveGit(a)}>{tr().common.save}</button></div>
            </div>
          {/each}
          {#if list.length === 0}
            <p class="small muted">{@html tr().accounts.noGithub}</p>
          {/if}
        </div>
        {#if setup && setup.provider === "github"}
          <div class="setup">
            <div class="row">
              <b>{tr().accounts.loginGithub}</b>
              <span class="spacer"></span>
              <button class="ghost small" onclick={() => { endSetup(); ghImport(); }}>{tr().accounts.done}</button>
            </div>
            <div class="setup-term">
              <Terminal setup="gh-login" onexit={() => { endSetup(); ghImport(); }} />
            </div>
          </div>
        {:else}
          <div class="add">
            <button class="primary" onclick={() => (setup = { accountId: null, provider: "github", command: "gh-login" })}><Icon name="plus" size={13} /> {tr().accounts.loginGithub}</button>
            <button disabled={busy} onclick={ghImport}><Icon name="refresh" size={13} /> {busy ? tr().accounts.importing : tr().accounts.importGh}</button>
          </div>
        {/if}
      {:else}
        <div class="settings">
          <div class="label">{tr().settings.language}</div>
          <label>
            <select class="lang" bind:value={settings.language}>
              <option value="auto">{tr().settings.languageAuto}</option>
              <option value="id">Bahasa Indonesia</option>
              <option value="en">English</option>
            </select>
          </label>

          <div class="label">{tr().settings.notifications}</div>
          <label class="check">
            <input type="checkbox" bind:checked={settings.notify} />
            {tr().settings.notify}
          </label>
          <div class="checkrow">
            <label class="check">
              <input type="checkbox" bind:checked={settings.sound} />
              {tr().settings.sound}
            </label>
            <button class="ghost small" onclick={() => chime("done")}><Icon name="sound" size={12} /> {tr().settings.try}</button>
          </div>

          <div class="label">{tr().settings.agent}</div>
          <label class="check">
            <input type="checkbox" bind:checked={settings.claudeChrome} />
            {@html tr().settings.chrome}
          </label>
          <label>{tr().settings.continuePrompt}
            <input type="text" bind:value={settings.continuePrompt} />
          </label>
          <label>{tr().settings.models}
            <input type="text" bind:value={modelsText} />
          </label>
          <label>{tr().settings.fontSize}
            <input type="number" min="10" max="22" bind:value={settings.fontSize} />
          </label>

          <div class="label">{tr().settings.programs} <span class="hint">{tr().settings.programsHint}</span></div>
          <div class="grid">
            <label>Bash <input type="text" placeholder={app.tools.bash ?? tr().common.notFound} bind:value={settings.bashPath} /></label>
            <label>Claude Code <input type="text" placeholder={app.tools.claude ?? tr().common.notFound} bind:value={settings.claudePath} /></label>
            <label>Codex <input type="text" placeholder={app.tools.codex ?? tr().common.notFound} bind:value={settings.codexPath} /></label>
            <label>GitHub CLI <input type="text" placeholder={app.tools.gh ?? tr().common.notFound} bind:value={settings.ghPath} /></label>
          </div>

          <div class="label">{tr().settings.about}</div>
          <p class="small muted">{tr().settings.dataDir} <code>{app.dataDir}</code></p>
          <div class="checkrow">
            <span class="small muted">Kilat Code v{app.version}{updater.version ? tr().settings.available(updater.version) : ""}</span>
            {#if updater.version}
              <button class="small primary" onclick={installUpdate} disabled={updater.progress != null}><Icon name="update" size={12} /> {tr().settings.updateNow}</button>
            {:else}
              <button class="ghost small" onclick={() => checkForUpdate(true)} disabled={updater.checking}>
                {updater.checking ? tr().settings.checking : tr().settings.check}
              </button>
            {/if}
          </div>
        </div>
      {/if}
    </div>
    {#if app.accountsTab === "settings"}
      <div class="foot">
        <button class="primary" onclick={saveSettings}>{tr().settings.save}</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(10, 10, 9, 0.6);
    display: grid;
    place-items: center;
    z-index: 50;
  }
  .modal {
    width: min(860px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    background: var(--bg-1);
    border: 1px solid var(--line-2);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-pop);
    overflow: hidden;
  }
  .top {
    display: flex;
    align-items: center;
    padding: 14px 12px 6px 18px;
  }
  .top b {
    font-size: 16px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .tabs {
    display: flex;
    gap: 4px;
    padding: 6px 14px 12px;
    border-bottom: 1px solid var(--line);
  }
  .tabs button {
    min-height: 30px;
    background: transparent;
    color: var(--muted);
    padding: 0 12px;
  }
  .tabs button:hover:not(:disabled) {
    background: var(--bg-2);
    color: var(--fg);
  }
  .tabs button.sel {
    background: var(--bg-3);
    color: var(--fg);
  }
  .tabs .n {
    font: 500 10.5px/1 var(--mono);
    color: var(--muted);
  }
  .content {
    padding: 16px 18px 18px;
    overflow-y: auto;
  }
  .intro {
    margin: 0 0 16px;
    font-size: 12.5px;
    line-height: 1.6;
    color: var(--fg-2);
    max-width: 70ch;
  }
  .small {
    font-size: 12px;
  }
  .label {
    margin: 4px 0 8px;
  }
  .label .hint {
    text-transform: none;
    letter-spacing: 0;
    margin-left: 6px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 20px;
  }
  .acc {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 8px 10px 14px;
    background: var(--bg-2);
    border-radius: var(--radius);
  }
  .who {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .name-err {
    color: var(--warn);
    font-size: 12px;
    margin: -2px 0 8px;
  }
  input.invalid {
    border-color: var(--warn);
  }
  .acc .name-input {
    width: 260px;
    font-weight: 600;
    padding: 2px 6px;
    margin-left: -6px;
    background: transparent;
    border-color: transparent;
  }
  .acc .name-input:hover,
  .acc .name-input:focus {
    border-color: var(--line-3);
  }
  .email {
    font-size: 11.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .del:hover:not(:disabled) {
    color: var(--err);
  }
  .add {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
  }
  .add input {
    flex: 1;
  }
  .add button {
    min-height: 30px;
  }
  .grow {
    flex: 1;
  }
  .setup {
    border-radius: var(--radius);
    padding: 14px;
    background: var(--bg-2);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .setup p {
    margin: 0;
    line-height: 1.6;
    color: var(--fg-2);
  }
  .setup-term {
    height: 260px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg);
  }
  .gh-acc {
    padding: 12px 14px 14px;
    background: var(--bg-2);
    border-radius: var(--radius);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px 12px;
  }
  .grid label,
  .settings label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--fg-2);
  }
  .grid .full {
    grid-column: 1 / -1;
  }
  .settings {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .settings .label {
    margin: 16px 0 0;
  }
  .settings .label:first-child {
    margin-top: 0;
  }
  .settings label.check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
  }
  .settings select.lang {
    width: 260px;
  }
  .settings input[type="number"] {
    width: 100px;
  }
  .settings p {
    margin: 0;
  }
  .checkrow {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    padding: 10px 18px;
    border-top: 1px solid var(--line);
  }
  .foot button {
    min-height: 30px;
  }
  .usage {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    margin-top: 2px;
    font: 400 11px/1.4 var(--mono);
    color: var(--fg-2);
  }
  .lim {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--warn);
  }
  .uw {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .uw.hot {
    color: var(--warn);
  }
  .ubar {
    width: 48px;
    height: 4px;
    border-radius: 2px;
    background: var(--line-2);
    overflow: hidden;
  }
  .ubar span {
    display: block;
    height: 100%;
    background: var(--ok);
  }
  .uw.hot .ubar span {
    background: var(--warn);
  }
</style>
