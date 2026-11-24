/**
 * Fuzzy file matching for @-mentions and quick open: every query character
 * must appear in order. Matches in the file name, at word starts and in a
 * row score higher.
 */
export function score(path: string, query: string): number {
  if (!query) return 1;
  const p = path.toLowerCase();
  const q = query.toLowerCase();
  const nameStart = p.lastIndexOf("/") + 1;
  let s = 0;
  let last = -1;
  for (const ch of q) {
    const i = p.indexOf(ch, last + 1);
    if (i < 0) return 0;
    s += 1;
    if (i === last + 1) s += 3; // consecutive
    if (i >= nameStart) s += 2; // in the file name
    if (i === 0 || "/._- ".includes(p[i - 1])) s += 4; // word start
    last = i;
  }
  if (p.slice(nameStart).startsWith(q)) s += 10;
  return s - path.length / 200;
}

export function fuzzy(paths: string[], query: string, limit = 50): string[] {
  if (!query) return paths.slice(0, limit);
  const scored: [number, string][] = [];
  for (const p of paths) {
    const s = score(p, query);
    if (s > 0) scored.push([s, p]);
  }
  scored.sort((a, b) => b[0] - a[0]);
  return scored.slice(0, limit).map((x) => x[1]);
}
