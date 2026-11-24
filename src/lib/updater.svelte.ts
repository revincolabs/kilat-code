import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { busyCount, fail, toast } from "./state.svelte";
import { t as tr } from "./i18n/index.svelte";

/** Auto-update from GitHub Releases (latest.json, signed). */
export const updater = $state({
  /** a newer version is available */
  version: null as string | null,
  notes: "",
  checking: false,
  /** download progress 0..1 while installing, null otherwise */
  progress: null as number | null,
});

let pending: Update | null = null;

/** `manual`: the user asked, so report "up to date" and errors too. */
export async function checkForUpdate(manual = false) {
  if (updater.checking || updater.progress != null) return;
  updater.checking = true;
  try {
    const u = await check();
    pending = u;
    updater.version = u?.version ?? null;
    updater.notes = u?.body ?? "";
    if (manual && !u) toast(tr().updater.upToDate, "ok");
    if (manual && u) toast(tr().updater.available(u.version), "info");
  } catch (e) {
    // Offline, or no release published yet: stay quiet unless asked.
    if (manual) fail(e);
  } finally {
    updater.checking = false;
  }
}

/** Download, install, restart. Running agents stop; their sessions resume after. */
export async function installUpdate() {
  if (!pending) return;
  const busy = busyCount();
  if (busy > 0 && !confirm(tr().updater.confirmBusy(busy))) {
    return;
  }
  let total = 0;
  let got = 0;
  updater.progress = 0;
  try {
    await pending.downloadAndInstall((ev) => {
      if (ev.event === "Started") total = ev.data.contentLength ?? 0;
      else if (ev.event === "Progress") {
        got += ev.data.chunkLength;
        updater.progress = total ? Math.min(1, got / total) : null;
      } else if (ev.event === "Finished") updater.progress = 1;
    });
    await relaunch();
  } catch (e) {
    updater.progress = null;
    fail(e);
  }
}

/** Check shortly after start, then every 6 hours. */
export function startUpdateChecks() {
  setTimeout(() => checkForUpdate(false), 8000);
  setInterval(() => checkForUpdate(false), 6 * 60 * 60 * 1000);
}
