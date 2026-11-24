// Everything that changes with a release lives here.

export const VERSION = "0.2.0";

export const REPO = "revincolabs/kilat-code";
export const REPO_URL = `https://github.com/${REPO}`;
export const RELEASES_URL = `${REPO_URL}/releases/latest`;

/** Release assets have version-less names (see .github/workflows/release.yml),
 *  so these links always point at the newest release. */
const latest = (file: string) => `${REPO_URL}/releases/latest/download/${file}`;

export type OsId = "windows" | "mac" | "linux";

export interface DownloadFile {
  label: string;
  url: string;
  size: string;
}

export interface Platform {
  id: OsId;
  name: string;
  requirement: string;
  files: DownloadFile[];
  note: string;
}

export const PLATFORMS: Platform[] = [
  {
    id: "windows",
    name: "Windows",
    requirement: "Windows 10/11 · 64-bit",
    files: [{ label: "Installer .exe", url: latest("KilatCode_windows_x64-setup.exe"), size: "2,9 MB" }],
    note: 'Klik "More info → Run anyway" kalau SmartScreen memperingatkan.',
  },
  {
    id: "mac",
    name: "macOS",
    requirement: "macOS 11+ · Apple Silicon & Intel",
    files: [{ label: "Disk image .dmg", url: latest("KilatCode_darwin_universal.dmg"), size: "7,3 MB" }],
    note: "Pertama kali: klik kanan aplikasi → Open.",
  },
  {
    id: "linux",
    name: "Linux",
    requirement: "x86_64 · WebKitGTK 4.1",
    files: [
      { label: "AppImage", url: latest("KilatCode_linux_amd64.AppImage"), size: "83 MB" },
      { label: ".deb", url: latest("KilatCode_linux_amd64.deb"), size: "4,5 MB" },
    ],
    note: "AppImage: chmod +x lalu jalankan. Fedora: pasang fuse-libs dulu; paket .rpm menyusul di v0.2.1.",
  },
];

/** Public address of the site (canonical links, social previews). */
export const SITE_URL = "https://kilat-code.vercel.app";

export const SITE_NAME = "Kilat Code";
export const DESCRIPTION =
  "Kilat Code menjalankan terminal Claude Code dan Codex asli secara paralel di Windows, macOS, dan Linux. Kena limit? Tunggu, atau lanjut dengan akun Claude atau ChatGPT lain tanpa kehilangan konteks.";
