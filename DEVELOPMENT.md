# Developing Kilat Code

Kilat Code is a Tauri 2 app: a Rust backend (`src-tauri/`) and a Svelte 5 UI (`src/`). The website lives in `website/` (Astro, see `website/README.md`).

## Build and run

Needs Node 22+, pnpm, Rust, and Git (Git for Windows on Windows), plus `claude`, `codex`, and `gh` on the PATH to try the agents.

```bash
pnpm install
pnpm tauri dev     # development build
pnpm tauri build   # installer in src-tauri/target/release/bundle/
```

- Backend tests: `cd src-tauri && cargo test --lib`
- Linux build packages: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libdbus-1-dev`. macOS needs the Xcode Command Line Tools.
- To run a dev build next to an installed Kilat without sharing data, set `KILAT_DATA_DIR` to another folder (and `WEBVIEW2_USER_DATA_FOLDER` on Windows if needed).
- UI-only preview without Rust or agents: `pnpm dev`, then open `http://localhost:1420/preview.html?s=limit` (other scenarios: `run`, `work`, `editor`, `wait`, `welcome`, `accounts`; add `&lang=en` or `&lang=id`). The backend is mocked in `src/dev/mock.ts`.

## Translations

The UI is in Indonesian and English. UI text lives in `src/lib/i18n/` (`id.ts` is the reference shape, `en.ts` is type-checked against it). Text produced by the backend (errors, notifications, the statusline, and messages typed to agents) uses `src-tauri/src/i18n.rs` (`tr(id, en)` and `trf!`). Add every new string in both languages.

## Releases

1. Bump the version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
2. Commit, then `git tag vX.Y.Z && git push origin main vX.Y.Z`.
3. GitHub Actions (`.github/workflows/release.yml`) builds Windows, macOS (universal), and Linux (AppImage, .deb, .rpm) and publishes the release with `latest.json` for the in-app updater.

Updates are signed with the key in the repository secret `TAURI_SIGNING_PRIVATE_KEY` (public key in `tauri.conf.json`). Keep a backup of the private key: without it, installed versions can't receive updates.

## How it works

- **Agents.** Each thread runs `claude` or `codex` in a pseudo-terminal (ConPTY on Windows) owned by the backend and shown with xterm.js, so agents keep running when you switch threads.
- **Claude status and limits.** Claude Code is started with `--settings <data>/agent/claude-settings.json`, which adds `SessionStart`, `UserPromptSubmit`, `Stop`, `StopFailure`, and `Notification` hooks and a statusline. They call `kilat --kilat-hook` / `--kilat-statusline`, a tiny process that forwards Claude's JSON to the app over a `127.0.0.1` socket (address and a random key are in the agent's environment). The user's own hooks keep working, and a user `statusLine` is left alone.
- **Codex status and limits.** Kilat reads the session (rollout) files Codex writes. Codex runs with `--no-daemon` so each account really uses its own login.
- **Accounts.** Claude accounts share `~/.claude` and differ only by token (`CLAUDE_CODE_OAUTH_TOKEN` / `ANTHROPIC_API_KEY`); switching resumes the same session with `--resume`. ChatGPT accounts each have their own `CODEX_HOME`, with `sessions/` linked to `~/.codex/sessions`, and continue with `codex resume`. Moving between Claude and ChatGPT writes the part of the conversation the new agent hasn't seen to a Markdown handoff file and asks it to read that first.
- **GitHub per project.** Every terminal and agent in a project gets `GIT_CONFIG_GLOBAL` pointing at a per-project gitconfig that includes `~/.gitconfig`, overrides `user.name`/`user.email`, and adds a credential helper that takes the selected account's token from `gh auth token --user <login>`. Switching accounts rewrites two small files, so it applies immediately.

## Layout

```
src/                         UI (Svelte 5)
  lib/state.svelte.ts        app state and agent events
  lib/i18n/                  UI text (id, en)
  lib/limits.ts              reset times from limit messages
  components/ThreadView      thread bar (account, model, mode, status) + terminal
  components/AgentTerminal   xterm attached to the backend's agent process
  components/LimitDialog     the options shown at a limit
src-tauri/src/
  agents/mod.rs              agent processes per thread, status, limits, waiting, switching
  agents/launch.rs           claude / codex arguments and the hook settings file
  agents/hook.rs             Claude hook/statusline bridge (local socket)
  agents/codex.rs            Codex rollout reader (session id, status, limits, usage)
  agents/transcript.rs       original transcripts and the handoff file
  accounts.rs                accounts, keychain, per-account CODEX_HOME, identity
  gitctx.rs / gitops.rs      per-project GitHub identity, commit / push / PR
  i18n.rs                    backend text language
  pty.rs                     bash terminals
  store.rs                   state.json and threads/*.json in the app data folder
  usage.rs                   account usage windows
```
