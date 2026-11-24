import { mount } from "svelte";
import "@xterm/xterm/css/xterm.css";
import "./app.css";
import App from "./App.svelte";

mount(App, { target: document.getElementById("app")! });

// Dev-only handle for automated UI checks.
if (import.meta.env.DEV) {
  import("./lib/state.svelte").then((m) => ((window as any).__kilat = m));
}
