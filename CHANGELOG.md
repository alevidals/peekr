# Changelog

## Unreleased

### Added

- The popover is now a non-activating macOS panel: it opens over full-screen apps and on
  any desktop without switching Spaces, and keeps keyboard focus.
- `Esc` hides the popover. `/` still opens the search from anywhere.
- The popover closes when switching desktop, instead of following the user.

### Changed

- The tray icon is a template image, so macOS tints it from the real menu bar background.
  The red badge and the orange dev marker are drawn as layers on top and keep their colours.
- PR rows and repo groups are memoized, search terms are parsed once and filtering uses a
  deferred value; the list is no longer remounted every time the popover opens.
- Account forms are memoized and use stable callbacks, so typing only re-renders that account.
- The settings window and its CSS load in their own lazy chunk; the popover no longer ships
  that code.
- `objc 0.2` was replaced with the maintained `objc2`/`tauri-nspanel` crates.

### Fixed

- The tray icon could turn the opposite colour of the menu bar (black on a dark background
  or white on a light one).
- Polling interval input could not be cleared and accepted out-of-range values.
- Listeners and timers are cleaned up on unmount, and `Esc` works with an empty list.

### Tooling

- Added ESLint with the React hooks rules.
- New scripts: `typecheck`, `rust:check`, `rust:test`, `rust:lint` and `check`.
- `rustfmt` applied across the crate; `cargo clippy -D warnings` is clean.
