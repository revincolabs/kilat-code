<script lang="ts">
  import { api, type Account, type ThreadMeta } from "../lib/api";
  import { app, account, accountsOf, accountIdentity, isLimited, providerLabel, switchAgent, fail } from "../lib/state.svelte";
  import { fmtReset } from "../lib/format";
  import { fmtIn, resetFor } from "../lib/limits";

  /** Shown over the terminal when the thread's account hits its limit. */
  let { thread }: { thread: ThreadMeta } = $props();

  let busy = $state<string | null>(null);
  let now = $state(Date.now());
  $effect(() => {
    const iv = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(iv);
  });

  const limit = $derived(thread.limit!);
  const limitedAcc = $derived(account(limit.accountId));
  const reset = $derived(resetFor(limit, limitedAcc, now));
  const claudeOthers = $derived(accountsOf("claude").filter((a) => a.id !== limit.accountId));
  const codexOthers = $derived(accountsOf("codex").filter((a) => a.id !== limit.accountId));

  async function waitUntil(until: number) {
    busy = "wait";
    try {
      await api.agentWait(thread.id, until);
    } catch (e) {
      fail(e);
    } finally {
      busy = null;
    }
  }

  async function moveTo(a: Account) {
    busy = a.id;
    const sameProvider = a.provider === thread.provider;
    await switchAgent({
      threadId: thread.id,
      provider: a.provider as "claude" | "codex",
      accountId: a.id,
      model: sameProvider ? thread.model : "",
      mode: thread.mode,
      reason: "limit",
    });
    busy = null;
  }

  function dismiss() {
    api.agentDismissLimit(thread.id).catch(fail);
  }

  function addAccount(provider: "claude" | "codex") {
    app.accountsTab = provider;
    app.accountsOpen = true;
  }

  function usageText(a: Account): string {
    if (isLimited(a)) return `⏳ limit s/d ${fmtReset(a.limitedUntil)}`;
    const w = a.usage?.windows ?? [];
    return w.length ? w.slice(0, 2).map((x) => `${x.label} ${Math.round(x.usedPercent)}%`).join(" · ") : "";
  }
</script>

{#snippet choice(a: Account)}
  <button
    class="acc"
    class:hot={isLimited(a)}
    disabled={!!busy || !a.ready}
    title={a.ready ? accountIdentity(a) : "Akun belum siap — login dulu di Akun & Pengaturan"}
    onclick={() => moveTo(a)}
  >
    <span class="name">{busy === a.id ? "Memindahkan…" : a.label}</span>
    <span class="id muted">{a.ready ? accountIdentity(a) || "—" : "belum login"}</span>
    {#if usageText(a)}<span class="use">{usageText(a)}</span>{/if}
  </button>
{/snippet}

<div class="veil">
  <div class="card" role="dialog" aria-label="Akun kena limit">
    <div class="head">
      <span class="icon">⏳</span>
      <div class="grow">
        <b>Akun {providerLabel(limit.provider)} "{limitedAcc?.label ?? limit.accountId}" kena limit</b>
        {#if limit.message}<div class="msg muted">{limit.message}</div>{/if}
      </div>
      <button class="ghost icon" title="Tutup (urus sendiri di terminal)" onclick={dismiss}>×</button>
    </div>

    <section>
      <h4>Tunggu</h4>
      {#if reset}
        <button class="primary wide" disabled={!!busy} onclick={() => waitUntil(reset + 60_000)}>
          ⏱ Tunggu sampai {fmtReset(reset)} ({fmtIn(reset - now)} lagi), lalu lanjut otomatis
        </button>
      {:else}
        <p class="muted small">Waktu reset tidak diketahui. Lanjut otomatis setelah:</p>
      {/if}
      <div class="row quick">
        {#if reset}<span class="muted small">atau tunggu</span>{/if}
        {#each [30, 60, 180, 300] as min (min)}
          <button class="small" disabled={!!busy} onclick={() => waitUntil(now + min * 60_000)}>
            {min < 60 ? `${min} menit` : `${min / 60} jam`}
          </button>
        {/each}
      </div>
    </section>

    <section>
      <h4>Pindah ke akun Claude lain <span class="muted">· sesi yang sama dilanjutkan</span></h4>
      {#each claudeOthers as a (a.id)}
        {@render choice(a)}
      {:else}
        <p class="muted small">Belum ada akun Claude lain. <button class="link" onclick={() => addAccount("claude")}>+ Tambah akun Claude</button></p>
      {/each}
    </section>

    <section>
      <h4>Pindah ke akun ChatGPT <span class="muted">· riwayat percakapan diteruskan</span></h4>
      {#each codexOthers as a (a.id)}
        {@render choice(a)}
      {:else}
        <p class="muted small">Belum ada akun ChatGPT lain. <button class="link" onclick={() => addAccount("codex")}>+ Tambah akun ChatGPT</button></p>
      {/each}
    </section>
  </div>
</div>

<style>
  .veil {
    position: absolute;
    inset: 0;
    background: rgba(8, 8, 10, 0.62);
    display: grid;
    place-items: center;
    z-index: 5;
    padding: 16px;
  }
  .card {
    width: min(560px, 100%);
    max-height: 100%;
    overflow-y: auto;
    background: var(--bg-1);
    border: 1px solid rgba(251, 191, 36, 0.35);
    border-radius: 12px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6);
    padding: 14px 16px 16px;
  }
  .head {
    display: flex;
    gap: 10px;
    align-items: flex-start;
  }
  .icon {
    font-size: 20px;
    line-height: 1.2;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .msg {
    font-size: 12px;
    margin-top: 2px;
    overflow-wrap: anywhere;
  }
  section {
    margin-top: 14px;
  }
  h4 {
    margin: 0 0 6px;
    font-size: 12px;
    font-weight: 600;
  }
  h4 .muted {
    font-weight: 400;
  }
  .wide {
    width: 100%;
    text-align: left;
  }
  .quick {
    gap: 6px;
    margin-top: 6px;
    flex-wrap: wrap;
    align-items: center;
  }
  button.small {
    font-size: 11px;
    padding: 2px 8px;
  }
  .small {
    font-size: 12px;
  }
  .acc {
    width: 100%;
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 10px;
    align-items: baseline;
    text-align: left;
    margin-bottom: 5px;
    padding: 7px 10px;
  }
  .acc .name {
    font-weight: 600;
  }
  .acc .id {
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .acc .use {
    font-size: 11px;
    color: var(--fg-2);
  }
  .acc.hot .use {
    color: var(--warn);
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
  }
</style>
