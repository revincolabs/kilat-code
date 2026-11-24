//! Small helpers shared by every module: process spawning without console
//! windows, executable lookup, ids, time and ANSI stripping.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// `Command::new` that never flashes a console window on Windows.
pub fn cmd(program: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(program);
    #[cfg(windows)]
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Short random id (12 hex chars) — enough for local objects.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..12].to_string()
}

/// Path with forward slashes; git and MSYS bash both accept `C:/x/y`.
pub fn fwd(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Find an executable on PATH, honouring PATHEXT on Windows.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
            .split(';')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(&path) {
        // Kilat's own gh shims (on PATH when Kilat is started from one of its
        // terminals) must never be taken for the real tools.
        if dir.join("kilat-git-cred.sh").is_file() {
            continue;
        }
        for e in &exts {
            let p = dir.join(format!("{name}{e}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

fn custom(p: Option<&str>) -> Option<PathBuf> {
    p.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_file())
}

/// macOS / Linux: an app started from the Dock, Finder or a desktop launcher
/// gets a bare PATH (no Homebrew, npm or ~/.local/bin), so `claude`, `codex`
/// and `gh` would not be found. Take PATH from the user's login shell.
#[cfg(unix)]
pub fn adopt_login_path() {
    use std::io::Read;
    use std::process::Stdio;
    let shell = std::env::var("SHELL").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "/bin/sh".into());
    let Ok(mut child) = Command::new(&shell)
        .args(["-ilc", "printf '__KILAT_PATH__%s__KILAT_END__' \"$PATH\""])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return;
    };
    let mut out = child.stdout.take();
    // A broken shell profile must not hang the app: give it 3 seconds.
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(o) = out.as_mut() {
            let _ = o.read_to_string(&mut s);
        }
        s
    });
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < std::time::Duration::from_secs(3) => {
                std::thread::sleep(std::time::Duration::from_millis(30))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        }
    }
    let Ok(text) = reader.join() else { return };
    let Some(path) = text
        .split("__KILAT_PATH__")
        .nth(1)
        .and_then(|r| r.split("__KILAT_END__").next())
        .filter(|p| !p.trim().is_empty())
    else {
        return;
    };
    // Keep anything the app already had that the shell did not list.
    let mut parts: Vec<String> = path.split(':').map(String::from).collect();
    for p in std::env::var("PATH").unwrap_or_default().split(':') {
        if !p.is_empty() && !parts.iter().any(|x| x == p) {
            parts.push(p.to_string());
        }
    }
    std::env::set_var("PATH", parts.join(":"));
}

#[cfg(not(unix))]
pub fn adopt_login_path() {}

/// Shell for the bash terminal panel. Windows: Git Bash (preferred), falling
/// back to any `bash` on PATH. macOS / Linux: the user's login shell.
pub fn resolve_bash(user: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = custom(user) {
        return Some(p);
    }
    if cfg!(unix) {
        return std::env::var_os("SHELL")
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .or_else(|| which("zsh"))
            .or_else(|| which("bash"))
            .or_else(|| Some(PathBuf::from("/bin/sh")));
    }
    let mut candidates: Vec<PathBuf> = vec![
        PathBuf::from(r"C:\Program Files\Git\bin\bash.exe"),
        PathBuf::from(r"C:\Program Files (x86)\Git\bin\bash.exe"),
    ];
    if let Some(local) = dirs::data_local_dir() {
        candidates.push(local.join("Programs").join("Git").join("bin").join("bash.exe"));
    }
    if let Some(git) = which("git") {
        // <Git>/cmd/git.exe -> <Git>/bin/bash.exe
        if let Some(root) = git.parent().and_then(Path::parent) {
            candidates.push(root.join("bin").join("bash.exe"));
        }
    }
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .or_else(|| which("bash"))
}

/// Claude Code CLI. npm installs a `.cmd` shim; prefer the native exe behind it.
pub fn resolve_claude(user: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = custom(user) {
        return Some(p);
    }
    // Native installer locations (checked first: not always on PATH).
    if let Some(home) = dirs::home_dir() {
        let native = if cfg!(windows) {
            vec![home.join(".local").join("bin").join("claude.exe")]
        } else {
            vec![
                home.join(".local").join("bin").join("claude"),
                home.join(".claude").join("local").join("claude"),
            ]
        };
        if let Some(p) = native.into_iter().find(|p| p.is_file()) {
            return Some(p);
        }
    }
    let p = which("claude")?;
    if !cfg!(windows) {
        return Some(p);
    }
    let is_exe = p
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("exe"));
    if !is_exe {
        if let Some(dir) = p.parent() {
            let native = dir
                .join("node_modules")
                .join("@anthropic-ai")
                .join("claude-code")
                .join("bin")
                .join("claude.exe");
            if native.is_file() {
                return Some(native);
            }
        }
    }
    Some(p)
}

pub fn resolve_codex(user: Option<&str>) -> Option<PathBuf> {
    custom(user).or_else(|| which("codex"))
}

pub fn resolve_gh(user: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = custom(user) {
        return Some(p);
    }
    which("gh").or_else(|| {
        [
            PathBuf::from(r"C:\Program Files\GitHub CLI\gh.exe"),
            PathBuf::from("/opt/homebrew/bin/gh"),
            PathBuf::from("/usr/local/bin/gh"),
        ]
        .into_iter()
        .find(|p| p.is_file())
    })
}

/// Kill a process and all of its children (agents spawn shells, node, etc).
pub fn kill_tree(pid: u32) {
    #[cfg(windows)]
    {
        let _ = cmd("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output();
    }
    #[cfg(not(windows))]
    {
        // Agents run in a PTY, which makes them the leader of their own
        // process group: kill the whole group (MCP servers, shells, …),
        // then the process itself in case it is not a group leader.
        let _ = cmd("kill").args(["-9", "--", &format!("-{pid}")]).output();
        let _ = cmd("kill").args(["-9", &pid.to_string()]).output();
    }
}

static ANSI_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\x1b\[[0-9;?<=>]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[@-Z\\-_]")
        .expect("ansi regex")
});

/// Strip ANSI escapes, leaving a space where each one was. ConPTY draws line
/// breaks and runs of blanks as cursor movements, so deleting them outright
/// glues neighbouring words together.
pub fn strip_ansi_spaced(s: &str) -> String {
    ANSI_RE.replace_all(s, " ").into_owned()
}

/// Decode bytes as UTF-8 while keeping an incomplete trailing sequence for
/// the next chunk (PTY / pipe reads can split multi-byte characters).
pub fn decode_utf8(pending: &mut Vec<u8>, chunk: &[u8]) -> String {
    pending.extend_from_slice(chunk);
    let mut out = String::new();
    loop {
        match std::str::from_utf8(pending) {
            Ok(s) => {
                out.push_str(s);
                pending.clear();
                return out;
            }
            Err(e) => {
                let valid = e.valid_up_to();
                out.push_str(std::str::from_utf8(&pending[..valid]).unwrap_or_default());
                match e.error_len() {
                    // Invalid bytes: replace and keep going.
                    Some(len) => {
                        out.push('\u{FFFD}');
                        pending.drain(..valid + len);
                    }
                    // Incomplete sequence at the end: wait for more bytes.
                    None => {
                        pending.drain(..valid);
                        return out;
                    }
                }
            }
        }
    }
}

/// Environment changes applied to a spawned process.
#[derive(Default, Clone, Debug)]
pub struct EnvPlan {
    pub set: Vec<(String, String)>,
    pub remove: Vec<String>,
}

impl EnvPlan {
    pub fn set(&mut self, k: &str, v: impl Into<String>) {
        self.set.push((k.to_string(), v.into()));
    }
    pub fn remove(&mut self, k: &str) {
        self.remove.push(k.to_string());
    }
    pub fn extend(&mut self, other: EnvPlan) {
        self.remove.extend(other.remove);
        self.set.extend(other.set);
    }
    pub fn apply(&self, c: &mut Command) {
        for k in &self.remove {
            c.env_remove(k);
        }
        for (k, v) in &self.set {
            c.env(k, v);
        }
    }
    pub fn apply_pty(&self, c: &mut portable_pty::CommandBuilder) {
        for k in &self.remove {
            c.env_remove(k);
        }
        for (k, v) in &self.set {
            c.env(k, v);
        }
    }
}

pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)
}

/// Error type returned to the frontend: a plain message string.
pub type CmdResult<T> = Result<T, String>;

pub trait ErrStr<T> {
    fn es(self) -> CmdResult<T>;
}

impl<T, E: std::fmt::Display> ErrStr<T> for Result<T, E> {
    fn es(self) -> CmdResult<T> {
        self.map_err(|e| e.to_string())
    }
}

/// Days since 1970-01-01 for a civil date (proleptic Gregorian).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// "2026-11-24T03:13:34.911Z" / "…+07:00" -> epoch milliseconds.
pub fn parse_iso_ms(s: &str) -> Option<i64> {
    let s = s.trim();
    let num = |a: usize, b: usize| s.get(a..b)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    if !matches!(s.as_bytes().get(10), Some(b'T') | Some(b' ')) {
        return None;
    }
    let (h, mi, sec) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    let mut rest = &s[19..];
    let mut ms = 0;
    if let Some(frac) = rest.strip_prefix('.') {
        let digits: String = frac.chars().take_while(|c| c.is_ascii_digit()).collect();
        rest = &frac[digits.len()..];
        ms = format!("{:0<3}", &digits[..digits.len().min(3)]).parse().unwrap_or(0);
    }
    let offset_min = match rest.chars().next() {
        None | Some('Z') | Some('z') => 0,
        Some(sign @ ('+' | '-')) => {
            let t = rest[1..].replace(':', "");
            let oh: i64 = t.get(0..2)?.parse().ok()?;
            let om: i64 = t.get(2..4).and_then(|x| x.parse().ok()).unwrap_or(0);
            let v = oh * 60 + om;
            if sign == '-' { -v } else { v }
        }
        _ => return None,
    };
    let days = days_from_civil(y, mo, d);
    Some(((days * 86_400 + h * 3600 + mi * 60 + sec - offset_min * 60) * 1000) + ms)
}

/// Epoch seconds or milliseconds (or an ISO string) -> milliseconds.
pub fn to_ms(v: &serde_json::Value) -> Option<i64> {
    match v {
        serde_json::Value::Number(n) => {
            let x = n.as_f64()?;
            Some(if x < 1e11 { (x * 1000.0) as i64 } else { x as i64 })
        }
        serde_json::Value::String(s) => s.parse::<f64>().ok().map(|x| if x < 1e11 { (x * 1000.0) as i64 } else { x as i64 }).or_else(|| parse_iso_ms(s)),
        _ => None,
    }
}

/// Same path, ignoring case and slash direction (Windows).
pub fn same_path(a: &str, b: &str) -> bool {
    let norm = |p: &str| p.replace('/', "\\").trim_end_matches('\\').to_lowercase();
    norm(a) == norm(b)
}

#[cfg(test)]
mod time_tests {
    use super::*;

    #[test]
    fn iso_times() {
        assert_eq!(parse_iso_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_iso_ms("2026-11-18T03:13:34.911Z"), Some(1_794_971_614_911));
        assert_eq!(parse_iso_ms("2026-11-18T10:13:34.911+07:00"), Some(1_794_971_614_911));
        assert_eq!(parse_iso_ms("nope"), None);
        assert_eq!(to_ms(&serde_json::json!(1794987600)), Some(1_794_987_600_000));
        assert_eq!(to_ms(&serde_json::json!(1794987600000_i64)), Some(1_794_987_600_000));
        assert!(same_path(r"C:\Proj\A\", "c:/proj/a"));
    }
}
