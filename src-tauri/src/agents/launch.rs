//! Command lines for the agent terminal UIs (`claude` / `codex`).

use std::path::{Path, PathBuf};

use portable_pty::CommandBuilder;
use serde_json::{json, Value};

use super::hook;
use crate::store::{home_dir, Store};
use crate::util;

/// What to start in a thread's terminal.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub provider: String,
    pub account_id: String,
    pub model: String,
    /// "full" | "cli" | "ask" | "plan"
    pub mode: String,
    /// Claude: the session id to use (always known up front).
    /// Codex: the thread to resume (None starts a new one).
    pub session: Option<String>,
    /// Claude: resume `session` (its transcript exists) instead of creating it.
    pub resume: bool,
    /// Submitted as the first message.
    pub prompt: Option<String>,
}

/// Kilat's exe as a shell word for hook commands (forward slashes work in
/// both Git Bash and cmd, the quotes keep spaces in "Kilat Code").
fn self_command(flag: &str) -> String {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("kilat.exe"));
    format!("\"{}\" {flag}", util::fwd(&exe))
}

/// The user's own Claude settings (to leave their statusline alone).
fn user_claude_settings() -> Value {
    let dir = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".claude"));
    std::fs::read_to_string(dir.join("settings.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null)
}

/// Settings file passed with `--settings`: hooks that report the session id,
/// work status and limits to Kilat, plus a statusline showing the account
/// (and its plan usage) unless the user has a statusline of their own.
fn claude_settings(store: &Store) -> anyhow::Result<PathBuf> {
    let hook_cmd = self_command(hook::FLAG_HOOK);
    let entry = || json!([{ "hooks": [{ "type": "command", "command": hook_cmd, "timeout": 10 }] }]);
    let mut v = json!({
        "hooks": {
            "SessionStart": entry(),
            "UserPromptSubmit": entry(),
            "Stop": entry(),
            "StopFailure": entry(),
            "Notification": entry(),
        }
    });
    let own_statusline = user_claude_settings().get("statusLine").is_some_and(|s| !s.is_null());
    if !own_statusline {
        v["statusLine"] = json!({ "type": "command", "command": self_command(hook::FLAG_STATUS), "padding": 0 });
    }
    let path = store.dir.join("agent").join("claude-settings.json");
    let text = serde_json::to_vec_pretty(&v)?;
    if std::fs::read(&path).ok().as_deref() != Some(text.as_slice()) {
        util::write_atomic(&path, &text)?;
    }
    Ok(path)
}

pub fn claude(store: &Store, exe: &Path, plan: &Plan) -> anyhow::Result<CommandBuilder> {
    let mut c = CommandBuilder::new(exe);
    let id = plan.session.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    if plan.resume {
        c.args(["--resume", &id]);
    } else {
        c.args(["--session-id", &id]);
    }
    if !plan.model.is_empty() {
        c.args(["--model", &plan.model]);
    }
    match plan.mode.as_str() {
        "full" => {
            c.arg("--dangerously-skip-permissions");
        }
        "ask" => {
            c.args(["--permission-mode", "default"]);
        }
        "plan" => {
            c.args(["--permission-mode", "plan"]);
        }
        _ => {}
    }
    if store.data.read().settings.claude_chrome {
        c.arg("--chrome");
    }
    let settings = claude_settings(store)?;
    c.arg("--settings");
    c.arg(settings.as_os_str());
    if let Some(p) = &plan.prompt {
        c.arg(p);
    }
    Ok(c)
}

pub fn codex(exe: &Path, plan: &Plan) -> CommandBuilder {
    let mut c = CommandBuilder::new(exe);
    if plan.session.is_some() {
        c.arg("resume");
    }
    // The shared background server would serve every account with the
    // login it was started with; each account must use its own CODEX_HOME.
    c.arg("--no-daemon");
    if !plan.model.is_empty() {
        c.args(["-m", &plan.model]);
    }
    match plan.mode.as_str() {
        "full" => {
            c.arg("--dangerously-bypass-approvals-and-sandbox");
        }
        "ask" => {
            c.args(["-s", "workspace-write", "-a", "on-request"]);
        }
        "plan" => {
            c.args(["-s", "read-only", "-a", "on-request"]);
        }
        _ => {}
    }
    if let Some(id) = &plan.session {
        c.arg(id);
    }
    if let Some(p) = &plan.prompt {
        c.arg(p);
    }
    c
}

/// Variables the agent must not inherit from whatever started Kilat (e.g. a
/// Claude Code session refuses to start inside another one).
pub const SCRUB_ENV: &[&str] = &[
    // Markers of a parent Claude Code session (they turn off transcript
    // saving, steer the effort, or connect to the parent's IDE / messaging).
    "CLAUDECODE",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_SSE_PORT",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_CHILD_NOTES_DIR",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_SESSION_ATTENDED",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CLAUDE_CODE_EXECPATH",
    "CLAUDE_PID",
    "CLAUDE_EFFORT",
    "CODEX_SANDBOX",
    "CODEX_SANDBOX_NETWORK_DISABLED",
    hook::ENV_ADDR,
    hook::ENV_KEY,
    hook::ENV_THREAD,
    hook::ENV_GEN,
];
