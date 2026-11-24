// Everything that changes with a release lives here.

export const VERSION = "0.2.1";

export const REPO = "revincolabs/kilat-code";
export const REPO_URL = `https://github.com/${REPO}`;
export const RELEASES_URL = `${REPO_URL}/releases/latest`;

/** Public address of the site (canonical links, hreflang, sitemap, social previews). */
export const SITE_URL = "https://kilat.revincolabs.com";
export const SITE_NAME = "Kilat Code";

/** Shown on the docs pages and used as the docs' dateModified. Update when the docs change. */
export const DOCS_UPDATED = "2026-11-24";

/** The publisher's entity, defined on revincolabs.com. */
export const PUBLISHER = {
  id: "https://revincolabs.com/#organization",
  name: "Revinco Labs",
  url: "https://revincolabs.com/",
  logo: "https://revincolabs.com/brand/logo.png",
};

/** Release assets have version-less names (see .github/workflows/release.yml),
 *  so these links always point at the newest release. */
const latest = (file: string) => `${REPO_URL}/releases/latest/download/${file}`;

export type OsId = "windows" | "mac" | "linux";

export interface DownloadFile {
  label: string;
  url: string;
  /** size in MB */
  mb: number;
}

export interface Platform {
  id: OsId;
  name: string;
  requirement: string;
  files: DownloadFile[];
  note: { id: string; en: string };
}

export const PLATFORMS: Platform[] = [
  {
    id: "windows",
    name: "Windows",
    requirement: "Windows 10/11 · 64-bit",
    files: [{ label: "Installer .exe", url: latest("KilatCode_windows_x64-setup.exe"), mb: 2.9 }],
    note: {
      id: 'Klik "More info → Run anyway" kalau SmartScreen memperingatkan.',
      en: 'If SmartScreen warns you, click "More info → Run anyway".',
    },
  },
  {
    id: "mac",
    name: "macOS",
    requirement: "macOS 11+ · Apple Silicon & Intel",
    files: [{ label: "Disk image .dmg", url: latest("KilatCode_darwin_universal.dmg"), mb: 7.5 }],
    note: {
      id: "Pertama kali: System Settings → Privacy & Security → Open Anyway.",
      en: "First launch: System Settings → Privacy & Security → Open Anyway.",
    },
  },
  {
    id: "linux",
    name: "Linux",
    requirement: "x86_64 · WebKitGTK 4.1",
    files: [
      { label: "AppImage", url: latest("KilatCode_linux_amd64.AppImage"), mb: 83 },
      { label: ".deb", url: latest("KilatCode_linux_amd64.deb"), mb: 4.6 },
      { label: ".rpm (Fedora)", url: latest("KilatCode_linux_x86_64.rpm"), mb: 4.6 },
    ],
    note: {
      id: "AppImage: chmod +x lalu jalankan. Fedora: pakai .rpm (atau pasang fuse-libs untuk AppImage).",
      en: "AppImage: chmod +x, then run it. Fedora: use the .rpm (or install fuse-libs for the AppImage).",
    },
  },
];
