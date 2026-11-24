<script lang="ts">
  import { onMount } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { api, errMsg } from "../lib/api";
  import { app } from "../lib/state.svelte";
  import { isMac } from "../lib/platform";

  let {
    cwd = null,
    projectId = null,
    setup = undefined,
    active = true,
    onexit,
  }: {
    cwd?: string | null;
    projectId?: string | null;
    setup?: string;
    active?: boolean;
    /** `setup` is the command this terminal was started with (props are live) */
    onexit?: (code: number | null, setup?: string) => void;
  } = $props();

  let el: HTMLDivElement;
  let term: Terminal;
  let fit: FitAddon;
  let ptyId: string | null = null;

  function doFit() {
    if (!el || el.offsetParent === null) return; // hidden
    try {
      fit.fit();
      if (ptyId) api.ptyResize(ptyId, term.cols, term.rows).catch(() => {});
    } catch {}
  }

  onMount(() => {
    const spawned = setup;
    term = new Terminal({
      fontFamily: "'Cascadia Code', 'Cascadia Mono', Consolas, 'SF Mono', Menlo, 'DejaVu Sans Mono', 'Ubuntu Mono', monospace",
      fontSize: app.data.settings.fontSize || 13,
      cursorBlink: true,
      scrollback: 5000,
      allowProposedApi: false,
      theme: {
        background: "#0f0f0e",
        foreground: "#ecebe5",
        cursor: "#ffc83d",
        cursorAccent: "#0f0f0e",
        selectionBackground: "#4a4128",
        black: "#1d1d1b",
        brightBlack: "#6b6a63",
      },
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(el);
    // Ctrl+V / Ctrl+C (with selection) go to the browser so paste/copy work;
    // on macOS every Cmd shortcut does.
    term.attachCustomKeyEventHandler((e) => {
      if (e.type !== "keydown") return true;
      if (isMac) return !e.metaKey;
      if (e.ctrlKey && !e.shiftKey && e.key.toLowerCase() === "v") return false;
      if (e.ctrlKey && e.key.toLowerCase() === "c" && term.hasSelection()) return false;
      if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "c") return false;
      return true;
    });
    requestAnimationFrame(() => {
      doFit();
      api
        .ptySpawn({ cwd: cwd ?? undefined, projectId: projectId ?? undefined, cols: term.cols, rows: term.rows, setup: spawned }, (ev) => {
          if (ev.type === "data") term.write(ev.data);
          else {
            term.write(`\r\n\x1b[90m[proses selesai${ev.code != null ? `, kode ${ev.code}` : ""}]\x1b[0m\r\n`);
            ptyId = null;
            onexit?.(ev.code, spawned);
          }
        })
        .then((id) => {
          ptyId = id;
          doFit();
        })
        .catch((e) => term.write(`\x1b[31m${errMsg(e)}\x1b[0m\r\n`));
    });
    const sub = term.onData((data) => {
      if (ptyId) api.ptyWrite(ptyId, data).catch(() => {});
    });
    const ro = new ResizeObserver(() => doFit());
    ro.observe(el);
    return () => {
      ro.disconnect();
      sub.dispose();
      if (ptyId) api.ptyKill(ptyId).catch(() => {});
      term.dispose();
    };
  });

  $effect(() => {
    if (active) requestAnimationFrame(() => {
      doFit();
      term?.focus();
    });
  });
</script>

<div class="term" bind:this={el}></div>

<style>
  .term {
    width: 100%;
    height: 100%;
    padding: 6px 0 0 12px;
  }
  .term :global(.xterm) {
    height: 100%;
  }
  .term :global(.xterm-viewport) {
    background: transparent !important;
  }
</style>
