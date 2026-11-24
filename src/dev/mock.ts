// Dev-only design preview (preview.html): the real UI on a mocked Tauri
// backend with example data. Runs before the app (see preview.ts).
// Scenarios via ?s=…
//   limit    (default) a Claude thread that hit its limit, with the limit dialog
//   run      a running agent
//   work     a running agent, Perubahan panel open
//   editor   a running agent, Editor panel open
//   wait     waiting for the limit to reset (wait bar)
//   welcome  no thread selected
//   accounts the Akun & Pengaturan dialog
// Add &w=1 for a terminal panel at the bottom.
import { mockIPC } from "@tauri-apps/api/mocks";
import type { Account, AppData, ThreadMeta } from "../lib/api";

const q = new URLSearchParams(location.search);
const scenario = q.get("s") ?? "limit";
const now = Date.now();
const MIN = 60_000;
const HOUR = 60 * MIN;

// A reset on a full hour, at least 40 minutes away, so the dialog text and the
// CLI message agree.
const resetAt = (() => {
  const d = new Date(now + 40 * MIN);
  d.setMinutes(0, 0, 0);
  d.setHours(d.getHours() + 1);
  return d.getTime();
})();
const resetText = (() => {
  const h = new Date(resetAt).getHours();
  return `${h % 12 || 12}${h < 12 ? "am" : "pm"}`;
})();

function acc(p: Partial<Account> & Pick<Account, "id" | "provider" | "label">): Account {
  return {
    kind: "oauth",
    email: null,
    displayName: null,
    plan: null,
    login: null,
    gitName: null,
    gitEmail: null,
    sshKey: null,
    orgId: null,
    ready: true,
    limitedUntil: null,
    usage: null,
    createdAt: now - 30 * 24 * HOUR,
    ...p,
  };
}
const usage = (five: number, week: number) => ({
  updatedAt: now - 2 * MIN,
  windows: [
    { label: "5 jam", usedPercent: five, resetsAt: five >= 100 ? resetAt : now + 3 * HOUR },
    { label: "7 hari", usedPercent: week, resetsAt: now + 4 * 24 * HOUR },
  ],
});

const accounts: Account[] = [
  acc({ id: "c-kerja", provider: "claude", label: "Kerja", email: "dev@kantor.co.id", plan: "max", usage: usage(scenario === "limit" || scenario === "wait" ? 100 : 71, 64), limitedUntil: scenario === "limit" || scenario === "wait" ? resetAt : null }),
  acc({ id: "c-pribadi", provider: "claude", label: "Pribadi", email: "saya@contoh.id", plan: "pro", usage: usage(12, 30) }),
  acc({ id: "c-default", provider: "claude", kind: "default", label: "Claude (login bawaan)", email: "saya@contoh.id", plan: "pro", usage: usage(12, 30) }),
  acc({ id: "x-tim", provider: "codex", kind: "chatgpt", label: "Tim", email: "tim@kantor.co.id", plan: "plus", usage: usage(18, 22) }),
  acc({ id: "x-default", provider: "codex", kind: "default", label: "ChatGPT (login bawaan)", email: "saya@contoh.id", plan: "go" }),
  acc({ id: "g-1", provider: "github", kind: "gh", label: "revincolabs", login: "revincolabs", gitName: "revincolabs", gitEmail: "dev@revincolabs.example" }),
  acc({ id: "g-2", provider: "github", kind: "gh", label: "dev-kantor", login: "dev-kantor", gitName: "Dev Kantor", gitEmail: "dev@kantor.co.id" }),
];

function thread(p: Partial<ThreadMeta> & Pick<ThreadMeta, "id" | "projectId" | "title">): ThreadMeta {
  return {
    provider: "claude",
    accountId: "c-pribadi",
    model: "opus",
    mode: "full",
    cwd: "G:/kode/toko-online",
    worktreeBranch: null,
    status: "ready",
    pinned: false,
    archived: false,
    limit: null,
    waitUntil: null,
    createdAt: now - 2 * HOUR,
    updatedAt: now - 30 * MIN,
    ...p,
  };
}

const limited = scenario === "limit" || scenario === "wait";
const threads: ThreadMeta[] = [
  thread({
    id: "t1",
    projectId: "p1",
    title: "Refactor pembayaran",
    accountId: "c-kerja",
    status: limited ? "limited" : "running",
    limit: limited ? { accountId: "c-kerja", provider: "claude", at: now - 3 * MIN, resetsAt: resetAt, message: `You've hit your session limit · resets ${resetText}` } : null,
    waitUntil: scenario === "wait" ? resetAt + MIN : null,
    updatedAt: now - 3 * MIN,
  }),
  thread({ id: "t2", projectId: "p1", title: "Halaman checkout baru", provider: "codex", accountId: "x-tim", model: "", status: "running", worktreeBranch: "kilat/checkout", updatedAt: now - 1 * MIN }),
  thread({ id: "t3", projectId: "p1", title: "Perbaiki bug login", status: "attention", updatedAt: now - 9 * MIN }),
  thread({ id: "t4", projectId: "p2", title: "Migrasi database", cwd: "G:/kode/api-server", status: "running", updatedAt: now - 4 * MIN }),
  thread({ id: "t5", projectId: "p2", title: "Tulis dokumentasi API", provider: "codex", accountId: "x-tim", model: "", cwd: "G:/kode/api-server", status: "ready", updatedAt: now - 3 * HOUR }),
  thread({ id: "t6", projectId: "p2", title: "Coba Bun untuk test", cwd: "G:/kode/api-server", status: "off", archived: true, updatedAt: now - 5 * 24 * HOUR }),
];

const data: AppData = {
  projects: [
    { id: "p1", name: "toko-online", path: "G:/kode/toko-online", githubAccountId: "g-1", createdAt: now - 20 * 24 * HOUR },
    { id: "p2", name: "api-server", path: "G:/kode/api-server", githubAccountId: "g-2", createdAt: now - 10 * 24 * HOUR },
  ],
  accounts,
  threads,
  settings: {
    bashPath: "",
    claudePath: "",
    codexPath: "",
    ghPath: "",
    claudeModels: ["opus", "sonnet", "haiku"],
    fontSize: 13,
    notify: true,
    sound: true,
    claudeChrome: false,
    continuePrompt: "Lanjutkan pekerjaan yang tadi terhenti karena limit akun.",
  },
};

// ---------------------------------------------------------- terminal screens

const esc = (code: string, s: string) => `\x1b[${code}m${s}\x1b[0m`;
const clay = (s: string) => esc("38;2;215;119;87", s);
const dim = (s: string) => esc("38;2;128;127;120", s);
const green = (s: string) => esc("38;2;111;201;139", s);
const red = (s: string) => esc("38;2;242;104;93", s);
const blue = (s: string) => esc("38;2;141;183;255", s);
const bold = (s: string) => esc("1", s);
const rule = dim("─".repeat(56));

function claudeScreen(t: ThreadMeta): string {
  const a = accounts.find((x) => x.id === t.accountId)!;
  const status = `${clay("⚡")} ${dim(`Kilat · ${a.label} · ${a.plan?.toUpperCase()} · ${a.usage?.windows.map((w) => `${w.label} ${w.usedPercent}%`).join(" · ")}`)}`;
  const head = [
    ` ${clay("▐▛███▜▌")}   ${bold("Claude Code")}`,
    `${clay("▝▜█████▛▘")}  ${dim(`Opus · Claude ${a.plan === "max" ? "Max" : "Pro"}`)}`,
    `  ${clay("▘▘ ▝▝")}    ${dim(t.cwd.replaceAll("/", "\\"))}`,
    "",
  ];
  const body =
    t.id === "t1"
      ? [
          `${dim(">")} refactor modul pembayaran, lalu jalankan semua test`,
          "",
          `● Saya mulai dari ${blue("src/payments/")}. Ada 6 file yang terlibat.`,
          "",
          `${green("●")} ${bold("Read")}(src/payments/charge.ts)`,
          `  ${dim("⎿  Read 214 lines")}`,
          "",
          `${green("●")} ${bold("Update")}(src/payments/charge.ts)`,
          `  ${dim("⎿  Updated src/payments/charge.ts with")} ${green("18 additions")} ${dim("and")} ${red("9 removals")}`,
          "",
          `${green("●")} ${bold("Bash")}(npm test)`,
          `  ${dim("⎿")}  42 passed, ${red("3 failed")}`,
          "",
          "● Memperbaiki 3 test yang gagal di charge.test.ts…",
          ...(limited ? [`  ${dim("⎿")}  ${red(`You've hit your session limit · resets ${resetText}`)}`] : [`${clay("✻")} ${clay("Menjalankan ulang test…")} ${dim("(48s · esc to interrupt)")}`]),
          "",
        ]
      : [
          `${dim(">")} buat migrasi untuk kolom invoice_due_date dan isi data lamanya`,
          "",
          `${green("●")} ${bold("Read")}(db/schema.sql)`,
          `  ${dim("⎿  Read 388 lines")}`,
          "",
          `${green("●")} ${bold("Write")}(db/migrations/0042_invoice_due_date.sql)`,
          `  ${dim("⎿  Wrote 31 lines to db/migrations/0042_invoice_due_date.sql")}`,
          "",
          `${clay("✻")} ${clay("Menjalankan migrasi di database lokal…")} ${dim("(12s · esc to interrupt)")}`,
          "",
        ];
  return [...head, ...body, rule, `${dim(">")} `, rule, `  ${status}`].join("\r\n");
}

function codexScreen(t: ThreadMeta): string {
  return [
    dim("╭────────────────────────────────────────────────╮"),
    `${dim("│")} ${bold(">_ OpenAI Codex")}                                 ${dim("│")}`,
    `${dim("│")}                                                ${dim("│")}`,
    `${dim("│")} ${dim("directory:")} ${t.cwd.replaceAll("/", "\\").padEnd(36)}${dim("│")}`,
    dim("╰────────────────────────────────────────────────╯"),
    "",
    `${dim("›")} buat halaman checkout baru dengan ringkasan pesanan`,
    "",
    `${blue("•")} Menambahkan komponen ringkasan pesanan dan validasi alamat.`,
    `  ${dim("└ Edited src/pages/checkout.tsx (+86 -4)")}`,
    "",
    `${blue("•")} ${dim("Working (31s · esc to interrupt)")}`,
  ].join("\r\n");
}

const bashScreen = `${green("saya@laptop")} ${clay("MINGW64")} ${blue("/g/kode/toko-online")} ${blue("(refactor/pembayaran)")}\r\n$ `;

// ------------------------------------------------------------------- git

const diff = `diff --git a/src/payments/charge.ts b/src/payments/charge.ts
index 4c1d2e0..9a7f3b1 100644
--- a/src/payments/charge.ts
+++ b/src/payments/charge.ts
@@ -12,14 +12,23 @@ import { Money } from "../money";
 export async function charge(order: Order, card: CardToken) {
-  const total = order.items.reduce((s, i) => s + i.price * i.qty, 0);
-  if (total <= 0) throw new Error("total tidak valid");
+  const total = orderTotal(order);
+  if (total.isZero()) {
+    throw new PaymentError("EMPTY_ORDER", "Pesanan kosong");
+  }
   const fee = gatewayFee(total);
-  return gateway.charge(card, total + fee);
+  const result = await gateway.charge(card, total.add(fee));
+  await ledger.record(order.id, result);
+  return result;
 }
+
+export function orderTotal(order: Order): Money {
+  return order.items.reduce((s, i) => s.add(i.price.times(i.qty)), Money.zero());
+}`;

const files: Record<string, string> = {
  "src/payments/charge.ts": `import { Money } from "../money";
import { gateway, gatewayFee } from "./gateway";
import { ledger } from "./ledger";

export async function charge(order: Order, card: CardToken) {
  const total = orderTotal(order);
  if (total.isZero()) {
    throw new PaymentError("EMPTY_ORDER", "Pesanan kosong");
  }
  const fee = gatewayFee(total);
  const result = await gateway.charge(card, total.add(fee));
  await ledger.record(order.id, result);
  return result;
}
`,
};

// ------------------------------------------------------------------- IPC

let views = 0;
mockIPC(
  (cmd, args: any) => {
    switch (cmd) {
      case "bootstrap":
        return {
          data: structuredClone(data),
          tools: { bash: "C:/Program Files/Git/bin/bash.exe", claude: "claude", codex: "codex", gh: "gh" },
          dataDir: "C:/Users/saya/AppData/Roaming/dev.kilat.code",
          version: "0.2.0",
        };
      case "codex_models":
        return [];
      case "agent_attach": {
        const t = threads.find((x) => x.id === args.threadId)!;
        setTimeout(() => args.onEvent.onmessage({ type: "data", data: t.provider === "claude" ? claudeScreen(t) : codexScreen(t) }), 50);
        return { viewId: ++views, live: true };
      }
      case "pty_spawn":
        setTimeout(() => args.onEvent.onmessage({ type: "data", data: bashScreen }), 50);
        return "pty-1";
      case "git_status":
        return {
          isRepo: true,
          branch: "refactor/pembayaran",
          ahead: 2,
          behind: 0,
          files: [
            { path: "src/payments/charge.ts", status: "M" },
            { path: "src/payments/ledger.ts", status: "M" },
            { path: "src/payments/errors.ts", status: "A" },
            { path: "tests/charge.test.ts", status: "M" },
            { path: "src/payments/legacy.ts", status: "D" },
          ],
        };
      case "git_recent":
        return [
          { hash: "7bb51df", subject: "Pisahkan perhitungan total pesanan", when: "2 jam lalu" },
          { hash: "5fd01da", subject: "Catat setiap transaksi di ledger", when: "kemarin" },
          { hash: "a3ad600", subject: "Tambah PaymentError dengan kode", when: "2 hari lalu" },
          { hash: "358b468", subject: "Perbarui SDK gateway ke v4", when: "3 hari lalu" },
        ];
      case "git_diff":
        return diff;
      case "fs_list_dir": {
        const base = String(args.path).replace(/\/$/, "");
        const top = ["src", "tests", "db", "package.json", "README.md", "tsconfig.json"];
        const src = ["payments", "pages", "auth", "money.ts"];
        const pay = ["charge.ts", "errors.ts", "gateway.ts", "ledger.ts"];
        const pick = base.endsWith("/payments") ? pay : base.endsWith("/src") ? src : top;
        return pick.map((name) => ({ name, path: `${base}/${name}`, isDir: !name.includes(".") }));
      }
      case "fs_read_file":
        return files[Object.keys(files).find((k) => String(args.path).endsWith(k)) ?? ""] ?? "// contoh\n";
      case "fs_list_files":
        return ["src/payments/charge.ts", "src/payments/ledger.ts", "src/payments/gateway.ts", "src/pages/checkout.tsx", "tests/charge.test.ts"];
      default:
        return null;
    }
  },
  { shouldMockEvents: true },
);

// Which project/thread is open comes from localStorage at start-up.
const open: Record<string, string | null> = { run: "t1", limit: "t1", wait: "t1", work: "t1", editor: "t1", accounts: "t1", welcome: null };
try {
  localStorage.setItem("kilat.project", "p1");
  if (open[scenario]) localStorage.setItem("kilat.thread", open[scenario]!);
  else localStorage.removeItem("kilat.thread");
} catch {}


export { scenario, q };
