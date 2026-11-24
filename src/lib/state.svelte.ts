import { listen } from "@tauri-apps/api/event";
import {
  api,
  errMsg,
  type Account,
  type AgentEvent,
  type AppData,
  type Mode,
  type Project,
  type Provider,
  type SwitchReq,
  type ThreadMeta,
  type Tools,
} from "./api";
import { t as tr, applyLang } from "./i18n/index.svelte";
import { chime } from "./sound";

export interface TermTab {
  key: string;
  title: string;
  projectId: string | null;
  cwd: string | null;
  setup?: string;
  exited?: boolean;
}

export interface Toast {
  id: number;
  level: "info" | "warn" | "error" | "ok";
  message: string;
}

/** Thread views kept alive (their terminals keep scrollback while hidden). */
const MAX_MOUNTED = 12;

export const app = $state({
  ready: false,
  data: { projects: [], accounts: [], settings: {} as any, threads: [] } as AppData,
  tools: {} as Tools,
  dataDir: "",
  version: "",
  activeProjectId: null as string | null,
  activeThreadId: null as string | null,
  /** threads whose terminal view exists, most recent last */
  mounted: [] as string[],
  /** the agent of this thread ended in this session (no auto restart) */
  exited: {} as Record<string, boolean>,
  rightPanel: "none" as "none" | "editor" | "changes",
  terminalOpen: false,
  terminals: [] as TermTab[],
  activeTerminal: null as string | null,
  codexModels: [] as { id: string; name: string }[],
  toasts: [] as Toast[],
  accountsOpen: false,
  accountsTab: "claude" as "claude" | "codex" | "github" | "settings",
  /** bumps whenever an agent finishes a turn — panels refresh on it */
  fsTick: 0,
  /** Ctrl+P file picker */
  quickOpen: false,
  /** file the editor panel should open (set by quick open) */
  openFileReq: null as string | null,
});

let seq = 1;

export function toast(message: string, level: Toast["level"] = "info") {
  const id = seq++;
  app.toasts.push({ id, level, message });
  setTimeout(() => {
    const i = app.toasts.findIndex((t) => t.id === id);
    if (i >= 0) app.toasts.splice(i, 1);
  }, level === "error" ? 7000 : 4500);
}

export function fail(e: unknown) {
  toast(errMsg(e), "error");
}

// ------------------------------------------------------------ selectors

export function activeProject(): Project | undefined {
  return app.data.projects.find((p) => p.id === app.activeProjectId);
}

export function threadMeta(id: string | null | undefined): ThreadMeta | undefined {
  return id ? app.data.threads.find((t) => t.id === id) : undefined;
}

export function activeThread(): ThreadMeta | undefined {
  return threadMeta(app.activeThreadId);
}

export function accountsOf(provider: string): Account[] {
  return app.data.accounts.filter((a) => a.provider === provider);
}

export function account(id: string | null | undefined): Account | undefined {
  return app.data.accounts.find((a) => a.id === id);
}

/** "email · PLAN" — enough to never confuse two accounts. */
export function accountIdentity(a: Account | undefined): string {
  if (!a) return "";
  const parts = [a.email ?? a.displayName ?? (a.login ? `@${a.login}` : null), a.plan?.toUpperCase()];
  if (a.kind === "apikey") parts.push("API key");
  return parts.filter(Boolean).join(" · ");
}

export function isLimited(a: Account | undefined): boolean {
  return !!a?.limitedUntil && a.limitedUntil > Date.now();
}

export function providerLabel(p: string): string {
  return p === "codex" ? "ChatGPT" : "Claude";
}

export function projectThreads(projectId: string): ThreadMeta[] {
  return app.data.threads
    .filter((t) => t.projectId === projectId)
    .sort((a, b) => b.updatedAt - a.updatedAt);
}

export function busyCount(): number {
  return app.data.threads.filter((t) => t.status === "running").length;
}

// -------------------------------------------------------------- actions

export async function reloadData() {
  const b = await api.bootstrap();
  app.data = b.data;
  app.tools = b.tools;
  app.dataDir = b.dataDir;
  app.version = b.version;
}

export async function init() {
  await reloadData();
  applyLang(app.data.settings.language);
  try {
    app.codexModels = await api.codexModels();
  } catch {}
  await listen<AgentEvent>("agent", (e) => onAgentEvent(e.payload));
  await listen<string>("accounts-changed", async () => {
    const b = await api.bootstrap();
    app.data.accounts = b.data.accounts;
  });
  await listen<string>("account-error", (e) => toast(e.payload, "error"));
  const lastProject = localGet("kilat.project");
  const lastThread = localGet("kilat.thread");
  if (lastProject && app.data.projects.some((p) => p.id === lastProject)) {
    app.activeProjectId = lastProject;
  } else {
    app.activeProjectId = app.data.projects[0]?.id ?? null;
  }
  if (lastThread && app.data.threads.some((t) => t.id === lastThread)) {
    openThread(lastThread);
  }
  app.ready = true;
}

function localGet(k: string): string | null {
  try {
    return localStorage.getItem(k);
  } catch {
    return null;
  }
}

function localSet(k: string, v: string) {
  try {
    localStorage.setItem(k, v);
  } catch {}
}

export async function addProject(path: string) {
  const p = await api.addProject(path);
  if (!app.data.projects.some((x) => x.id === p.id)) app.data.projects.unshift(p);
  selectProject(p.id);
  return p;
}

export function selectProject(id: string) {
  app.activeProjectId = id;
  localSet("kilat.project", id);
}

export function openThread(id: string) {
  const t = threadMeta(id);
  if (!t) return;
  app.activeThreadId = id;
  app.activeProjectId = t.projectId;
  app.mounted = [...app.mounted.filter((x) => x !== id), id];
  // Drop the oldest hidden views (their agents keep running in the background).
  while (app.mounted.length > MAX_MOUNTED) app.mounted.shift();
  localSet("kilat.thread", id);
  localSet("kilat.project", t.projectId);
}

/** Defaults for a new thread: whatever the user used last. */
function lastChoice(): { provider: Provider; accountId: string; model: string; mode: Mode } {
  const t = [...app.data.threads].sort((a, b) => b.updatedAt - a.updatedAt)[0];
  if (t && account(t.accountId)) return { provider: t.provider, accountId: t.accountId, model: t.model, mode: t.mode };
  return { provider: "claude", accountId: "claude-default", model: "", mode: "full" };
}

export async function newThread(projectId: string, worktree = false, choice?: Partial<ReturnType<typeof lastChoice>>) {
  try {
    const meta = await api.createThread({ projectId, worktree, ...lastChoice(), ...choice });
    app.data.threads.unshift(meta);
    openThread(meta.id);
    return meta;
  } catch (e) {
    fail(e);
  }
}

export async function deleteThread(id: string) {
  await api.deleteThread(id);
  app.data.threads = app.data.threads.filter((t) => t.id !== id);
  app.mounted = app.mounted.filter((x) => x !== id);
  delete app.exited[id];
  if (app.activeThreadId === id) app.activeThreadId = null;
}

export async function setThreadFlags(id: string, flags: { pinned?: boolean; archived?: boolean }) {
  try {
    putMeta(await api.setThreadFlags(id, flags));
  } catch (e) {
    fail(e);
  }
}

/** Continue a thread with another account / provider / model / mode. */
export async function switchAgent(req: SwitchReq) {
  const t = threadMeta(req.threadId);
  if (t?.status === "running" && req.reason === "manual") {
    const ok = confirm(tr().state.confirmSwitch);
    if (!ok) return false;
  }
  try {
    await api.agentSwitch(req);
    const acc = account(req.accountId);
    const cross = t && t.provider !== req.provider;
    if (t && (t.accountId !== req.accountId || cross)) {
      toast(
        cross
          ? tr().state.movedProvider(providerLabel(req.provider), acc?.label ?? "")
          : tr().state.movedAccount(acc?.label ?? ""),
        "ok",
      );
    }
    return true;
  } catch (e) {
    fail(e);
    return false;
  }
}

// ---------------------------------------------------------------- files

const fileCache: Record<string, { tick: number; list: Promise<string[]> }> = {};

/** Project files (relative paths), refreshed after every agent turn. */
export function listFiles(root: string): Promise<string[]> {
  const hit = fileCache[root];
  if (hit && hit.tick === app.fsTick) return hit.list;
  const list = api.listFiles(root).catch(() => [] as string[]);
  fileCache[root] = { tick: app.fsTick, list };
  return list;
}

// ------------------------------------------------------------ terminals

export function openTerminal(opts: { projectId?: string | null; cwd?: string | null; setup?: string; title?: string }) {
  const key = `t${seq++}`;
  const project = app.data.projects.find((p) => p.id === opts.projectId);
  app.terminals.push({
    key,
    title: opts.title ?? (project ? project.name : "bash"),
    projectId: opts.projectId ?? null,
    cwd: opts.cwd ?? project?.path ?? null,
    setup: opts.setup,
  });
  app.activeTerminal = key;
  if (!opts.setup) app.terminalOpen = true;
  return key;
}

export function closeTerminal(key: string) {
  const i = app.terminals.findIndex((t) => t.key === key);
  if (i < 0) return;
  app.terminals.splice(i, 1);
  if (app.activeTerminal === key) app.activeTerminal = app.terminals.filter((t) => !t.setup).at(-1)?.key ?? null;
  if (!app.terminals.some((t) => !t.setup)) app.terminalOpen = false;
}

// -------------------------------------------------------- notifications

/**
 * Tell the user an agent needs them. In the background: Windows notification
 * + sound; in the app but on another thread: a toast.
 */
function alertUser(threadId: string, kind: "done" | "attention" | "error" | "limited", body: string) {
  const s = app.data.settings;
  const title = threadMeta(threadId)?.title ?? "Thread";
  const sound = kind === "limited" ? "error" : kind;
  if (!document.hasFocus()) {
    if (s.sound) chime(sound);
    if (s.notify) api.notify(`Kilat · ${title}`, body || tr().state.agentDone).catch(() => {});
  } else if (threadId !== app.activeThreadId) {
    const level = kind === "done" ? "ok" : kind === "error" ? "error" : "warn";
    toast(`${title}: ${body || tr().state.done}`, level);
  }
}

// --------------------------------------------------------------- events

function putMeta(meta: ThreadMeta) {
  const i = app.data.threads.findIndex((t) => t.id === meta.id);
  if (i >= 0) app.data.threads[i] = meta;
  else app.data.threads.unshift(meta);
}

function onAgentEvent(ev: AgentEvent) {
  switch (ev.type) {
    case "thread":
      putMeta(ev.thread);
      break;
    case "alert":
      if (ev.kind === "done") app.fsTick++;
      alertUser(ev.threadId, ev.kind, ev.message);
      break;
  }
}
