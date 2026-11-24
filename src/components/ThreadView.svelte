<script lang="ts">
  import { api, type Mode, type Provider, type ThreadMeta } from "../lib/api";
  import { app, account, accountsOf, accountIdentity, isLimited, providerLabel, switchAgent, fail } from "../lib/state.svelte";
  import { fmtReset } from "../lib/format";
  import { fmtIn } from "../lib/limits";
  import AgentTerminal from "./AgentTerminal.svelte";
  import LimitDialog from "./LimitDialog.svelte";
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
  const statusText = $derived.by(() => {
    if (app.exited[thread.id] && thread.status !== "limited") return "tidak berjalan";
    switch (thread.status) {
      case "running":
        return "bekerja…";
      case "attention":
        return "butuh izin";
      case "limited":
        return "kena limit";
      case "ready":
        return "siap";
      default:
        return "memulai…";
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
    const extra = isLimited(a) ? " · ⏳ limit" : w ? ` · ${w.label} ${Math.round(w.usedPercent)}%` : "";
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
    <select class="acc" title={accountIdentity(acc) || "Akun yang menjalankan agent ini"} value={thread.accountId} onchange={pickAccount}>
      {#each ["claude", "codex"] as p (p)}
        <optgroup label={p === "claude" ? "Claude" : "ChatGPT / Codex"}>
          {#each accountsOf(p) as a (a.id)}
            <option value={a.id}>{accLabel(a)}</option>
          {/each}
        </optgroup>
      {/each}
      <option value="__add">+ Tambah akun…</option>
    </select>
    <select title="Model (bisa juga diganti dengan /model di terminal)" value={thread.model} onchange={pickModel}>
      <option value="">model bawaan</option>
      {#each models as m (m.id)}
        <option value={m.id}>{m.name || m.id}</option>
      {/each}
      {#if thread.model && !models.some((m) => m.id === thread.model)}
        <option value={thread.model}>{thread.model}</option>
      {/if}
    </select>
    <select title={modes.find((m) => m.id === thread.mode)?.hint} value={thread.mode} onchange={pickMode}>
      {#each modes as m (m.id)}
        <option value={m.id} title={m.hint}>{m.label}</option>
      {/each}
    </select>
    <span class="status s-{thread.status}" class:off={app.exited[thread.id] && thread.status !== "limited"}>
      <span class="dot"></span>{statusText}
    </span>
    <span class="spacer"></span>
    <span class="muted hint">Shift+Enter baris baru · {keyLabel("V")} tempel</span>
    <button class="ghost icon" title="Mulai ulang agent (sesi dilanjutkan)" onclick={restart}>↻</button>
    <button class="ghost icon" title="Hentikan agent" disabled={app.exited[thread.id]} onclick={stop}>⏹</button>
  </div>

  {#if thread.waitUntil}
    <div class="waitbar">
      <span>
        ⏳ Menunggu limit akun "{account(thread.limit?.accountId)?.label ?? acc?.label}" reset. Lanjut otomatis {fmtReset(thread.waitUntil)}
        ({fmtIn(Math.max(0, thread.waitUntil - now))} lagi).
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
  }
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    border-bottom: 1px solid var(--line);
    background: var(--bg-1);
    font-size: 12px;
  }
  .bar select {
    font-size: 12px;
    height: 24px;
    padding: 0 4px;
    max-width: 210px;
  }
  .bar select.acc {
    max-width: 300px;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-left: 4px;
    color: var(--fg-2);
  }
  .status .dot {
    background: var(--ok);
  }
  .status.s-running .dot {
    background: var(--accent);
    animation: pulse 1.1s ease-in-out infinite;
  }
  .status.s-running {
    color: var(--accent);
  }
  .status.s-attention,
  .status.s-limited {
    color: var(--warn);
  }
  .status.s-attention .dot,
  .status.s-limited .dot {
    background: var(--warn);
  }
  .status.s-off .dot,
  .status.off .dot {
    background: var(--muted);
  }
  .hint {
    font-size: 11px;
    margin-right: 4px;
  }
  .waitbar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    font-size: 12px;
    color: var(--warn);
    background: rgba(251, 191, 36, 0.07);
    border-bottom: 1px solid rgba(251, 191, 36, 0.25);
  }
  .waitbar span {
    flex: 1;
  }
  .waitbar button.small {
    font-size: 11px;
    padding: 1px 8px;
  }
  .body {
    flex: 1;
    min-height: 0;
    position: relative;
  }
</style>
