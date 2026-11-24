<script lang="ts">
  import { api, type Account, type ThreadMeta } from "../lib/api";
  import { app, account, accountsOf, accountIdentity, isLimited, providerLabel, switchAgent, fail } from "../lib/state.svelte";
  import { fmtReset } from "../lib/format";
  import { fmtIn, resetFor } from "../lib/limits";
  import Icon from "./Icon.svelte";
  import { t } from "../lib/i18n/index.svelte";

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
    if (isLimited(a)) return t().limit.limitedUntil(fmtReset(a.limitedUntil));
    const w = a.usage?.windows ?? [];
    return w.length ? w.slice(0, 2).map((x) => `${t().fmt.window(x.label)} ${Math.round(x.usedPercent)}%`).join(" · ") : "";
  }
</script>

{#snippet choice(a: Account)}
  <button
    class="acc"
    class:hot={isLimited(a)}
    disabled={!!busy || !a.ready}
    title={a.ready ? accountIdentity(a) : t().limit.notReady}
    onclick={() => moveTo(a)}
  >
    <span class="name">{busy === a.id ? t().limit.moving : a.label}</span>
    <span class="id">{a.ready ? accountIdentity(a) || "—" : t().limit.notSignedIn}</span>
    {#if usageText(a)}<span class="use">{usageText(a)}</span>{/if}
    <Icon name="chevronRight" size={13} />
  </button>
{/snippet}

<div class="veil">
  <div class="card" role="dialog" aria-label={t().limit.dialog}>
    <div class="head">
      <span class="ic"><Icon name="clock" size={16} /></span>
      <div class="grow">
        <b>{t().limit.title(providerLabel(limit.provider), limitedAcc?.label ?? limit.accountId)}</b>
        {#if limit.message}<div class="msg">{limit.message}</div>{/if}
      </div>
      <button class="ghost icon" title={t().limit.closeTitle} aria-label={t().common.close} onclick={dismiss}><Icon name="x" size={14} /></button>
    </div>

    <section>
      <h4><span class="step-n">1</span>{t().limit.wait}</h4>
      {#if reset}
        <button class="primary wide" disabled={!!busy} onclick={() => waitUntil(reset + 60_000)}>
          <Icon name="clock" size={13} />
          <span>{t().limit.waitUntil(fmtReset(reset))}</span>
          <span class="spacer"></span>
          <span class="mono">{t().limit.left(fmtIn(reset - now))}</span>
        </button>
      {:else}
        <p class="note">{t().limit.unknownReset}</p>
      {/if}
      <div class="row quick">
        {#if reset}<span class="note">{t().limit.orWait}</span>{/if}
        {#each [30, 60, 180, 300] as min (min)}
          <button class="small" disabled={!!busy} onclick={() => waitUntil(now + min * 60_000)}>
            {min < 60 ? t().limit.minutes(min) : t().limit.hours(min / 60)}
          </button>
        {/each}
      </div>
    </section>

    <section>
      <h4><span class="step-n">2</span>{t().limit.otherClaude} <span class="why">{t().limit.sameSession}</span></h4>
      {#each claudeOthers as a (a.id)}
        {@render choice(a)}
      {:else}
        <p class="note">{t().limit.noOtherClaude} <button class="link" onclick={() => addAccount("claude")}>{t().limit.addClaude}</button></p>
      {/each}
    </section>

    <section>
      <h4><span class="step-n">3</span>{t().limit.toChatgpt} <span class="why">{t().limit.historyHandedOver}</span></h4>
      {#each codexOthers as a (a.id)}
        {@render choice(a)}
      {:else}
        <p class="note">{t().limit.noOtherChatgpt} <button class="link" onclick={() => addAccount("codex")}>{t().limit.addChatgpt}</button></p>
      {/each}
    </section>
  </div>
</div>

<style>
  .veil {
    position: absolute;
    inset: 0;
    background: rgba(10, 10, 9, 0.66);
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
    border: 1px solid var(--line-2);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-pop);
  }
  .head {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 16px 12px 14px 18px;
  }
  .ic {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    flex: none;
    border-radius: 8px;
    background: var(--warn-soft);
    color: var(--warn);
  }
  .head b {
    font-size: 14.5px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .msg {
    font: 400 11.5px/1.5 var(--mono);
    color: var(--muted);
    margin-top: 4px;
    overflow-wrap: anywhere;
  }
  section {
    padding: 4px 18px 18px;
  }
  h4 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 10px;
    font-size: 12.5px;
    font-weight: 600;
  }
  .why {
    font-weight: 400;
    font-size: 12px;
    color: var(--muted);
  }
  .wide {
    width: 100%;
    min-height: 38px;
    border-radius: var(--radius);
    justify-content: flex-start;
    padding: 0 12px;
  }
  .wide .mono {
    font-size: 11.5px;
    font-weight: 500;
  }
  .quick {
    gap: 6px;
    margin-top: 8px;
    flex-wrap: wrap;
  }
  .note {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .acc {
    width: 100%;
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    gap: 12px;
    align-items: center;
    justify-content: stretch;
    text-align: left;
    min-height: 38px;
    margin-bottom: 6px;
    padding: 0 10px 0 12px;
    background: var(--bg-2);
    border-radius: var(--radius);
  }
  .acc:last-child {
    margin-bottom: 0;
  }
  .acc:hover:not(:disabled) {
    background: var(--bg-3);
  }
  .acc .name {
    font-weight: 600;
  }
  .acc .id {
    font-size: 11.5px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .acc .use {
    font: 400 11px/1 var(--mono);
    color: var(--fg-2);
  }
  .acc.hot .use {
    color: var(--warn);
  }
  .acc :global(svg) {
    color: var(--muted);
  }
</style>
