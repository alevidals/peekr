# ADR-0007: Theming and appearance

- **Status:** Accepted
- **Date:** 2026-11-19

## Context

A tray app is looked at for seconds at a time, next to the menu bar or a taskbar.
It has to read correctly on light and dark wallpapers, respect the system choice,
and still offer enough identity that it does not feel like a system dialog. The
UI is plain CSS, so a theme can be a set of variables rather than a component
library theme.

## Decision

1. **Three modes plus named themes.** `System`, `Light` and `Dark` are the base
   modes. Eight named themes (One Dark, Gruvbox Dark, Gruvbox Light, Nord,
   Dracula, Solarized Light, Tokyo Night, Catppuccin Mocha) pin their own scheme.
2. **Palette vs mode.** The config stores a `theme` (`system|light|dark`) and an
   `appearance.palette`. A named palette sets its own dark/light class, so the
   two values cannot disagree in the UI.
3. **CSS variables are the contract.** `applyAppearance` writes `--bg`,
   `--surface`, `--text`, `--accent`, status colours and derived inks on the
   document root, and toggles the `dark` class.
4. **Contrast is computed.** Status inks are mixed toward black or white until
   they clear 4.5:1 on their soft background. `readableOn` chooses the foreground
   for filled badges. A custom accent goes through the same path.
5. **Accent validation.** Only `#rrggbb` is accepted; an invalid value is dropped
   on save and falls back to the palette's accent.
6. **User controls.** Density (comfortable/compact), text size (small/medium/
   large) and an optional accent are stored in `config.json` and applied to both
   windows.
7. **No flash.** The frontend applies the stored appearance as soon as config
   loads, and falls back to the default appearance if the read fails.
8. **Motion respects the user.** `prefers-reduced-motion` disables the popover
   entrance, list stagger and other non-essential transitions.

## Consequences

**Positive**

- Every theme works in both windows without duplicated component styles.
- Badge and pill text remain readable on custom accents.
- Adding a named theme is one entry in `themes.ts` plus its palette.
- The design system is small enough to review as a few CSS files.

**Negative / accepted**

- Contrast is computed at runtime for derived inks, so the palette values must
  remain valid hex colours.
- A named theme's `dark` flag is authoritative; changing the OS does not override
  it.
- The CSS variable names are an implicit contract between `theme.ts` and
  `App.css`; renaming one requires both.
- There is no import/export of themes yet.

## Alternatives considered

- **Tailwind + shadcn tokens:** a larger stack for two windows and no component
  library needs.
- **Only system light/dark:** less identity and no way to match a user's editor
  theme.
- **Per-component theme objects:** duplicates the token mapping and makes
  contrast audits harder.
- **Trusting the palette's contrast:** some palettes (amber, custom accents)
  failed 4.5:1 on soft backgrounds; computing the ink fixes that.
