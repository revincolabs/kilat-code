<script lang="ts">
  import { api, type Mode, type Provider, type ThreadMeta } from "../lib/api";
  import { app, account, accountsOf, accountIdentity, isLimited, providerLabel, switchAgent, fail } from "../lib/state.svelte";
  import { fmtReset } from "../lib/format";
  import { fmtIn } from "../lib/limits";
  import AgentTerminal from "./AgentTerminal.svelte";
  import LimitDialog from "./LimitDialog.svelte";
  import Icon from "./Icon.svelte";
  import { keyLabel } from "../lib/platform";

  let { thread, active }: { thread: ThreadMeta; active: boolean } = $props();

  const modes: { id: Mode; label: string; hint: string }[] = [
    { id: "full", label: "Akses penuh", hint: "Tanpa minta izin (Claude: --dangerously-skip-permissions, Codex: tanpa sandbox)" },
    { id: "cli", label: "Setelan CLI", hint: "Ikuti setelan izin milik CLI-nya sendiri (mis. mode auto di Claude)" },
    { id: "ask", label: "Minta izin", hint: "Agent minta persetujuan sebelum mengubah file / menjalankan perintah" },
    { id: "plan", label: "Plan / read-only", hint: "Hanya membaca dan menyusun rencana" },
  ];

  const acc = $derived(account(thread.accountId));
  const models = $derived(
    thread.provider === "claude"
      ? (app.data.settings.claudeModels ?? []).map((m) => ({ id: m, name: m }))
      : app.codexModels,
  );
  const off = $derived(!!app.exited[thread.id] && thread.status !== "limited");
  const statusText = $derived.by(() => {
    if (off) return "tidak berjalan";
    switch (thread.status) {
      case "running":
        return "bekerja";
      case "attention":
        return "butuh izin";
      case "limited":
        return "kena limit";
      case "ready":
        return "siap";
      default:
        return "memulai";
    }
  });

  let now = $state(Date.now());
  $effect(() => {
    if (!thread.waitUntil) return;
    const iv = setInterval(() => (now = Date.now()), 15_000);
    return () => clearInterval(iv);
  });

  function accLabel(a: ReturnType<typeof account>) {
    if (!a) return "";
    const w = a.usage?.windows?.[0];
    const extra = isLimited(a) ? " · limit" : w ? ` · ${w.label} ${Math.round(w.usedPercent)}%` : "";
    return `${a.label}${a.plan ? ` (${a.plan.toUpperCase()})` : ""}${a.ready ? "" : " · belum login"}${extra}`;
  }

  async function pickAccount(e: Event) {
    const sel = e.target as HTMLSelectElement;
    const id = sel.value;
    if (id === "__add") {
      sel.value = thread.accountId;
      app.accountsTab = thread.provider;
      app.accountsOpen = true;
      return;
    }
    const a = account(id);
    if (!a || id === thread.accountId) return;
    const provider = a.provider as Provider;
    const ok = await switchAgent({
      threadId: thread.id,
      provider,
      accountId: id,
      model: provider === thread.provider ? thread.model : "",
      mode: thread.mode,
      reason: "manual",
    });
    if (!ok) sel.value = thread.accountId;
  }

  async function pickModel(e: Event) {
    const sel = e.target as HTMLSelectElement;
    const ok = await switchAgent({ ...base(), model: sel.value, reason: "manual" });
    if (!ok) sel.value = thread.model;
  }

  async function pickMode(e: Event) {
    const sel = e.target as HTMLSelectElement;
    const ok = await switchAgent({ ...base(), mode: sel.value as Mode, reason: "manual" });
    if (!ok) sel.value = thread.mode;
  }

  function base() {
    return {
      threadId: thread.id,
      provider: thread.provider,
      accountId: thread.accountId,
      model: thread.model,
      mode: thread.mode,
    };
  }

  function restart() {
    switchAgent({ ...base(), reason: "manual" });
  }

  function stop() {
    api.agentStop(thread.id).catch(fail);
  }

  function continueNow() {
    api.agentContinue(thread.id).catch(fail);
  }

  function cancelWait() {
    api.agentWaitCancel(thread.id).catch(fail);
  }
</script>

<div class="tv">
  <div class="bar">
    <span class="badge {thread.provider}">{providerLabel(thread.provider)}</span>
    <select class="acc" aria-label="Akun" title={accountIdentity(acc) || "Akun yang menjalankan agent ini"} value={thread.accountId} onchange={pickAccount}>
      {#each ["claude", "codex"] as p (p)}
        <optgroup label={p === "claude" ? "Claude" : "ChatGPT / Codex"}>
          {#each accountsOf(p) as a (a.id)}
            <option value={a.id}>{accLabel(a)}</option>
          {/each}
        </optgroup>
      {/each}
      <option value="__add">+ Tambah akun…</option>
    </select>
    <select aria-label="Model" title="Model (bisa juga diganti dengan /model di terminal)" value={thread.model} onchange={pickModel}>
      <option value="">model bawaan</option>
      {#each models as m (m.id)}
        <option value={m.id}>{m.name || m.id}</option>
      {/each}
      {#if thread.model && !models.some((m) => m.id === thread.model)}
        <option value={thread.model}>{thread.model}</option>
      {/if}
    </select>
    <select aria-label="Mode izin" title={modes.find((m) => m.id === thread.mode)?.hint} value={thread.mode} onchange={pickMode}>
      {#each modes as m (m.id)}
        <option value={m.id} title={m.hint}>{m.label}</option>
      {/each}
    </select>
    <span class="status s-{thread.status}" class:off>
      <span class="dot"></span>{statusText}
    </span>
    <span class="spacer"></span>
    <span class="hint">Shift+Enter baris baru · {keyLabel("V")} tempel</span>
    <button class="ghost icon" title="Mulai ulang agent (sesi dilanjutkan)" aria-label="Mulai ulang agent" onclick={restart}><Icon name="refresh" size={14} /></button>
    <button class="ghost icon" title="Hentikan agent" aria-label="Hentikan agent" disabled={app.exited[thread.id]} onclick={stop}><Icon name="stop" size={13} /></button>
  </div>

  {#if thread.waitUntil}
    <div class="waitbar">
      <Icon name="clock" size={14} />
      <span class="grow">
        Menunggu limit akun "{account(thread.limit?.accountId)?.label ?? acc?.label}" reset. Lanjut otomatis {fmtReset(thread.waitUntil)}
        <span class="mono">({fmtIn(Math.max(0, thread.waitUntil - now))} lagi)</span>
      </span>
      <button class="small" onclick={continueNow}>Lanjut sekarang</button>
      <button class="small ghost" onclick={cancelWait}>Batal</button>
    </div>
  {/if}

  <div class="body">
    <AgentTerminal {thread} {active} />
    {#if thread.limit && !thread.waitUntil}
      <LimitDialog {thread} />
    {/if}
  </div>
</div>

<style>
  .tv {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    /* the bar adapts to the terminal's width, which shrinks when a side panel opens */
    container-type: inline-size;
  }
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 38px;
    padding: 0 8px 0 12px;
    border-bottom: 1px solid var(--line);
    font-size: 12px;
    overflow: hidden;
  }
  .bar .badge {
    margin-right: 2px;
  }
  .bar select {
    font-size: 12px;
    min-width: 0;
    flex: 0 1 auto;
    max-width: 200px;
    background-color: transparent;
    border-color: var(--line);
  }
  .bar select:hover {
    background-color: var(--bg-2);
  }
  .bar select.acc {
    max-width: 300px;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-left: 6px;
    flex: none;
    white-space: nowrap;
    font: 500 11px/1 var(--mono);
    color: var(--fg-2);
  }
  .status .dot {
    background: var(--ok);
  }
  .status.s-running {
    color: var(--accent);
  }
  .status.s-running .dot {
    background: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .status.s-attention,
  .status.s-limited {
    color: var(--warn);
  }
  .status.s-attention .dot,
  .status.s-limited .dot {
    background: var(--warn);
  }
  .status.s-off,
  .status.off {
    color: var(--muted);
  }
  .status.s-off .dot,
  .status.off .dot {
    background: var(--line-3);
    animation: none;
  }
  .hint {
    font: 400 11px/1 var(--mono);
    color: var(--muted);
    margin-right: 6px;
    white-space: nowrap;
  }
  .waitbar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px 7px 12px;
    font-size: 12px;
    color: var(--warn);
    background: var(--warn-soft);
    border-bottom: 1px solid var(--line);
    box-shadow: inset 2px 0 0 var(--warn);
  }
  .waitbar .grow {
    flex: 1;
    color: var(--fg);
  }
  .waitbar .mono {
    color: var(--warn);
    font-size: 11.5px;
  }
  .body {
    flex: 1;
    min-height: 0;
    position: relative;
  }
  @container (max-width: 980px) {
    .hint {
      display: none;
    }
  }
  @container (max-width: 640px) {
    .bar .badge {
      display: none;
    }
  }
</style>
