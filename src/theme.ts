import { NAMED_THEMES, type NamedTheme } from "./themes";
import {
  DEFAULT_APPEARANCE,
  type Appearance,
  type NamedPalette,
  type Palette,
  type Theme,
} from "./types";

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

function softColor(hex: string, alpha: number): string {
  const [r, g, b] = hexToRgb(hex);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/**
 * Text colour for `color` drawn on a soft tint of itself over `background`, mixed toward
 * black (light themes) or white (dark themes) until it clears 4.5:1.
 */
function inkOnSoft(
  color: string,
  background: string,
  alpha: number,
  dark: boolean
): string {
  const base = hexToRgb(color);
  const soft = mix(base, hexToRgb(background), alpha);
  const ink: [number, number, number] = dark ? [255, 255, 255] : [0, 0, 0];
  for (let step = 0; step <= 20; step++) {
    const candidate = mix(base, ink, step / 20);
    if (contrast(candidate, soft) >= 4.5) return rgbToHex(candidate);
  }
  return rgbToHex(ink);
}

/** Accent used for text on `--accent-soft`. The fill keeps the palette accent. */
function accentInk(accent: string, background: string, dark: boolean): string {
  return inkOnSoft(accent, background, dark ? 0.18 : 0.12, dark);
}

/** Maps a named theme's tokens onto peekr's variables. */
function namedExtras(theme: NamedTheme): Record<string, string> {
  return {
    "--bg": theme.background,
    "--surface": theme.card,
    "--surface-hover": theme.secondary,
    "--text": theme.foreground,
    "--text-secondary": theme.muted,
    "--text-faint": theme.muted,
    "--border": theme.border,
    "--border-strong": theme.input,
    "--success": theme.success,
    "--success-soft": softColor(theme.success, 0.14),
    "--success-ink": inkOnSoft(theme.success, theme.background, 0.14, theme.dark),
    "--danger": theme.destructive,
    "--danger-soft": softColor(theme.destructive, 0.14),
    "--danger-ink": inkOnSoft(theme.destructive, theme.background, 0.14, theme.dark),
    "--warning": theme.warning,
    "--warning-soft": softColor(theme.warning, 0.16),
    "--warning-ink": inkOnSoft(theme.warning, theme.background, 0.16, theme.dark),
    "--on-success": readableOn(theme.success),
    "--on-danger": readableOn(theme.destructive),
  };
}

const NAMED_PALETTES = Object.fromEntries(
  NAMED_THEMES.map((theme) => [
    theme.id,
    {
      label: theme.label,
      light: { accent: theme.primary, extra: namedExtras(theme) },
      dark: { accent: theme.primary, extra: namedExtras(theme) },
    } satisfies PaletteDef,
  ])
) as Record<NamedPalette, PaletteDef>;

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
  ...NAMED_PALETTES,
};

const TEXT_SCALE = { small: "0.92", medium: "1", large: "1.1" } as const;

/** Variables set inline by [`applyAppearance`], so the next call can clear them. */
let applied: string[] = [];

const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

function isDark(theme: Theme): boolean {
  return theme === "dark" || (theme === "system" && darkQuery.matches);
}

let current: { theme: Theme; appearance: Appearance } = { theme: "system", appearance: DEFAULT_APPEARANCE };

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

  const dark = isDark(theme);
  const colors = PALETTES[appearance.palette][dark ? "dark" : "light"];
  const accent = appearance.accent ?? colors.accent;
  const background = colors.extra?.["--bg"] ?? (dark ? "#17181c" : "#ffffff");
  // The default palette with no custom accent is the stylesheet itself.
  if (appearance.palette !== "indigo" || appearance.accent) {
    set("--accent", accent);
    set("--accent-soft", softColor(accent, dark ? 0.18 : 0.12));
    set("--accent-ink", accentInk(accent, background, dark));
    set("--accent-text", readableOn(accent));
  }
  for (const [name, value] of Object.entries(colors.extra ?? {})) set(name, value);
}

// With theme "system" the effective colours change with the OS, so palettes must follow.
darkQuery.addEventListener("change", () => applyAppearance(current.theme, current.appearance));
