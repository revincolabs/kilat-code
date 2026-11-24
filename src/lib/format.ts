/** 38_400 -> "38k", 1_000_000 -> "1M" */
export function fmtTokens(n: number): string {
  if (n >= 1_000_000) return `${+(n / 1_000_000).toFixed(n % 1_000_000 ? 1 : 0)}M`;
  if (n >= 1000) return `${Math.round(n / 1000)}k`;
  return String(n);
}

import { t, locale } from "./i18n/index.svelte";

/** When a limit resets: "15:30", "besok 09:00", "Sen 09:00", "9 Des 18:49" (or the English equivalents). */
export function fmtReset(ms: number | null | undefined): string {
  if (!ms) return t().fmt.unknown;
  const d = new Date(ms);
  const now = new Date();
  const hm = d.toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" });
  const day = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = Math.round((day(d) - day(now)) / 86_400_000);
  if (diff <= 0) return hm;
  if (diff === 1) return `${t().fmt.tomorrow} ${hm}`;
  if (diff < 7) return `${t().fmt.days[d.getDay()]} ${hm}`;
  return `${d.toLocaleDateString(locale(), { day: "numeric", month: "short" })} ${hm}`;
}
