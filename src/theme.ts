import { DEFAULT_APPEARANCE, type Appearance, type Palette, type Theme } from "./types";

interface Colors {
  accent: string;
  /** Variables beyond the accent, for palettes that change more than the highlight. */
  extra?: Record<string, string>;
}

interface PaletteDef {
  label: string;
  light: Colors;
  dark: Colors;
}

/** Indigo is the stylesheet's own look, so it overrides nothing. */
export const PALETTES: Record<Palette, PaletteDef> = {
  indigo: { label: "Índigo", light: { accent: "#5b5bf0" }, dark: { accent: "#8083ff" } },
  graphite: { label: "Grafito", light: { accent: "#3d4452" }, dark: { accent: "#b4bac6" } },
  green: { label: "Verde", light: { accent: "#12875a" }, dark: { accent: "#3ecf8e" } },
  amber: { label: "Ámbar", light: { accent: "#c26a00" }, dark: { accent: "#f5a524" } },
  contrast: {
    label: "Contraste",
    light: {
      accent: "#0b3fd1",
      extra: {
        "--text": "#000000",
        "--text-secondary": "#2b303a",
        "--text-faint": "#4a505c",
        "--border": "rgba(0, 0, 0, 0.35)",
        "--border-strong": "rgba(0, 0, 0, 0.6)",
      },
    },
    dark: {
      accent: "#7fb0ff",
      extra: {
        "--text": "#ffffff",
        "--text-secondary": "#e2e5ea",
        "--text-faint": "#bcc2cc",
        "--border": "rgba(255, 255, 255, 0.4)",
        "--border-strong": "rgba(255, 255, 255, 0.7)",
      },
    },
  },
};

const TEXT_SCALE = { small: "0.92", medium: "1", large: "1.1" } as const;

/** Variables set inline by [`applyAppearance`], so the next call can clear them. */
let applied: string[] = [];

const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

function isDark(theme: Theme): boolean {
  return theme === "dark" || (theme === "system" && darkQuery.matches);
}

function hexToRgb(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function rgbToHex([r, g, b]: [number, number, number]): string {
  return `#${[r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")}`;
}

function mix(
  from: [number, number, number],
  to: [number, number, number],
  t: number
): [number, number, number] {
  return from.map((v, i) => Math.round(v + (to[i] - v) * t)) as [number, number, number];
}

function luminance([r, g, b]: [number, number, number]): number {
  const channel = (v: number) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrast(a: [number, number, number], b: [number, number, number]): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/** Black or white text, whichever reads better on `hex`. */
function readableOn(hex: string): string {
  const [r, g, b] = hexToRgb(hex);
  return 0.299 * r + 0.587 * g + 0.114 * b > 150 ? "#0e0f14" : "#ffffff";
}

/**
 * Accent colour for text drawn on `--accent-soft`, mixed toward the ink until it clears
 * 4.5:1. The fill keeps the palette accent; only text gets the darker/lighter variant.
 */
function accentInk(accent: string, dark: boolean): string {
  const base = hexToRgb(accent);
  const soft = mix(base, hexToRgb(dark ? "#17181c" : "#ffffff"), dark ? 0.18 : 0.12);
  const ink: [number, number, number] = dark ? [255, 255, 255] : [0, 0, 0];
  for (let step = 0; step <= 20; step++) {
    const candidate = mix(base, ink, step / 20);
    if (contrast(candidate, soft) >= 4.5) return rgbToHex(candidate);
  }
  return rgbToHex(ink);
}

let current: { theme: Theme; appearance: Appearance } = { theme: "system", appearance: DEFAULT_APPEARANCE };

/** The accent a palette resolves to; the settings preview uses it for its swatches. */
export function paletteAccent(palette: Palette, theme: Theme = current.theme): string {
  return PALETTES[palette][isDark(theme) ? "dark" : "light"].accent;
}

/**
 * Applies light/dark/system plus the palette, accent, density and text size to the page.
 * "system" leaves `data-theme` off so the `prefers-color-scheme` CSS applies.
 */
export function applyAppearance(theme: Theme, appearance: Appearance) {
  current = { theme, appearance };
  const root = document.documentElement;
  if (theme === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", theme);

  root.dataset.density = appearance.density;
  root.style.setProperty("--text-scale", TEXT_SCALE[appearance.text_size]);

  for (const name of applied) root.style.removeProperty(name);
  applied = [];
  const set = (name: string, value: string) => {
    root.style.setProperty(name, value);
    applied.push(name);
  };

  const colors = PALETTES[appearance.palette][isDark(theme) ? "dark" : "light"];
  const accent = appearance.accent ?? colors.accent;
  // The default palette with no custom accent is the stylesheet itself.
  if (appearance.palette !== "indigo" || appearance.accent) {
    const dark = isDark(theme);
    const [r, g, b] = hexToRgb(accent);
    set("--accent", accent);
    set("--accent-soft", `rgba(${r}, ${g}, ${b}, ${dark ? 0.18 : 0.12})`);
    set("--accent-ink", accentInk(accent, dark));
    set("--accent-text", readableOn(accent));
  }
  for (const [name, value] of Object.entries(colors.extra ?? {})) set(name, value);
}

// With theme "system" the effective colours change with the OS, so palettes must follow.
darkQuery.addEventListener("change", () => applyAppearance(current.theme, current.appearance));
