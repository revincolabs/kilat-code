/** macOS uses Cmd where Windows / Linux use Ctrl. */
export const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

/** The platform's main shortcut modifier is held (Cmd on macOS, Ctrl elsewhere). */
export function modKey(e: KeyboardEvent | MouseEvent): boolean {
  return isMac ? e.metaKey : e.ctrlKey;
}

/** "Ctrl+V" / "⌘V" for hints in the UI. */
export function keyLabel(key: string): string {
  return isMac ? `⌘${key}` : `Ctrl+${key}`;
}
