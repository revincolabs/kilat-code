// Languages of the site. Indonesian lives at the root, English under /en.
import { REPO_URL, VERSION } from "../site";

export const LANGS = ["id", "en"] as const;
export type Lang = (typeof LANGS)[number];
export const DEFAULT_LANG: Lang = "id";
/** Language searchers without a matching hreflang get (the "global" version). */
export const X_DEFAULT: Lang = "en";

/** BCP-47 tags for schema.org `inLanguage`, and Open Graph locales. */
export const LOCALE = { id: "id-ID", en: "en-US" } as const;
export const OG_LOCALE = { id: "id_ID", en: "en_US" } as const;

/** Path of a page in a language. `path` is the page path without a language
 *  prefix and without a trailing slash: "/" or "/docs". */
export function localePath(lang: Lang, path = "/"): string {
  if (lang === DEFAULT_LANG) return path;
  return path === "/" ? "/en" : `/en${path}`;
}

export const other = (lang: Lang): Lang => (lang === "id" ? "en" : "id");

/** Format a size in MB the way each language writes decimals. */
export const mb = (lang: Lang, n: number) => `${n.toLocaleString(LOCALE[lang], { maximumFractionDigits: 1 })} MB`;

/** Anchors of the home page sections and docs headings, per language. */
export const anchors = {
  id: {
    features: "fitur",
    limits: "limit",
    start: "mulai",
    download: "download",
    faq: "faq",
    docsStart: "mulai-cepat",
    docsRequirements: "persyaratan",
    docsAccounts: "menambah-akun",
    docsLimits: "saat-kena-limit",
    docsTrouble: "pemecahan-masalah",
  },
  en: {
    features: "features",
    limits: "limits",
    start: "get-started",
    download: "download",
    faq: "faq",
    docsStart: "quick-start",
    docsRequirements: "requirements",
    docsAccounts: "adding-accounts",
    docsLimits: "when-you-hit-a-limit",
    docsTrouble: "troubleshooting",
  },
} satisfies Record<Lang, Record<string, string>>;

/** Strings shared by the layout and components. */
export const ui = {
  id: {
    skip: "Langsung ke isi",
    home: "Kilat Code, beranda",
    nav: { features: "Fitur", limits: "Saat kena limit", docs: "Dokumentasi", faq: "FAQ", download: "Download" },
    navLabel: "Navigasi utama",
    github: "Kode sumber di GitHub",
    menuOpen: "Buka menu",
    menuClose: "Tutup menu",
    switchTo: "Baca dalam bahasa Inggris",
    switchLabel: "EN",
    downloadFor: (os: string) => `Download untuk ${os}`,
    yourSystem: "Sistem kamu",
    dlFoot: `Versi ${VERSION}. <a href="${REPO_URL}/releases/latest">Semua file dan catatan rilis ada di GitHub</a>.`,
    footTagline: 'Terminal Claude Code dan Codex yang asli, banyak akun, satu jendela. Dibuat dan dirawat oleh <a href="https://revincolabs.com">Revinco Labs</a>.',
    footCols: [
      { title: "Produk", links: [["Fitur", "/#fitur"], ["Saat kena limit", "/#limit"], ["Download", "/#download"], ["Catatan rilis", `${REPO_URL}/releases`]] },
      { title: "Dokumentasi", links: [["Mulai cepat", "/docs#mulai-cepat"], ["Menambah akun", "/docs#menambah-akun"], ["Pemecahan masalah", "/docs#pemecahan-masalah"]] },
      { title: "Kode", links: [["GitHub", REPO_URL], ["Lapor masalah", `${REPO_URL}/issues`], ["Revinco Labs", "https://revincolabs.com"]] },
    ],
    disclaimer:
      "Kilat Code adalah proyek independen dan tidak berafiliasi dengan, didukung, atau disponsori oleh Anthropic maupun OpenAI. Claude dan Claude Code adalah merek dagang Anthropic. ChatGPT dan Codex adalah merek dagang OpenAI.",
    docs: {
      title: "Dokumentasi Kilat Code: instalasi, akun, dan limit",
      description:
        "Panduan Kilat Code: instalasi di Windows, macOS, dan Linux, menambah akun Claude, ChatGPT, dan GitHub, ganti akun, saat kena limit, dan pemecahan masalah.",
      crumb: "Dokumentasi",
      h1: "Panduan Kilat Code",
      lead: "Kilat Code menjalankan terminal <strong>Claude Code</strong> dan <strong>Codex</strong> yang asli di dalam satu aplikasi, dengan banyak akun yang bisa diganti tanpa login ulang. Halaman ini menjelaskan semuanya, dari instalasi sampai cara kerjanya.",
      updated: "Diperbarui",
      contents: "Daftar isi",
      tocLabel: "Daftar isi",
      groups: { "mulai-cepat": "Mulai", "project-dan-thread": "Memakai", "editor-diff-dan-commit": "Alat lain", pengaturan: "Referensi" } as Record<string, string>,
    },
    notFound: {
      title: "Halaman tidak ditemukan",
      text: "Alamat ini tidak ada di situs Kilat Code. Mungkin halamannya sudah dipindah.",
      home: "Ke beranda",
      docs: "Buka dokumentasi",
    },
  },
  en: {
    skip: "Skip to content",
    home: "Kilat Code, home",
    nav: { features: "Features", limits: "Usage limits", docs: "Docs", faq: "FAQ", download: "Download" },
    navLabel: "Main navigation",
    github: "Source code on GitHub",
    menuOpen: "Open menu",
    menuClose: "Close menu",
    switchTo: "Baca dalam Bahasa Indonesia",
    switchLabel: "ID",
    downloadFor: (os: string) => `Download for ${os}`,
    yourSystem: "Your system",
    dlFoot: `Version ${VERSION}. <a href="${REPO_URL}/releases/latest">All files and release notes are on GitHub</a>.`,
    footTagline: 'The real Claude Code and Codex terminals, many accounts, one window. Built and maintained by <a href="https://revincolabs.com">Revinco Labs</a>.',
    footCols: [
      { title: "Product", links: [["Features", "/en#features"], ["Usage limits", "/en#limits"], ["Download", "/en#download"], ["Release notes", `${REPO_URL}/releases`]] },
      { title: "Documentation", links: [["Quick start", "/en/docs#quick-start"], ["Adding accounts", "/en/docs#adding-accounts"], ["Troubleshooting", "/en/docs#troubleshooting"]] },
      { title: "Code", links: [["GitHub", REPO_URL], ["Report a bug", `${REPO_URL}/issues`], ["Revinco Labs", "https://revincolabs.com"]] },
    ],
    disclaimer:
      "Kilat Code is an independent project and is not affiliated with, endorsed by, or sponsored by Anthropic or OpenAI. Claude and Claude Code are trademarks of Anthropic. ChatGPT and Codex are trademarks of OpenAI.",
    docs: {
      title: "Kilat Code docs: install, accounts, and usage limits",
      description:
        "How to install Kilat Code on Windows, macOS, and Linux, add Claude, ChatGPT, and GitHub accounts, switch accounts, handle usage limits, and fix common problems.",
      crumb: "Documentation",
      h1: "Kilat Code guide",
      lead: "Kilat Code runs the real <strong>Claude Code</strong> and <strong>Codex</strong> terminals in one app, with multiple accounts you can switch between without logging in again. This page covers everything from installation to how it works.",
      updated: "Updated",
      contents: "Contents",
      tocLabel: "Table of contents",
      groups: { "quick-start": "Start", "projects-and-threads": "Using Kilat", "editor-diff-and-commit": "Other tools", settings: "Reference" } as Record<string, string>,
    },
    notFound: {
      title: "Page not found",
      text: "This address doesn't exist on the Kilat Code site. The page may have moved.",
      home: "Go to the home page",
      docs: "Open the docs",
    },
  },
};

/** Long date in the page language, e.g. "24 November 2026" / "November 24, 2026". */
export const longDate = (lang: Lang, iso: string) =>
  new Date(`${iso}T00:00:00Z`).toLocaleDateString(LOCALE[lang], { day: "numeric", month: "long", year: "numeric", timeZone: "UTC" });
