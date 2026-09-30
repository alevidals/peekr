import type { Theme } from "./types";

/** "system" leaves the attribute off so the `prefers-color-scheme` CSS applies. */
export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  if (theme === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", theme);
}
