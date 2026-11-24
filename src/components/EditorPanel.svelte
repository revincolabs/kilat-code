<script lang="ts">
  import { onMount } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState, Compartment, type Extension } from "@codemirror/state";
  import { keymap } from "@codemirror/view";
  import { indentWithTab } from "@codemirror/commands";
  import { oneDark } from "@codemirror/theme-one-dark";
  import { api } from "../lib/api";
  import { app, fail, toast } from "../lib/state.svelte";
  import FileTree from "./FileTree.svelte";
  import Icon from "./Icon.svelte";
  import { t as tr } from "../lib/i18n/index.svelte";
  import { keyLabel } from "../lib/platform";

  let { root }: { root: string } = $props();

  interface Tab {
    path: string;
    name: string;
    state: EditorState;
    saved: string;
    dirty: boolean;
  }

  let tabs = $state<Tab[]>([]);
  let current = $state<string | null>(null);
  let host: HTMLDivElement;
  let view: EditorView;
  const lang = new Compartment();

  async function languageFor(path: string): Promise<Extension> {
    const ext = path.split(".").pop()?.toLowerCase() ?? "";
    switch (ext) {
      case "js": case "mjs": case "cjs": case "jsx":
        return (await import("@codemirror/lang-javascript")).javascript({ jsx: true });
      case "ts": case "mts": case "cts": case "tsx":
        return (await import("@codemirror/lang-javascript")).javascript({ jsx: true, typescript: true });
      case "svelte": case "vue": case "html": case "htm":
        return (await import("@codemirror/lang-html")).html();
      case "css": case "scss": case "less":
        return (await import("@codemirror/lang-css")).css();
      case "json": case "jsonc":
        return (await import("@codemirror/lang-json")).json();
      case "md": case "markdown":
        return (await import("@codemirror/lang-markdown")).markdown();
      case "py":
        return (await import("@codemirror/lang-python")).python();
      case "rs":
        return (await import("@codemirror/lang-rust")).rust();
      default:
        return [];
    }
  }

  const theme = EditorView.theme({
    "&": { height: "100%", fontSize: `${app.data.settings.fontSize || 13}px`, backgroundColor: "#0f0f0e" },
    ".cm-scroller": { fontFamily: "var(--mono)" },
    ".cm-gutters": { backgroundColor: "#0f0f0e", borderRight: "1px solid #262624", color: "#5f5d57" },
    ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "#171715" },
    "&.cm-focused .cm-cursor": { borderLeftColor: "#ffc83d" },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": { backgroundColor: "#3d3622 !important" },
  });

  function baseExtensions(path: string, langExt: Extension): Extension[] {
    return [
      basicSetup,
      keymap.of([indentWithTab, { key: "Mod-s", run: () => (save(path), true) }]),
      oneDark,
      theme,
      lang.of(langExt),
      EditorView.updateListener.of((u) => {
        if (!u.docChanged) return;
        const t = tabs.find((x) => x.path === path);
        if (t) t.dirty = u.state.doc.toString() !== t.saved;
      }),
    ];
  }

  async function openFile(path: string) {
    const existing = tabs.find((t) => t.path === path);
    if (existing) return activate(path);
    try {
      const text = await api.readFile(path);
      const state = EditorState.create({ doc: text, extensions: baseExtensions(path, await languageFor(path)) });
      tabs.push({ path, name: path.split(/[\\/]/).pop() ?? path, state, saved: text, dirty: false });
      activate(path);
    } catch (e) {
      fail(e);
    }
  }

  function stash() {
    const t = tabs.find((x) => x.path === current);
    if (t && view) t.state = view.state;
  }

  function activate(path: string) {
    stash();
    current = path;
    const t = tabs.find((x) => x.path === path);
    if (t && view) {
      view.setState(t.state);
      view.focus();
    }
  }

  function close(path: string) {
    const t = tabs.find((x) => x.path === path);
    if (t?.dirty && !confirm(tr().editor.confirmClose(t.name))) return;
    const i = tabs.findIndex((x) => x.path === path);
    tabs.splice(i, 1);
    if (current === path) {
      current = null;
      const next = tabs[Math.max(0, i - 1)];
      if (next) {
        current = next.path;
        view.setState(next.state);
      } else view.setState(EditorState.create({ doc: "" }));
    }
  }

  async function save(path: string) {
    const t = tabs.find((x) => x.path === path);
    if (!t) return;
    const text = path === current ? view.state.doc.toString() : t.state.doc.toString();
    try {
      await api.writeFile(path, text);
      t.saved = text;
      t.dirty = false;
      toast(tr().editor.saved(t.name), "ok");
    } catch (e) {
      fail(e);
    }
  }

  // Reload clean tabs after an agent turn (files may have changed).
  let lastTick = app.fsTick;
  $effect(() => {
    const tick = app.fsTick;
    if (tick === lastTick) return;
    lastTick = tick;
    for (const t of tabs) {
      if (t.dirty) continue;
      api.readFile(t.path).then(async (text) => {
        if (text === t.saved) return;
        t.saved = text;
        t.state = EditorState.create({ doc: text, extensions: baseExtensions(t.path, await languageFor(t.path)) });
        if (t.path === current) view.setState(t.state);
      }).catch(() => {});
    }
  });

  onMount(() => {
    view = new EditorView({ parent: host, state: EditorState.create({ doc: "" }) });
    return () => view.destroy();
  });

  // Files requested from elsewhere (Ctrl+P quick open).
  $effect(() => {
    const req = app.openFileReq;
    if (!req || !view) return;
    app.openFileReq = null;
    openFile(req);
  });
</script>

<div class="editor">
  <div class="tree">
    <div class="tree-head" title={root}><span class="label">{root.split(/[\\/]/).pop()}</span></div>
    <div class="tree-body">
      <FileTree path={root} onopen={openFile} depth={0} />
    </div>
  </div>
  <div class="main">
    <div class="tabs">
      {#each tabs as t (t.path)}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="tab" class:sel={t.path === current} onclick={() => activate(t.path)} title={t.path}>
          <span>{t.name}</span>
          {#if t.dirty}<span class="dirty" title={tr().editor.unsaved}></span>{/if}
          <button class="ghost icon x" aria-label={tr().common.close} onclick={(e) => { e.stopPropagation(); close(t.path); }}><Icon name="x" size={12} /></button>
        </div>
      {/each}
    </div>
    <div class="cm" class:empty={!current} bind:this={host}></div>
    {#if !current}
      <div class="placeholder"><p>{@html tr().editor.placeholder(keyLabel("S"), keyLabel("P"))}</p></div>
    {/if}
  </div>
</div>

<style>
  .editor {
    display: flex;
    height: 100%;
    min-height: 0;
  }
  .tree {
    width: 200px;
    flex: none;
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .tree-head {
    height: 34px;
    flex: none;
    display: flex;
    align-items: center;
    padding: 0 12px;
    overflow: hidden;
    white-space: nowrap;
  }
  .tree-head .label {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tree-body {
    flex: 1;
    overflow: auto;
    padding-bottom: 8px;
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    position: relative;
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    overflow-x: auto;
    border-bottom: 1px solid var(--line);
    flex: none;
    height: 34px;
    padding: 0 5px;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 25px;
    padding: 0 2px 0 10px;
    font-size: 12px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    color: var(--muted);
    white-space: nowrap;
  }
  .tab:hover {
    color: var(--fg);
  }
  .tab.sel {
    background: var(--bg-3);
    color: var(--fg);
  }
  .dirty {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }
  .x {
    width: 20px;
    min-width: 20px;
    min-height: 20px;
  }
  .cm {
    flex: 1;
    min-height: 0;
  }
  .cm.empty {
    visibility: hidden;
  }
  .placeholder {
    position: absolute;
    inset: 34px 0 0;
    display: grid;
    place-items: center;
    padding: 0 24px;
    text-align: center;
    font-size: 12px;
    line-height: 1.7;
    color: var(--muted);
  }
</style>
