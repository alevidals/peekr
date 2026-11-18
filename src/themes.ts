import type { NamedPalette, Palette, Theme } from "./types";

/** Theme ids in the order they are offered in the UI. */
export type ThemeChoice = Theme | NamedPalette;

export interface ThemePreview {
  /** Any CSS background value (the live theme uses a two-tone gradient). */
  background: string;
  foreground: string;
  accent: string;
}

export interface ThemeOption {
  value: ThemeChoice;
  label: string;
  description: string;
  /** Colour scheme the option pins (`system` follows the OS). */
  theme: Theme;
  /** Palette stored in the config for this option. */
  palette: Palette;
  preview: ThemePreview;
}

/** Full palette for one of the named themes, taken from sftply's `themes.ts`. */
export interface NamedTheme {
  id: NamedPalette;
  label: string;
  description: string;
  dark: boolean;
  background: string;
  foreground: string;
  muted: string;
  card: string;
  secondary: string;
  border: string;
  input: string;
  primary: string;
  primaryForeground: string;
  destructive: string;
  success: string;
  warning: string;
}

export const NAMED_THEMES: readonly NamedTheme[] = [
  {
    id: "oneDark",
    label: "One Dark",
    description: "El clásico de Atom",
    dark: true,
    background: "#282c34",
    foreground: "#abb2bf",
    muted: "#9da2ad",
    card: "#2f343d",
    secondary: "#333842",
    border: "#3b4048",
    input: "#3b4048",
    primary: "#61afef",
    primaryForeground: "#1e2127",
    destructive: "#e5848c",
    success: "#98c379",
    warning: "#e5c07b",
  },
  {
    id: "gruvboxDark",
    label: "Gruvbox Dark",
    description: "Cálido y retro",
    dark: true,
    background: "#282828",
    foreground: "#ebdbb2",
    muted: "#afa18e",
    card: "#32302f",
    secondary: "#3c3836",
    border: "#504945",
    input: "#504945",
    primary: "#fabd2f",
    primaryForeground: "#282828",
    destructive: "#fc7d6f",
    success: "#b8bb26",
    warning: "#fabd2f",
  },
  {
    id: "gruvboxLight",
    label: "Gruvbox Light",
    description: "Papel cálido",
    dark: false,
    background: "#fbf1c7",
    foreground: "#3c3836",
    muted: "#6a5f56",
    card: "#f9f5d7",
    secondary: "#ebdbb2",
    border: "#d5c4a1",
    input: "#d5c4a1",
    primary: "#966111",
    primaryForeground: "#fbf1c7",
    destructive: "#9d0006",
    success: "#68640c",
    warning: "#86570f",
  },
  {
    id: "nord",
    label: "Nord",
    description: "Azules árticos",
    dark: true,
    background: "#2e3440",
    foreground: "#d8dee9",
    muted: "#a6aebf",
    card: "#333b4a",
    secondary: "#3b4252",
    border: "#434c5e",
    input: "#434c5e",
    primary: "#88c0d0",
    primaryForeground: "#2e3440",
    destructive: "#d89fa4",
    success: "#a3be8c",
    warning: "#ebcb8b",
  },
  {
    id: "dracula",
    label: "Dracula",
    description: "Púrpura intenso",
    dark: true,
    background: "#282a36",
    foreground: "#f8f8f2",
    muted: "#979fc0",
    card: "#2f3140",
    secondary: "#343746",
    border: "#44475a",
    input: "#44475a",
    primary: "#bd93f9",
    primaryForeground: "#21222c",
    destructive: "#ff7878",
    success: "#50fa7b",
    warning: "#f1fa8c",
  },
  {
    id: "solarizedLight",
    label: "Solarized Light",
    description: "Luz de precisión",
    dark: false,
    background: "#fdf6e3",
    foreground: "#586e75",
    muted: "#5d6a6b",
    card: "#fffcf5",
    secondary: "#eee8d5",
    border: "#ded8c4",
    input: "#ded8c4",
    primary: "#2076b3",
    primaryForeground: "#fdf6e3",
    destructive: "#c62d2a",
    success: "#617000",
    warning: "#846400",
  },
  {
    id: "tokyoNight",
    label: "Tokyo Night",
    description: "Ciudad de neón",
    dark: true,
    background: "#24283b",
    foreground: "#c0caf5",
    muted: "#939dc0",
    card: "#292e42",
    secondary: "#2f334d",
    border: "#3b4261",
    input: "#3b4261",
    primary: "#7aa2f7",
    primaryForeground: "#1a1b26",
    destructive: "#f7768e",
    success: "#9ece6a",
    warning: "#e0af68",
  },
  {
    id: "catppuccinMocha",
    label: "Catppuccin Mocha",
    description: "Pasteles suaves",
    dark: true,
    background: "#1e1e2e",
    foreground: "#cdd6f4",
    muted: "#a6adc8",
    card: "#242438",
    secondary: "#313244",
    border: "#313244",
    input: "#45475a",
    primary: "#cba6f7",
    primaryForeground: "#11111b",
    destructive: "#f38ba8",
    success: "#a6e3a1",
    warning: "#f9e2af",
  },
];

const DEFAULT_OPTIONS: readonly ThemeOption[] = [
  {
    value: "system",
    label: "Sistema",
    description: "Sigue al sistema",
    theme: "system",
    palette: "indigo",
    // Two-tone strip: the icon and text take their colour from the OS.
    preview: {
      background: "linear-gradient(90deg, #17181c 50%, #ffffff 50%)",
      foreground: "#8b92a0",
      accent: "#5b5bf0",
    },
  },
  {
    value: "dark",
    label: "Oscuro",
    description: "Peekr oscuro",
    theme: "dark",
    palette: "indigo",
    preview: { background: "#17181c", foreground: "#eceef2", accent: "#8083ff" },
  },
  {
    value: "light",
    label: "Claro",
    description: "Peekr claro",
    theme: "light",
    palette: "indigo",
    preview: { background: "#ffffff", foreground: "#14161a", accent: "#5b5bf0" },
  },
];

export const THEME_OPTIONS: readonly ThemeOption[] = [
  ...DEFAULT_OPTIONS,
  ...NAMED_THEMES.map(
    (theme): ThemeOption => ({
      value: theme.id,
      label: theme.label,
      description: theme.description,
      theme: theme.dark ? "dark" : "light",
      palette: theme.id,
      preview: {
        background: theme.background,
        foreground: theme.foreground,
        accent: theme.primary,
      },
    })
  ),
];
