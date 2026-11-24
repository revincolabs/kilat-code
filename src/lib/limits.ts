import type { Account, LimitInfo } from "./api";

const MONTHS = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];

function hour12(h: number, ap?: string): number {
  if (!ap) return h;
  const pm = ap.toLowerCase() === "pm";
  if (h === 12) return pm ? 12 : 0;
  return pm ? h + 12 : h;
}

/** A clock time (optionally with a date) in the local time zone, moved to the future. */
function at(now: Date, h: number, m: number, month?: string, day?: number, year?: number): number | null {
  if (h > 23 || m > 59) return null;
  const d = new Date(now);
  d.setSeconds(0, 0);
  d.setHours(h, m);
  if (month && day) {
    const mi = MONTHS.indexOf(month.slice(0, 3).toLowerCase());
    if (mi < 0) return null;
    d.setMonth(mi, day);
    if (year) d.setFullYear(year);
    else if (d.getTime() < now.getTime() - 86_400_000) d.setFullYear(d.getFullYear() + 1);
    return d.getTime();
  }
  if (d.getTime() <= now.getTime()) d.setDate(d.getDate() + 1);
  return d.getTime();
}

/**
 * Reset time written in a limit message:
 * Claude "resets 3pm", "resets 3:30pm (Asia/Jakarta)", "resets Nov 25, 3pm";
 * Codex "try again at 3:04 PM", "try again at Dec 9th, 2026 3:00 PM",
 * "try again in 2 days 3 hours 5 minutes".
 */
export function parseReset(message: string, now = new Date()): number | null {
  const rel = /(?:try again|resets?|reset)\s+in\s+((?:\d+\s*(?:days?|d|hours?|hrs?|h|minutes?|mins?|m|seconds?|secs?|s)\b[\s,]*(?:and\s+)?)+)/i.exec(message);
  if (rel) {
    let ms = 0;
    for (const [, n, unit] of rel[1].matchAll(/(\d+)\s*([a-z]+)/gi)) {
      const u = unit.toLowerCase();
      const k = u.startsWith("d") ? 86_400_000 : u.startsWith("h") ? 3_600_000 : u.startsWith("m") ? 60_000 : 1000;
      ms += Number(n) * k;
    }
    if (ms > 0) return now.getTime() + ms;
  }
  const codex =
    /try again at\s+(?:([A-Za-z]{3,9})\s+(\d{1,2})(?:st|nd|rd|th)?,?\s*(\d{4})?\s+)?(\d{1,2}):(\d{2})\s*([AaPp][Mm])?/.exec(message);
  if (codex) {
    const [, mon, day, year, h, m, ap] = codex;
    return at(now, hour12(Number(h), ap), Number(m), mon, day ? Number(day) : undefined, year ? Number(year) : undefined);
  }
  const claude =
    /resets?\s+(?:at\s+)?(?:(?:on\s+)?([A-Za-z]{3,9})\s+(\d{1,2}),?\s+(?:at\s+)?)?(\d{1,2})(?::(\d{2}))?\s*([AaPp][Mm])/.exec(message);
  if (claude) {
    const [, mon, day, h, m, ap] = claude;
    return at(now, hour12(Number(h), ap), Number(m ?? 0), mon, day ? Number(day) : undefined);
  }
  return null;
}

/** Best guess of when a limited account can work again (null = unknown). */
export function resetFor(limit: LimitInfo, acc: Account | undefined, now = Date.now()): number | null {
  if (limit.resetsAt && limit.resetsAt > now) return limit.resetsAt;
  const fromText = parseReset(limit.message, new Date(now));
  if (fromText && fromText > now) return fromText;
  const full = (acc?.usage?.windows ?? [])
    .filter((w) => w.usedPercent >= 90 && w.resetsAt && w.resetsAt > now)
    .sort((a, b) => b.usedPercent - a.usedPercent)[0];
  return full?.resetsAt ?? null;
}

/** 8_100_000 -> "2 jam 15 mnt" */
export function fmtIn(ms: number): string {
  if (ms <= 60_000) return "< 1 menit";
  const m = Math.round(ms / 60_000);
  const d = Math.floor(m / 1440);
  const h = Math.floor((m % 1440) / 60);
  const mm = m % 60;
  if (d) return h ? `${d} hari ${h} jam` : `${d} hari`;
  if (h) return mm ? `${h} jam ${mm} mnt` : `${h} jam`;
  return `${mm} menit`;
}
