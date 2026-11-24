//! File explorer + editor I/O, and git helpers for the Changes panel.

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::util;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

const HIDDEN: &[&str] = &[".git", "node_modules", "target", ".DS_Store"];

pub fn list_dir(path: &str) -> anyhow::Result<Vec<Entry>> {
    let mut out: Vec<Entry> = fs::read_dir(path)?
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if HIDDEN.contains(&name.as_str()) {
                return None;
            }
            let is_dir = e.file_type().ok()?.is_dir();
            Some(Entry {
                path: e.path().to_string_lossy().to_string(),
                name,
                is_dir,
            })
        })
        .collect();
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

pub fn read_file(path: &str) -> anyhow::Result<String> {
    let meta = fs::metadata(path)?;
    if meta.len() > 8 * 1024 * 1024 {
        anyhow::bail!("File terlalu besar untuk dibuka (> 8 MB)");
    }
    let bytes = fs::read(path)?;
    if bytes.iter().take(8000).any(|b| *b == 0) {
        anyhow::bail!("File biner tidak bisa dibuka di editor");
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn write_file(path: &str, content: &str) -> anyhow::Result<()> {
    if let Some(dir) = Path::new(path).parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, content)?;
    Ok(())
}

// ------------------------------------------------------------------ git

fn git(cwd: &str, args: &[&str]) -> anyhow::Result<String> {
    let out = util::cmd("git").args(args).current_dir(cwd).output()?;
    if !out.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub is_repo: bool,
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<GitFile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFile {
    pub path: String,
    pub status: String,
}

pub fn status(cwd: &str) -> GitStatus {
    let Ok(text) = git(cwd, &["status", "--porcelain=v1", "-b", "-uall"]) else {
        return GitStatus::default();
    };
    let mut st = GitStatus {
        is_repo: true,
        ..Default::default()
    };
    let re = regex::Regex::new(r"\[(?:ahead (\d+))?(?:, )?(?:behind (\d+))?\]").unwrap();
    for line in text.lines() {
        if let Some(head) = line.strip_prefix("## ") {
            let name = head.split("...").next().unwrap_or(head);
            st.branch = name
                .trim_start_matches("No commits yet on ")
                .split(' ')
                .next()
                .unwrap_or("")
                .to_string();
            if let Some(c) = re.captures(head) {
                st.ahead = c.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                st.behind = c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            }
        } else if line.len() > 3 {
            let path = line[3..].rsplit(" -> ").next().unwrap_or("").trim_matches('"');
            st.files.push(GitFile {
                status: line[..2].trim().to_string(),
                path: path.to_string(),
            });
        }
    }
    st
}

pub fn diff(cwd: &str, path: &str) -> anyhow::Result<String> {
    let tracked = git(cwd, &["ls-files", "--error-unmatch", "--", path]).is_ok();
    if tracked {
        let has_head = git(cwd, &["rev-parse", "--verify", "HEAD"]).is_ok();
        let base = if has_head { "HEAD" } else { "--cached" };
        let d = git(cwd, &["diff", base, "--", path])?;
        if !d.trim().is_empty() || !has_head {
            return Ok(d);
        }
        return git(cwd, &["diff", "--", path]);
    }
    // Untracked: show the whole file as added.
    let out = util::cmd("git")
        .args(["diff", "--no-index", "--", "/dev/null", path])
        .current_dir(cwd)
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Create a git worktree for a thread so parallel agents don't collide.
pub fn add_worktree(repo: &str, dest: &Path, branch: &str) -> anyhow::Result<()> {
    if git(repo, &["rev-parse", "--verify", "HEAD"]).is_err() {
        anyhow::bail!("Repo belum punya commit — worktree butuh minimal 1 commit");
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    git(
        repo,
        &["worktree", "add", "-b", branch, &dest.to_string_lossy()],
    )?;
    Ok(())
}

pub fn remove_worktree(repo: &str, dest: &str) {
    let _ = git(repo, &["worktree", "remove", "--force", dest]);
}

pub fn is_repo(path: &str) -> bool {
    git(path, &["rev-parse", "--is-inside-work-tree"]).is_ok()
}
