//! Codex: the model list and the rollout reader.
//!
//! The Codex TUI writes every event of a session to its rollout file
//! (`sessions/YYYY/MM/DD/rollout-<time>-<id>.jsonl`). Kilat reads it to learn
//! the session id, whether the agent is working, the plan usage, and whether
//! the account hit its limit.

use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde_json::{json, Value};

use super::transcript::codex_sessions_dir;
use crate::store::AccountUsage;
use crate::usage;
use crate::util;

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

pub fn models() -> Vec<Value> {
    let path = crate::store::default_codex_home().join("models_cache.json");
    let Ok(text) = fs::read_to_string(path) else {
        return vec![];
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return vec![];
    };
    v["models"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|m| s(m, "visibility") == "list")
                .map(|m| json!({ "id": s(m, "slug"), "name": s(m, "display_name") }))
                .collect()
        })
        .unwrap_or_default()
}

static LIMIT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)usage limit|spend cap|hit your .{0,24}limit|quota|rate[ _-]?limit|too many requests").unwrap()
});

pub fn is_limit(msg: &str) -> bool {
    LIMIT_RE.is_match(msg)
}

/// The newest day folders of the sessions tree (YYYY/MM/DD), newest first.
fn recent_days(root: &Path, max: usize) -> Vec<PathBuf> {
    let sorted = |p: &Path| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = fs::read_dir(p).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        v.sort();
        v.reverse();
        v
    };
    let mut out = vec![];
    for y in sorted(root) {
        for m in sorted(&y) {
            for d in sorted(&m) {
                out.push(d);
                if out.len() >= max {
                    return out;
                }
            }
        }
    }
    out
}

/// `session_meta` of a rollout: (id, cwd).
fn session_meta(path: &Path) -> Option<(String, String)> {
    let f = fs::File::open(path).ok()?;
    let mut line = String::new();
    BufReader::new(f).take(256 * 1024).read_line(&mut line).ok()?;
    let v: Value = serde_json::from_str(line.trim()).ok()?;
    if s(&v, "type") != "session_meta" {
        return None;
    }
    let p = &v["payload"];
    let id = [s(p, "id"), s(p, "session_id")].into_iter().find(|x| !x.is_empty())?;
    Some((id.to_string(), s(p, "cwd").to_string()))
}

/// The rollout of a session started in `cwd` after `since` (epoch ms) that no
/// other thread has claimed yet.
pub fn find_new(cwd: &str, since: i64, taken: &HashSet<PathBuf>) -> Option<(PathBuf, String)> {
    let mut files: Vec<PathBuf> = recent_days(&codex_sessions_dir(), 2)
        .into_iter()
        .flat_map(|d| fs::read_dir(d).into_iter().flatten().flatten().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl") && !taken.contains(p))
        .filter(|p| {
            let modified = fs::metadata(p)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            modified >= since - 2000
        })
        .collect();
    files.sort();
    files.into_iter().find_map(|p| {
        let (id, meta_cwd) = session_meta(&p)?;
        util::same_path(&meta_cwd, cwd).then_some((p, id))
    })
}

/// The newest rollout of a known session (resumed sessions may start a new file).
pub fn find_session(id: &str) -> Option<PathBuf> {
    usage::find_rollout(&codex_sessions_dir(), id)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Signal {
    Running,
    /// Turn finished; the agent's last message.
    Done(String),
    Limit(String),
    Error(String),
    Attention(String),
    Usage(AccountUsage),
    /// The first message the user typed (for the thread title).
    UserText(String),
}

/// Reads a rollout as it grows.
pub struct Rollout {
    pub path: PathBuf,
    offset: u64,
    pending: String,
}

impl Rollout {
    pub fn new(path: PathBuf) -> Self {
        Self { path, offset: 0, pending: String::new() }
    }

    /// Signals from lines written since the last call. Lines older than
    /// `since` (history of a resumed session) are skipped.
    pub fn poll(&mut self, since: i64) -> Vec<Signal> {
        let Ok(mut f) = fs::File::open(&self.path) else { return vec![] };
        let len = f.metadata().map(|m| m.len()).unwrap_or(0);
        if len < self.offset {
            self.offset = 0; // rewritten
            self.pending.clear();
        }
        if len == self.offset || f.seek(SeekFrom::Start(self.offset)).is_err() {
            return vec![];
        }
        let mut buf = Vec::new();
        if f.read_to_end(&mut buf).is_err() {
            return vec![];
        }
        self.offset += buf.len() as u64;
        self.pending.push_str(&String::from_utf8_lossy(&buf));
        let mut out = vec![];
        while let Some(nl) = self.pending.find('\n') {
            let line: String = self.pending.drain(..=nl).collect();
            if let Ok(v) = serde_json::from_str::<Value>(line.trim()) {
                let at = v["timestamp"].as_str().and_then(util::parse_iso_ms).unwrap_or(i64::MAX);
                if at >= since - 1000 {
                    out.extend(signal(&v));
                }
            }
        }
        out
    }
}

fn signal(v: &Value) -> Option<Signal> {
    if s(v, "type") != "event_msg" {
        return None;
    }
    let p = &v["payload"];
    match s(p, "type") {
        "task_started" => Some(Signal::Running),
        "task_complete" => {
            let err = s(&p["error"], "message");
            if err.is_empty() {
                Some(Signal::Done(s(p, "last_agent_message").to_string()))
            } else if is_limit(err) {
                Some(Signal::Limit(err.to_string()))
            } else {
                Some(Signal::Error(err.to_string()))
            }
        }
        "turn_aborted" => Some(Signal::Done(String::new())),
        "error" | "stream_error" => {
            let m = s(p, "message");
            if is_limit(m) {
                Some(Signal::Limit(m.to_string()))
            } else {
                None
            }
        }
        "token_count" => usage::from_codex_token_count(p).0.map(Signal::Usage),
        "exec_approval_request" | "apply_patch_approval_request" | "request_permissions" | "elicitation_request" => {
            Some(Signal::Attention(crate::i18n::tr("Codex minta izin", "Codex needs permission").into()))
        }
        "user_message" => Some(Signal::UserText(s(p, "message").to_string())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_new_lines_only_and_finds_limits() {
        let dir = std::env::temp_dir().join(format!("kilat-ro-{}", util::new_id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rollout-x.jsonl");
        let old = r#"{"timestamp":"2020-01-01T00:00:00Z","type":"event_msg","payload":{"type":"task_started"}}"#;
        fs::write(&path, format!("{old}\n")).unwrap();
        let mut r = Rollout::new(path.clone());
        let since = util::parse_iso_ms("2026-11-24T00:00:00Z").unwrap();
        assert!(r.poll(since).is_empty(), "history of a resumed session is skipped");
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        use std::io::Write;
        writeln!(f, r#"{{"timestamp":"2026-11-24T01:00:00Z","type":"event_msg","payload":{{"type":"task_started"}}}}"#).unwrap();
        write!(f, r#"{{"timestamp":"2026-11-24T01:00:05Z","type":"event_msg","payload":{{"type":"task_complete","error":{{"message":"You’ve hit your usage limit. Try again at 3:04 PM."}}}}}}"#).unwrap();
        assert_eq!(r.poll(since), vec![Signal::Running], "a half-written line waits");
        writeln!(f).unwrap();
        assert!(matches!(&r.poll(since)[..], [Signal::Limit(m)] if m.contains("3:04 PM")));
        assert!(!is_limit("Renamed add() to sum()"));
        drop(f);
        let _ = fs::remove_dir_all(&dir);
    }
}
