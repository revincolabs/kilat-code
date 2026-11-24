import { Channel, invoke } from "@tauri-apps/api/core";

export type Provider = "claude" | "codex";
/** full: no permission prompts · cli: the CLI's own setting · ask: asks before acting · plan: read-only */
export type Mode = "full" | "cli" | "ask" | "plan";

export interface Project {
  id: string;
  name: string;
  path: string;
  githubAccountId: string | null;
  createdAt: number;
}

export interface Account {
  id: string;
  provider: "claude" | "codex" | "github";
  kind: string;
  label: string;
  email: string | null;
  displayName: string | null;
  plan: string | null;
  login: string | null;
  gitName: string | null;
  gitEmail: string | null;
  sshKey: string | null;
  orgId: string | null;
  ready: boolean;
  limitedUntil: number | null;
  usage: AccountUsage | null;
  createdAt: number;
}

export interface UsageWindow {
  label: string;
  usedPercent: number;
  resetsAt: number | null;
}

export interface AccountUsage {
  updatedAt: number;
  windows: UsageWindow[];
}

export interface Settings {
  bashPath: string;
  claudePath: string;
  codexPath: string;
  ghPath: string;
  claudeModels: string[];
  fontSize: number;
  notify: boolean;
  sound: boolean;
  /** Claude agents get the Claude in Chrome tools */
  claudeChrome: boolean;
  /** typed into the agent when work continues after a limit */
  continuePrompt: string;
  /** UI language: "auto" (follow the system), "id" or "en" */
  language: string;
}

export interface LimitInfo {
  accountId: string;
  provider: Provider;
  at: number;
  resetsAt: number | null;
  message: string;
}

export type Status = "off" | "ready" | "running" | "attention" | "limited";

export interface ThreadMeta {
  id: string;
  projectId: string;
  title: string;
  provider: Provider;
  accountId: string;
  model: string;
  mode: Mode;
  cwd: string;
  worktreeBranch: string | null;
  status: Status;
  pinned: boolean;
  archived: boolean;
  limit: LimitInfo | null;
  waitUntil: number | null;
  createdAt: number;
  updatedAt: number;
}

export interface AppData {
  projects: Project[];
  accounts: Account[];
  settings: Settings;
  threads: ThreadMeta[];
}

export interface Tools {
  bash: string | null;
  claude: string | null;
  codex: string | null;
  gh: string | null;
}

export interface Bootstrap {
  data: AppData;
  tools: Tools;
  dataDir: string;
  version: string;
}

export type AgentEvent =
  | { type: "thread"; thread: ThreadMeta }
  | { type: "alert"; threadId: string; kind: "done" | "attention" | "error" | "limited"; message: string };

export type TermEvent = { type: "reset" } | { type: "data"; data: string } | { type: "exit"; code: number | null };

export type PtyEvent = { type: "data"; data: string } | { type: "exit"; code: number | null };

export interface GitStatus {
  isRepo: boolean;
  branch: string;
  ahead: number;
  behind: number;
  files: { path: string; status: string }[];
}

export interface Commit {
  hash: string;
  subject: string;
  when: string;
}

export interface Entry {
  name: string;
  path: string;
  isDir: boolean;
}

export interface SwitchReq {
  threadId: string;
  provider: Provider;
  accountId: string;
  model: string;
  mode: Mode;
  /** "limit": the previous account hit its limit, so the agent continues the work */
  reason: "manual" | "limit";
}

export const api = {
  bootstrap: () => invoke<Bootstrap>("bootstrap"),
  saveSettings: (settings: Settings) => invoke<Tools>("settings_save", { settings }),

  addProject: (path: string) => invoke<Project>("project_add", { path }),
  removeProject: (id: string) => invoke<void>("project_remove", { id }),
  setProjectGithub: (id: string, accountId: string | null) =>
    invoke<Project>("project_set_github", { id, accountId }),

  createThread: (req: {
    projectId: string;
    worktree?: boolean;
    provider?: Provider;
    accountId?: string;
    model?: string;
    mode?: Mode;
  }) => invoke<ThreadMeta>("thread_create", { req }),
  renameThread: (id: string, title: string) => invoke<void>("thread_rename", { id, title }),
  deleteThread: (id: string) => invoke<void>("thread_delete", { id }),
  setThreadFlags: (id: string, flags: { pinned?: boolean; archived?: boolean }) =>
    invoke<ThreadMeta>("thread_set_flags", { id, ...flags }),

  gitCommit: (projectId: string, cwd: string, message: string, paths: string[] = []) =>
    invoke<string>("git_commit", { projectId, cwd, message, paths }),
  gitPush: (projectId: string, cwd: string) => invoke<string>("git_push", { projectId, cwd }),
  gitBranch: (projectId: string, cwd: string, name: string) => invoke<void>("git_branch", { projectId, cwd, name }),
  gitPr: (projectId: string, cwd: string, title: string, body: string, draft: boolean) =>
    invoke<string>("git_pr", { projectId, cwd, title, body, draft }),
  gitSuggestMessage: (projectId: string, cwd: string, provider: Provider, accountId: string) =>
    invoke<string>("git_suggest_message", { projectId, cwd, provider, accountId }),
  gitRecent: (cwd: string) => invoke<Commit[]>("git_recent", { cwd }),
  listFiles: (root: string) => invoke<string[]>("fs_list_files", { root }),
  notify: (title: string, body: string) => invoke<void>("notify", { title, body }),

  /** Show a thread's agent terminal; `live`: an agent is already running in it. */
  agentAttach: (threadId: string, onEvent: (e: TermEvent) => void) => {
    const ch = new Channel<TermEvent>();
    ch.onmessage = onEvent;
    return invoke<{ viewId: number; live: boolean }>("agent_attach", { threadId, onEvent: ch });
  },
  agentDetach: (threadId: string, viewId: number) => invoke<void>("agent_detach", { threadId, viewId }),
  agentStart: (threadId: string, cols: number, rows: number) => invoke<void>("agent_start", { threadId, cols, rows }),
  agentWrite: (threadId: string, data: string) => invoke<void>("agent_write", { threadId, data }),
  agentResize: (threadId: string, cols: number, rows: number) => invoke<void>("agent_resize", { threadId, cols, rows }),
  agentStop: (threadId: string) => invoke<void>("agent_stop", { threadId }),
  agentSwitch: (req: SwitchReq) => invoke<void>("agent_switch", { req }),
  agentWait: (threadId: string, until: number) => invoke<void>("agent_wait", { threadId, until }),
  agentWaitCancel: (threadId: string) => invoke<void>("agent_wait_cancel", { threadId }),
  agentContinue: (threadId: string) => invoke<void>("agent_continue", { threadId }),
  agentDismissLimit: (threadId: string) => invoke<void>("agent_dismiss_limit", { threadId }),
  codexModels: () => invoke<{ id: string; name: string }[]>("codex_models"),

  addAccount: (provider: string, kind: string, label: string) =>
    invoke<Account>("account_add", { provider, kind, label }),
  updateAccount: (
    id: string,
    patch: { label?: string; gitName?: string; gitEmail?: string; sshKey?: string; clearLimit?: boolean },
  ) => invoke<Account>("account_update", { id, patch }),
  removeAccount: (id: string) => invoke<void>("account_remove", { id }),
  setSecret: (id: string, secret: string) => invoke<Account>("account_set_secret", { id, secret }),
  refreshAccount: (id: string) => invoke<Account>("account_refresh", { id }),
  /** after `claude auth login` in the setup terminal: read and verify the email */
  claudeIdentity: (id: string) =>
    invoke<{ account: Account; warning: string | null }>("account_claude_identity", { id }),
  githubImport: () => invoke<Account[]>("github_import"),

  ptySpawn: (
    opts: { cwd?: string; projectId?: string; cols: number; rows: number; setup?: string },
    onEvent: (e: PtyEvent) => void,
  ) => {
    const ch = new Channel<PtyEvent>();
    ch.onmessage = onEvent;
    return invoke<string>("pty_spawn", { opts, onEvent: ch });
  },
  ptyWrite: (id: string, data: string) => invoke<void>("pty_write", { id, data }),
  ptyResize: (id: string, cols: number, rows: number) => invoke<void>("pty_resize", { id, cols, rows }),
  ptyKill: (id: string) => invoke<void>("pty_kill", { id }),

  listDir: (path: string) => invoke<Entry[]>("fs_list_dir", { path }),
  readFile: (path: string) => invoke<string>("fs_read_file", { path }),
  writeFile: (path: string, content: string) => invoke<void>("fs_write_file", { path, content }),
  gitStatus: (cwd: string) => invoke<GitStatus>("git_status", { cwd }),
  gitDiff: (cwd: string, path: string) => invoke<string>("git_diff", { cwd, path }),
};

export function errMsg(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);
}
