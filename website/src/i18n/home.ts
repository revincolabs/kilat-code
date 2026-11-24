// Home page copy, one object per language. HTML is allowed where marked.
import type { Lang } from "./index";

type Row = [title: string, text: string];

export interface HomeCopy {
  title: string;
  description: string;
  eyebrow: string;
  h1: [string, string];
  lead: string;
  meta: string;
  alt: { limit: string; wait: string; terminal: string; sidebar: string; changes: string };
  panelCaption: string;
  facts: { big: string; text: string; small: string }[];
  problem: { label: string; title: string; lead: string; trio: string[]; after: string };
  limits: { label: string; title: string; intro: string; rows: Row[]; more: string; caption: string };
  terminal: { label: string; title: string; intro: string; rows: Row[]; caption: string };
  handoff: {
    label: string;
    title: string;
    intro: string;
    rows: Row[];
    aria: string;
    hops: { provider: "claude" | "codex"; tag: string; account: string; task: string; state: string; warn?: boolean }[];
    links: [string, string];
    caption: string;
  };
  agents: { label: string; title: string; intro: string; rows: Row[]; legend: string[]; letters: string };
  around: { label: string; title: string; lead: string; caption: string; extras: Row[] };
  start: { label: string; title: string; steps: Row[]; note: string };
  download: { label: string; title: string; lead: string; aria: string };
  faq: { label: string; title: string; items: Row[] };
  cta: { label: string; title: string; lead: string };
}

export const home: Record<Lang, HomeCopy> = {
  id: {
    title: "Kilat Code: Claude Code dan Codex dengan banyak akun",
    description:
      "Jalankan terminal Claude Code dan Codex asli dengan banyak akun. Kena limit? Tunggu sampai reset atau lanjut di akun lain tanpa kehilangan konteks.",
    eyebrow: "Untuk Claude Code dan Codex",
    h1: ["Kuota habis?", "Lanjut di akun lain."],
    lead: "Kilat Code menjalankan terminal Claude Code dan Codex yang asli, dengan banyak akun yang tinggal dipilih. Saat satu akun kena limit, tunggu sampai reset atau lanjut di akun lain tanpa kehilangan konteks.",
    meta: "Windows, macOS, Linux · Gratis",
    alt: {
      limit:
        "Kilat Code: daftar thread di kiri, terminal Claude Code di kanan, dan dialog pilihan karena akun Claude bernama Kerja kena limit",
      wait: "Bar oranye di atas terminal: menunggu limit akun Kerja reset, lalu lanjut otomatis, dengan sisa waktu yang dihitung mundur",
      terminal: "Terminal Claude Code di dalam Kilat, dengan bar akun, model, dan mode izin di atasnya",
      sidebar: "Daftar thread Kilat dengan status tiap agent",
      changes: "Kilat dengan panel Perubahan terbuka: daftar file yang berubah, kotak pesan commit, tombol commit, push, dan buat PR",
    },
    panelCaption: 'Akun Claude "Kerja" kena limit di tengah refactor. Kilat menawarkan tiga cara melanjutkan, langsung di atas terminal.',
    facts: [
      { big: "2,9 MB", text: "Installer Windows. Dibuat dengan Tauri dan Rust, bukan Electron.", small: "macOS 7,3 MB · Linux .deb 4,5 MB" },
      { big: "0 server", text: "Token tersimpan di penyimpan rahasia sistem. Kilat tidak punya server sendiri.", small: "Credential Manager · Keychain · Secret Service" },
      { big: "Gratis", text: "Pakai langganan Claude atau ChatGPT, atau API key, milikmu sendiri.", small: "Tanpa akun Kilat" },
    ],
    problem: {
      label: "Masalahnya",
      title: "Limit datang persis di tengah pekerjaan",
      lead: "Claude Code dan Codex sudah bagus. Yang melelahkan adalah semua di sekitarnya.",
      trio: [
        "Kuota habis persis di tengah tugas, lalu kamu harus menunggu jam reset.",
        "Ganti akun berarti logout, lalu login lagi.",
        "Lima jendela terminal, dan tidak jelas mana yang sudah selesai atau butuh izin.",
      ],
      after: "Kilat mengurus bagian itu dan membiarkan agent-nya apa adanya: tiap thread adalah terminal Claude Code atau Codex yang asli.",
    },
    limits: {
      label: "Saat kena limit",
      title: "Kuota habis, pekerjaan tetap jalan",
      intro: "Kilat menangkap limit dari hook Claude Code dan file sesi Codex, lalu langsung menawarkan jalan keluar di atas terminal agent itu.",
      rows: [
        ["Tunggu, lalu lanjut sendiri", "Jam reset dibaca dari CLI. Saat waktunya tiba, agent dijalankan ulang dan diminta melanjutkan, selama Kilat terbuka."],
        ["Pindah ke akun Claude lain", "Sesi yang sama dilanjutkan, lengkap dengan seluruh percakapannya."],
        ["Pindah ke akun ChatGPT", "Riwayat percakapan diteruskan ke Codex sebagai konteks."],
      ],
      more: "Cara kerja saat kena limit",
      caption: 'Setelah memilih "tunggu": Kilat menghitung mundur, lalu menjalankan ulang agent di sesi yang sama.',
    },
    terminal: {
      label: "Terminal asli",
      title: "Claude Code dan Codex, apa adanya",
      intro: "Tidak ada antarmuka chat buatan. Slash command, <code>/model</code>, izin, dan mode bash jalan persis seperti di terminalmu.",
      rows: [
        ["Agent tetap jalan di belakang", "saat kamu pindah thread."],
        ["Statusline menunjukkan akun", "yang sedang dipakai beserta sisa kuotanya."],
        ["Akun, model, dan mode izin", "diganti dari bar di atas terminal."],
      ],
      caption: "Baris paling bawah adalah statusline dari Kilat: akun, paket, dan pemakaian kuota.",
    },
    handoff: {
      label: "Pindah akun",
      title: "Ganti akun atau provider tanpa mulai dari nol",
      intro: "Kilat mencatat akun dan sesi asli di setiap bagian percakapan, jadi tiap jenis perpindahan memakai cara yang paling setia.",
      rows: [
        ["Claude ke Claude", "Sesi yang sama di-<code>--resume</code>, hanya tokennya yang berganti."],
        ["ChatGPT ke ChatGPT", "Folder sesi dibagi, <code>codex resume</code> melanjutkan thread."],
        ["Claude ke ChatGPT, atau sebaliknya", "Bagian yang belum dilihat agent baru diteruskan sebagai file riwayat."],
      ],
      aria: "Contoh satu thread yang berpindah akun dua kali",
      hops: [
        { provider: "claude", tag: "Claude", account: "Kerja", task: "refactor modul pembayaran", state: "kena limit", warn: true },
        { provider: "claude", tag: "Claude", account: "Pribadi", task: "melanjutkan perbaikan test", state: "lanjut" },
        { provider: "codex", tag: "ChatGPT", account: "Tim", task: "review dan rapikan kode", state: "konteks utuh" },
      ],
      links: ["resume sesi yang sama · <code>claude --resume</code>", "riwayat diteruskan sebagai file handoff"],
      caption: "Satu thread, tiga akun, dua provider. Percakapan tidak pernah diulang dari awal.",
    },
    agents: {
      label: "Banyak agent",
      title: "Banyak agent, satu jendela",
      intro: "Jalankan beberapa thread sekaligus di project yang sama atau berbeda. Kilat memberi tahu saat ada yang selesai atau butuh kamu.",
      rows: [
        ["Worktree git terpisah", "per thread, supaya agent tidak saling menimpa file."],
        ["Notifikasi sistem dan suara", "saat agent selesai, butuh izin, atau kena limit."],
      ],
      legend: [
        "<b>Bekerja.</b> Titik kuning berkedip.",
        "<b>Siap.</b> Agent menunggu perintahmu.",
        "<b>Butuh izin atau kena limit.</b> Yang kena limit diberi ikon jam.",
        "<b>Tidak berjalan.</b> Sesinya tetap tersimpan dan bisa dilanjutkan.",
      ],
      letters: "<b>C</b> untuk Claude, <b>G</b> untuk ChatGPT. Nama akun GitHub project tampil di sebelah nama project.",
    },
    around: {
      label: "Di sekitar agent",
      title: "Supaya tidak bolak-balik ke aplikasi lain",
      lead: "Hal-hal di sekitar agent yang biasanya butuh aplikasi terpisah, ada di jendela yang sama.",
      caption:
        "Panel Perubahan: centang file, minta agent menulis pesan commit, lalu commit, push, atau buat pull request dengan akun GitHub project itu.",
      extras: [
        ["Git dan GitHub bawaan", "Editor, diff, commit, push, branch, dan pull request. Pesan commit bisa ditulis agent."],
        ["GitHub multi-akun", "Akun GitHub dipilih per project dan langsung berlaku, termasuk di terminal yang sedang jalan."],
        ["Pemakaian kuota", "Persentase limit 5 jam dan 7 hari beserta jam resetnya, di statusline dan daftar akun."],
        ["Notifikasi", "Notifikasi sistem dan suara saat agent selesai, butuh izin, atau kena limit."],
        ["Skill, plugin, dan MCP", "Semua yang sudah kamu pasang di Claude Code dan Codex tetap jalan di setiap akun."],
        ["Update otomatis", "Aplikasi memeriksa versi baru sendiri. Paket update ditandatangani, jadi hanya update resmi yang diterima."],
      ],
    },
    start: {
      label: "Mulai",
      title: "Pasang sekali, langsung kerja",
      steps: [
        ["Unduh dan pasang", "Installer untuk Windows, macOS, atau Linux. Di Windows terpasang per pengguna, tanpa hak admin."],
        ["Tambah akun", "Buka <b>Akun &amp; Pengaturan</b>, login Claude, ChatGPT, dan GitHub. Masing-masing satu kali di browser."],
        ["Buka folder, buat thread", "Pilih folder project, klik <b>Thread baru</b>. Terminal agent langsung siap dipakai."],
      ],
      note: 'Yang kamu perlukan: Git, dan Claude Code CLI atau Codex CLI. <a href="/docs#persyaratan">Lihat persyaratan lengkap</a>.',
    },
    download: {
      label: "Download",
      title: "Unduh Kilat Code",
      lead: "Gratis untuk Windows, macOS, dan Linux. Aplikasi memeriksa update sendiri, jadi cukup unduh sekali.",
      aria: "Unduh Kilat Code",
    },
    faq: {
      label: "FAQ",
      title: "Pertanyaan yang sering muncul",
      items: [
        ["Apakah Kilat Code gratis?", "Ya. Kamu tetap butuh langganan atau API key Claude dan/atau ChatGPT sendiri. Kilat hanya menjalankan CLI resminya dengan akunmu."],
        [
          "Apakah kuota saya bisa digabung dari beberapa akun?",
          "Kilat tidak menggabungkan kuota. Ia hanya memudahkan kamu memilih akun mana yang menjalankan agent, dan melanjutkan pekerjaan saat satu akun kena limit. Pastikan cara kamu memakai akun sesuai ketentuan layanan masing-masing penyedia.",
        ],
        [
          "Skill, plugin, dan MCP server saya tetap jalan?",
          "Ya. Claude Code memakai folder <code>~/.claude</code> milikmu di semua akun, jadi plugin, skill, MCP, hook, dan <code>CLAUDE.md</code> ikut. Untuk Codex, skill, aturan, prompt, plugin, dan <code>config.toml</code> dibagikan ke setiap akun ChatGPT.",
        ],
        [
          "Di mana token dan data saya disimpan?",
          "Semuanya di komputermu. Token disimpan di penyimpan rahasia sistem (Credential Manager di Windows, Keychain di macOS, Secret Service di Linux). Kilat tidak punya server sendiri.",
        ],
        [
          "Muncul peringatan keamanan saat memasang. Aman?",
          "Installer Kilat belum ditandatangani secara digital. Di Windows, klik <b>More info</b> lalu <b>Run anyway</b> pada SmartScreen. Di macOS, klik kanan aplikasi lalu <b>Open</b>. Kodenya bisa kamu periksa di GitHub.",
        ],
        [
          "Bisa dipakai di macOS atau Linux?",
          "Bisa: macOS 11+ (Apple Silicon dan Intel) dan Linux x86_64 (AppImage, .deb, atau .rpm untuk Fedora). Versi Windows yang paling banyak diuji; macOS dan Linux masih tahap awal, jadi laporan masalah sangat membantu.",
        ],
        [
          "Apa bedanya dengan menjalankan CLI di terminal biasa?",
          "Agent-nya sama persis. Kilat menambahkan banyak akun tanpa login ulang, pilihan saat kena limit, perpindahan Claude ↔ ChatGPT dengan konteks, status semua agent di satu tempat, notifikasi, editor, diff, dan GitHub multi-akun.",
        ],
      ],
    },
    cta: {
      label: "Langkah berikutnya",
      title: "Coba di project kamu sendiri",
      lead: "Tanpa akun baru: login dengan langganan Claude atau ChatGPT yang sudah kamu punya, lalu buka folder project.",
    },
  },

  en: {
    title: "Kilat Code: Claude Code and Codex with multiple accounts",
    description:
      "Run the real Claude Code and Codex CLIs with multiple accounts. Hit a usage limit? Wait for the reset or continue on another account with your context intact.",
    eyebrow: "For Claude Code and Codex",
    h1: ["Hit your usage limit?", "Use another account."],
    lead: "Kilat Code runs the real Claude Code and Codex terminals, with as many accounts as you need. When one account hits its limit, wait for the reset or continue on another account without losing context.",
    meta: "Windows, macOS, Linux · Free",
    alt: {
      limit:
        "Kilat Code: the thread list on the left, a Claude Code terminal on the right, and a dialog with options because the Claude account named Work hit its limit",
      wait: "Orange bar above the terminal: waiting for the Work account's limit to reset, then continuing automatically, with a countdown",
      terminal: "A Claude Code terminal inside Kilat, with the account, model, and permission mode bar above it",
      sidebar: "Kilat's thread list showing the status of each agent",
      changes: "Kilat with the Changes panel open: changed files, a commit message box, and commit, push, and create pull request buttons",
    },
    panelCaption:
      'The Claude account "Work" hit its limit in the middle of a refactor. Kilat offers three ways to continue, right above the terminal.',
    facts: [
      { big: "2.9 MB", text: "Windows installer. Built with Tauri and Rust, not Electron.", small: "macOS 7.3 MB · Linux .deb 4.5 MB" },
      { big: "0 servers", text: "Tokens stay in your system's secret store. Kilat has no servers of its own.", small: "Credential Manager · Keychain · Secret Service" },
      { big: "Free", text: "Use your own Claude or ChatGPT subscription, or an API key.", small: "No Kilat account" },
    ],
    problem: {
      label: "The problem",
      title: "Limits hit in the middle of your work",
      lead: "Claude Code and Codex are great. What wears you down is everything around them.",
      trio: [
        "Your quota runs out mid-task, and then you wait for the reset.",
        "Switching accounts means logging out and logging in again.",
        "Five terminal windows, and no idea which one is done or waiting for permission.",
      ],
      after: "Kilat takes care of that and leaves the agents as they are: every thread is the real Claude Code or Codex terminal.",
    },
    limits: {
      label: "Usage limits",
      title: "Out of quota, the work keeps going",
      intro: "Kilat detects limits from Claude Code hooks and Codex session files, and offers a way out right above that agent's terminal.",
      rows: [
        ["Wait, then continue automatically", "The reset time is read from the CLI. When it arrives, the agent restarts and is asked to continue, as long as Kilat is open."],
        ["Move to another Claude account", "The same session continues, with the whole conversation."],
        ["Move to a ChatGPT account", "The conversation history is handed to Codex as context."],
      ],
      more: "How usage limits are handled",
      caption: 'After choosing "wait": Kilat counts down, then restarts the agent in the same session.',
    },
    terminal: {
      label: "Real terminals",
      title: "Claude Code and Codex, as they are",
      intro: "No imitation chat UI. Slash commands, <code>/model</code>, permissions, and bash mode work exactly as in your own terminal.",
      rows: [
        ["Agents keep running in the background", "when you switch threads."],
        ["The status line shows the account", "in use and its remaining quota."],
        ["Account, model, and permission mode", "are changed from the bar above the terminal."],
      ],
      caption: "The bottom line is Kilat's status line: account, plan, and quota usage.",
    },
    handoff: {
      label: "Switching accounts",
      title: "Change account or provider without starting over",
      intro: "Kilat records the account and the original session for every part of the conversation, so each kind of switch uses the most faithful method.",
      rows: [
        ["Claude to Claude", "The same session is resumed with <code>--resume</code>; only the token changes."],
        ["ChatGPT to ChatGPT", "Session folders are shared, so <code>codex resume</code> continues the thread."],
        ["Claude to ChatGPT, or back", "The part the new agent hasn't seen is handed over as a history file."],
      ],
      aria: "Example of one thread moving between accounts twice",
      hops: [
        { provider: "claude", tag: "Claude", account: "Work", task: "refactor the payment module", state: "limit hit", warn: true },
        { provider: "claude", tag: "Claude", account: "Personal", task: "keep fixing the tests", state: "continues" },
        { provider: "codex", tag: "ChatGPT", account: "Team", task: "review and tidy the code", state: "context intact" },
      ],
      links: ["same session resumed · <code>claude --resume</code>", "history handed over as a file"],
      caption: "One thread, three accounts, two providers. The conversation never starts over.",
    },
    agents: {
      label: "Many agents",
      title: "Many agents, one window",
      intro: "Run several threads at once, in the same project or different ones. Kilat tells you when one finishes or needs you.",
      rows: [
        ["A separate git worktree", "per thread, so agents don't overwrite each other's files."],
        ["System notifications and sound", "when an agent finishes, needs permission, or hits a limit."],
      ],
      legend: [
        "<b>Working.</b> Blinking yellow dot.",
        "<b>Ready.</b> The agent is waiting for you.",
        "<b>Needs permission or hit a limit.</b> Limited threads get a clock icon.",
        "<b>Not running.</b> The session is kept and can be resumed.",
      ],
      letters: "<b>C</b> for Claude, <b>G</b> for ChatGPT. The project's GitHub account is shown next to the project name.",
    },
    around: {
      label: "Around the agents",
      title: "No more jumping between apps",
      lead: "The things around your agents that usually need separate apps are in the same window.",
      caption:
        "The Changes panel: tick files, let the agent write the commit message, then commit, push, or open a pull request with the project's GitHub account.",
      extras: [
        ["Git and GitHub built in", "Editor, diff, commit, push, branches, and pull requests. The agent can write the commit message."],
        ["Multiple GitHub accounts", "Pick a GitHub account per project. It applies immediately, including in terminals that are already running."],
        ["Quota usage", "5-hour and 7-day limit percentages with their reset times, in the status line and the account list."],
        ["Notifications", "System notifications and sound when an agent finishes, needs permission, or hits a limit."],
        ["Skills, plugins, and MCP", "Everything you've installed in Claude Code and Codex keeps working on every account."],
        ["Automatic updates", "The app checks for new versions itself. Updates are signed, so only official ones are accepted."],
      ],
    },
    start: {
      label: "Get started",
      title: "Install once, start working",
      steps: [
        ["Download and install", "Installers for Windows, macOS, and Linux. On Windows it installs for your user only, without admin rights."],
        ["Add your accounts", "Open <b>Akun &amp; Pengaturan</b> (Accounts &amp; Settings) and sign in to Claude, ChatGPT, and GitHub. Once each, in your browser."],
        ["Open a folder, start a thread", "Pick a project folder and click <b>Thread baru</b> (New thread). The agent's terminal is ready right away."],
      ],
      note: 'What you need: Git, and the Claude Code CLI or the Codex CLI. <a href="/en/docs#requirements">See the full requirements</a>.',
    },
    download: {
      label: "Download",
      title: "Download Kilat Code",
      lead: "Free for Windows, macOS, and Linux. The app checks for updates itself, so you only download it once.",
      aria: "Download Kilat Code",
    },
    faq: {
      label: "FAQ",
      title: "Frequently asked questions",
      items: [
        ["Is Kilat Code free?", "Yes. You still need your own Claude and/or ChatGPT subscription or API key. Kilat only runs the official CLIs with your accounts."],
        [
          "Does it combine the quota of several accounts?",
          "No. Kilat doesn't pool quotas. It makes it easy to choose which account runs an agent, and to continue when one account hits its limit. Make sure the way you use your accounts follows each provider's terms of service.",
        ],
        [
          "Do my skills, plugins, and MCP servers still work?",
          "Yes. Claude Code uses your own <code>~/.claude</code> folder on every account, so plugins, skills, MCP servers, hooks, and <code>CLAUDE.md</code> come along. For Codex, skills, rules, prompts, plugins, and <code>config.toml</code> are shared with every ChatGPT account.",
        ],
        [
          "Where are my tokens and data stored?",
          "On your computer. Tokens go into your system's secret store (Credential Manager on Windows, Keychain on macOS, Secret Service on Linux). Kilat has no servers of its own.",
        ],
        [
          "I get a security warning when installing. Is it safe?",
          "The Kilat installers aren't code-signed yet. On Windows, click <b>More info</b> and then <b>Run anyway</b> in SmartScreen. On macOS, right-click the app and choose <b>Open</b>. You can review the code on GitHub.",
        ],
        [
          "Does it run on macOS and Linux?",
          "Yes: macOS 11+ (Apple Silicon and Intel) and Linux x86_64 (AppImage, .deb, or .rpm for Fedora). Windows is the most tested; macOS and Linux are still early, so bug reports help a lot.",
        ],
        [
          "Is the app in English?",
          "Yes. Kilat follows your system language (English, or Indonesian on an Indonesian system), and you can switch it in Settings → Language. Messages Kilat sends to the agents follow the same language.",
        ],
        [
          "How is this different from running the CLI in a normal terminal?",
          "The agents are exactly the same. Kilat adds multiple accounts without logging in again, options when you hit a limit, Claude ↔ ChatGPT switching with context, the status of every agent in one place, notifications, an editor, diffs, and multiple GitHub accounts.",
        ],
      ],
    },
    cta: {
      label: "Next step",
      title: "Try it on your own project",
      lead: "No new account needed: sign in with the Claude or ChatGPT subscription you already have, then open a project folder.",
    },
  },
};
