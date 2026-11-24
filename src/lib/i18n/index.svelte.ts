// UI language. "auto" follows the system language: Indonesian when the
// system is Indonesian, English otherwise. The choice is stored in the app
// settings (`language`), and the backend is told so its own text matches.
import { invoke } from "@tauri-apps/api/core";
import { id, type Dict } from "./id";
import { en } from "./en";

export type Lang = "id" | "en";
export type LangPref = "auto" | Lang;

const dicts: Record<Lang, Dict> = { id, en };

function systemLang(): Lang {
  const langs = navigator.languages?.length ? navigator.languages : [navigator.language];
  return langs.some((l) => l?.toLowerCase().startsWith("id")) ? "id" : "en";
}

export const i18n = $state<{ lang: Lang }>({ lang: systemLang() });

/** Apply a language preference (from the settings) to the UI and the backend. */
export function applyLang(pref: string | undefined) {
  const lang: Lang = pref === "id" || pref === "en" ? pref : systemLang();
  i18n.lang = lang;
  document.documentElement.lang = lang;
  invoke("set_ui_lang", { lang }).catch(() => {});
}

/** The text of the current language. Reading it inside markup keeps it reactive. */
export function t(): Dict {
  return dicts[i18n.lang];
}

/** BCP-47 locale for dates and times. */
export function locale(): string {
  return i18n.lang === "id" ? "id-ID" : "en-US";
}
