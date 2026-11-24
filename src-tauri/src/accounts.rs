//! Multi-account support. Each account is set up once; afterwards switching
//! only changes the environment a process is spawned with:
//!
//! * Claude: every account shares ONE Claude config dir (so sessions can be
//!   resumed by any account). Subscription accounts are long-lived OAuth
//!   tokens from `claude setup-token` injected as `CLAUDE_CODE_OAUTH_TOKEN`;
//!   API accounts inject `ANTHROPIC_API_KEY`. Secrets live in the OS keychain.
//! * Codex / ChatGPT: each account gets its own `CODEX_HOME` holding its
//!   `auth.json`, while `sessions/` (and skills, rules, prompts) are directory
//!   junctions to the default home, so `codex exec resume <id>` works from any
//!   account.
//! * GitHub: accounts come from `gh`'s multi-account keyring; see gitctx.rs.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::LazyLock;

use base64::Engine;
use serde_json::Value;

use crate::store::{default_codex_home, Account, Store, CLAUDE_DEFAULT, CODEX_DEFAULT};
use crate::util::{self, now_ms, EnvPlan};

const KEYRING_SERVICE: &str = "kilat-code";

pub fn secret_set(account_id: &str, secret: &str) -> anyhow::Result<()> {
    keyring::Entry::new(KEYRING_SERVICE, account_id)?.set_password(secret)?;
    Ok(())
}

pub fn secret_get(account_id: &str) -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, account_id)
        .ok()?
        .get_password()
        .ok()
}

pub fn secret_delete(account_id: &str) {
    if let Ok(e) = keyring::Entry::new(KEYRING_SERVICE, account_id) {
        let _ = e.delete_credential();
    }
}

pub fn codex_home(store: &Store, account_id: &str) -> PathBuf {
    if account_id == CODEX_DEFAULT {
        default_codex_home()
    } else {
        store.dir.join("codex-homes").join(account_id)
    }
}

/// Shared folders linked into every per-account CODEX_HOME: sessions (so any
/// account can resume them) and everything the user installed once — skills,
/// rules, prompts, plugins (+ the files they import) — so every account has
/// the same tools.
const CODEX_SHARED_DIRS: &[&str] = &["sessions", "skills", "rules", "prompts", "plugins", "vendor_imports"];
/// Shared files copied into every per-account CODEX_HOME before each run.
const CODEX_SHARED_FILES: &[&str] = &["config.toml", "AGENTS.md"];

fn make_junction(link: &Path, target: &Path) -> anyhow::Result<()> {
    if link.exists() {
        return Ok(());
    }
    fs::create_dir_all(target)?;
    #[cfg(windows)]
    {
        let out = util::cmd("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(link)
            .arg(target)
            .output()?;
        if !out.status.success() {
            anyhow::bail!(
                "mklink /J gagal: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
    }
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(target, link)?;
    Ok(())
}

/// Create / refresh a per-account CODEX_HOME.
pub fn prepare_codex_home(store: &Store, account_id: &str) -> anyhow::Result<PathBuf> {
    let home = codex_home(store, account_id);
    if account_id == CODEX_DEFAULT {
        return Ok(home);
    }
    fs::create_dir_all(&home)?;
    let base = default_codex_home();
    for d in CODEX_SHARED_DIRS {
        let target = base.join(d);
        if *d == "sessions" || target.is_dir() {
            make_junction(&home.join(d), &target)?;
        }
    }
    for f in CODEX_SHARED_FILES {
        let src = base.join(f);
        let dst = home.join(f);
        if src.is_file() {
            let newer = match (fs::metadata(&src), fs::metadata(&dst)) {
                (Ok(s), Ok(d)) => s.modified().ok() > d.modified().ok(),
                (Ok(_), Err(_)) => true,
                _ => false,
            };
            if newer {
                let _ = fs::copy(&src, &dst);
            }
        }
    }
    Ok(home)
}

/// Delete a per-account CODEX_HOME without ever following the junctions.
fn remove_codex_home(store: &Store, account_id: &str) {
    if account_id == CODEX_DEFAULT {
        return;
    }
    let home = codex_home(store, account_id);
    for d in CODEX_SHARED_DIRS {
        // remove_dir on a junction removes the link only.
        let _ = fs::remove_dir(home.join(d));
    }
    let _ = fs::remove_dir_all(&home);
}

/// Who an account belongs to, as reported by the provider.
#[derive(Default)]
struct Identity {
    email: Option<String>,
    name: Option<String>,
    plan: Option<String>,
}

fn str_at(v: &Value, ptr: &str) -> Option<String> {
    v.pointer(ptr)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// Email / name / plan from the id_token inside a Codex auth.json.
fn codex_identity(auth_json: &Path) -> Identity {
    let claims = (|| {
        let v: Value = serde_json::from_str(&fs::read_to_string(auth_json).ok()?).ok()?;
        let token = v.pointer("/tokens/id_token")?.as_str()?;
        let payload = token.split('.').nth(1)?;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload.trim_end_matches('='))
            .ok()?;
        serde_json::from_slice::<Value>(&bytes).ok()
    })();
    let Some(c) = claims else { return Identity::default() };
    Identity {
        email: str_at(&c, "/email"),
        name: str_at(&c, "/name"),
        plan: c
            .get("https://api.openai.com/auth")
            .and_then(|a| a.get("chatgpt_plan_type"))
            .and_then(Value::as_str)
            .map(String::from),
    }
}

/// `oauthAccount` written by Claude Code into `<config dir>/.claude.json`.
fn claude_config_identity(v: &Value) -> Identity {
    Identity {
        email: str_at(v, "/oauthAccount/emailAddress"),
        name: str_at(v, "/oauthAccount/displayName")
            .or_else(|| str_at(v, "/oauthAccount/fullName")),
        plan: str_at(v, "/oauthAccount/organizationType").map(|t| plan_name(&t)),
    }
}

/// "claude_pro" → "pro", "claude_max" → "max".
fn plan_name(org_type: &str) -> String {
    org_type.trim_start_matches("claude_").to_string()
}

fn read_json(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null)
}

/// `claude auth status --json` run with the account's environment.
fn claude_status(store: &Store, env: &EnvPlan) -> Identity {
    let Some(exe) = store.tools().claude else { return Identity::default() };
    let mut c = util::cmd(exe);
    c.args(["auth", "status", "--json"]);
    env.apply(&mut c);
    let Ok(out) = c.output() else { return Identity::default() };
    let Ok(v) = serde_json::from_slice::<Value>(&out.stdout) else {
        return Identity::default();
    };
    Identity {
        email: str_at(&v, "/email"),
        name: None,
        plan: str_at(&v, "/planTier").or_else(|| str_at(&v, "/subscriptionType")),
    }
}

/// The login Claude Code itself uses (`~/.claude`).
fn claude_default_identity(store: &Store) -> Identity {
    let mut env = EnvPlan::default();
    env.remove("CLAUDE_CODE_OAUTH_TOKEN");
    let mut id = claude_status(store, &env);
    let cfg = claude_config_identity(&read_json(&crate::store::home_dir().join(".claude.json")));
    id.name = cfg.name;
    id.email = id.email.or(cfg.email);
    id
}

static SETUP_TOKEN_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"sk-ant-oat\d{2}-[A-Za-z0-9_\-]{40,}").unwrap());

/// The long-lived token printed by `claude setup-token`, found in raw PTY
/// output. Returns None while the token may still be arriving (it touches
/// the end of the output) unless `ended` says nothing more will come.
pub fn find_setup_token(raw: &str, ended: bool) -> Option<String> {
    let plain = util::strip_ansi_spaced(raw);
    let m = SETUP_TOKEN_RE.find_iter(&plain).last()?;
    if m.end() == plain.len() && !ended {
        return None;
    }
    Some(m.as_str().to_string())
}

/// HTTPS request through the system curl (shipped with Windows 10+), so Kilat
/// needs no HTTP client of its own. Headers go over stdin, which keeps
/// secrets out of the process list. `post` sends an empty POST body.
/// Returns (status, body); None when the request never got an answer.
fn curl(url: &str, post: bool, headers: &[String]) -> Option<(u16, String)> {
    let quote = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let mut config = format!("url = \"{url}\"\n");
    if post {
        config.push_str("data = \"\"\n");
    }
    for h in headers {
        config.push_str(&format!("header = \"{}\"\n", quote(h)));
    }
    let mut child = util::cmd("curl")
        .args(["-s", "--max-time", "10", "-w", "\n%{http_code}", "--config", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(config.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (body, code) = text.rsplit_once('\n')?;
    let code: u16 = code.trim().parse().ok()?;
    (code != 0).then(|| (code, body.to_string()))
}

fn oauth_headers(token: &str) -> Vec<String> {
    vec![
        format!("Authorization: Bearer {token}"),
        "anthropic-beta: oauth-2025-04-20".into(),
        "Content-Type: application/json".into(),
    ]
}

/// What Anthropic says about a Claude secret.
enum SecretCheck {
    /// Accepted; identity is filled in when the token may read the profile.
    Valid(Identity),
    /// 401: the token / key itself is wrong.
    Rejected,
    /// Offline, rate limited, curl missing… never block the user on this.
    Unknown,
}

/// Cheap request that needs no quota. `setup-token` tokens only carry the
/// inference scope, so the profile endpoint answers 403 for a good token and
/// 401 for a bad one.
fn check_claude_secret(kind: &str, secret: &str) -> SecretCheck {
    let res = if kind == "apikey" {
        curl(
            "https://api.anthropic.com/v1/models?limit=1",
            false,
            &[format!("x-api-key: {secret}"), "anthropic-version: 2023-06-01".into()],
        )
    } else {
        curl("https://api.anthropic.com/api/oauth/profile", false, &oauth_headers(secret))
    };
    match res {
        Some((401, _)) => SecretCheck::Rejected,
        Some((200, body)) => {
            let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            SecretCheck::Valid(Identity {
                email: str_at(&v, "/account/email").or_else(|| str_at(&v, "/account/email_address")),
                name: str_at(&v, "/account/display_name").or_else(|| str_at(&v, "/account/full_name")),
                plan: str_at(&v, "/organization/organization_type").map(|t| plan_name(&t)),
            })
        }
        Some((c, _)) if (200..300).contains(&c) || c == 403 => SecretCheck::Valid(Identity::default()),
        _ => SecretCheck::Unknown,
    }
}

/// True only when Anthropic definitely refuses the secret.
pub fn claude_secret_rejected(kind: &str, secret: &str) -> bool {
    matches!(check_claude_secret(kind, secret), SecretCheck::Rejected)
}

/// Organization behind a subscription token. This one works with the
/// inference-only scope of `setup-token` tokens (Claude Code itself uses it
/// for tokens from the environment).
fn claude_token_org(token: &str) -> Option<String> {
    let (code, body) = curl("https://api.anthropic.com/api/oauth/org", true, &oauth_headers(token))?;
    if code != 200 {
        return None;
    }
    let v: Value = serde_json::from_str(&body).ok()?;
    str_at(&v, "/organization_uuid").map(|s| s.to_ascii_lowercase())
}

/// Check a Claude subscription token with Anthropic, then save it.
pub fn store_claude_token(store: &Store, id: &str, token: &str) -> anyhow::Result<()> {
    if claude_secret_rejected("oauth", token) {
        anyhow::bail!("Token ditolak Anthropic (401). Pastikan yang disalin hanya tokennya, utuh, tanpa teks lain.");
    }
    let org = claude_token_org(token);
    secret_set(id, token)?;
    store.update_account(id, |a| {
        // A token of another Claude account: the old email/plan are wrong now.
        if org.is_some() && a.org_id.is_some() && a.org_id != org {
            a.email = None;
            a.display_name = None;
            a.plan = None;
        }
        if org.is_some() {
            a.org_id = org.clone();
        }
        a.kind = "oauth".into();
        a.ready = true;
    });
    Ok(())
}

/// Isolated Claude config dir used while setting an account up, so the
/// one-off logins never touch the main `~/.claude` login.
pub fn claude_setup_dir(store: &Store, id: &str) -> PathBuf {
    store.dir.join("claude-setup").join(id)
}

/// Drop the full sign-in `claude auth login` leaves in a setup dir. Kilat
/// only needs to know who signed in; the agent runs on the setup-token.
pub fn forget_claude_login(dir: &Path) {
    let _ = fs::remove_file(dir.join(".credentials.json"));
    let cfg = dir.join(".claude.json");
    let mut v = read_json(&cfg);
    if let Some(o) = v.as_object_mut() {
        if o.remove("oauthAccount").is_some() {
            if let Ok(bytes) = serde_json::to_vec_pretty(&v) {
                let _ = fs::write(&cfg, bytes);
            }
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedIdentity {
    pub account: Account,
    /// Set when the email turns out to belong to another Kilat account too.
    pub warning: Option<String>,
}

/// After `claude auth login` ran in the account's setup dir: read who signed
/// in and keep it only if it is the same Claude account as the stored token
/// (same organization). setup-token tokens cannot read the email themselves.
pub fn claude_confirm_identity(store: &Store, id: &str) -> anyhow::Result<ConfirmedIdentity> {
    let acc = store.account(id).ok_or_else(|| anyhow::anyhow!("akun tidak ada"))?;
    let dir = claude_setup_dir(store, id);
    let cfg = read_json(&dir.join(".claude.json"));
    forget_claude_login(&dir);
    let ident = claude_config_identity(&cfg);
    let email = ident
        .email
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Login belum selesai, email belum terbaca. Coba lagi."))?;
    let token = secret_get(id)
        .ok_or_else(|| anyhow::anyhow!("Token akun \"{}\" belum ada. Jalankan Setup dulu.", acc.label))?;
    let token_org = claude_token_org(&token).ok_or_else(|| {
        anyhow::anyhow!("Tidak bisa memastikan pemilik token (gagal menghubungi Anthropic). Coba lagi.")
    })?;
    let same = str_at(&cfg, "/oauthAccount/organizationUuid").is_some_and(|o| o.eq_ignore_ascii_case(&token_org));
    if !same {
        anyhow::bail!(
            "Kamu login sebagai {email}, tapi token akun \"{}\" milik akun Claude lain. \
             Ganti ke akun yang benar di claude.ai (browser), lalu klik Deteksi email lagi.",
            acc.label
        );
    }
    let account = store
        .update_account(id, |a| {
            a.email = Some(email.clone());
            a.display_name = ident.name.clone();
            if ident.plan.is_some() {
                a.plan = ident.plan.clone();
            }
            a.org_id = Some(token_org.clone());
        })
        .ok_or_else(|| anyhow::anyhow!("akun tidak ada"))?;
    let warning = store
        .data
        .read()
        .accounts
        .iter()
        .find(|o| {
            o.provider == "claude" && o.id != id && o.email.as_deref().is_some_and(|e| e.eq_ignore_ascii_case(&email))
        })
        .map(|o| {
            format!(
                "Akun \"{}\" ternyata akun Claude yang sama dengan \"{}\" ({email}). \
                 Kemungkinan browser masih login ke akun itu waktu Setup.",
                acc.label, o.label
            )
        });
    Ok(ConfirmedIdentity { account, warning })
}

fn codex_has_auth(home: &Path) -> bool {
    home.join("auth.json").is_file()
}

/// Re-check whether an account is usable and fill in who it belongs to.
pub fn refresh(store: &Store, id: &str) -> Option<Account> {
    let acc = store.account(id)?;
    let tools = store.tools();
    let (ready, ident) = match (acc.provider.as_str(), acc.kind.as_str()) {
        ("claude", "default") => (true, claude_default_identity(store)),
        // A stored token can still be bad (cut short, extra text, revoked):
        // ask Anthropic so a broken account shows up red instead of 401-ing
        // in the middle of a thread. The email of a setup-token account comes
        // from claude_confirm_identity and stays in the store.
        ("claude", "oauth") => match secret_get(id).map(|s| check_claude_secret("oauth", &s)) {
            None | Some(SecretCheck::Rejected) => (false, Identity::default()),
            Some(SecretCheck::Valid(ident)) => (true, ident),
            Some(SecretCheck::Unknown) => (true, Identity::default()),
        },
        ("claude", kind) => match secret_get(id) {
            Some(s) => (!claude_secret_rejected(kind, &s), Identity::default()),
            None => (false, Identity::default()),
        },
        ("codex", _) => {
            let home = codex_home(store, id);
            (codex_has_auth(&home), codex_identity(&home.join("auth.json")))
        }
        ("github", _) => {
            let ok = match (&tools.gh, &acc.login) {
                (Some(gh), Some(login)) => gh_token(gh, login).is_some(),
                _ => false,
            };
            (ok, Identity::default())
        }
        _ => (false, Identity::default()),
    };
    store.update_account(id, |a| {
        a.ready = ready;
        if ident.email.is_some() {
            a.email = ident.email;
        }
        if ident.name.is_some() {
            a.display_name = ident.name;
        }
        if ident.plan.is_some() {
            a.plan = ident.plan;
        }
    })
}

/// Refresh every agent account (startup); GitHub is refreshed by import.
pub fn refresh_all(store: &Store) {
    let ids: Vec<String> = store
        .data
        .read()
        .accounts
        .iter()
        .filter(|a| a.provider != "github")
        .map(|a| a.id.clone())
        .collect();
    for id in ids {
        refresh(store, &id);
    }
}

/// Account names must be filled in and unique per provider, so the right
/// account is always obvious when switching.
pub fn validate_label(store: &Store, provider: &str, label: &str, except: Option<&str>) -> anyhow::Result<String> {
    let label = label.trim();
    if label.is_empty() {
        anyhow::bail!("Nama akun wajib diisi, supaya tidak tertukar dengan akun lain");
    }
    let taken = store.data.read().accounts.iter().any(|a| {
        a.provider == provider && Some(a.id.as_str()) != except && a.label.eq_ignore_ascii_case(label)
    });
    if taken {
        anyhow::bail!("Nama \"{label}\" sudah dipakai akun lain. Pakai nama yang berbeda.");
    }
    Ok(label.to_string())
}

pub fn add(store: &Store, provider: &str, kind: &str, label: &str) -> anyhow::Result<Account> {
    let label = validate_label(store, provider, label, None)?;
    let id = format!("{provider}-{}", util::new_id());
    let acc = Account {
        id: id.clone(),
        provider: provider.into(),
        kind: kind.into(),
        label,
        created_at: now_ms(),
        ..Default::default()
    };
    if provider == "codex" {
        prepare_codex_home(store, &id)?;
    }
    store.data.write().accounts.push(acc.clone());
    store.save();
    Ok(acc)
}

pub fn remove(store: &Store, id: &str) -> anyhow::Result<()> {
    if id == CLAUDE_DEFAULT || id == CODEX_DEFAULT {
        anyhow::bail!("Akun bawaan tidak bisa dihapus");
    }
    let acc = store.account(id).ok_or_else(|| anyhow::anyhow!("akun tidak ada"))?;
    secret_delete(id);
    if acc.provider == "codex" {
        remove_codex_home(store, id);
    }
    if acc.provider == "claude" {
        let _ = fs::remove_dir_all(claude_setup_dir(store, id));
    }
    {
        let mut d = store.data.write();
        d.accounts.retain(|a| a.id != id);
        for p in &mut d.projects {
            if p.github_account_id.as_deref() == Some(id) {
                p.github_account_id = None;
            }
        }
    }
    store.save();
    Ok(())
}

/// Store a pasted secret (Claude token / API key, or Codex API key).
pub fn set_secret(store: &Store, id: &str, secret: &str) -> anyhow::Result<Account> {
    let acc = store.account(id).ok_or_else(|| anyhow::anyhow!("akun tidak ada"))?;
    let secret = secret.trim();
    if secret.is_empty() {
        anyhow::bail!("Token kosong");
    }
    match acc.provider.as_str() {
        "claude" => {
            if secret.starts_with("sk-ant-api") {
                if claude_secret_rejected("apikey", secret) {
                    anyhow::bail!("API key ditolak Anthropic (401). Pastikan key disalin utuh.");
                }
                secret_set(id, secret)?;
                store.update_account(id, |a| a.kind = "apikey".into());
            } else {
                store_claude_token(store, id, secret)?;
            }
        }
        "codex" => {
            let codex = store
                .tools()
                .codex
                .ok_or_else(|| anyhow::anyhow!("Codex CLI tidak ditemukan"))?;
            let home = prepare_codex_home(store, id)?;
            let mut child = util::cmd(codex)
                .args(["login", "--with-api-key"])
                .env("CODEX_HOME", &home)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(secret.as_bytes())?;
            }
            let out = child.wait_with_output()?;
            if !out.status.success() {
                anyhow::bail!(
                    "codex login gagal: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            store.update_account(id, |a| a.kind = "apikey".into());
        }
        _ => anyhow::bail!("Provider ini tidak memakai token"),
    }
    refresh(store, id).ok_or_else(|| anyhow::anyhow!("akun tidak ada"))
}

/// Environment for running an agent as `account_id`.
pub fn agent_env(store: &Store, account_id: &str) -> anyhow::Result<EnvPlan> {
    let acc = store
        .account(account_id)
        .ok_or_else(|| anyhow::anyhow!("Akun tidak ditemukan"))?;
    let mut env = EnvPlan::default();
    match (acc.provider.as_str(), acc.kind.as_str()) {
        ("claude", "default") => {
            env.remove("CLAUDE_CODE_OAUTH_TOKEN");
        }
        ("claude", kind) => {
            let secret = secret_get(account_id).ok_or_else(|| {
                anyhow::anyhow!("Akun '{}' belum di-setup (token belum ada)", acc.label)
            })?;
            env.remove("ANTHROPIC_AUTH_TOKEN");
            env.remove("CLAUDE_CODE_USE_BEDROCK");
            env.remove("CLAUDE_CODE_USE_VERTEX");
            if kind == "apikey" {
                env.remove("CLAUDE_CODE_OAUTH_TOKEN");
                env.set("ANTHROPIC_API_KEY", secret);
            } else {
                env.remove("ANTHROPIC_API_KEY");
                env.set("CLAUDE_CODE_OAUTH_TOKEN", secret);
            }
        }
        ("codex", _) => {
            let home = prepare_codex_home(store, account_id)?;
            if !codex_has_auth(&home) {
                anyhow::bail!("Akun '{}' belum login (jalankan setup dulu)", acc.label);
            }
            if account_id != CODEX_DEFAULT {
                env.set("CODEX_HOME", home.to_string_lossy());
            }
        }
        _ => anyhow::bail!("Akun ini bukan akun agent"),
    }
    Ok(env)
}

// ---------------------------------------------------------------- GitHub

pub fn gh_token(gh: &Path, login: &str) -> Option<String> {
    let out = util::cmd(gh)
        .args(["auth", "token", "--hostname", "github.com", "--user", login])
        .env_remove("GH_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .output()
        .ok()?;
    let tok = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !tok.is_empty()).then_some(tok)
}

/// Logins known to `gh` (it supports several accounts per host).
pub fn gh_logins(gh: &Path) -> Vec<String> {
    let Ok(out) = util::cmd(gh)
        .args(["auth", "status", "--hostname", "github.com"])
        .env_remove("GH_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .output()
    else {
        return vec![];
    };
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let re = regex::Regex::new(r"Logged in to github\.com (?:account|as) ([A-Za-z0-9-]+)").unwrap();
    let mut v: Vec<String> = re
        .captures_iter(&text)
        .map(|c| c[1].to_string())
        .collect();
    v.dedup();
    v
}

fn gh_profile(gh: &Path, login: &str) -> (Option<String>, Option<String>) {
    let Some(tok) = gh_token(gh, login) else {
        return (None, None);
    };
    let Ok(out) = util::cmd(gh)
        .args(["api", "user"])
        .env("GH_TOKEN", &tok)
        .output()
    else {
        return (None, None);
    };
    let v: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
    let name = v
        .get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let email = v
        .get("email")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or_else(|| {
            v.get("id")
                .and_then(Value::as_i64)
                .map(|id| format!("{id}+{login}@users.noreply.github.com"))
        });
    (name, email)
}

/// Add every gh login that Kilat doesn't know yet; refresh the rest.
pub fn github_import(store: &Store) -> anyhow::Result<Vec<Account>> {
    let gh = store
        .tools()
        .gh
        .ok_or_else(|| anyhow::anyhow!("GitHub CLI (gh) tidak ditemukan. Install: winget install GitHub.cli"))?;
    let logins = gh_logins(&gh);
    for login in &logins {
        let exists = store
            .data
            .read()
            .accounts
            .iter()
            .any(|a| a.provider == "github" && a.login.as_deref() == Some(login));
        if exists {
            continue;
        }
        let (name, email) = gh_profile(&gh, login);
        let acc = Account {
            id: format!("github-{}", util::new_id()),
            provider: "github".into(),
            kind: "gh".into(),
            label: login.clone(),
            login: Some(login.clone()),
            git_name: name.or(Some(login.clone())),
            git_email: email,
            ready: true,
            created_at: now_ms(),
            ..Default::default()
        };
        store.data.write().accounts.push(acc);
    }
    {
        let mut d = store.data.write();
        for a in d.accounts.iter_mut().filter(|a| a.provider == "github") {
            a.ready = a.login.as_ref().is_some_and(|l| logins.contains(l));
        }
    }
    store.save();
    Ok(store
        .data
        .read()
        .accounts
        .iter()
        .filter(|a| a.provider == "github")
        .cloned()
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{find_setup_token, forget_claude_login};

    fn fake_token() -> String {
        format!("sk-ant-oat01-{}xyzAA", "aB3_-".repeat(18))
    }

    #[test]
    fn conpty_cursor_moves_do_not_glue_next_line() {
        // ConPTY draws line breaks as cursor positioning, not CRLF.
        let tok = fake_token();
        let raw = format!(
            "\x1b[?25l\x1b[5;1HYour OAuth token (valid for 1 year):\x1b[7;1H{tok}\x1b[9;1HStore this token securely."
        );
        assert_eq!(find_setup_token(&raw, false).as_deref(), Some(tok.as_str()));
    }

    #[test]
    fn plain_newlines() {
        let tok = fake_token();
        let raw = format!("token:\r\n\r\n{tok}\r\n\r\nStore this token securely.\r\n");
        assert_eq!(find_setup_token(&raw, false).as_deref(), Some(tok.as_str()));
    }

    #[test]
    fn waits_for_token_split_across_reads() {
        let tok = fake_token();
        let partial = format!("token:\r\n{}", &tok[..70]);
        assert_eq!(find_setup_token(&partial, false), None);
        let full = format!("token:\r\n{tok}");
        assert_eq!(find_setup_token(&full, false), None);
        assert_eq!(find_setup_token(&full, true).as_deref(), Some(tok.as_str()));
    }

    #[test]
    fn forget_login_keeps_the_rest_of_the_config() {
        let dir = std::env::temp_dir().join(format!("kilat-test-{}", crate::util::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".credentials.json"), "{}").unwrap();
        std::fs::write(
            dir.join(".claude.json"),
            r#"{"userID":"u","oauthAccount":{"emailAddress":"a@b.c"}}"#,
        )
        .unwrap();
        forget_claude_login(&dir);
        assert!(!dir.join(".credentials.json").exists());
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join(".claude.json")).unwrap()).unwrap();
        assert!(v.get("oauthAccount").is_none());
        assert_eq!(v["userID"], "u");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

