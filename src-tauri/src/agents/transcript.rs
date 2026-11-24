//! Context handoff between providers.
//!
//! The same provider simply resumes its own native session (session storage
//! is shared by all accounts, see accounts.rs). The other provider gets a
//! Markdown file with everything it has not seen yet, read from the native
//! transcripts (Claude: `~/.claude/projects/*/<id>.jsonl`, Codex: the rollout
//! file), and is told to read it first.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::store::{home_dir, default_codex_home, Segment, Store, Thread};
use crate::usage;
use crate::util::{self, now_ms};

const TOTAL_BUDGET: usize = 150_000;
const PER_MESSAGE: usize = 8_000;

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    User,
    Assistant,
    Action,
    Error,
}

#[derive(Clone, Debug)]
pub struct Entry {
    /// epoch ms (0 when unknown)
    pub at: i64,
    pub kind: Kind,
    pub text: String,
}

pub fn provider_name(p: &str) -> &'static str {
    match p {
        "claude" => "Claude Code",
        "codex" => "Codex (ChatGPT)",
        _ => "agent",
    }
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max * 2 / 3).collect();
    let tail_rev: String = s.chars().rev().take(max / 3).collect();
    let tail: String = tail_rev.chars().rev().collect();
    format!("{head}\n…[dipotong]…\n{tail}")
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

fn push(out: &mut Vec<Entry>, at: i64, kind: Kind, text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    // The same message often appears twice (event + item): keep one.
    if out.last().is_some_and(|e| e.kind == kind && e.text == text) {
        return;
    }
    out.push(Entry { at, kind, text: text.to_string() });
}

/// Text that is the CLI talking to the model, not the user.
fn is_injected(t: &str) -> bool {
    let t = t.trim_start();
    [
        "<system-reminder>",
        "<command-name>",
        "<command-message>",
        "<local-command-",
        "Caveat:",
        "<environment_context>",
        "<user_instructions>",
        "<skills_instructions>",
        "<permissions instructions>",
        "<turn_aborted>",
        "# AGENTS.md",
    ]
    .iter()
    .any(|p| t.starts_with(p))
}

fn claude_action(name: &str, input: &Value) -> String {
    let detail = match name {
        "Bash" | "PowerShell" => format!("`{}`", clip(s(input, "command"), 300)),
        "Read" | "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => {
            let p = s(input, "file_path");
            if p.is_empty() { s(input, "notebook_path").to_string() } else { p.to_string() }
        }
        "Glob" | "Grep" => s(input, "pattern").to_string(),
        "WebFetch" => s(input, "url").to_string(),
        "WebSearch" => s(input, "query").to_string(),
        "Task" | "Agent" => s(input, "description").to_string(),
        _ => String::new(),
    };
    if detail.is_empty() { name.to_string() } else { format!("{name}: {detail}") }
}

/// A Claude Code session transcript.
pub fn claude_entries(path: &Path) -> Vec<Entry> {
    let Ok(text) = std::fs::read_to_string(path) else { return vec![] };
    let mut out = vec![];
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        if v["isSidechain"] == true || v["isMeta"] == true {
            continue;
        }
        let at = v["timestamp"].as_str().and_then(util::parse_iso_ms).unwrap_or(0);
        let content = &v["message"]["content"];
        match s(&v, "type") {
            "user" => {
                if v.get("toolUseResult").is_some() {
                    continue;
                }
                if let Some(t) = content.as_str() {
                    let trimmed = t.trim_start();
                    // `!command` typed by the user in the terminal (bash mode).
                    if let Some(cmd) = trimmed
                        .strip_prefix("<bash-input>")
                        .and_then(|r| r.split("</bash-input>").next())
                    {
                        push(&mut out, at, Kind::User, &crate::trf!("(menjalankan di terminal) `{}`", "(ran in the terminal) `{}`", clip(cmd.trim(), 300)));
                    } else if !is_injected(t) && !trimmed.starts_with("<bash-std") {
                        push(&mut out, at, Kind::User, t);
                    }
                } else if let Some(blocks) = content.as_array() {
                    let mut parts = vec![];
                    for b in blocks {
                        match s(b, "type") {
                            "text" if !is_injected(s(b, "text")) => parts.push(s(b, "text").to_string()),
                            "image" => parts.push("(gambar terlampir)".into()),
                            _ => {}
                        }
                    }
                    push(&mut out, at, Kind::User, &parts.join("\n"));
                }
            }
            "assistant" => {
                let Some(blocks) = content.as_array() else { continue };
                let error = v["isApiErrorMessage"] == true;
                for b in blocks {
                    match s(b, "type") {
                        "text" if error => push(&mut out, at, Kind::Error, s(b, "text")),
                        "text" => push(&mut out, at, Kind::Assistant, s(b, "text")),
                        "tool_use" => push(&mut out, at, Kind::Action, &claude_action(s(b, "name"), &b["input"])),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn content_text(content: &Value) -> String {
    match content {
        Value::String(t) => t.clone(),
        Value::Array(items) => items
            .iter()
            .filter(|c| matches!(s(c, "type"), "text" | "input_text" | "output_text"))
            .map(|c| s(c, "text"))
            .filter(|t| !is_injected(t))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn codex_command(v: &Value) -> String {
    match v {
        Value::Array(a) => {
            let parts: Vec<&str> = a.iter().filter_map(Value::as_str).collect();
            // ["powershell", "-Command", "npm test"] -> "npm test"
            parts.last().map(|x| x.to_string()).unwrap_or_default()
        }
        Value::String(t) => t.clone(),
        _ => String::new(),
    }
}

/// A Codex rollout file (`sessions/YYYY/MM/DD/rollout-…-<id>.jsonl`).
pub fn codex_entries(path: &Path) -> Vec<Entry> {
    let Ok(text) = std::fs::read_to_string(path) else { return vec![] };
    // Messages appear both as events and as model input/output items; the
    // events are preferred, the items are the fallback for older files.
    let (mut events, mut items) = (vec![], vec![]);
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let at = v["timestamp"].as_str().and_then(util::parse_iso_ms).unwrap_or(0);
        let p = &v["payload"];
        match (s(&v, "type"), s(p, "type")) {
            ("event_msg", "user_message") => {
                let t = s(p, "message");
                if !is_injected(t) {
                    push(&mut events, at, Kind::User, t);
                }
            }
            ("event_msg", "agent_message") => push(&mut events, at, Kind::Assistant, s(p, "message")),
            ("event_msg", "item_completed") => match s(&p["item"], "type") {
                "UserMessage" => push(&mut events, at, Kind::User, &content_text(&p["item"]["content"])),
                "AgentMessage" => push(&mut events, at, Kind::Assistant, &content_text(&p["item"]["content"])),
                _ => {}
            },
            ("event_msg", "exec_command_begin") => {
                let c = codex_command(&p["command"]);
                push(&mut events, at, Kind::Action, &format!("ran `{}`", clip(&c, 300)));
            }
            ("event_msg", "patch_apply_begin") => {
                let files: Vec<String> = p["changes"].as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default();
                if !files.is_empty() {
                    push(&mut events, at, Kind::Action, &format!("edited {}", files.join(", ")));
                }
            }
            ("event_msg", "task_complete") => {
                push(&mut events, at, Kind::Error, s(&p["error"], "message"));
            }
            ("event_msg", "error") => push(&mut events, at, Kind::Error, s(p, "message")),
            ("response_item", "message") => {
                let kind = match s(p, "role") {
                    "user" => Kind::User,
                    "assistant" => Kind::Assistant,
                    _ => continue,
                };
                push(&mut items, at, kind, &content_text(&p["content"]));
            }
            ("response_item", "function_call") | ("response_item", "custom_tool_call") => {
                push(&mut items, at, Kind::Action, &format!("used {}", s(p, "name")));
            }
            _ => {}
        }
    }
    let has_messages = events.iter().any(|e| matches!(e.kind, Kind::User | Kind::Assistant));
    if has_messages { events } else { items }
}

fn claude_projects_dir() -> PathBuf {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".claude"))
        .join("projects")
}

/// Where a Claude session's transcript lives.
pub fn claude_transcript(id: &str, known: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = known.map(PathBuf::from).filter(|p| p.is_file()) {
        return Some(p);
    }
    let name = format!("{id}.jsonl");
    std::fs::read_dir(claude_projects_dir())
        .ok()?
        .flatten()
        .map(|e| e.path().join(&name))
        .find(|p| p.is_file())
}

pub fn codex_sessions_dir() -> PathBuf {
    default_codex_home().join("sessions")
}

fn entries_of(thread: &Thread, provider: &str, id: &str) -> Vec<Entry> {
    match provider {
        "claude" => {
            let known = thread
                .native
                .claude
                .as_ref()
                .filter(|n| n.id == id)
                .and_then(|n| n.transcript.clone());
            claude_transcript(id, known.as_deref()).map(|p| claude_entries(&p)).unwrap_or_default()
        }
        "codex" => usage::find_rollout(&codex_sessions_dir(), id)
            .map(|p| codex_entries(&p))
            .unwrap_or_default(),
        _ => vec![],
    }
}

/// Segments whose work the target provider has not seen. `resumed`: the
/// native session of the target that will be resumed (None = a new one).
pub fn unseen(thread: &Thread, target: &str, resumed: Option<&str>) -> Vec<Segment> {
    if thread.segments.is_empty() {
        // Threads from the chat-UI era: the other provider's whole session.
        return ["claude", "codex"]
            .iter()
            .filter(|p| **p != target)
            .filter_map(|p| {
                let n = thread.native.get(p)?;
                (!n.id.is_empty()).then(|| Segment {
                    provider: p.to_string(),
                    native_id: n.id.clone(),
                    ..Default::default()
                })
            })
            .collect();
    }
    let start = resumed
        .and_then(|id| {
            thread
                .segments
                .iter()
                .rposition(|s| s.provider == target && s.native_id == id)
        })
        .map(|i| i + 1)
        .unwrap_or(0);
    thread.segments[start..]
        .iter()
        .filter(|s| !s.native_id.is_empty())
        .filter(|s| !(s.provider == target && Some(s.native_id.as_str()) == resumed))
        .cloned()
        .collect()
}

struct Part {
    who: String,
    entries: Vec<Entry>,
}

fn label_of(store: &Store, account_id: &str) -> String {
    store.account(account_id).map(|a| a.label).unwrap_or_default()
}

/// Group consecutive segments of one native session and read their part of
/// the transcript.
fn parts(store: &Store, thread: &Thread, segs: &[Segment]) -> Vec<Part> {
    let mut out: Vec<Part> = vec![];
    let mut i = 0;
    while i < segs.len() {
        let first = &segs[i];
        let mut j = i;
        while j + 1 < segs.len() && segs[j + 1].provider == first.provider && segs[j + 1].native_id == first.native_id {
            j += 1;
        }
        let (from, to) = (first.started_at, segs[j].ended_at.unwrap_or_else(now_ms));
        let entries: Vec<Entry> = entries_of(thread, &first.provider, &first.native_id)
            .into_iter()
            .filter(|e| e.at == 0 || from == 0 || (e.at >= from - 5000 && e.at <= to + 5000))
            .collect();
        let mut accounts: Vec<String> = segs[i..=j].iter().map(|s| label_of(store, &s.account_id)).filter(|l| !l.is_empty()).collect();
        accounts.dedup();
        let who = if accounts.is_empty() {
            provider_name(&first.provider).to_string()
        } else {
            crate::trf!("{} (akun {})", "{} (account {})", provider_name(&first.provider), accounts.join(", "))
        };
        if !entries.is_empty() {
            out.push(Part { who, entries });
        }
        i = j + 1;
    }
    out
}

fn render_blocks(parts: &[Part]) -> Vec<String> {
    let mut blocks = vec![];
    for p in parts {
        let mut actions: Vec<String> = vec![];
        let flush = |actions: &mut Vec<String>, blocks: &mut Vec<String>| {
            if actions.is_empty() {
                return;
            }
            let shown: Vec<String> = actions.iter().take(30).map(|a| format!("- {a}")).collect();
            let more = actions.len().saturating_sub(30);
            let mut b = crate::trf!("_Aksi {}:_\n{}", "_Actions by {}:_\n{}", p.who, shown.join("\n"));
            if more > 0 {
                b.push_str(&crate::trf!("\n- …dan {more} aksi lain", "\n- …and {more} more actions"));
            }
            blocks.push(b);
            actions.clear();
        };
        for e in &p.entries {
            match e.kind {
                Kind::Action => actions.push(e.text.replace('\n', " ")),
                Kind::User => {
                    flush(&mut actions, &mut blocks);
                    blocks.push(crate::trf!("### 👤 Pengguna\n{}", "### 👤 User\n{}", clip(&e.text, PER_MESSAGE)));
                }
                Kind::Assistant => {
                    flush(&mut actions, &mut blocks);
                    blocks.push(format!("### 🤖 {}\n{}", p.who, clip(&e.text, PER_MESSAGE)));
                }
                Kind::Error => {
                    flush(&mut actions, &mut blocks);
                    blocks.push(format!("> ⚠ {}", clip(&e.text, 600).replace('\n', " ")));
                }
            }
        }
        flush(&mut actions, &mut blocks);
    }
    blocks
}

/// Render the handoff document (most recent blocks win when it is long).
pub fn render(parts_in: &[(String, Vec<Entry>)], cwd: &str, to: &str) -> String {
    let parts: Vec<Part> = parts_in
        .iter()
        .map(|(who, entries)| Part { who: who.clone(), entries: entries.clone() })
        .collect();
    let blocks = render_blocks(&parts);
    let mut kept: Vec<&String> = vec![];
    let mut used = 0;
    for b in blocks.iter().rev() {
        if used + b.len() > TOTAL_BUDGET && !kept.is_empty() {
            break;
        }
        used += b.len();
        kept.push(b);
    }
    let skipped = blocks.len() - kept.len();
    kept.reverse();
    let from: Vec<&str> = parts.iter().map(|p| p.who.as_str()).collect();
    let mut doc = crate::trf!(
        "# Riwayat percakapan sebelumnya\n\n\
Thread ini dikerjakan oleh {} lalu dilanjutkan oleh {}. Di bawah ini semua yang terjadi \
yang belum kamu lihat (paling lama di atas): pesan pengguna, jawaban agent, dan aksinya.\n\n\
Folder kerja: `{cwd}`\n\n---\n\n",
        "# Earlier conversation\n\n\
This thread was handled by {} and is now continued by {}. Below is everything that happened \
that you haven't seen yet (oldest first): the user's messages, the agent's replies, and its actions.\n\n\
Working folder: `{cwd}`\n\n---\n\n",
        from.join(" → "),
        provider_name(to),
    );
    if skipped > 0 {
        doc.push_str(&crate::trf!("_({skipped} bagian lama dilewati karena terlalu panjang)_\n\n", "_({skipped} older parts skipped because the history is long)_\n\n"));
    }
    doc.push_str(&kept.iter().map(|b| b.as_str()).collect::<Vec<_>>().join("\n\n"));
    doc.push('\n');
    doc
}

pub struct Handoff {
    pub path: PathBuf,
    /// Providers whose work is in the file.
    pub from: Vec<String>,
}

/// Write the history `target` has not seen yet to a file. None when there
/// is nothing to hand over.
pub fn write_handoff(store: &Store, thread: &Thread, target: &str, resumed: Option<&str>) -> Option<Handoff> {
    let segs = unseen(thread, target, resumed);
    let parts = parts(store, thread, &segs);
    if parts.is_empty() {
        return None;
    }
    let mut from: Vec<String> = segs.iter().map(|s| s.provider.clone()).collect();
    from.dedup();
    let doc = render(
        &parts.into_iter().map(|p| (p.who, p.entries)).collect::<Vec<_>>(),
        &thread.meta.cwd,
        target,
    );
    let path = store.handoff_dir(&thread.meta.id).join(format!("{}.md", now_ms()));
    util::write_atomic(&path, doc.as_bytes()).ok()?;
    Some(Handoff { path, from })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{NativeSession, NativeSessions};

    fn tmp(name: &str, text: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kilat-tr-{}", util::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, text).unwrap();
        p
    }

    #[test]
    fn reads_claude_transcript() {
        let lines = [
            r#"{"type":"user","timestamp":"2026-11-24T01:00:00Z","message":{"role":"user","content":"rahasianya BANYUWANGI"}}"#,
            r#"{"type":"user","isMeta":true,"message":{"content":"<system-reminder>x</system-reminder>"}}"#,
            r#"{"type":"assistant","timestamp":"2026-11-24T01:00:05Z","message":{"content":[{"type":"thinking","thinking":"hm"},{"type":"text","text":"Siap"},{"type":"tool_use","name":"Bash","input":{"command":"npm test"}}]}}"#,
            r#"{"type":"user","toolUseResult":{},"message":{"content":[{"type":"tool_result","content":"ok"}]}}"#,
            r#"{"type":"assistant","isApiErrorMessage":true,"message":{"content":[{"type":"text","text":"You've hit your limit · resets 3pm"}]}}"#,
            r#"{"type":"assistant","isSidechain":true,"message":{"content":[{"type":"text","text":"subagent"}]}}"#,
            r#"{"type":"user","message":{"content":"<bash-input>git status</bash-input>"}}"#,
            r#"{"type":"user","message":{"content":"<bash-stdout>clean</bash-stdout><bash-stderr></bash-stderr>"}}"#,
        ];
        let p = tmp("s.jsonl", &lines.join("\n"));
        let e = claude_entries(&p);
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
        let kinds: Vec<&Kind> = e.iter().map(|x| &x.kind).collect();
        assert_eq!(kinds, [&Kind::User, &Kind::Assistant, &Kind::Action, &Kind::Error, &Kind::User]);
        assert_eq!(e[2].text, "Bash: `npm test`");
        assert_eq!(e[4].text, "(menjalankan di terminal) `git status`");
        assert!(e[0].at > 0);
    }

    #[test]
    fn reads_codex_rollout() {
        let lines = [
            r#"{"timestamp":"2026-11-24T02:00:00Z","type":"session_meta","payload":{"id":"x","cwd":"C:\\p"}}"#,
            r#"{"timestamp":"2026-11-24T02:00:01Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>..."}]}}"#,
            r#"{"timestamp":"2026-11-24T02:00:01Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"ganti nama fungsi"}]}}"#,
            r#"{"timestamp":"2026-11-24T02:00:01Z","type":"event_msg","payload":{"type":"user_message","message":"ganti nama fungsi"}}"#,
            r#"{"timestamp":"2026-11-24T02:00:02Z","type":"event_msg","payload":{"type":"exec_command_begin","command":["powershell","-Command","rg add"]}}"#,
            r#"{"timestamp":"2026-11-24T02:00:03Z","type":"event_msg","payload":{"type":"agent_message","message":"add() jadi sum()"}}"#,
            r#"{"timestamp":"2026-11-24T02:00:03Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"add() jadi sum()"}]}}"#,
            r#"{"timestamp":"2026-11-24T02:00:04Z","type":"event_msg","payload":{"type":"task_complete","error":{"message":"You’ve hit your usage limit."}}}"#,
        ];
        let p = tmp("rollout-x.jsonl", &lines.join("\n"));
        let e = codex_entries(&p);
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
        let texts: Vec<&str> = e.iter().map(|x| x.text.as_str()).collect();
        assert_eq!(texts, ["ganti nama fungsi", "ran `rg add`", "add() jadi sum()", "You’ve hit your usage limit."]);
    }

    fn seg(p: &str, id: &str, a: i64, b: Option<i64>) -> Segment {
        Segment { provider: p.into(), account_id: String::new(), native_id: id.into(), started_at: a, ended_at: b }
    }

    #[test]
    fn only_unseen_segments_are_handed_over() {
        let t = Thread {
            segments: vec![
                seg("claude", "c1", 1, Some(2)),
                seg("codex", "x1", 2, Some(3)),
                seg("claude", "c1", 3, Some(4)),
                seg("codex", "x1", 4, Some(5)),
            ],
            ..Default::default()
        };
        // Back to Claude's session c1: only the last Codex stretch is new.
        let u = unseen(&t, "claude", Some("c1"));
        assert_eq!(u.len(), 1);
        assert_eq!((u[0].provider.as_str(), u[0].started_at), ("codex", 4));
        // A brand new Claude session gets everything.
        assert_eq!(unseen(&t, "claude", None).len(), 4);
        // Codex resuming x1: the Claude stretch after it.
        let u = unseen(&t, "codex", Some("x1"));
        assert!(u.is_empty(), "codex saw everything up to its last stretch");
    }

    #[test]
    fn chat_era_thread_hands_over_whole_session() {
        let t = Thread {
            native: NativeSessions {
                claude: Some(NativeSession { id: "c1".into(), transcript: None }),
                codex: None,
            },
            ..Default::default()
        };
        let u = unseen(&t, "codex", None);
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].native_id, "c1");
        assert!(unseen(&t, "claude", Some("c1")).is_empty());
    }

    #[test]
    fn render_keeps_recent_and_names_agents() {
        let entries = vec![
            Entry { at: 1, kind: Kind::User, text: "tugas A".into() },
            Entry { at: 2, kind: Kind::Action, text: "Bash: `ls`".into() },
            Entry { at: 3, kind: Kind::Assistant, text: "selesai A".into() },
        ];
        let doc = render(&[("Claude Code (akun Kerja)".into(), entries)], "C:/p", "codex");
        assert!(doc.contains("### 👤 Pengguna\ntugas A"));
        assert!(doc.contains("- Bash: `ls`"));
        assert!(doc.contains("### 🤖 Claude Code (akun Kerja)\nselesai A"));
        assert!(doc.contains("dilanjutkan oleh Codex (ChatGPT)"));
    }
}
