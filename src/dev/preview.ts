// Dev-only design preview entry: mocked backend first, then the real app.
// Static imports only — they run in order, so the mock exists before init().
import { scenario, q } from "./mock";
import "../main";
import { app, openTerminal } from "../lib/state.svelte";

// Panels and dialogs once the app is up.
const iv = setInterval(() => {
  if (!app.ready) return;
  clearInterval(iv);
  if (scenario === "work") app.rightPanel = "changes";
  if (scenario === "editor") app.rightPanel = "editor";
  if (scenario === "accounts") app.accountsOpen = true;
  if (q.get("w")) openTerminal({ projectId: "p1", cwd: "G:/kode/toko-online" });
}, 20);
