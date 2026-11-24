//! Commit / push / branch / pull request from the Changes panel.
//!
//! Every command runs with the project's git context (see gitctx.rs), so the
//! commit author, the push credentials and `gh` all belong to the GitHub
//! account selected for that project. Nothing may prompt: there is no
//! terminal, so prompts are disabled and every call has a timeout.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::store::Store;
use crate::util::{self, EnvPlan};
use crate::{accounts, gitctx};

const SHORT: Duration = Duration::from_secs(30);
const NETWORK: Duration = Duration::from_secs(180);

/// Run a command; kill it (and its children) after `timeout`.
fn run(mut c: Command, input: Option<&[u8]>, timeout: Duration) -> anyhow::Result<String> {
    c.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn()?;
    if let (Some(data), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin.write_all(data)?;
    }
    let done = Arc::new(AtomicBool::new(false));
    let timed_out = Arc::new(AtomicBool::new(false));
    {
        let (done, timed_out, pid) = (done.clone(), timed_out.clone(), child.id());
        std::thread::spawn(move || {
            let step = Duration::from_millis(200);
            let mut waited = Duration::ZERO;
            while waited < timeout {
                if done.load(Ordering::SeqCst) {
                    return;
                }
                std::thread::sleep(step);
                waited += step;
            }
            if !done.load(Ordering::SeqCst) {
                timed_out.store(true, Ordering::SeqCst);
                util::kill_tree(pid);
            }
        });
    }
    let out = child.wait_with_output()?;
    done.store(true, Ordering::SeqCst);
    if timed_out.load(Ordering::SeqCst) {
        anyhow::bail!("Waktu habis ({} detik). Cek koneksi atau login akun.", timeout.as_secs());
    }
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        anyhow::bail!("{}", if err.is_empty() { stdout.trim().to_string() } else { err });
    }
    Ok(stdout)
}

fn env_for(store: &Store, project_id: &str) -> EnvPlan {
    let mut env = gitctx::ctx_env(store, project_id);
    env.set("GIT_TERMINAL_PROMPT", "0");
    env.set("GCM_INTERACTIVE", "never");
    env.set("GH_PROMPT_DISABLED", "1");
    env.set("GH_NO_UPDATE_NOTIFIER", "1");
    env
}

fn git(store: &Store, project_id: &str, cwd: &str, args: &[&str], input: Option<&[u8]>, timeout: Duration) -> anyhow::Result<String> {
    let mut c = util::cmd("git");
    c.args(["-c", "core.quotepath=off"]).args(args).current_dir(cwd);
    env_for(store, project_id).apply(&mut c);
    run(c, input, timeout)
}

/// Commit everything (or only `paths`). The message is passed on stdin so
/// it can span several lines.
pub fn commit(store: &Store, project_id: &str, cwd: &str, message: &str, paths: &[String]) -> anyhow::Result<String> {
    if message.trim().is_empty() {
        anyhow::bail!("Pesan commit masih kosong");
    }
    if paths.is_empty() {
        git(store, project_id, cwd, &["add", "-A"], None, SHORT)?;
    } else {
        let mut args = vec!["add", "-A", "--"];
        args.extend(paths.iter().map(String::as_str));
        git(store, project_id, cwd, &args, None, SHORT)?;
    }
    // Hooks (lint, tests) may take a while.
    git(store, project_id, cwd, &["commit", "-F", "-"], Some(message.as_bytes()), NETWORK)?;
    Ok(git(store, project_id, cwd, &["log", "-1", "--format=%h %s"], None, SHORT)?.trim().to_string())
}

/// Push the current branch (sets the upstream the first time).
pub fn push(store: &Store, project_id: &str, cwd: &str) -> anyhow::Result<String> {
    let has_upstream = git(store, project_id, cwd, &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"], None, SHORT).is_ok();
    let args: &[&str] = if has_upstream { &["push"] } else { &["push", "-u", "origin", "HEAD"] };
    git(store, project_id, cwd, args, None, NETWORK)?;
    Ok(current_branch(store, project_id, cwd))
}

fn current_branch(store: &Store, project_id: &str, cwd: &str) -> String {
    git(store, project_id, cwd, &["branch", "--show-current"], None, SHORT)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

pub fn create_branch(store: &Store, project_id: &str, cwd: &str, name: &str) -> anyhow::Result<()> {
    let name = name.trim();
    if name.is_empty() || name.contains(char::is_whitespace) {
        anyhow::bail!("Nama branch tidak boleh kosong atau berisi spasi");
    }
    git(store, project_id, cwd, &["switch", "-c", name], None, SHORT)?;
    Ok(())
}

/// Push, then open a pull request with `gh`. Returns the PR URL.
pub fn create_pr(store: &Store, project_id: &str, cwd: &str, title: &str, body: &str, draft: bool) -> anyhow::Result<String> {
    let gh = store
        .tools()
        .gh
        .ok_or_else(|| anyhow::anyhow!("GitHub CLI (gh) tidak ditemukan"))?;
    let branch = push(store, project_id, cwd)?;
    if branch.is_empty() {
        anyhow::bail!("Tidak sedang di sebuah branch");
    }
    let mut env = env_for(store, project_id);
    // Act as the project's GitHub account (not gh's default one).
    let login = store
        .project(project_id)
        .and_then(|p| p.github_account_id)
        .and_then(|id| store.account(&id))
        .and_then(|a| a.login);
    if let Some(tok) = login.and_then(|l| accounts::gh_token(&gh, &l)) {
        env.set("GH_TOKEN", tok);
    }
    let mut c = util::cmd(&gh);
    c.args(["pr", "create", "--head", &branch, "--title", title.trim(), "--body", body]);
    if draft {
        c.arg("--draft");
    }
    c.current_dir(cwd);
    env.apply(&mut c);
    let out = run(c, None, NETWORK)?;
    let url = out
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with("https://"))
        .unwrap_or(out.trim())
        .trim()
        .to_string();
    Ok(url)
}

fn clip(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let cut = (0..=max).rev().find(|i| s.is_char_boundary(*i)).unwrap_or(0);
    format!("{}\n…[dipotong]", &s[..cut])
}

fn strip_fences(s: &str) -> String {
    let t = s.trim();
    let t = t.strip_prefix("```").map(|r| r.split_once('\n').map(|x| x.1).unwrap_or(r)).unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    t.trim().trim_matches('"').trim().to_string()
}

/// Ask an agent (one quick call, no tools) for a commit message.
pub fn suggest_message(store: &Store, project_id: &str, cwd: &str, provider: &str, account_id: &str) -> anyhow::Result<String> {
    let g = |args: &[&str]| git(store, project_id, cwd, args, None, SHORT).unwrap_or_default();
    let has_head = !g(&["rev-parse", "--verify", "-q", "HEAD"]).trim().is_empty();
    let base = if has_head { "HEAD" } else { "--cached" };
    let stat = g(&["diff", base, "--stat"]);
    let diff = g(&["diff", base]);
    let untracked = g(&["ls-files", "-o", "--exclude-standard"]);
    if stat.trim().is_empty() && untracked.trim().is_empty() {
        anyhow::bail!("Tidak ada perubahan untuk di-commit");
    }
    let untracked: String = untracked.lines().take(60).collect::<Vec<_>>().join("\n");
    let recent = g(&["log", "-8", "--format=%s"]);
    let prompt = format!(
        "Write a git commit message for the changes below.\n\
Rules: one summary line (max 72 characters, imperative mood); if it helps, a blank line and a few short bullet points. \
Match the language and style of the recent commit messages when there are any. \
Output only the commit message, no code fences, no commentary.\n\n\
Recent commits:\n{}\n\nNew (untracked) files:\n{}\n\nChanges:\n{}\n\n{}",
        if recent.trim().is_empty() { "(none)" } else { recent.trim() },
        if untracked.trim().is_empty() { "(none)" } else { untracked.trim() },
        stat.trim(),
        clip(&diff, 40_000)
    );
    let scratch = store.dir.join("scratch");
    std::fs::create_dir_all(&scratch)?;
    let env = accounts::agent_env(store, account_id)?;
    let text = match provider {
        "codex" => {
            let exe = store.tools().codex.ok_or_else(|| anyhow::anyhow!("Codex CLI tidak ditemukan"))?;
            let mut c = util::cmd(&exe);
            c.args(["exec", "--json", "--skip-git-repo-check", "-s", "read-only", "-c", "model_reasoning_effort=\"low\"", "-"])
                .current_dir(&scratch);
            env.apply(&mut c);
            let out = run(c, Some(prompt.as_bytes()), NETWORK)?;
            out.lines()
                .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                .filter(|v| v["type"] == "item.completed" && v["item"]["type"] == "agent_message")
                .filter_map(|v| v["item"]["text"].as_str().map(String::from))
                .last()
                .unwrap_or_default()
        }
        _ => {
            let exe = store.tools().claude.ok_or_else(|| anyhow::anyhow!("Claude Code CLI tidak ditemukan"))?;
            let mut c = util::cmd(&exe);
            c.args([
                "-p", "--model", "haiku", "--output-format", "text", "--tools", "",
                "--no-session-persistence", "--setting-sources", "", "--strict-mcp-config",
            ])
            .current_dir(&scratch);
            env.apply(&mut c);
            run(c, Some(prompt.as_bytes()), NETWORK)?
        }
    };
    let msg = strip_fences(&text);
    if msg.is_empty() {
        anyhow::bail!("Agent tidak memberi pesan commit");
    }
    Ok(msg)
}

/// Project files for @-mentions and quick open: relative paths, `/`
/// separators. Git repos list tracked + untracked-but-not-ignored files.
pub fn list_files(root: &str) -> Vec<String> {
    const MAX: usize = 50_000;
    let out = util::cmd("git")
        .args(["-c", "core.quotepath=off", "ls-files", "-co", "--exclude-standard", "-z"])
        .current_dir(root)
        .stdin(Stdio::null())
        .output();
    if let Ok(o) = out {
        if o.status.success() {
            let mut v: Vec<String> = String::from_utf8_lossy(&o.stdout)
                .split('\0')
                .filter(|s| !s.is_empty())
                .take(MAX)
                .map(String::from)
                .collect();
            v.sort();
            v.dedup();
            return v;
        }
    }
    let skip = [".git", "node_modules", "target", "dist", "build", ".next", ".venv", "venv", "__pycache__", ".cache"];
    let base = PathBuf::from(root);
    let mut stack = vec![base.clone()];
    let mut files = vec![];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            let name = e.file_name().to_string_lossy().to_string();
            if ft.is_dir() {
                if !skip.contains(&name.as_str()) {
                    stack.push(e.path());
                }
            } else if let Ok(rel) = e.path().strip_prefix(&base) {
                files.push(rel.to_string_lossy().replace('\\', "/"));
                if files.len() >= MAX {
                    files.sort();
                    return files;
                }
            }
        }
    }
    files.sort();
    files
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    pub subject: String,
    pub when: String,
}

/// The last few commits (shown under the commit box).
pub fn recent(cwd: &str) -> Vec<Commit> {
    let out = util::cmd("git")
        .args(["log", "-5", "--format=%h%x1f%s%x1f%cr"])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .output();
    let Ok(o) = out else { return vec![] };
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .filter_map(|l| {
            let mut p = l.split('\u{1f}');
            Some(Commit { hash: p.next()?.into(), subject: p.next()?.into(), when: p.next().unwrap_or("").into() })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_agent_output() {
        assert_eq!(strip_fences("```\nFix login bug\n```"), "Fix login bug");
        assert_eq!(strip_fences("```text\nAdd x\n\n- y\n```\n"), "Add x\n\n- y");
        assert_eq!(strip_fences("\"Update docs\""), "Update docs");
    }
}
