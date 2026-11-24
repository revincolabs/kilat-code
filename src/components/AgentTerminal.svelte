<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { api, errMsg, type TermEvent, type ThreadMeta } from "../lib/api";
  import { app, providerLabel } from "../lib/state.svelte";
  import { isMac } from "../lib/platform";

  /**
   * The real `claude` / `codex` terminal UI of one thread. The process lives
   * in the backend: this view only attaches to it, so hiding or closing the
   * view never stops the agent.
   */
  let { thread, active }: { thread: ThreadMeta; active: boolean } = $props();

  // A view belongs to one thread for its whole life (keyed by id).
  const tid = untrack(() => thread.id);
  let el: HTMLDivElement;
  let term: Terminal;
  let fit: FitAddon;
  let viewId: number | null = null;
  let starting = $state(false);
  const ended = $derived(!!app.exited[tid]);

  function doFit() {
    if (!el || el.offsetParent === null) return; // hidden
    try {
      fit.fit();
      api.agentResize(tid, term.cols, term.rows).catch(() => {});
    } catch {}
  }

  function onEvent(ev: TermEvent) {
    if (ev.type === "reset") {
      term.reset();
      app.exited[tid] = false;
    } else if (ev.type === "data") {
      term.write(ev.data);
    } else {
      term.write(`\r\n\x1b[90m[agent berhenti${ev.code != null ? `, kode ${ev.code}` : ""}]\x1b[0m\r\n`);
      app.exited[tid] = true;
    }
  }

  async function start() {
    if (starting) return;
    starting = true;
    try {
      doFit();
      await api.agentStart(tid, term.cols, term.rows);
    } catch (e) {
      term.write(`\r\n\x1b[31m${errMsg(e)}\x1b[0m\r\n`);
      app.exited[tid] = true;
    } finally {
      starting = false;
    }
  }

  onMount(() => {
    term = new Terminal({
      fontFamily: "'Cascadia Code', 'Cascadia Mono', Consolas, 'SF Mono', Menlo, 'DejaVu Sans Mono', 'Ubuntu Mono', monospace",
      fontSize: app.data.settings.fontSize || 13,
      cursorBlink: true,
      scrollback: 10000,
      allowProposedApi: false,
      theme: {
        background: "#0c0c0e",
        foreground: "#e7e7ea",
        cursor: "#ffc83d",
        selectionBackground: "#3a3a48",
        black: "#1c1c22",
        brightBlack: "#5c5c68",
      },
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(el);
    term.attachCustomKeyEventHandler((e) => {
      if (e.type !== "keydown") return true;
      const k = e.key.toLowerCase();
      // macOS: every Cmd shortcut (copy, paste, Kilat's own) is the app's;
      // Ctrl keeps its terminal meaning (Ctrl+C interrupts the agent).
      if (isMac && e.metaKey) return false;
      if (!isMac) {
        // Paste / copy go through the browser.
        if (e.ctrlKey && !e.shiftKey && k === "v") return false;
        if (e.ctrlKey && k === "c" && term.hasSelection()) return false;
        if (e.ctrlKey && e.shiftKey && k === "c") return false;
        // Kilat shortcuts.
        if (e.ctrlKey && !e.shiftKey && !e.altKey && (k === "`" || k === "p")) return false;
      }
      // Shift+Enter: new line in the prompt (both CLIs read Enter alone as "send").
      if (e.key === "Enter" && e.shiftKey && !e.ctrlKey && !e.altKey) {
        e.preventDefault();
        api.agentWrite(tid, thread.provider === "codex" ? "\n" : "\x1b\r").catch(() => {});
        return false;
      }
      return true;
    });
    const sub = term.onData((d) => {
      api.agentWrite(tid, d).catch(() => {});
    });
    requestAnimationFrame(async () => {
      doFit();
      try {
        const r = await api.agentAttach(tid, onEvent);
        viewId = r.viewId;
        if (!r.live && !app.exited[tid]) await start();
        else doFit();
      } catch (e) {
        term.write(`\x1b[31m${errMsg(e)}\x1b[0m\r\n`);
      }
    });
    const ro = new ResizeObserver(() => doFit());
    ro.observe(el);
    return () => {
      ro.disconnect();
      sub.dispose();
      if (viewId != null) api.agentDetach(tid, viewId).catch(() => {});
      term.dispose();
    };
  });

  $effect(() => {
    if (active)
      requestAnimationFrame(() => {
        doFit();
        term?.focus();
      });
  });
</script>

<div class="wrap">
  <div class="term" bind:this={el}></div>
  {#if ended}
    <div class="ended">
      <span>Agent {providerLabel(thread.provider)} tidak berjalan.</span>
      <button class="primary" disabled={starting} onclick={start}>{starting ? "Memulai…" : "▶ Jalankan lagi"}</button>
      <span class="muted small">Sesi sebelumnya dilanjutkan.</span>
    </div>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
    width: 100%;
    height: 100%;
  }
  .term {
    width: 100%;
    height: 100%;
    padding: 6px 0 0 10px;
  }
  .term :global(.xterm) {
    height: 100%;
  }
  .term :global(.xterm-viewport) {
    background: transparent !important;
  }
  .ended {
    position: absolute;
    left: 50%;
    bottom: 18px;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px 8px 14px;
    background: var(--bg-2);
    border: 1px solid var(--line-2);
    border-radius: 10px;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.5);
    font-size: 12px;
    white-space: nowrap;
  }
  .small {
    font-size: 11px;
  }
</style>
