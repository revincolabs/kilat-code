//! Terminals: Git Bash (or a setup command) inside a ConPTY, streamed to
//! xterm.js through a Tauri channel.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter};

use crate::accounts;
use crate::gitctx;
use crate::store::{default_codex_home, Store};
use crate::util::{self, EnvPlan};

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PtyEvent {
    Data { data: String },
    Exit { code: Option<u32> },
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SpawnOpts {
    pub cwd: Option<String>,
    pub project_id: Option<String>,
    pub cols: u16,
    pub rows: u16,
    /// One-time account setup instead of a shell:
    /// "claude-token:<accountId>" | "codex-login:<accountId>" | "gh-login"
    pub setup: Option<String>,
}

struct Session {
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    /// Setup terminals keep a wide PTY so long tokens are never wrapped.
    min_cols: u16,
}

pub struct PtyManager {
    app: AppHandle,
    store: Arc<Store>,
    sessions: Arc<Mutex<HashMap<String, Arc<Session>>>>,
}

/// Check a token caught from `claude setup-token`, then store it. Runs off
/// the reader thread because it makes a network request.
fn save_setup_token(store: Arc<Store>, app: AppHandle, acc: String, token: String) {
    std::thread::spawn(move || match accounts::store_claude_token(&store, &acc, &token) {
        Ok(()) => {
            let _ = app.emit("accounts-changed", acc);
        }
        Err(e) => {
            let label = store.account(&acc).map(|a| a.label).unwrap_or_default();
            let _ = app.emit(
                "account-error",
                format!(
                    "Token akun \"{label}\" tidak tersimpan. {e} Kalau perlu, salin token dari terminal \
                     ke kolom di bawahnya lalu klik Simpan token."
                ),
            );
        }
    });
}

impl PtyManager {
    pub fn new(app: AppHandle, store: Arc<Store>) -> Self {
        Self {
            app,
            store,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn build_command(&self, opts: &SpawnOpts) -> anyhow::Result<(CommandBuilder, Option<String>)> {
        let tools = self.store.tools();
        let mut env = EnvPlan::default();
        env.set("TERM", "xterm-256color");
        env.set("COLORTERM", "truecolor");
        env.set("TERM_PROGRAM", "kilat");
        if let Some(pid) = &opts.project_id {
            env.extend(gitctx::ctx_env(&self.store, pid));
        }
        let mut capture = None;

        let mut cmd = match opts.setup.as_deref() {
            Some(s) if s.starts_with("claude-token:") || s.starts_with("claude-identity:") => {
                let (step, id) = s.split_once(':').unwrap_or_default();
                // Only real accounts: the id ends up in a path we write to.
                self.store
                    .account(id)
                    .filter(|a| a.provider == "claude")
                    .ok_or_else(|| anyhow::anyhow!("Akun Claude tidak ditemukan"))?;
                let claude = tools
                    .claude
                    .ok_or_else(|| anyhow::anyhow!("Claude Code CLI tidak ditemukan"))?;
                // Isolated config dir: these logins must not touch the main one.
                let dir = accounts::claude_setup_dir(&self.store, id);
                std::fs::create_dir_all(&dir)?;
                env.set("CLAUDE_CONFIG_DIR", dir.to_string_lossy());
                for k in [
                    "CLAUDE_CODE_OAUTH_TOKEN",
                    "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
                    "ANTHROPIC_API_KEY",
                    "ANTHROPIC_AUTH_TOKEN",
                ] {
                    env.remove(k);
                }
                let mut c = CommandBuilder::new(claude);
                if step == "claude-token" {
                    capture = Some(id.to_string());
                    c.arg("setup-token");
                } else {
                    // A full sign-in only to learn the email (setup-token
                    // tokens cannot read it). claude_confirm_identity reads
                    // it and deletes this login right away.
                    accounts::forget_claude_login(&dir);
                    c.args(["auth", "login", "--claudeai"]);
                }
                c
            }
            Some(s) if s.starts_with("codex-login:") => {
                let id = &s["codex-login:".len()..];
                let codex = tools
                    .codex
                    .ok_or_else(|| anyhow::anyhow!("Codex CLI tidak ditemukan"))?;
                let home = accounts::prepare_codex_home(&self.store, id)?;
                if home != default_codex_home() {
                    env.set("CODEX_HOME", home.to_string_lossy());
                }
                let mut c = CommandBuilder::new(codex);
                c.arg("login");
                c
            }
            Some("gh-login") => {
                let gh = tools
                    .gh
                    .ok_or_else(|| anyhow::anyhow!("GitHub CLI (gh) tidak ditemukan"))?;
                env.remove("GIT_CONFIG_GLOBAL");
                env.remove("GH_TOKEN");
                env.remove("GITHUB_TOKEN");
                let mut c = CommandBuilder::new(gh);
                c.args(["auth", "login", "--hostname", "github.com", "--git-protocol", "https", "--web"]);
                c
            }
            Some(other) => anyhow::bail!("setup tidak dikenal: {other}"),
            None => {
                let bash = tools.bash.ok_or_else(|| {
                    anyhow::anyhow!("Shell tidak ditemukan. Install Git for Windows atau atur path bash di Pengaturan.")
                })?;
                let mut c = CommandBuilder::new(&bash);
                if cfg!(windows) {
                    // Git Bash: start in the given folder, MinGW environment.
                    env.set("CHERE_INVOKING", "1");
                    if std::env::var_os("MSYSTEM").is_none() {
                        env.set("MSYSTEM", "MINGW64");
                    }
                    c.args(["--login", "-i"]);
                } else {
                    // The user's own shell (zsh, bash, fish…) as a login shell.
                    c.arg("-l");
                }
                c
            }
        };
        let cwd = opts
            .cwd
            .clone()
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
            .unwrap_or_else(crate::store::home_dir);
        cmd.cwd(cwd);
        env.apply_pty(&mut cmd);
        Ok((cmd, capture))
    }

    pub fn spawn(&self, opts: SpawnOpts, chan: Channel<PtyEvent>) -> anyhow::Result<String> {
        let (cmd, capture) = self.build_command(&opts)?;
        let min_cols = if opts.setup.is_some() { 240 } else { 2 };
        let size = PtySize {
            rows: opts.rows.max(2),
            cols: opts.cols.max(min_cols),
            pixel_width: 0,
            pixel_height: 0,
        };
        let pair = native_pty_system().openpty(size)?;
        let mut child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let killer = child.clone_killer();
        let id = util::new_id();
        let session = Arc::new(Session {
            writer: Mutex::new(writer),
            master: Mutex::new(Some(pair.master)),
            killer: Mutex::new(killer),
            min_cols,
        });
        self.sessions.lock().insert(id.clone(), session.clone());

        // Reader: PTY -> frontend (+ token capture for setup terminals).
        let out_chan = chan.clone();
        let store = self.store.clone();
        let app = self.app.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 16 * 1024];
            let mut pending = Vec::new();
            let mut captured = String::new();
            let mut done = capture.is_none();
            loop {
                let n = match reader.read(&mut buf) {
                    Ok(0) | Err(_) => {
                        // Output ended: a token right at the very end is complete now.
                        if let (Some(acc), false) = (&capture, done) {
                            if let Some(token) = accounts::find_setup_token(&captured, true) {
                                save_setup_token(store.clone(), app.clone(), acc.clone(), token);
                            }
                        }
                        break;
                    }
                    Ok(n) => n,
                };
                let text = util::decode_utf8(&mut pending, &buf[..n]);
                if text.is_empty() {
                    continue;
                }
                if let (Some(acc), false) = (&capture, done) {
                    captured.push_str(&text);
                    if captured.len() > 256 * 1024 {
                        // Keep the tail; cut on a char boundary (a mid-char
                        // cut panics, and release builds abort on panic).
                        let mut cut = captured.len() - 128 * 1024;
                        while !captured.is_char_boundary(cut) {
                            cut += 1;
                        }
                        captured.drain(..cut);
                    }
                    if let Some(token) = accounts::find_setup_token(&captured, false) {
                        done = true;
                        save_setup_token(store.clone(), app.clone(), acc.clone(), token);
                    }
                }
                if out_chan.send(PtyEvent::Data { data: text }).is_err() {
                    break;
                }
            }
        });

        // Waiter: report exit and release the ConPTY so the reader unblocks.
        let sessions_id = id.clone();
        let mgr_sessions = self.sessions.clone();
        std::thread::spawn(move || {
            let code = child.wait().ok().map(|s| s.exit_code());
            if let Some(s) = mgr_sessions.lock().remove(&sessions_id) {
                s.master.lock().take();
            }
            let _ = chan.send(PtyEvent::Exit { code });
        });
        Ok(id)
    }

    pub fn write(&self, id: &str, data: &str) -> anyhow::Result<()> {
        let s = self
            .sessions
            .lock()
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("terminal sudah ditutup"))?;
        let mut w = s.writer.lock();
        w.write_all(data.as_bytes())?;
        w.flush()?;
        Ok(())
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> anyhow::Result<()> {
        if let Some(s) = self.sessions.lock().get(id).cloned() {
            if let Some(m) = s.master.lock().as_ref() {
                m.resize(PtySize {
                    rows: rows.max(2),
                    cols: cols.max(s.min_cols),
                    pixel_width: 0,
                    pixel_height: 0,
                })?;
            }
        }
        Ok(())
    }

    pub fn kill(&self, id: &str) {
        if let Some(s) = self.sessions.lock().remove(id) {
            let _ = s.killer.lock().kill();
            s.master.lock().take();
        }
    }

    pub fn kill_all(&self) {
        let all: Vec<_> = self.sessions.lock().drain().collect();
        for (_, s) in all {
            let _ = s.killer.lock().kill();
            s.master.lock().take();
        }
    }
}
