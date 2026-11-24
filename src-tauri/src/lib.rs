//! Kilat Code — a light desktop editor for running many coding agents
//! (the Claude Code and Codex terminal UIs) across many accounts.

mod i18n;
mod accounts;
mod agents;
mod fsops;
mod gitctx;
mod gitops;
mod pty;
mod store;
mod usage;
mod util;

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::ipc::Channel;
use tauri::{Emitter, Manager, State};

use agents::{AgentManager, Attached, SwitchReq, TermEvent};
use store::{Account, AppData, Persist, Project, Settings, Store, Thread, ThreadMeta};
use util::{now_ms, CmdResult, ErrStr};

pub struct AppState {
    pub store: Arc<Store>,
    pub pty: Arc<pty::PtyManager>,
    pub agents: Arc<AgentManager>,
}

type S<'a> = State<'a, AppState>;

// ------------------------------------------------------------------ state

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    data: AppData,
    tools: store::Tools,
    data_dir: String,
    version: String,
}

#[tauri::command]
fn bootstrap(app: tauri::AppHandle, st: S) -> Bootstrap {
    Bootstrap {
        data: st.store.snapshot(),
        tools: st.store.tools(),
        data_dir: st.store.dir.to_string_lossy().into(),
        version: app.package_info().version.to_string(),
    }
}

#[tauri::command]
fn settings_save(st: S, settings: Settings) -> store::Tools {
    st.store.data.write().settings = settings;
    st.store.save();
    st.store.refresh_tools();
    gitctx::install_shims(&st.store);
    st.store.tools()
}

// --------------------------------------------------------------- projects

#[tauri::command]
fn project_add(st: S, path: String) -> CmdResult<Project> {
    let p = Path::new(&path);
    if !p.is_dir() {
        return Err(crate::i18n::tr("Folder tidak ditemukan", "Folder not found").into());
    }
    if let Some(existing) = st
        .store
        .data
        .read()
        .projects
        .iter()
        .find(|x| Path::new(&x.path) == p)
    {
        return Ok(existing.clone());
    }
    let project = Project {
        id: util::new_id(),
        name: p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone()),
        path: path.clone(),
        github_account_id: None,
        created_at: now_ms(),
    };
    st.store.data.write().projects.insert(0, project.clone());
    st.store.save();
    Ok(project)
}

#[tauri::command]
fn project_remove(st: S, id: String) {
    let threads: Vec<String> = st
        .store
        .data
        .read()
        .threads
        .iter()
        .filter(|t| t.project_id == id)
        .map(|t| t.id.clone())
        .collect();
    for t in threads {
        delete_thread_full(&st, &t);
    }
    st.store.data.write().projects.retain(|p| p.id != id);
    st.store.save();
}

/// Bind a GitHub account to a project. Applies instantly to running
/// terminals and agents of that project (see gitctx.rs).
#[tauri::command]
fn project_set_github(st: S, id: String, account_id: Option<String>) -> CmdResult<Project> {
    let p = {
        let mut d = st.store.data.write();
        let p = d
            .projects
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(crate::i18n::tr("Project tidak ditemukan", "Project not found"))?;
        p.github_account_id = account_id.filter(|s| !s.is_empty());
        p.clone()
    };
    st.store.save();
    gitctx::write_ctx(&st.store, &id);
    Ok(p)
}

// ---------------------------------------------------------------- threads

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewThread {
    project_id: String,
    #[serde(default)]
    worktree: bool,
    #[serde(default)]
    provider: String,
    #[serde(default)]
    account_id: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    mode: String,
}

#[tauri::command]
fn thread_create(st: S, req: NewThread) -> CmdResult<ThreadMeta> {
    let project = st.store.project(&req.project_id).ok_or(crate::i18n::tr("Project tidak ditemukan", "Project not found"))?;
    let id = util::new_id();
    let mut cwd = project.path.clone();
    let mut branch = None;
    if req.worktree {
        let name = format!("kilat/{}", &id[..6]);
        let dest = st
            .store
            .dir
            .join("worktrees")
            .join(format!("{}-{}", project.name.replace(' ', "-"), &id[..6]));
        fsops::add_worktree(&project.path, &dest, &name).es()?;
        cwd = dest.to_string_lossy().into();
        branch = Some(name);
    }
    let provider = if req.provider.is_empty() { "claude".to_string() } else { req.provider };
    let account_id = if req.account_id.is_empty() {
        if provider == "codex" { store::CODEX_DEFAULT } else { store::CLAUDE_DEFAULT }.to_string()
    } else {
        req.account_id
    };
    let meta = ThreadMeta {
        id,
        project_id: project.id,
        title: crate::i18n::tr("Thread baru", "New thread").into(),
        provider,
        account_id,
        model: req.model,
        mode: if req.mode.is_empty() { "full".into() } else { req.mode },
        cwd,
        worktree_branch: branch,
        status: "off".into(),
        created_at: now_ms(),
        updated_at: now_ms(),
        ..Default::default()
    };
    st.store.insert_thread(Thread {
        meta: meta.clone(),
        ..Default::default()
    });
    Ok(meta)
}

#[tauri::command]
fn thread_get(st: S, id: String) -> CmdResult<Thread> {
    st.store.thread(&id).ok_or_else(|| crate::i18n::tr("Thread tidak ditemukan", "Thread not found").into())
}

#[tauri::command]
fn thread_rename(st: S, id: String, title: String) -> CmdResult<()> {
    st.store
        .update_thread(&id, Persist::Always, |t| t.meta.title = title)
        .ok_or(crate::i18n::tr("Thread tidak ditemukan", "Thread not found"))?;
    Ok(())
}

#[tauri::command]
fn thread_delete(st: S, id: String) {
    delete_thread_full(&st, &id);
}

/// Stop the agent, delete the thread and its git worktree
/// (unless a forked thread still works in it).
fn delete_thread_full(st: &AppState, id: &str) {
    st.agents.kill(id);
    if let Some(meta) = st.store.delete_thread(id) {
        let shared = st.store.data.read().threads.iter().any(|t| t.cwd == meta.cwd);
        if meta.worktree_branch.is_some() && !shared {
            if let Some(p) = st.store.project(&meta.project_id) {
                fsops::remove_worktree(&p.path, &meta.cwd);
            }
        }
    }
}

#[tauri::command]
fn thread_set_flags(st: S, id: String, pinned: Option<bool>, archived: Option<bool>) -> CmdResult<ThreadMeta> {
    st.store
        .update_thread(&id, Persist::Always, |t| {
            if let Some(p) = pinned {
                t.meta.pinned = p;
            }
            if let Some(a) = archived {
                t.meta.archived = a;
                if a {
                    t.meta.pinned = false;
                }
            }
            t.meta.clone()
        })
        .ok_or_else(|| crate::i18n::tr("Thread tidak ditemukan", "Thread not found").into())
}

// --------------------------------------------------------------- git ops

#[tauri::command]
async fn git_commit(st: S<'_>, project_id: String, cwd: String, message: String, paths: Vec<String>) -> CmdResult<String> {
    let store = st.store.clone();
    tauri::async_runtime::spawn_blocking(move || gitops::commit(&store, &project_id, &cwd, &message, &paths))
        .await
        .es()?
        .es()
}

#[tauri::command]
async fn git_push(st: S<'_>, project_id: String, cwd: String) -> CmdResult<String> {
    let store = st.store.clone();
    tauri::async_runtime::spawn_blocking(move || gitops::push(&store, &project_id, &cwd))
        .await
        .es()?
        .es()
}

#[tauri::command]
async fn git_branch(st: S<'_>, project_id: String, cwd: String, name: String) -> CmdResult<()> {
    let store = st.store.clone();
    tauri::async_runtime::spawn_blocking(move || gitops::create_branch(&store, &project_id, &cwd, &name))
        .await
        .es()?
        .es()
}

#[tauri::command]
async fn git_pr(st: S<'_>, project_id: String, cwd: String, title: String, body: String, draft: bool) -> CmdResult<String> {
    let store = st.store.clone();
    tauri::async_runtime::spawn_blocking(move || gitops::create_pr(&store, &project_id, &cwd, &title, &body, draft))
        .await
        .es()?
        .es()
}

#[tauri::command]
async fn git_suggest_message(
    st: S<'_>,
    project_id: String,
    cwd: String,
    provider: String,
    account_id: String,
) -> CmdResult<String> {
    let store = st.store.clone();
    tauri::async_runtime::spawn_blocking(move || gitops::suggest_message(&store, &project_id, &cwd, &provider, &account_id))
        .await
        .es()?
        .es()
}

#[tauri::command]
async fn git_recent(cwd: String) -> Vec<gitops::Commit> {
    tauri::async_runtime::spawn_blocking(move || gitops::recent(&cwd))
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn fs_list_files(root: String) -> Vec<String> {
    tauri::async_runtime::spawn_blocking(move || gitops::list_files(&root))
        .await
        .unwrap_or_default()
}

/// The UI's language, for the text the backend writes (see i18n.rs).
#[tauri::command]
fn set_ui_lang(app: tauri::AppHandle, st: S, lang: String) {
    i18n::set(&lang);
    if st.store.localize_defaults() {
        let _ = app.emit("accounts-changed", "");
    }
}

// ---------------------------------------------------------- notifications

/// Windows notification + taskbar flash (the UI calls this when the window
/// is in the background).
#[tauri::command]
fn notify(app: tauri::AppHandle, title: String, body: String) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.request_user_attention(Some(tauri::UserAttentionType::Informational));
    }
}

// ----------------------------------------------------------------- agents

/// Show a thread's agent terminal; returns whether an agent is running.
#[tauri::command]
fn agent_attach(st: S, thread_id: String, on_event: Channel<TermEvent>) -> Attached {
    st.agents.attach(&thread_id, on_event)
}

#[tauri::command]
fn agent_detach(st: S, thread_id: String, view_id: u64) {
    st.agents.detach(&thread_id, view_id);
}

/// Start the thread's agent (resuming its session) unless it runs already.
#[tauri::command]
async fn agent_start(st: S<'_>, thread_id: String, cols: u16, rows: u16) -> CmdResult<()> {
    let agents = st.agents.clone();
    tauri::async_runtime::spawn_blocking(move || agents.start(&thread_id, cols, rows))
        .await
        .es()?
        .es()
}

#[tauri::command]
fn agent_write(st: S, thread_id: String, data: String) -> CmdResult<()> {
    st.agents.write(&thread_id, &data).es()
}

#[tauri::command]
fn agent_resize(st: S, thread_id: String, cols: u16, rows: u16) {
    st.agents.resize(&thread_id, cols, rows);
}

#[tauri::command]
fn agent_stop(st: S, thread_id: String) {
    st.agents.stop(&thread_id);
}

/// Continue the thread with another account, provider, model or mode.
#[tauri::command]
async fn agent_switch(st: S<'_>, req: SwitchReq) -> CmdResult<()> {
    let agents = st.agents.clone();
    tauri::async_runtime::spawn_blocking(move || agents.switch(req))
        .await
        .es()?
        .es()
}

/// Wait for the account's limit to reset, then continue automatically.
#[tauri::command]
fn agent_wait(st: S, thread_id: String, until: i64) {
    st.agents.wait(&thread_id, until);
}

#[tauri::command]
fn agent_wait_cancel(st: S, thread_id: String) {
    st.agents.cancel_wait(&thread_id);
}

/// Continue with the same account right now.
#[tauri::command]
async fn agent_continue(st: S<'_>, thread_id: String) -> CmdResult<()> {
    let agents = st.agents.clone();
    tauri::async_runtime::spawn_blocking(move || agents.continue_now(&thread_id))
        .await
        .es()?
        .es()
}

#[tauri::command]
fn agent_dismiss_limit(st: S, thread_id: String) {
    st.agents.dismiss_limit(&thread_id);
}

#[tauri::command]
fn codex_models() -> Vec<Value> {
    agents::codex::models()
}

// --------------------------------------------------------------- accounts

#[tauri::command]
fn account_add(st: S, provider: String, kind: String, label: String) -> CmdResult<Account> {
    accounts::add(&st.store, &provider, &kind, &label).es()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountPatch {
    label: Option<String>,
    git_name: Option<String>,
    git_email: Option<String>,
    ssh_key: Option<String>,
    clear_limit: Option<bool>,
}

#[tauri::command]
fn account_update(st: S, id: String, patch: AccountPatch) -> CmdResult<Account> {
    let current = st.store.account(&id).ok_or(crate::i18n::tr("Akun tidak ditemukan", "Account not found"))?;
    let label = match &patch.label {
        Some(l) => Some(accounts::validate_label(&st.store, &current.provider, l, Some(&id)).es()?),
        None => None,
    };
    let patch = AccountPatch { label, ..patch };
    let acc = st
        .store
        .update_account(&id, |a| {
            if let Some(v) = patch.label {
                a.label = v;
            }
            if let Some(v) = patch.git_name {
                a.git_name = Some(v);
            }
            if let Some(v) = patch.git_email {
                a.git_email = Some(v);
            }
            if let Some(v) = patch.ssh_key {
                a.ssh_key = Some(v).filter(|s| !s.is_empty());
            }
            if patch.clear_limit == Some(true) {
                a.limited_until = None;
            }
        })
        .ok_or(crate::i18n::tr("Akun tidak ditemukan", "Account not found"))?;
    // Refresh git contexts that use this account.
    let projects: Vec<String> = st
        .store
        .data
        .read()
        .projects
        .iter()
        .filter(|p| p.github_account_id.as_deref() == Some(id.as_str()))
        .map(|p| p.id.clone())
        .collect();
    for p in projects {
        gitctx::write_ctx(&st.store, &p);
    }
    Ok(acc)
}

#[tauri::command]
fn account_remove(st: S, id: String) -> CmdResult<()> {
    accounts::remove(&st.store, &id).es()
}

// The account commands below talk to the network: keep them off the main thread.

#[tauri::command]
async fn account_set_secret(st: S<'_>, id: String, secret: String) -> CmdResult<Account> {
    let store = st.store.clone();
    tauri::async_runtime::spawn_blocking(move || accounts::set_secret(&store, &id, &secret))
        .await
        .es()?
        .es()
}

#[tauri::command]
async fn account_refresh(st: S<'_>, id: String) -> CmdResult<Account> {
    let store = st.store.clone();
    tauri::async_runtime::spawn_blocking(move || accounts::refresh(&store, &id))
        .await
        .es()?
        .ok_or_else(|| crate::i18n::tr("Akun tidak ditemukan", "Account not found").into())
}

#[tauri::command]
async fn account_claude_identity(st: S<'_>, id: String) -> CmdResult<accounts::ConfirmedIdentity> {
    let store = st.store.clone();
    tauri::async_runtime::spawn_blocking(move || accounts::claude_confirm_identity(&store, &id))
        .await
        .es()?
        .es()
}

#[tauri::command]
async fn github_import(st: S<'_>) -> CmdResult<Vec<Account>> {
    let store = st.store.clone();
    tauri::async_runtime::spawn_blocking(move || accounts::github_import(&store))
        .await
        .es()?
        .es()
}

// -------------------------------------------------------------- terminals

#[tauri::command]
fn pty_spawn(st: S, opts: pty::SpawnOpts, on_event: Channel<pty::PtyEvent>) -> CmdResult<String> {
    st.pty.spawn(opts, on_event).es()
}

#[tauri::command]
fn pty_write(st: S, id: String, data: String) -> CmdResult<()> {
    st.pty.write(&id, &data).es()
}

#[tauri::command]
fn pty_resize(st: S, id: String, cols: u16, rows: u16) -> CmdResult<()> {
    st.pty.resize(&id, cols, rows).es()
}

#[tauri::command]
fn pty_kill(st: S, id: String) {
    st.pty.kill(&id);
}

// ------------------------------------------------------------- files / git

#[tauri::command]
fn fs_list_dir(path: String) -> CmdResult<Vec<fsops::Entry>> {
    fsops::list_dir(&path).es()
}

#[tauri::command]
fn fs_read_file(path: String) -> CmdResult<String> {
    fsops::read_file(&path).es()
}

#[tauri::command]
fn fs_write_file(path: String, content: String) -> CmdResult<()> {
    fsops::write_file(&path, &content).es()
}

#[tauri::command]
async fn git_status(cwd: String) -> fsops::GitStatus {
    tauri::async_runtime::spawn_blocking(move || fsops::status(&cwd))
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn git_diff(cwd: String, path: String) -> CmdResult<String> {
    tauri::async_runtime::spawn_blocking(move || fsops::diff(&cwd, &path))
        .await
        .es()?
        .es()
}

#[tauri::command]
fn is_git_repo(path: String) -> bool {
    fsops::is_repo(&path)
}

// --------------------------------------------------------------------- run

/// Run as Claude Code's hook / statusline helper instead of the app (see
/// agents/hook.rs). Returns the exit code when it did.
pub fn helper_cli() -> Option<i32> {
    agents::hook::cli()
}

pub fn run() {
    // Before any tool lookup: macOS / Linux launchers give a bare PATH.
    util::adopt_login_path();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // KILAT_DATA_DIR: separate data (e.g. a dev build next to the
            // installed app, which would otherwise share state.json).
            let dir = match std::env::var_os("KILAT_DATA_DIR").filter(|d| !d.is_empty()) {
                Some(d) => std::path::PathBuf::from(d),
                None => app.path().app_data_dir()?,
            };
            let store = Arc::new(Store::load(dir).map_err(|e| e.to_string())?);
            gitctx::install_shims(&store);
            let pty = Arc::new(pty::PtyManager::new(app.handle().clone(), store.clone()));
            let agents = AgentManager::new(app.handle().clone(), store.clone());
            app.manage(AppState { store: store.clone(), pty, agents });
            // Find out which email / plan each account belongs to.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                accounts::refresh_all(&store);
                let _ = handle.emit("accounts-changed", "*");
            });
            let _ = app.emit("ready", ());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_ui_lang,
            bootstrap,
            settings_save,
            project_add,
            project_remove,
            project_set_github,
            thread_create,
            thread_get,
            thread_rename,
            thread_delete,
            thread_set_flags,
            git_commit,
            git_push,
            git_branch,
            git_pr,
            git_suggest_message,
            git_recent,
            fs_list_files,
            notify,
            agent_attach,
            agent_detach,
            agent_start,
            agent_write,
            agent_resize,
            agent_stop,
            agent_switch,
            agent_wait,
            agent_wait_cancel,
            agent_continue,
            agent_dismiss_limit,
            codex_models,
            account_add,
            account_update,
            account_remove,
            account_set_secret,
            account_refresh,
            account_claude_identity,
            github_import,
            pty_spawn,
            pty_write,
            pty_resize,
            pty_kill,
            fs_list_dir,
            fs_read_file,
            fs_write_file,
            git_status,
            git_diff,
            is_git_repo,
        ])
        .build(tauri::generate_context!())
        .expect("gagal memulai Kilat Code");

    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            if let Some(st) = handle.try_state::<AppState>() {
                st.agents.shutdown();
                st.pty.kill_all();
            }
        }
    });
}
