<script lang="ts">
  import { onDestroy } from "svelte";
  import { api, type Account, type Settings } from "../lib/api";
  import { app, accountsOf, isLimited, accountIdentity, fail, toast, reloadData } from "../lib/state.svelte";
  import Terminal from "./Terminal.svelte";
  import Icon from "./Icon.svelte";
  import { chime } from "../lib/sound";
  import { fmtReset } from "../lib/format";
  import { updater, checkForUpdate, installUpdate } from "../lib/updater.svelte";

  type Tab = "claude" | "codex" | "github" | "settings";
  const tabs: { id: Tab; label: string }[] = [
    { id: "claude", label: "Claude" },
    { id: "codex", label: "ChatGPT / Codex" },
    { id: "github", label: "GitHub" },
    { id: "settings", label: "Pengaturan" },
  ];

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
  let modelsText = $state((app.data.settings.claudeModels ?? []).join(", "));
  let busy = $state(false);

  const list = $derived(app.accountsTab === "settings" ? [] : accountsOf(app.accountsTab));
  /** Name is required and must be unique so accounts are never confused. */
  const nameError = $derived.by(() => {
    const n = newLabel.trim();
    if (!n) return "Isi nama akun dulu (wajib)";
    if (list.some((a) => a.label.toLowerCase() === n.toLowerCase())) return `Nama "${n}" sudah dipakai`;
    return "";
  });
  const setupAccount = $derived(setup?.accountId ? app.data.accounts.find((a) => a.id === setup!.accountId) : undefined);

  const identityStep = $derived(!!setup?.command.startsWith("claude-identity:"));

  // Once the account becomes ready: Claude subscriptions without an email go
  // on to the email step, everything else closes the setup terminal.
  $effect(() => {
    if (!setup || setup.provider === "github" || identityStep || !setupAccount?.ready) return;
    if (setupAccount.provider === "claude" && setupAccount.kind === "oauth" && !setupAccount.email) {
      toast(`Token "${setupAccount.label}" tersimpan. Lanjut deteksi email.`, "ok");
      pasteToken = "";
      setup = { ...setup, command: `claude-identity:${setupAccount.id}` };
      return;
    }
    toast(`Akun "${setupAccount.label}" siap dipakai`, "ok");
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
      toast(`Akun "${r.account.label}" siap: ${accountIdentity(r.account)}`, "ok");
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
      toast(`Akun "${ready.label}" tersimpan`, "ok");
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
      toast(`Nama akun diganti: ${a.label}`, "ok");
    } catch (e) {
      input.value = a.label;
      fail(e);
    }
  }

  async function clearLimit(a: Account) {
    Object.assign(a, await api.updateAccount(a.id, { clearLimit: true }));
  }

  async function remove(a: Account) {
    if (!confirm(`Hapus akun "${a.label}" dari Kilat?`)) return;
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
      toast("Akun GitHub diperbarui dari gh", "ok");
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
      toast(`Identitas git @${a.login} disimpan`, "ok");
    } catch (e) {
      fail(e);
    }
  }

  async function saveSettings() {
    settings.claudeModels = modelsText.split(",").map((s) => s.trim()).filter(Boolean);
    try {
      app.tools = await api.saveSettings($state.snapshot(settings) as Settings);
      app.data.settings = structuredClone($state.snapshot(settings)) as Settings;
      toast("Pengaturan disimpan", "ok");
    } catch (e) {
      fail(e);
    }
  }

  function kindLabel(a: Account) {
    if (a.kind === "default") return "login bawaan";
    if (a.kind === "apikey") return "API key";
    if (a.provider === "claude") return "langganan";
    if (a.provider === "codex") return "ChatGPT";
    return "gh";
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" onclick={close}>
  <div class="modal" role="dialog" tabindex="-1" aria-label="Akun & Pengaturan" onclick={(e) => e.stopPropagation()}>
    <div class="top">
      <b>Akun &amp; Pengaturan</b>
      <span class="spacer"></span>
      <button class="ghost icon" aria-label="Tutup" onclick={close}><Icon name="x" size={15} /></button>
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
          {#if provider === "claude"}
            Setup tiap akun Claude <b>sekali saja</b>. Token disimpan di penyimpan rahasia sistem. Semua akun memakai
            penyimpanan sesi yang sama, jadi ganti akun di tengah thread langsung melanjutkan sesi tanpa kehilangan konteks.
          {:else}
            Tiap akun ChatGPT punya folder login sendiri, sedangkan folder <code>sessions</code> dipakai bersama, jadi
            <code>codex resume</code> jalan dari akun mana pun. Login sekali, selanjutnya cukup pilih akun di bar atas terminal.
          {/if}
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
                  aria-label="Nama akun"
                  title="Klik untuk ganti nama"
                  onchange={(e) => rename(a, e.target as HTMLInputElement)}
                />
                <span class="email">
                  {accountIdentity(a) || (a.ready ? "email belum terdeteksi" : "belum di-setup")}
                  {#if a.displayName && a.email}· {a.displayName}{/if}
                </span>
                {#if a.usage?.windows?.length || isLimited(a)}
                  <span class="usage">
                    {#if isLimited(a)}<span class="lim"><Icon name="clock" size={11} /> limit sampai {fmtReset(a.limitedUntil)}</span>{/if}
                    {#each a.usage?.windows ?? [] as w (w.label)}
                      <span class="uw" class:hot={w.usedPercent >= 80} title="Diperbarui {fmtReset(a.usage?.updatedAt)}">
                        <span class="ubar"><span style:width="{Math.min(100, w.usedPercent)}%"></span></span>
                        {w.label} {Math.round(w.usedPercent)}% · reset {fmtReset(w.resetsAt)}
                      </span>
                    {/each}
                  </span>
                {/if}
              </div>
              <span class="spacer"></span>
              <span class="badge">{kindLabel(a)}</span>
              {#if isLimited(a)}
                <button class="ghost small" onclick={() => clearLimit(a)} title="Limit sudah reset?">Hapus tanda limit</button>
              {/if}
              {#if !a.ready && a.kind !== "apikey" && a.kind !== "default"}
                <button class="small" onclick={() => resetup(a)}>Setup</button>
              {:else if a.provider === "claude" && a.kind === "oauth" && !a.email}
                <button class="small" onclick={() => detectEmail(a)} title="Login sekali lagi di browser untuk membaca email & paket akun ini">
                  Deteksi email
                </button>
              {/if}
              {#if a.kind !== "default"}
                <button class="ghost icon del" title="Hapus akun" aria-label="Hapus akun" onclick={() => remove(a)}><Icon name="trash" size={14} /></button>
              {/if}
            </div>
          {/each}
        </div>

        {#if setup && setup.provider === provider}
          <div class="setup">
            <div class="row">
              <b>Setup: {setupAccount?.label}</b>
              <span class="spacer"></span>
              <button class="ghost small" onclick={endSetup}>Tutup</button>
            </div>
            <p class="small">
              {#if provider === "claude" && identityStep}
                <b>Langkah 2 dari 2: deteksi email.</b> Browser terbuka sekali lagi. Pakai akun Claude yang
                <b>sama</b>, lalu izinkan. Login ini hanya dibaca untuk email &amp; paket, lalu langsung dihapus. Agent tetap
                memakai token 1 tahun. Kilat mengecek bahwa login ini memang milik akun yang sama dengan tokennya.
              {:else if provider === "claude"}
                <b>Langkah 1 dari 2: token.</b> Browser akan terbuka. Login dengan akun Claude yang ingin ditambahkan, lalu
                izinkan. Kalau browser masih login ke akun Claude lain, ganti dulu di claude.ai. Token panjang (1 tahun)
                akan <b>tertangkap otomatis</b> dari terminal di bawah. Kalau tidak, salin tokennya ke kolom ini.
              {:else}
                Browser akan terbuka. Login dengan akun ChatGPT yang ingin ditambahkan. Setelah berhasil, akun otomatis
                ditandai siap.
              {/if}
            </p>
            <div class="setup-term">
              {#key `${setup.command}#${setup.run ?? 0}`}
                <Terminal setup={setup.command} onexit={identityStep ? identityExit : undefined} />
              {/key}
            </div>
            {#if provider === "claude" && identityStep}
              {#if identityFailed}
                <div class="row">
                  <span class="small grow">Email belum terdeteksi. Akun tetap bisa dipakai.</span>
                  <button onclick={retryIdentity}>Ulangi</button>
                  <button class="ghost" onclick={endSetup}>Lewati</button>
                </div>
              {/if}
            {:else if provider === "claude"}
              <div class="row">
                <input type="password" class="grow" placeholder="sk-ant-oat01-… (opsional, kalau tidak tertangkap otomatis)" bind:value={pasteToken} />
                <button onclick={savePasted} disabled={!pasteToken.trim()}>Simpan token</button>
              </div>
            {/if}
          </div>
        {:else}
          <div class="label">Tambah akun</div>
          <div class="add">
            <input
              type="text"
              class:invalid={newLabel.trim() !== "" && !!nameError}
              placeholder={provider === "claude" ? "Nama akun (wajib), mis. Claude Kantor" : "Nama akun (wajib), mis. GPT Pribadi"}
              aria-label="Nama akun baru"
              bind:value={newLabel}
            />
            <button class="primary" disabled={!!nameError} title={nameError} onclick={() => startLogin(provider)}>
              <Icon name="plus" size={13} />
              {provider === "claude" ? "Login langganan (Pro/Max)" : "Login ChatGPT"}
            </button>
            <button disabled={!!nameError} title={nameError} onclick={() => (showKeyFor = showKeyFor === provider ? null : provider)}>
              <Icon name="key" size={13} /> Pakai API key
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
              <button class="primary" disabled={busy || !apiKey.trim()} onclick={() => addWithKey(provider)}>Simpan</button>
            </div>
          {/if}
        {/if}
      {:else if app.accountsTab === "github"}
        <p class="intro">
          Akun GitHub disimpan oleh GitHub CLI (<code>gh</code>) di keyring sistem. Login tiap akun <b>sekali</b>, lalu pilih
          akun per project di status bar bawah. Push, pull, dan <code>gh</code> di terminal maupun agent langsung pakai akun
          itu, termasuk yang sedang berjalan.
        </p>
        <div class="list">
          {#each list as a (a.id)}
            <div class="gh-acc">
              <div class="row">
                <span class="dot" class:ok={a.ready} class:err={!a.ready}></span>
                <b>@{a.login}</b>
                {#if !a.ready}<span class="muted small">tidak login di gh</span>{/if}
                <span class="spacer"></span>
                <button class="ghost icon del" title="Hapus dari Kilat" aria-label="Hapus dari Kilat" onclick={() => remove(a)}><Icon name="trash" size={14} /></button>
              </div>
              <div class="grid">
                <label>Nama commit <input type="text" bind:value={a.gitName} /></label>
                <label>Email commit <input type="text" bind:value={a.gitEmail} /></label>
                <label class="full">SSH key (opsional, untuk remote git@) <input type="text" placeholder="~/.ssh/id_ed25519_kerja" bind:value={a.sshKey} /></label>
              </div>
              <div class="row"><span class="spacer"></span><button class="small" onclick={() => saveGit(a)}>Simpan</button></div>
            </div>
          {/each}
          {#if list.length === 0}
            <p class="small muted">Belum ada. Klik "Impor dari gh" kalau kamu sudah pernah <code>gh auth login</code>.</p>
          {/if}
        </div>
        {#if setup && setup.provider === "github"}
          <div class="setup">
            <div class="row">
              <b>Login akun GitHub baru</b>
              <span class="spacer"></span>
              <button class="ghost small" onclick={() => { endSetup(); ghImport(); }}>Selesai</button>
            </div>
            <div class="setup-term">
              <Terminal setup="gh-login" onexit={() => { endSetup(); ghImport(); }} />
            </div>
          </div>
        {:else}
          <div class="add">
            <button class="primary" onclick={() => (setup = { accountId: null, provider: "github", command: "gh-login" })}><Icon name="plus" size={13} /> Login akun GitHub baru</button>
            <button disabled={busy} onclick={ghImport}><Icon name="refresh" size={13} /> {busy ? "Mengimpor…" : "Impor dari gh"}</button>
          </div>
        {/if}
      {:else}
        <div class="settings">
          <div class="label">Notifikasi</div>
          <label class="check">
            <input type="checkbox" bind:checked={settings.notify} />
            Notifikasi sistem saat agent selesai, butuh izin, atau kena limit (ketika jendela Kilat tidak aktif)
          </label>
          <div class="checkrow">
            <label class="check">
              <input type="checkbox" bind:checked={settings.sound} />
              Bunyikan suara juga
            </label>
            <button class="ghost small" onclick={() => chime("done")}><Icon name="sound" size={12} /> coba</button>
          </div>

          <div class="label">Agent</div>
          <label class="check">
            <input type="checkbox" bind:checked={settings.claudeChrome} />
            Claude in Chrome: agent Claude bisa membuka &amp; mengendalikan Chrome lewat extension Claude
          </label>
          <label>Pesan yang dikirim ke agent saat melanjutkan setelah limit
            <input type="text" bind:value={settings.continuePrompt} />
          </label>
          <label>Daftar model Claude (pisahkan koma)
            <input type="text" bind:value={modelsText} />
          </label>
          <label>Ukuran font terminal
            <input type="number" min="10" max="22" bind:value={settings.fontSize} />
          </label>

          <div class="label">Lokasi program <span class="hint">kosongkan untuk deteksi otomatis</span></div>
          <div class="grid">
            <label>Bash <input type="text" placeholder={app.tools.bash ?? "tidak ditemukan"} bind:value={settings.bashPath} /></label>
            <label>Claude Code <input type="text" placeholder={app.tools.claude ?? "tidak ditemukan"} bind:value={settings.claudePath} /></label>
            <label>Codex <input type="text" placeholder={app.tools.codex ?? "tidak ditemukan"} bind:value={settings.codexPath} /></label>
            <label>GitHub CLI <input type="text" placeholder={app.tools.gh ?? "tidak ditemukan"} bind:value={settings.ghPath} /></label>
          </div>

          <div class="label">Tentang</div>
          <p class="small muted">Data aplikasi: <code>{app.dataDir}</code></p>
          <div class="checkrow">
            <span class="small muted">Kilat Code v{app.version}{updater.version ? ` · versi ${updater.version} tersedia` : ""}</span>
            {#if updater.version}
              <button class="small primary" onclick={installUpdate} disabled={updater.progress != null}><Icon name="update" size={12} /> Update sekarang</button>
            {:else}
              <button class="ghost small" onclick={() => checkForUpdate(true)} disabled={updater.checking}>
                {updater.checking ? "Memeriksa…" : "Periksa update"}
              </button>
            {/if}
          </div>
        </div>
      {/if}
    </div>
    {#if app.accountsTab === "settings"}
      <div class="foot">
        <button class="primary" onclick={saveSettings}>Simpan pengaturan</button>
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
