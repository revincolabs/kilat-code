//! Agent threads as real terminals. Each thread runs the provider's own
//! terminal UI (`claude` / `codex`) in a ConPTY. Kilat starts it with the
//! right account, watches it (Claude: hooks, Codex: its rollout file), and
//! when an account hits its limit lets the user wait for the reset or move
//! the conversation to another Claude / ChatGPT account without losing
//! context:
//!
//! * same provider, other account -> its native session is resumed (session
//!   storage is shared between accounts, see accounts.rs);
//! * other provider -> the history it has not seen is written to a file it
//!   is told to read first (see transcript.rs).
//!
//! Processes belong to the backend, not to the view: closing or switching
//! the thread in the UI keeps the agent running; reopening it replays the
//! recent output and redraws.

pub mod codex;
pub mod hook;
pub mod launch;
pub mod transcript;

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use portable_pty::{native_pty_system, ChildKiller, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter};

use crate::accounts;
use crate::gitctx;
use crate::store::{AccountUsage, LimitInfo, NativeSession, Persist, Segment, Store, Thread, ThreadMeta};
use crate::usage;
use crate::util::{self, now_ms, EnvPlan};
use launch::Plan;

/// Output of a thread's terminal, streamed to every view showing it.
#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TermEvent {
    /// A new agent process starts in this terminal: clear the screen.
    Reset,
    Data { data: String },
    Exit { code: Option<u32> },
}

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentEvent {
    Thread { thread: ThreadMeta },
    /// Something the user should hear about (notification / toast):
    /// kind "done" | "attention" | "error" | "limited".
    #[serde(rename_all = "camelCase")]
    Alert { thread_id: String, kind: String, message: String },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attached {
    pub view_id: u64,
    /// An agent process is running in this thread.
    pub live: bool,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SwitchReq {
    pub thread_id: String,
    pub provider: String,
    pub account_id: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub mode: String,
    /// "manual" | "limit" (the previous account hit its limit: continue the work)
    #[serde(default)]
    pub reason: String,
}

/// Recent output kept per process to repaint a view that (re)attaches.
const BUF_MAX: usize = 1024 * 1024;
const BUF_KEEP: usize = 768 * 1024;

struct Live {
    gen: u64,
    account_id: String,
    cwd: String,
    started_at: i64,
    pid: Option<u32>,
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    buf: Mutex<String>,
    alive: AtomicBool,
    /// Stopped on purpose to be replaced: no "process ended" in the view.
    quiet: AtomicBool,
    size: Mutex<(u16, u16)>,
}

impl Live {
    fn write(&self, data: &[u8]) -> std::io::Result<()> {
        let mut w = self.writer.lock();
        w.write_all(data)?;
        w.flush()
    }

    fn kill(&self) {
        if let Some(pid) = self.pid {
            util::kill_tree(pid);
        }
        let _ = self.killer.lock().kill();
    }

    fn resize(&self, cols: u16, rows: u16) {
        *self.size.lock() = (cols, rows);
        if let Some(m) = self.master.lock().as_ref() {
            let _ = m.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
        }
    }
}

pub struct AgentManager {
    pub app: AppHandle,
    pub store: Arc<Store>,
    live: Mutex<HashMap<String, Arc<Live>>>,
    views: Mutex<HashMap<String, Vec<(u64, Channel<TermEvent>)>>>,
    hook: Mutex<Option<hook::HookServer>>,
    seq: AtomicU64,
    /// Codex rollout files already tied to a thread.
    claimed: Mutex<HashSet<PathBuf>>,
    /// One launch / switch at a time per thread.
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

pub fn title_from(text: &str) -> String {
    let line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    let mut t: String = line.chars().take(48).collect();
    if line.chars().count() > 48 {
        t.push('…');
    }
    t
}

fn excerpt(text: &str, max: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        return flat;
    }
    let mut s: String = flat.chars().take(max).collect();
    s.push('…');
    s
}

fn handoff_prompt(path: &std::path::Path, from: &[String], limited: bool) -> String {
    let names: Vec<&str> = from.iter().map(|p| transcript::provider_name(p)).collect();
    let tail = if limited {
        crate::i18n::tr(
            "lanjutkan pekerjaan terakhir yang terhenti karena akun sebelumnya kena limit.",
            "continue the last task, which stopped because the previous account hit its limit.",
        )
    } else {
        crate::i18n::tr(
            "lanjutkan pekerjaan yang belum selesai. Kalau tidak ada yang tertunda, bilang singkat bahwa kamu siap dan tunggu instruksi.",
            "continue any unfinished work. If nothing is pending, briefly say you're ready and wait for instructions.",
        )
    };
    crate::trf!(
        "Lanjutkan thread ini. Sebelumnya dikerjakan oleh {}; riwayat lengkapnya (pesan, jawaban, dan aksi) ada di file {}. Baca seluruh isi file itu dulu supaya konteksmu sama, lalu {tail}",
        "Continue this thread. It was previously handled by {}; the full history (messages, replies, and actions) is in the file {}. Read the whole file first so you have the same context, then {tail}",
        names.join(crate::i18n::tr(" dan ", " and ")),
        util::fwd(path)
    )
}

impl AgentManager {
    pub fn new(app: AppHandle, store: Arc<Store>) -> Arc<Self> {
        let mgr = Arc::new(Self {
            app,
            store,
            live: Mutex::new(HashMap::new()),
            views: Mutex::new(HashMap::new()),
            hook: Mutex::new(None),
            seq: AtomicU64::new(1),
            claimed: Mutex::new(HashSet::new()),
            locks: Mutex::new(HashMap::new()),
        });
        match hook::serve(Arc::downgrade(&mgr)) {
            Ok(h) => *mgr.hook.lock() = Some(h),
            Err(e) => eprintln!("[kilat] hook server failed: {e}"),
        }
        // Continue threads whose limit wait is over.
        let weak = Arc::downgrade(&mgr);
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(10));
            let Some(m) = weak.upgrade() else { break };
            m.tick_waits();
        });
        mgr
    }

    // ------------------------------------------------------------ events

    fn emit_meta(&self, thread_id: &str) {
        if let Some(t) = self.store.thread(thread_id) {
            let _ = self.app.emit("agent", AgentEvent::Thread { thread: t.meta });
        }
    }

    fn alert(&self, thread_id: &str, kind: &str, message: impl Into<String>) {
        let _ = self.app.emit(
            "agent",
            AgentEvent::Alert { thread_id: thread_id.into(), kind: kind.into(), message: message.into() },
        );
    }

    fn update(&self, thread_id: &str, f: impl FnOnce(&mut Thread)) {
        if self.store.update_thread(thread_id, Persist::Always, f).is_some() {
            self.emit_meta(thread_id);
        }
    }

    fn set_status(&self, thread_id: &str, status: &str) {
        self.update(thread_id, |t| t.meta.status = status.into());
    }

    fn send_views(&self, thread_id: &str, ev: TermEvent) {
        let mut views = self.views.lock();
        if let Some(list) = views.get_mut(thread_id) {
            list.retain(|(_, ch)| ch.send(ev.clone()).is_ok());
        }
    }

    /// Remember an account's plan usage (5-hour / weekly windows).
    fn record_usage(&self, account_id: &str, usage: AccountUsage) {
        let changed = self
            .store
            .account(account_id)
            .is_some_and(|a| a.usage.as_ref().map(|u| &u.windows) != Some(&usage.windows));
        if changed {
            self.store.update_account(account_id, |a| a.usage = Some(usage));
            let _ = self.app.emit("accounts-changed", account_id.to_string());
        }
    }

    // ------------------------------------------------------------- state

    fn lock_for(&self, thread_id: &str) -> Arc<Mutex<()>> {
        self.locks.lock().entry(thread_id.to_string()).or_default().clone()
    }

    fn current(&self, thread_id: &str) -> Option<Arc<Live>> {
        self.live
            .lock()
            .get(thread_id)
            .filter(|l| l.alive.load(Ordering::SeqCst))
            .cloned()
    }

    /// The running process of the thread, if it is the one with `gen`.
    fn current_gen(&self, thread_id: &str, gen: u64) -> Option<Arc<Live>> {
        self.current(thread_id).filter(|l| l.gen == gen)
    }

    pub fn live_count(&self) -> usize {
        self.live.lock().values().filter(|l| l.alive.load(Ordering::SeqCst)).count()
    }

    pub fn is_live(&self, thread_id: &str) -> bool {
        self.current(thread_id).is_some()
    }

    // ------------------------------------------------------------- views

    /// Show a thread's terminal in a view. A running agent's recent output is
    /// replayed, then the agent redraws.
    pub fn attach(&self, thread_id: &str, chan: Channel<TermEvent>) -> Attached {
        let view_id = self.seq.fetch_add(1, Ordering::SeqCst);
        let live = self.current(thread_id);
        match &live {
            Some(l) => {
                // Hold the buffer so no output slips in between replay and subscribe.
                let buf = l.buf.lock();
                let _ = chan.send(TermEvent::Reset);
                if !buf.is_empty() {
                    let _ = chan.send(TermEvent::Data { data: buf.clone() });
                }
                self.views.lock().entry(thread_id.to_string()).or_default().push((view_id, chan));
            }
            None => {
                self.views.lock().entry(thread_id.to_string()).or_default().push((view_id, chan));
            }
        }
        if let Some(l) = live.clone() {
            // A size change makes the TUI repaint the whole screen.
            std::thread::spawn(move || {
                let (c, r) = *l.size.lock();
                if c > 20 {
                    l.resize(c - 1, r);
                    std::thread::sleep(Duration::from_millis(80));
                    l.resize(c, r);
                }
            });
        }
        Attached { view_id, live: live.is_some() }
    }

    pub fn detach(&self, thread_id: &str, view_id: u64) {
        if let Some(list) = self.views.lock().get_mut(thread_id) {
            list.retain(|(id, _)| *id != view_id);
        }
    }

    pub fn write(&self, thread_id: &str, data: &str) -> anyhow::Result<()> {
        let l = self
            .current(thread_id)
            .ok_or_else(|| anyhow::anyhow!(crate::i18n::tr("Agent tidak berjalan", "The agent isn't running")))?;
        l.write(data.as_bytes())?;
        // Answering a permission prompt in the terminal: back to work.
        let asking = self.store.data.read().threads.iter().any(|t| t.id == thread_id && t.status == "attention");
        if asking && (data.contains('\r') || data.len() == 1) {
            self.set_status(thread_id, "running");
        }
        Ok(())
    }

    pub fn resize(&self, thread_id: &str, cols: u16, rows: u16) {
        if let Some(l) = self.current(thread_id) {
            l.resize(cols.max(20), rows.max(5));
        }
    }

    // ----------------------------------------------------------- launch

    /// Plan that continues the thread's own session of `provider`.
    fn resume_plan(&self, t: &Thread, provider: &str, account_id: &str, model: &str, mode: &str) -> Plan {
        let mut plan = Plan {
            provider: provider.into(),
            account_id: account_id.into(),
            model: model.into(),
            mode: if mode.is_empty() { "full".into() } else { mode.into() },
            ..Default::default()
        };
        match provider {
            "claude" => match t.native.claude.as_ref().filter(|n| !n.id.is_empty()) {
                Some(n) => {
                    plan.resume = transcript::claude_transcript(&n.id, n.transcript.as_deref()).is_some();
                    plan.session = Some(n.id.clone());
                }
                None => plan.session = Some(uuid::Uuid::new_v4().to_string()),
            },
            _ => {
                plan.session = t
                    .native
                    .codex
                    .as_ref()
                    .filter(|n| !n.id.is_empty() && codex::find_session(&n.id).is_some())
                    .map(|n| n.id.clone());
                plan.resume = plan.session.is_some();
            }
        }
        plan
    }

    /// Start the thread's agent if it is not running (resuming its session).
    pub fn start(self: &Arc<Self>, thread_id: &str, cols: u16, rows: u16) -> anyhow::Result<()> {
        let lock = self.lock_for(thread_id);
        let _g = lock.lock();
        if self.is_live(thread_id) {
            return Ok(());
        }
        let t = self.store.thread(thread_id).ok_or_else(|| anyhow::anyhow!(crate::i18n::tr("Thread tidak ditemukan", "Thread not found")))?;
        let plan = self.resume_plan(&t, &t.meta.provider, &t.meta.account_id, &t.meta.model, &t.meta.mode);
        self.launch(&t, plan, cols, rows)
    }

    fn launch(self: &Arc<Self>, t: &Thread, plan: Plan, cols: u16, rows: u16) -> anyhow::Result<()> {
        let tid = t.meta.id.clone();
        let acc = self
            .store
            .account(&plan.account_id)
            .ok_or_else(|| anyhow::anyhow!(crate::i18n::tr("Akun tidak ditemukan", "Account not found")))?;
        if acc.provider != plan.provider {
            anyhow::bail!(crate::trf!("Akun '{}' bukan akun {}", "The account '{}' isn't a {} account", acc.label, transcript::provider_name(&plan.provider)));
        }
        if !std::path::Path::new(&t.meta.cwd).is_dir() {
            anyhow::bail!(crate::trf!("Folder kerja tidak ditemukan: {}", "Working folder not found: {}", t.meta.cwd));
        }
        let tools = self.store.tools();
        let mut env = EnvPlan::default();
        for k in launch::SCRUB_ENV {
            env.remove(k);
        }
        env.set("TERM", "xterm-256color");
        env.set("COLORTERM", "truecolor");
        env.extend(gitctx::ctx_env(&self.store, &t.meta.project_id));
        env.extend(accounts::agent_env(&self.store, &plan.account_id)?);
        let gen = self.seq.fetch_add(1, Ordering::SeqCst);
        if let Some(h) = self.hook.lock().clone() {
            env.set(hook::ENV_ADDR, h.addr);
            env.set(hook::ENV_KEY, h.key);
        }
        env.set(hook::ENV_THREAD, &tid);
        env.set(hook::ENV_GEN, gen.to_string());

        let mut cmd = match plan.provider.as_str() {
            "claude" => {
                let exe = tools.claude.ok_or_else(|| {
                    anyhow::anyhow!("Claude Code CLI tidak ditemukan. Install: npm i -g @anthropic-ai/claude-code")
                })?;
                launch::claude(&self.store, &exe, &plan)?
            }
            "codex" => {
                let exe = tools
                    .codex
                    .ok_or_else(|| anyhow::anyhow!("Codex CLI tidak ditemukan. Install: npm i -g @openai/codex"))?;
                launch::codex(&exe, &plan)
            }
            other => anyhow::bail!(crate::trf!("Provider tidak dikenal: {other}", "Unknown provider: {other}")),
        };
        cmd.cwd(&t.meta.cwd);
        env.apply_pty(&mut cmd);

        let (cols, rows) = (cols.max(20), rows.max(5));
        let pair = native_pty_system().openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })?;
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| anyhow::anyhow!(crate::trf!("Gagal menjalankan {}: {e}", "Couldn't start {}: {e}", transcript::provider_name(&plan.provider))))?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let live = Arc::new(Live {
            gen,
            account_id: plan.account_id.clone(),
            cwd: t.meta.cwd.clone(),
            started_at: now_ms(),
            pid: child.process_id(),
            writer: Mutex::new(writer),
            master: Mutex::new(Some(pair.master)),
            killer: Mutex::new(child.clone_killer()),
            buf: Mutex::new(String::new()),
            alive: AtomicBool::new(true),
            quiet: AtomicBool::new(false),
            size: Mutex::new((cols, rows)),
        });
        self.live.lock().insert(tid.clone(), live.clone());

        // Remember what runs now; a new account opens a new segment.
        let now = now_ms();
        let native_id = plan.session.clone().unwrap_or_default();
        self.store.update_thread(&tid, Persist::Always, |t| {
            if plan.provider == "claude" && !native_id.is_empty() {
                match &mut t.native.claude {
                    Some(n) if n.id == native_id => {}
                    slot => *slot = Some(NativeSession { id: native_id.clone(), transcript: None }),
                }
            }
            let open = t
                .segments
                .last()
                .is_some_and(|s| s.provider == plan.provider && s.account_id == plan.account_id && s.ended_at.is_none());
            if !open {
                if let Some(last) = t.segments.last_mut() {
                    last.ended_at.get_or_insert(now);
                }
                t.segments.push(Segment {
                    provider: plan.provider.clone(),
                    account_id: plan.account_id.clone(),
                    native_id: native_id.clone(),
                    started_at: now,
                    ended_at: None,
                });
            }
            t.meta.provider = plan.provider.clone();
            t.meta.account_id = plan.account_id.clone();
            t.meta.model = plan.model.clone();
            t.meta.mode = plan.mode.clone();
            t.meta.status = if t.meta.limit.is_some() { "limited" } else { "ready" }.into();
        });
        self.send_views(&tid, TermEvent::Reset);
        self.emit_meta(&tid);

        // Reader: PTY -> buffer + views.
        {
            let m = self.clone();
            let l = live.clone();
            let tid = tid.clone();
            std::thread::spawn(move || {
                let mut chunk = [0u8; 16 * 1024];
                let mut pending = Vec::new();
                loop {
                    let n = match reader.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    let text = util::decode_utf8(&mut pending, &chunk[..n]);
                    if text.is_empty() {
                        continue;
                    }
                    let mut buf = l.buf.lock();
                    buf.push_str(&text);
                    if buf.len() > BUF_MAX {
                        let mut cut = buf.len() - BUF_KEEP;
                        while !buf.is_char_boundary(cut) {
                            cut += 1;
                        }
                        buf.drain(..cut);
                    }
                    if !l.quiet.load(Ordering::SeqCst) {
                        m.send_views(&tid, TermEvent::Data { data: text });
                    }
                }
            });
        }

        // Waiter: the agent exited (or was stopped).
        {
            let m = self.clone();
            let l = live.clone();
            let tid = tid.clone();
            std::thread::spawn(move || {
                let code = child.wait().ok().map(|s| s.exit_code());
                l.alive.store(false, Ordering::SeqCst);
                l.master.lock().take(); // closes the ConPTY so the reader ends
                let current = {
                    let mut map = m.live.lock();
                    if map.get(&tid).is_some_and(|x| Arc::ptr_eq(x, &l)) {
                        map.remove(&tid);
                        true
                    } else {
                        false
                    }
                };
                std::thread::sleep(Duration::from_millis(150));
                if !l.quiet.load(Ordering::SeqCst) {
                    m.send_views(&tid, TermEvent::Exit { code });
                }
                if current {
                    m.update(&tid, |t| {
                        t.meta.status = if t.meta.limit.is_some() || t.meta.wait_until.is_some() { "limited" } else { "off" }.into();
                    });
                }
            });
        }

        if plan.provider == "codex" {
            let m = self.clone();
            let l = live.clone();
            let known = plan.session.clone();
            std::thread::spawn(move || m.watch_codex(&tid, &l, known));
        }
        Ok(())
    }

    /// Stop the thread's agent on purpose and wait until it is gone.
    fn stop_and_wait(&self, thread_id: &str) {
        let Some(l) = self.live.lock().get(thread_id).cloned() else { return };
        l.quiet.store(true, Ordering::SeqCst);
        l.kill();
        for _ in 0..60 {
            if !l.alive.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let mut map = self.live.lock();
        if map.get(thread_id).is_some_and(|x| Arc::ptr_eq(x, &l)) {
            map.remove(thread_id);
        }
    }

    /// The user stops the agent (the view shows that the process ended).
    pub fn stop(&self, thread_id: &str) {
        if let Some(l) = self.current(thread_id) {
            l.kill();
        }
    }

    /// Thread deleted: stop the agent, forget its views.
    pub fn kill(&self, thread_id: &str) {
        self.stop_and_wait(thread_id);
        self.views.lock().remove(thread_id);
    }

    pub fn shutdown(&self) {
        let all: Vec<Arc<Live>> = self.live.lock().drain().map(|(_, l)| l).collect();
        for l in all {
            l.quiet.store(true, Ordering::SeqCst);
            l.kill();
        }
    }

    // ------------------------------------------------------------ switch

    /// Continue the thread with another account / provider / model / mode.
    pub fn switch(self: &Arc<Self>, req: SwitchReq) -> anyhow::Result<()> {
        let tid = req.thread_id.clone();
        let lock = self.lock_for(&tid);
        let _g = lock.lock();
        let acc = self.store.account(&req.account_id).ok_or_else(|| anyhow::anyhow!(crate::i18n::tr("Akun tidak ditemukan", "Account not found")))?;
        if acc.provider != req.provider {
            anyhow::bail!(crate::trf!("Akun '{}' bukan akun {}", "The account '{}' isn't a {} account", acc.label, transcript::provider_name(&req.provider)));
        }
        if !acc.ready {
            anyhow::bail!(crate::trf!("Akun '{}' belum siap. Login dulu di Akun & Pengaturan.", "The account '{}' isn't ready. Sign in first in Accounts & Settings.", acc.label));
        }
        // Fail before stopping anything when the account cannot run.
        accounts::agent_env(&self.store, &req.account_id)?;
        let (cols, rows) = self.current(&tid).map(|l| *l.size.lock()).unwrap_or((120, 32));
        self.stop_and_wait(&tid);

        let now = now_ms();
        self.store.update_thread(&tid, Persist::Always, |t| {
            if let Some(last) = t.segments.last_mut() {
                last.ended_at.get_or_insert(now);
            }
            t.meta.limit = None;
            t.meta.wait_until = None;
        });
        let t = self.store.thread(&tid).ok_or_else(|| anyhow::anyhow!(crate::i18n::tr("Thread tidak ditemukan", "Thread not found")))?;
        let limited = req.reason == "limit";
        let continue_text = self.store.data.read().settings.continue_text();
        let mut plan = self.resume_plan(&t, &req.provider, &req.account_id, &req.model, &req.mode);
        let resumed = plan.resume.then(|| plan.session.clone()).flatten();
        if t.meta.provider == req.provider {
            if limited && resumed.is_some() {
                plan.prompt = Some(continue_text);
            }
        } else if let Some(h) = transcript::write_handoff(&self.store, &t, &req.provider, resumed.as_deref()) {
            plan.prompt = Some(handoff_prompt(&h.path, &h.from, limited));
        } else if limited && resumed.is_some() {
            plan.prompt = Some(continue_text);
        }
        self.launch(&t, plan, cols, rows)
    }

    // -------------------------------------------------------------- limit

    fn on_limit(&self, thread_id: &str, account_id: &str, provider: &str, message: &str) {
        let now = now_ms();
        let dup = self
            .store
            .thread(thread_id)
            .and_then(|t| t.meta.limit)
            .is_some_and(|l| l.account_id == account_id && now - l.at < 120_000);
        if dup {
            return;
        }
        let usage = self.store.account(account_id).and_then(|a| a.usage);
        let resets = usage
            .as_ref()
            .and_then(usage::reset_time)
            .or_else(|| {
                // The last report may lag behind: take the fullest window.
                let u = usage.as_ref()?;
                let w = u.windows.iter().filter(|w| w.used_percent >= 90.0).max_by(|a, b| a.used_percent.total_cmp(&b.used_percent))?;
                w.resets_at
            })
            .filter(|t| *t > now);
        let acc = self.store.update_account(account_id, |a| a.limited_until = Some(resets.unwrap_or(now + 3_600_000)));
        let _ = self.app.emit("accounts-changed", account_id.to_string());
        self.update(thread_id, |t| {
            t.meta.limit = Some(LimitInfo {
                account_id: account_id.into(),
                provider: provider.into(),
                at: now,
                resets_at: resets,
                message: excerpt(message, 400),
            });
            t.meta.wait_until = None;
            t.meta.status = "limited".into();
        });
        let label = acc.map(|a| a.label).unwrap_or_default();
        self.alert(thread_id, "limited", crate::trf!("Akun \"{label}\" kena limit. Pilih: tunggu atau pindah akun.", "The account \"{label}\" hit its limit. Choose: wait or switch accounts."));
    }

    /// Wait for the limit to reset, then continue automatically.
    pub fn wait(&self, thread_id: &str, until: i64) {
        let limited = self.store.thread(thread_id).and_then(|t| t.meta.limit).map(|l| l.account_id);
        self.update(thread_id, |t| {
            t.meta.wait_until = Some(until);
            t.meta.status = "limited".into();
        });
        // The user's choice is the best known reset time for that account.
        if let Some(acc) = limited {
            self.store.update_account(&acc, |a| a.limited_until = Some(until));
            let _ = self.app.emit("accounts-changed", acc);
        }
    }

    pub fn cancel_wait(&self, thread_id: &str) {
        self.update(thread_id, |t| t.meta.wait_until = None);
    }

    /// Close the limit dialog without doing anything.
    pub fn dismiss_limit(&self, thread_id: &str) {
        let live = self.is_live(thread_id);
        self.update(thread_id, |t| {
            t.meta.limit = None;
            t.meta.wait_until = None;
            t.meta.status = if live { "ready" } else { "off" }.into();
        });
    }

    fn tick_waits(self: &Arc<Self>) {
        let now = now_ms();
        let due: Vec<String> = self
            .store
            .data
            .read()
            .threads
            .iter()
            .filter(|t| t.wait_until.is_some_and(|w| w <= now))
            .map(|t| t.id.clone())
            .collect();
        for tid in due {
            if let Err(e) = self.continue_now(&tid) {
                self.alert(&tid, "error", crate::trf!("Gagal melanjutkan setelah limit: {e}", "Couldn't continue after the limit: {e}"));
                self.update(&tid, |t| t.meta.wait_until = None);
            }
        }
    }

    /// Continue with the same account now (its limit should be over): the
    /// agent restarts on its session with the continue message.
    pub fn continue_now(self: &Arc<Self>, thread_id: &str) -> anyhow::Result<()> {
        let lock = self.lock_for(thread_id);
        let _g = lock.lock();
        let t = self.store.thread(thread_id).ok_or_else(|| anyhow::anyhow!(crate::i18n::tr("Thread tidak ditemukan", "Thread not found")))?;
        let account_id = t.meta.limit.as_ref().map(|l| l.account_id.clone()).unwrap_or(t.meta.account_id.clone());
        self.store.update_account(&account_id, |a| a.limited_until = None);
        let _ = self.app.emit("accounts-changed", account_id.clone());
        let (cols, rows) = self.current(thread_id).map(|l| *l.size.lock()).unwrap_or((120, 32));
        self.stop_and_wait(thread_id);
        self.store.update_thread(thread_id, Persist::Always, |t| {
            t.meta.limit = None;
            t.meta.wait_until = None;
        });
        let t = self.store.thread(thread_id).ok_or_else(|| anyhow::anyhow!(crate::i18n::tr("Thread tidak ditemukan", "Thread not found")))?;
        let mut plan = self.resume_plan(&t, &t.meta.provider, &t.meta.account_id, &t.meta.model, &t.meta.mode);
        if plan.resume {
            plan.prompt = Some(self.store.data.read().settings.continue_text());
        }
        self.launch(&t, plan, cols, rows)
    }

    // -------------------------------------------------------------- hooks

    /// A Claude Code hook fired in a thread's agent.
    pub fn on_claude_hook(&self, thread_id: &str, gen: u64, data: &Value) {
        let Some(live) = self.current_gen(thread_id, gen) else { return };
        let str_of = |k: &str| data.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        match str_of("hook_event_name").as_str() {
            "SessionStart" => {
                let id = str_of("session_id");
                let path = str_of("transcript_path");
                if id.is_empty() {
                    return;
                }
                self.store.update_thread(thread_id, Persist::Always, |t| {
                    t.native.claude = Some(NativeSession { id: id.clone(), transcript: (!path.is_empty()).then_some(path) });
                    if let Some(s) = t.segments.last_mut().filter(|s| s.provider == "claude") {
                        s.native_id = id.clone();
                    }
                });
            }
            "UserPromptSubmit" => {
                let prompt = str_of("prompt");
                self.update(thread_id, |t| {
                    t.meta.status = "running".into();
                    t.meta.limit = None;
                    t.meta.wait_until = None;
                    if (t.meta.title.is_empty() || t.meta.title == "Thread baru" || t.meta.title == "New thread") && !prompt.trim().is_empty() {
                        t.meta.title = title_from(&prompt);
                    }
                });
            }
            "Stop" => {
                if self.thread_status(thread_id) != "limited" {
                    self.set_status(thread_id, "ready");
                }
                self.alert(thread_id, "done", excerpt(&str_of("last_assistant_message"), 140));
            }
            "StopFailure" => {
                let error = str_of("error");
                let detail = [str_of("last_assistant_message"), str_of("error_details")]
                    .into_iter()
                    .find(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| error.clone());
                if matches!(error.as_str(), "rate_limit" | "billing_error") {
                    self.on_limit(thread_id, &live.account_id, "claude", &detail);
                } else {
                    self.set_status(thread_id, "ready");
                    self.alert(thread_id, "error", excerpt(&detail, 160));
                }
            }
            "Notification" => {
                let kind = str_of("notification_type");
                let msg = str_of("message");
                let asks = kind == "permission_prompt" || (kind.is_empty() && msg.to_lowercase().contains("permission"));
                if asks {
                    self.set_status(thread_id, "attention");
                    self.alert(thread_id, "attention", if msg.is_empty() { crate::i18n::tr("Butuh izin", "Needs permission").into() } else { msg });
                }
            }
            _ => {}
        }
    }

    fn thread_status(&self, thread_id: &str) -> String {
        self.store
            .data
            .read()
            .threads
            .iter()
            .find(|t| t.id == thread_id)
            .map(|t| t.status.clone())
            .unwrap_or_default()
    }

    /// Claude Code asks for its statusline: record the plan usage it reports
    /// and answer with the account the agent runs as.
    pub fn on_statusline(&self, thread_id: &str, gen: u64, data: &Value) -> String {
        let account_id = match self.current_gen(thread_id, gen) {
            Some(l) => l.account_id.clone(),
            None => self.store.thread(thread_id).map(|t| t.meta.account_id).unwrap_or_default(),
        };
        if let Some(u) = data.get("rate_limits").filter(|v| !v.is_null()).and_then(usage::from_claude) {
            self.record_usage(&account_id, u);
        }
        let Some(acc) = self.store.account(&account_id) else { return "⚡ Kilat".into() };
        let mut parts = vec![format!("⚡ Kilat · {}", acc.label)];
        if let Some(p) = acc.plan.as_ref().filter(|p| !p.is_empty()) {
            parts.push(p.to_uppercase());
        } else if acc.kind == "apikey" {
            parts.push("API key".into());
        }
        if let Some(u) = &acc.usage {
            for w in u.windows.iter().take(2) {
                parts.push(format!("{} {:.0}%", crate::i18n::window(&w.label), w.used_percent));
            }
        }
        parts.join(" · ")
    }

    // -------------------------------------------------------------- codex

    fn watch_codex(self: &Arc<Self>, thread_id: &str, live: &Arc<Live>, known: Option<String>) {
        let mut known = known;
        let mut rollout: Option<codex::Rollout> = None;
        let mut tick: u32 = 0;
        loop {
            let alive = live.alive.load(Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(1500));
            tick += 1;
            let look = rollout.is_none() || tick % 20 == 0;
            if look {
                let found = match &known {
                    Some(id) => codex::find_session(id).map(|p| (p, id.clone())),
                    None => {
                        let taken = self.claimed.lock().clone();
                        codex::find_new(&live.cwd, live.started_at, &taken)
                    }
                };
                if let Some((path, id)) = found {
                    if rollout.as_ref().map(|r| &r.path) != Some(&path) {
                        self.claimed.lock().insert(path.clone());
                        rollout = Some(codex::Rollout::new(path));
                        if known.as_deref() != Some(id.as_str()) || self.native_codex(thread_id).as_deref() != Some(id.as_str()) {
                            self.store.update_thread(thread_id, Persist::Always, |t| {
                                t.native.codex = Some(NativeSession { id: id.clone(), transcript: None });
                                if let Some(s) = t.segments.last_mut().filter(|s| s.provider == "codex") {
                                    s.native_id = id.clone();
                                }
                            });
                        }
                        known = Some(id);
                    }
                }
            }
            if let Some(r) = rollout.as_mut() {
                for sig in r.poll(live.started_at) {
                    self.on_codex_signal(thread_id, live, sig);
                }
            }
            if !alive {
                break; // one last read after the process ended
            }
        }
    }

    fn native_codex(&self, thread_id: &str) -> Option<String> {
        self.store.thread(thread_id)?.native.codex.map(|n| n.id)
    }

    fn on_codex_signal(&self, thread_id: &str, live: &Arc<Live>, sig: codex::Signal) {
        use codex::Signal;
        let is_current = self.live.lock().get(thread_id).is_some_and(|l| Arc::ptr_eq(l, live));
        if let Signal::Usage(u) = sig {
            self.record_usage(&live.account_id, u);
            return;
        }
        if !is_current {
            return;
        }
        match sig {
            Signal::Running => self.update(thread_id, |t| {
                t.meta.status = "running".into();
                t.meta.limit = None;
                t.meta.wait_until = None;
            }),
            Signal::Done(msg) => {
                if self.thread_status(thread_id) != "limited" {
                    self.set_status(thread_id, "ready");
                }
                self.alert(thread_id, "done", excerpt(&msg, 140));
            }
            Signal::Limit(msg) => self.on_limit(thread_id, &live.account_id, "codex", &msg),
            Signal::Error(msg) => {
                self.set_status(thread_id, "ready");
                self.alert(thread_id, "error", excerpt(&msg, 160));
            }
            Signal::Attention(msg) => {
                self.set_status(thread_id, "attention");
                self.alert(thread_id, "attention", msg);
            }
            Signal::UserText(text) => self.update(thread_id, |t| {
                if (t.meta.title.is_empty() || t.meta.title == "Thread baru" || t.meta.title == "New thread") && !text.trim().is_empty() {
                    t.meta.title = title_from(&text);
                }
            }),
            Signal::Usage(_) => {}
        }
    }
}

/// The app-wide manager (lives in Tauri state for the whole app lifetime).
#[allow(dead_code)]
pub fn manager(app: &AppHandle) -> Option<Arc<AgentManager>> {
    use tauri::Manager;
    app.try_state::<crate::AppState>().map(|s| s.agents.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_prompt_points_at_the_file() {
        let p = handoff_prompt(std::path::Path::new("C:\\data\\handoff\\t\\1.md"), &["claude".into()], true);
        assert!(p.contains("C:/data/handoff/t/1.md"));
        assert!(p.contains("Claude Code"));
        assert!(p.contains("kena limit"));
        // Safe as one command-line argument.
        assert!(!p.contains('"') && !p.contains('&') && !p.contains('%'));
    }

    #[test]
    fn titles_and_excerpts() {
        assert_eq!(title_from("\n  perbaiki bug login\nlainnya"), "perbaiki bug login");
        assert_eq!(excerpt("a\n b   c", 10), "a b c");
        assert!(excerpt(&"x".repeat(50), 10).ends_with('…'));
    }
}
