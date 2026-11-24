# ⚡ Kilat Code

Editor desktop ringan untuk menjalankan banyak agent coding sekaligus, dengan banyak akun yang bisa diganti tanpa login ulang. **Berbasis terminal**: tiap thread adalah TUI asli `claude` (Claude Code) atau `codex` (ChatGPT), persis seperti di terminal biasa, sehingga slash command, `/model`, izin, dan lainnya tetap jalan. Dibangun dengan **Tauri + Rust** (webview bawaan sistem) supaya tetap ringan.

**Website & dokumentasi:** https://kilat.revincolabs.com

**Suka Kilat Code?** [Traktir kopi di Saweria](https://saweria.co/gdwpran) ☕

## Download

Ambil installer di [Releases terbaru](https://github.com/revincolabs/kilat-code/releases/latest):

| OS | File | Catatan |
|---|---|---|
| Windows 10/11 (64-bit) | `…-setup.exe` | Belum ditandatangani: di SmartScreen klik *More info → Run anyway* |
| macOS 11+ (Apple Silicon & Intel) | `.dmg` | Belum dinotarisasi: klik kanan aplikasi → *Open*, atau `xattr -cr "/Applications/Kilat Code.app"` |
| Linux (x86_64) | `.AppImage` / `.deb` / `.rpm` (Fedora) | Butuh WebKitGTK 4.1 dan Secret Service (GNOME Keyring / KWallet) untuk menyimpan token |

Versi Windows yang paling banyak diuji; macOS dan Linux masih tahap awal. Aplikasi memeriksa update sendiri dan menawarkannya di status bar.

Kilat Code adalah proyek independen, tidak berafiliasi dengan Anthropic maupun OpenAI. Kamu tetap butuh Claude Code CLI dan/atau Codex CLI serta langganan atau API key sendiri.

## Fitur

| Fitur | Cara kerja |
|---|---|
| **Thread = terminal agent asli** | Tiap thread menjalankan `claude` atau `codex` di ConPTY + xterm.js. Prosesnya milik backend: pindah thread atau menutup tampilan tidak menghentikan agent, dan saat dibuka lagi layar digambar ulang |
| **Multi-agent** | Banyak thread bisa bekerja paralel. Opsi *worktree terpisah* membuat git worktree per thread supaya agent tidak saling menimpa file |
| **Status & notifikasi** | Kilat tahu agent sedang *bekerja*, *siap*, *butuh izin*, atau *kena limit*. Claude: lewat hook Claude Code yang memanggil Kilat. Codex: lewat file sesi (rollout) yang ditulis Codex. Notifikasi Windows + suara saat Kilat di belakang, toast kalau thread lain |
| **Kena limit → pilih** | Muncul dialog di atas terminal: **Tunggu** sampai reset (jam reset dibaca dari CLI/pesan limit) lalu lanjut otomatis, **pindah ke akun Claude lain** (sesi yang sama di-resume), atau **pindah ke akun ChatGPT** (riwayat diteruskan). Bisa juga tunggu 30 menit / 1 / 3 / 5 jam |
| **Ganti akun Claude ↔ Claude** | Semua akun Claude berbagi `~/.claude`; yang berbeda hanya token (`CLAUDE_CODE_OAUTH_TOKEN` / `ANTHROPIC_API_KEY`). Agent dijalankan ulang dengan `--resume <sesi>`, konteks utuh |
| **Ganti akun ChatGPT ↔ ChatGPT** | Tiap akun punya `CODEX_HOME` sendiri, `sessions/` di-*junction* ke `~/.codex/sessions`, lalu `codex resume <id>`. Codex dijalankan dengan `--no-daemon` supaya tiap akun benar-benar memakai login-nya sendiri |
| **Ganti Claude ↔ ChatGPT** | Kilat mencatat segmen (akun + sesi asli + waktu). Bagian percakapan yang belum dilihat agent tujuan dibaca dari transkrip asli (`~/.claude/projects/…/<id>.jsonl` atau rollout Codex), ditulis ke file Markdown, lalu agent tujuan disuruh membacanya dulu |
| **Pemakaian akun** | Persentase limit 5 jam & 7 hari (Claude, dari statusline) atau jendela limit Codex beserta jam resetnya. Statusline Claude di terminal menampilkan `⚡ Kilat · <akun> · PLAN · 5 jam x% · 7 hari y%` |
| **Banyak akun GitHub** | Dipilih per project di status bar. Berlaku **langsung**, termasuk di terminal dan agent yang sedang berjalan |
| **Editor & diff** | CodeMirror 6 + file explorer, panel *Perubahan* (git status + diff), commit / push / branch / PR, pesan commit otomatis |
| **Terminal bash** | Git Bash tambahan di panel bawah (`Ctrl+\``) |
| **Identitas akun** | Nama akun wajib dan unik. Email, nama, dan plan dideteksi otomatis dan tampil di dropdown akun di atas terminal |

### Di atas terminal thread

- **Dropdown akun**: semua akun Claude dan ChatGPT. Memilih akun lain = agent dijalankan ulang dengan akun itu, konteks tetap utuh.
- **Model** dan **mode izin**: *Akses penuh* (tanpa minta izin), *Setelan CLI* (ikut setelan CLI-nya, mis. mode auto Claude), *Minta izin*, *Plan / read-only*. Mengganti = agent dijalankan ulang di sesi yang sama.
- **↻** mulai ulang (sesi dilanjutkan), **⏹** hentikan.
- `Shift+Enter` baris baru, `Ctrl+V` tempel, `Ctrl+C` salin kalau ada teks terpilih (selain itu dikirim ke agent).

> Mode *Akses penuh* memakai `--dangerously-skip-permissions`. Claude Code menampilkan peringatan sekali di terminal yang harus kamu setujui sendiri.

## Setup sekali di awal

Buka **👤 Akun & Pengaturan**:

- **Claude (langganan Pro/Max)**: isi nama → *Login langganan*. Dua langkah, masing-masing satu klik *Izinkan* di browser:
  1. `claude setup-token`: token 1 tahun ditangkap otomatis dari terminal, dicek ke Anthropic, lalu disimpan di Windows Credential Manager.
  2. `claude auth login` di folder terisolasi: hanya untuk membaca email & paket, lalu langsung dihapus.
- **Claude (API key)**: isi nama → *Pakai API key*.
- **ChatGPT**: isi nama → *Login ChatGPT* (`codex login` di `CODEX_HOME` milik akun itu).
- **GitHub**: *Login akun GitHub baru* (`gh auth login`) atau *Impor dari gh*.

Setelah itu cukup pilih akun di dropdown thread. Tidak perlu login lagi.

### Cara kerja ganti akun GitHub tanpa restart terminal

Setiap terminal/agent di sebuah project dijalankan dengan:

```
GIT_CONFIG_GLOBAL = %APPDATA%\dev.kilat.code\gitctx\<project>\gitconfig
KILAT_GITCTX      = %APPDATA%\dev.kilat.code\gitctx\<project>
PATH              = %APPDATA%\dev.kilat.code\bin;...   (shim gh / gh.cmd)
```

`gitconfig` meng-*include* `~/.gitconfig` milikmu, lalu menimpa `user.name/email` dan memasang credential helper yang mengambil token akun terpilih dari `gh auth token --user <login>`. Ganti akun hanya menulis ulang dua file kecil itu, jadi efeknya langsung.

## Cara kerja hook (Claude)

Claude dijalankan dengan `--settings %APPDATA%\dev.kilat.code\agent\claude-settings.json`. File itu memasang hook `SessionStart`, `UserPromptSubmit`, `Stop`, `StopFailure`, `Notification` dan statusline, yang semuanya memanggil `kilat.exe --kilat-hook` / `--kilat-statusline`. Proses kecil itu meneruskan JSON dari Claude ke Kilat lewat socket `127.0.0.1` (alamat + kunci acak ada di environment agent), lalu langsung keluar. Hook milikmu sendiri tetap jalan. Kalau kamu punya `statusLine` sendiri di `~/.claude/settings.json`, Kilat tidak menimpanya (pemakaian 5 jam/7 hari Claude jadi tidak terbaca).

## Pengembangan

Butuh: Node 22+, pnpm, Rust (MSVC), Git for Windows, plus `claude`, `codex`, dan `gh` di PATH.

```bash
pnpm install
pnpm tauri dev        # mode pengembangan
pnpm tauri build      # installer NSIS di src-tauri/target/release/bundle/nsis/
```

Unit test backend: `cd src-tauri && cargo test --lib`

Linux butuh paket build: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libdbus-1-dev`. macOS butuh Xcode Command Line Tools.

### Rilis

1. Naikkan versi di `package.json`, `src-tauri/Cargo.toml`, dan `src-tauri/tauri.conf.json`.
2. Commit, lalu `git tag vX.Y.Z && git push origin main vX.Y.Z`.
3. GitHub Actions (`.github/workflows/release.yml`) membuild Windows, macOS (universal), dan Linux, lalu menerbitkan Release beserta `latest.json` untuk auto-update.

Update ditandatangani dengan kunci di `~/.tauri/kilat-updater.key` (secret repo `TAURI_SIGNING_PRIVATE_KEY`). **Simpan cadangan kunci ini.** Kalau hilang, versi yang sudah terpasang tidak bisa menerima update lagi.

Menjalankan build dev berdampingan dengan Kilat yang terpasang tanpa berbagi data: set `KILAT_DATA_DIR` ke folder lain (dan `WEBVIEW2_USER_DATA_FOLDER` kalau perlu opsi WebView2 berbeda).

### Struktur

```
src/                       UI (Svelte 5)
  lib/state.svelte.ts      state global + event agent
  lib/limits.ts            membaca jam reset dari pesan limit
  components/ThreadView    header thread (akun, model, mode, status) + terminal
  components/AgentTerminal xterm yang menempel ke proses agent di backend
  components/LimitDialog   pilihan saat kena limit
src-tauri/src/
  agents/mod.rs            proses agent per thread (PTY), status, limit, tunggu, pindah akun/provider
  agents/launch.rs         argumen `claude` / `codex` + file settings hook
  agents/hook.rs           jembatan hook/statusline Claude → Kilat (socket lokal)
  agents/codex.rs          membaca rollout Codex (id sesi, status, limit, pemakaian)
  agents/transcript.rs     membaca transkrip asli & menulis file oper konteks
  usage.rs                 pemakaian limit akun
  accounts.rs              akun, keychain, CODEX_HOME per akun, deteksi identitas
  gitctx.rs / gitops.rs    identitas GitHub per project, commit / push / PR
  pty.rs                   terminal bash
  store.rs                 state.json + threads/*.json di %APPDATA%\dev.kilat.code
```

## Catatan

- Thread lama dari versi chat tetap bisa dibuka: sesi Claude/Codex aslinya di-resume di terminal.
- Kode versi chat disimpan di `backup/chat-ui-2026-11-24/` (bisa dihapus kalau tidak perlu).
- Sesi juga tersimpan di `~/.claude` / `~/.codex` seperti biasa, jadi bisa dilanjutkan dari terminal lain (`claude --resume`, `codex resume`).
