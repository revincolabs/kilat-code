//! Per-project GitHub identity that can be switched while terminals and
//! agents are running — without logging in again.
//!
//! Every process spawned for a project gets:
//!   GIT_CONFIG_GLOBAL = <data>/gitctx/<project>/gitconfig
//!   KILAT_GITCTX      = <data>/gitctx/<project>
//!   PATH              = <data>/bin;<PATH>      (gh shims)
//!
//! The generated gitconfig includes the user's real ~/.gitconfig, then sets
//! user.name/email and a credential helper that asks `gh` for the selected
//! account's token. The `gh` shims read `$KILAT_GITCTX/account` on every call.
//! Switching account only rewrites those two files, so it applies instantly
//! to already-open terminals and running agents.

use std::fs;
use std::path::{Path, PathBuf};

use crate::store::{home_dir, Store};
use crate::util::{fwd, EnvPlan};

pub fn bin_dir(store: &Store) -> PathBuf {
    store.dir.join("bin")
}

pub fn ctx_dir(store: &Store, project_id: &str) -> PathBuf {
    store.dir.join("gitctx").join(project_id)
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

fn gitconfig_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Write the gh / git-credential shims (called at startup and when the gh path changes).
pub fn install_shims(store: &Store) {
    let dir = bin_dir(store);
    let _ = fs::create_dir_all(&dir);
    let Some(gh) = store.tools().gh else {
        return;
    };
    let gh_fwd = fwd(&gh);

    let cred = format!(
        r#"#!/bin/sh
# Kilat git credential helper.  $1 = GitHub login, $2 = git action.
[ "$2" = "get" ] || exit 0
tok=$(env -u GH_TOKEN -u GITHUB_TOKEN {gh} auth token --hostname github.com --user "$1" 2>/dev/null)
[ -n "$tok" ] || exit 0
echo "username=$1"
echo "password=$tok"
"#,
        gh = sh_quote(&gh_fwd)
    );

    let gh_sh = format!(
        r#"#!/bin/sh
# Kilat gh shim: runs gh as the GitHub account selected for this project.
REAL={gh}
login=""
if [ -n "$KILAT_GITCTX" ] && [ -f "$KILAT_GITCTX/account" ]; then
  login=$(cat "$KILAT_GITCTX/account")
fi
if [ -n "$login" ]; then
  tok=$(env -u GH_TOKEN -u GITHUB_TOKEN "$REAL" auth token --hostname github.com --user "$login" 2>/dev/null)
  [ -n "$tok" ] && export GH_TOKEN="$tok"
fi
exec "$REAL" "$@"
"#,
        gh = sh_quote(&gh_fwd)
    );

    let gh_cmd = format!(
        "@echo off\r\n\
setlocal\r\n\
set \"KGH_REAL={gh}\"\r\n\
set \"KGH_USER=\"\r\n\
set \"GH_TOKEN=\"\r\n\
set \"GITHUB_TOKEN=\"\r\n\
if defined KILAT_GITCTX if exist \"%KILAT_GITCTX%\\account\" set /p KGH_USER=<\"%KILAT_GITCTX%\\account\"\r\n\
if defined KGH_USER for /f \"usebackq delims=\" %%t in (`call \"%KGH_REAL%\" auth token --hostname github.com --user %KGH_USER% 2^>nul`) do set \"GH_TOKEN=%%t\"\r\n\
\"%KGH_REAL%\" %*\r\n\
exit /b %ERRORLEVEL%\r\n",
        gh = gh.to_string_lossy()
    );

    let _ = fs::write(dir.join("kilat-git-cred.sh"), cred);
    let _ = fs::write(dir.join("gh"), gh_sh);
    #[cfg(unix)]
    {
        // macOS / Linux run the shim directly: it must be executable.
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(dir.join("gh"), fs::Permissions::from_mode(0o755));
    }
    #[cfg(windows)]
    let _ = fs::write(dir.join("gh.cmd"), gh_cmd);
    #[cfg(not(windows))]
    let _ = gh_cmd;
}

/// (Re)write the git context of a project. Cheap; called on every switch and spawn.
pub fn write_ctx(store: &Store, project_id: &str) -> PathBuf {
    let dir = ctx_dir(store, project_id);
    let _ = fs::create_dir_all(&dir);
    let project = store.project(project_id);
    let account = project
        .as_ref()
        .and_then(|p| p.github_account_id.as_deref())
        .and_then(|id| store.account(id))
        .filter(|a| a.provider == "github");

    let mut cfg = String::from("# Dibuat otomatis oleh Kilat Code. Ganti akun dari aplikasi.\n");
    let home = home_dir();
    for real in [home.join(".gitconfig"), home.join(".config").join("git").join("config")] {
        if real.is_file() {
            cfg.push_str(&format!("[include]\n\tpath = {}\n", gitconfig_quote(&fwd(&real))));
        }
    }
    let mut login = String::new();
    if let Some(a) = &account {
        login = a.login.clone().unwrap_or_default();
        cfg.push_str("[user]\n");
        if let Some(n) = a.git_name.as_deref().filter(|s| !s.is_empty()) {
            cfg.push_str(&format!("\tname = {}\n", gitconfig_quote(n)));
        }
        if let Some(e) = a.git_email.as_deref().filter(|s| !s.is_empty()) {
            cfg.push_str(&format!("\temail = {}\n", gitconfig_quote(e)));
        }
        if !login.is_empty() {
            let helper = format!(
                "!sh {} {}",
                sh_quote(&fwd(&bin_dir(store).join("kilat-git-cred.sh"))),
                sh_quote(&login)
            );
            for host in ["https://github.com", "https://gist.github.com"] {
                cfg.push_str(&format!(
                    "[credential \"{host}\"]\n\thelper = \n\thelper = {}\n",
                    gitconfig_quote(&helper)
                ));
            }
        }
        if let Some(key) = a.ssh_key.as_deref().filter(|s| !s.is_empty()) {
            let ssh = format!("ssh -i {} -o IdentitiesOnly=yes", sh_quote(&fwd(Path::new(key))));
            cfg.push_str(&format!("[core]\n\tsshCommand = {}\n", gitconfig_quote(&ssh)));
        }
    }
    let _ = fs::write(dir.join("gitconfig"), cfg);
    let _ = fs::write(dir.join("account"), login);
    dir
}

/// Environment that binds a process to a project's GitHub identity.
pub fn ctx_env(store: &Store, project_id: &str) -> EnvPlan {
    let dir = write_ctx(store, project_id);
    let mut env = EnvPlan::default();
    env.set("GIT_CONFIG_GLOBAL", dir.join("gitconfig").to_string_lossy());
    env.set("KILAT_GITCTX", dir.to_string_lossy());
    env.remove("GH_TOKEN");
    env.remove("GITHUB_TOKEN");
    // Git Bash's /etc/profile prefers an inherited ORIGINAL_PATH over PATH
    // (present when Kilat itself was started from a bash shell).
    env.remove("ORIGINAL_PATH");
    // Windows names it "Path"; reuse the exact key so ConPTY children don't
    // end up with two differently-cased PATH variables.
    let (key, path) = std::env::vars()
        .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
        .unwrap_or_else(|| ("PATH".into(), String::new()));
    let sep = if cfg!(windows) { ";" } else { ":" };
    env.set(&key, format!("{}{sep}{path}", bin_dir(store).to_string_lossy()));
    env
}
