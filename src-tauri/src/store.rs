//! Persistent app state: projects, accounts (metadata only — secrets live in
//! the OS keychain), settings, and threads.
//!
//! A thread is one agent conversation shown as the real `claude` / `codex`
//! terminal UI. It remembers the provider-native session of each provider
//! and which account ran when (segments), so switching account or provider
//! never loses context: the same provider resumes its own session, the other
//! provider gets the history it has not seen (see agents/transcript.rs).

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};

use crate::util::{self, now_ms};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    /// GitHub account used by terminals and agents in this project.
    pub github_account_id: Option<String>,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Account {
    pub id: String,
    /// "claude" | "codex" | "github"
    pub provider: String,
    /// claude: "default" | "oauth" | "apikey"
    /// codex:  "default" | "chatgpt" | "apikey"
    /// github: "gh"
    pub kind: String,
    pub label: String,
    pub email: Option<String>,
    /// Person / org name reported by the provider (to tell accounts apart).
    pub display_name: Option<String>,
    /// Subscription plan reported by the provider (pro, max, plus, go, …).
    pub plan: Option<String>,
    pub login: Option<String>,
    pub git_name: Option<String>,
    pub git_email: Option<String>,
    pub ssh_key: Option<String>,
    /// Claude: organization behind the stored token (a personal Pro/Max plan
    /// is its own organization). Ties the detected email to the token.
    pub org_id: Option<String>,
    pub ready: bool,
    pub limited_until: Option<i64>,
    /// Plan usage last reported by the CLI (5-hour / weekly windows).
    pub usage: Option<AccountUsage>,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct UsageWindow {
    /// "5 jam", "7 hari", …
    pub label: String,
    pub used_percent: f64,
    pub resets_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountUsage {
    pub updated_at: i64,
    pub windows: Vec<UsageWindow>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub bash_path: String,
    pub claude_path: String,
    pub codex_path: String,
    pub gh_path: String,
    pub claude_models: Vec<String>,
    pub font_size: u32,
    /// Windows notification when an agent finishes / needs you (app in background).
    pub notify: bool,
    pub sound: bool,
    /// Give Claude agents the Claude in Chrome tools (`claude --chrome`).
    pub claude_chrome: bool,
    /// Typed into the agent when work resumes after a limit.
    pub continue_prompt: String,
}

pub const DEFAULT_CONTINUE: &str = "Lanjutkan pekerjaan yang tadi terhenti karena limit akun.";

impl Default for Settings {
    fn default() -> Self {
        Self {
            bash_path: String::new(),
            claude_path: String::new(),
            codex_path: String::new(),
            gh_path: String::new(),
            claude_models: vec![
                "opus".into(),
                "sonnet".into(),
                "haiku".into(),
                "claude-fable-5-2".into(),
            ],
            font_size: 13,
            notify: true,
            sound: true,
            claude_chrome: true,
            continue_prompt: DEFAULT_CONTINUE.into(),
        }
    }
}

/// An account of this thread ran out of quota. The UI offers: wait for the
/// reset, or continue with another Claude / ChatGPT account.
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct LimitInfo {
    pub account_id: String,
    pub provider: String,
    pub at: i64,
    /// Reset time when the CLI reported one.
    pub resets_at: Option<i64>,
    /// What the CLI said (the UI can read a reset time out of it).
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ThreadMeta {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub provider: String,
    pub account_id: String,
    pub model: String,
    /// "full" | "ask" | "plan"
    pub mode: String,
    /// Working directory (project path, or a dedicated git worktree).
    pub cwd: String,
    pub worktree_branch: Option<String>,
    /// "off" (no process) | "ready" (waiting for you) | "running" |
    /// "attention" (asks for permission) | "limited"
    pub status: String,
    pub pinned: bool,
    pub archived: bool,
    pub limit: Option<LimitInfo>,
    /// Continue automatically at this time (waiting for a limit reset).
    pub wait_until: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A provider-native session (Claude session id / Codex thread id).
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct NativeSession {
    pub id: String,
    /// Claude: the session's transcript file (reported by the SessionStart hook).
    pub transcript: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct NativeSessions {
    pub claude: Option<NativeSession>,
    pub codex: Option<NativeSession>,
}

impl NativeSessions {
    pub fn get(&self, provider: &str) -> Option<&NativeSession> {
        match provider {
            "claude" => self.claude.as_ref(),
            "codex" => self.codex.as_ref(),
            _ => None,
        }
    }
    pub fn slot(&mut self, provider: &str) -> &mut Option<NativeSession> {
        if provider == "codex" {
            &mut self.codex
        } else {
            &mut self.claude
        }
    }
}

/// A stretch of the conversation run by one account in one native session.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Segment {
    pub provider: String,
    pub account_id: String,
    /// Empty until known (Codex reports its id once the session starts).
    pub native_id: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Thread {
    #[serde(flatten)]
    pub meta: ThreadMeta,
    pub native: NativeSessions,
    pub segments: Vec<Segment>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct AppData {
    pub projects: Vec<Project>,
    pub accounts: Vec<Account>,
    pub settings: Settings,
    pub threads: Vec<ThreadMeta>,
}

pub const CLAUDE_DEFAULT: &str = "claude-default";
pub const CODEX_DEFAULT: &str = "codex-default";

impl AppData {
    fn ensure_defaults(&mut self) {
        let defaults = [
            (CLAUDE_DEFAULT, "claude", "Claude (login bawaan)"),
            (CODEX_DEFAULT, "codex", "ChatGPT (login bawaan)"),
        ];
        for (i, (id, provider, label)) in defaults.into_iter().enumerate() {
            if !self.accounts.iter().any(|a| a.id == id) {
                self.accounts.insert(
                    i.min(self.accounts.len()),
                    Account {
                        id: id.into(),
                        provider: provider.into(),
                        kind: "default".into(),
                        label: label.into(),
                        ready: true,
                        created_at: now_ms(),
                        ..Default::default()
                    },
                );
            }
        }
    }
}

/// No agent process survives a restart: everything is "off", except threads
/// still waiting for a limit reset.
fn status_at_start(t: &ThreadMeta) -> &'static str {
    if t.limit.is_some() || t.wait_until.is_some() {
        "limited"
    } else {
        "off"
    }
}

/// Resolved tool locations.
#[derive(Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Tools {
    pub bash: Option<PathBuf>,
    pub claude: Option<PathBuf>,
    pub codex: Option<PathBuf>,
    pub gh: Option<PathBuf>,
}

pub struct Store {
    pub dir: PathBuf,
    pub data: RwLock<AppData>,
    pub tools: RwLock<Tools>,
    threads: Mutex<HashMap<String, Thread>>,
    last_flush: Mutex<HashMap<String, Instant>>,
}

impl Store {
    pub fn load(dir: PathBuf) -> anyhow::Result<Self> {
        fs::create_dir_all(dir.join("threads"))?;
        let mut data: AppData = fs::read_to_string(dir.join("state.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        data.ensure_defaults();
        for t in &mut data.threads {
            t.status = status_at_start(t).into();
            if t.mode.is_empty() || t.mode == "supervised" {
                t.mode = if t.mode.is_empty() { "full" } else { "ask" }.into();
            }
        }
        if data.settings.continue_prompt.trim().is_empty() {
            data.settings.continue_prompt = DEFAULT_CONTINUE.into();
        }
        let store = Self {
            dir,
            data: RwLock::new(data),
            tools: RwLock::new(Tools::default()),
            threads: Mutex::new(HashMap::new()),
            last_flush: Mutex::new(HashMap::new()),
        };
        store.refresh_tools();
        store.save();
        Ok(store)
    }

    pub fn refresh_tools(&self) {
        let s = self.data.read().settings.clone();
        *self.tools.write() = Tools {
            bash: util::resolve_bash(Some(&s.bash_path)),
            claude: util::resolve_claude(Some(&s.claude_path)),
            codex: util::resolve_codex(Some(&s.codex_path)),
            gh: util::resolve_gh(Some(&s.gh_path)),
        };
    }

    pub fn tools(&self) -> Tools {
        self.tools.read().clone()
    }

    pub fn save(&self) {
        let json = serde_json::to_vec_pretty(&*self.data.read()).unwrap_or_default();
        if let Err(e) = util::write_atomic(&self.dir.join("state.json"), &json) {
            eprintln!("[kilat] failed to save state: {e}");
        }
    }

    pub fn snapshot(&self) -> AppData {
        self.data.read().clone()
    }

    pub fn project(&self, id: &str) -> Option<Project> {
        self.data.read().projects.iter().find(|p| p.id == id).cloned()
    }

    pub fn account(&self, id: &str) -> Option<Account> {
        self.data.read().accounts.iter().find(|a| a.id == id).cloned()
    }

    pub fn update_account(&self, id: &str, f: impl FnOnce(&mut Account)) -> Option<Account> {
        let out = {
            let mut d = self.data.write();
            let a = d.accounts.iter_mut().find(|a| a.id == id)?;
            f(a);
            a.clone()
        };
        self.save();
        Some(out)
    }

    /// Context handoff files written for a thread (see agents/transcript.rs).
    pub fn handoff_dir(&self, thread_id: &str) -> PathBuf {
        self.dir.join("handoff").join(thread_id)
    }

    fn thread_path(&self, id: &str) -> PathBuf {
        self.dir.join("threads").join(format!("{id}.json"))
    }

    pub fn thread(&self, id: &str) -> Option<Thread> {
        let mut cache = self.threads.lock();
        if let Some(t) = cache.get(id) {
            return Some(t.clone());
        }
        let mut t: Thread = serde_json::from_str(&fs::read_to_string(self.thread_path(id)).ok()?).ok()?;
        // The list in state.json has the current status.
        if let Some(m) = self.data.read().threads.iter().find(|m| m.id == id) {
            t.meta = m.clone();
        }
        cache.insert(id.to_string(), t.clone());
        Some(t)
    }

    pub fn insert_thread(&self, t: Thread) {
        {
            let mut d = self.data.write();
            d.threads.retain(|m| m.id != t.meta.id);
            d.threads.insert(0, t.meta.clone());
        }
        self.write_thread(&t);
        self.threads.lock().insert(t.meta.id.clone(), t);
        self.save();
    }

    fn write_thread(&self, t: &Thread) {
        let json = serde_json::to_vec(t).unwrap_or_default();
        if let Err(e) = util::write_atomic(&self.thread_path(&t.meta.id), &json) {
            eprintln!("[kilat] failed to save thread {}: {e}", t.meta.id);
        }
        self.last_flush.lock().insert(t.meta.id.clone(), Instant::now());
    }

    /// Mutate a thread. `persist`: Always writes the thread file, Throttled
    /// at most every 2s, Never keeps it in memory. The thread list in
    /// state.json is saved whenever the metadata changes.
    pub fn update_thread<R>(
        &self,
        id: &str,
        persist: Persist,
        f: impl FnOnce(&mut Thread) -> R,
    ) -> Option<R> {
        self.thread(id)?; // make sure it's cached
        let (out, meta, snapshot) = {
            let mut cache = self.threads.lock();
            let t = cache.get_mut(id)?;
            let before = t.meta.clone();
            let out = f(t);
            if t.meta != before {
                t.meta.updated_at = now_ms();
            }
            let write = match persist {
                Persist::Always => true,
                Persist::Never => false,
                Persist::Throttled => self
                    .last_flush
                    .lock()
                    .get(id)
                    .is_none_or(|i| i.elapsed().as_secs() >= 2),
            };
            (out, t.meta.clone(), write.then(|| t.clone()))
        };
        if let Some(t) = snapshot {
            self.write_thread(&t);
        }
        let meta_changed = {
            let mut d = self.data.write();
            match d.threads.iter_mut().find(|m| m.id == id) {
                Some(m) if *m != meta => {
                    *m = meta;
                    true
                }
                _ => false,
            }
        };
        if meta_changed {
            self.save();
        }
        Some(out)
    }

    pub fn delete_thread(&self, id: &str) -> Option<ThreadMeta> {
        let meta = {
            let mut d = self.data.write();
            let pos = d.threads.iter().position(|m| m.id == id)?;
            d.threads.remove(pos)
        };
        self.threads.lock().remove(id);
        let _ = fs::remove_file(self.thread_path(id));
        let _ = fs::remove_dir_all(self.handoff_dir(id));
        let _ = fs::remove_dir_all(self.dir.join("attachments").join(id));
        self.save();
        Some(meta)
    }
}

#[derive(Clone, Copy)]
pub enum Persist {
    Always,
    #[allow(dead_code)]
    Throttled,
    #[allow(dead_code)]
    Never,
}

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn default_codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| home_dir().join(".codex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_settings_get_new_defaults() {
        let s: Settings = serde_json::from_str(r#"{"fontSize": 15, "idleMinutes": 5}"#).unwrap();
        assert_eq!(s.font_size, 15);
        assert!(s.notify && s.sound && s.claude_chrome);
        assert_eq!(s.continue_prompt, DEFAULT_CONTINUE);
    }

    #[test]
    fn chat_era_threads_still_load() {
        // Threads from the chat UI had turns and a sync counter; the native
        // session ids are what matters (they are resumed in the terminal).
        let json = r#"{"id":"t1","title":"x","provider":"claude","status":"idle",
            "native":{"claude":{"id":"abc","synced":3},"codex":null},
            "turns":[{"id":"u","userText":"hi"}],"queue":[]}"#;
        let t: Thread = serde_json::from_str(json).unwrap();
        assert_eq!(t.native.claude.unwrap().id, "abc");
        assert!(t.segments.is_empty());
        let mut m = t.meta;
        m.status = status_at_start(&m).into();
        assert_eq!(m.status, "off");
        m.limit = Some(LimitInfo::default());
        assert_eq!(status_at_start(&m), "limited");
    }
}
