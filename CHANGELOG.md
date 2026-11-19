# Changelog

## Unreleased

## 0.1.0 — 2026-11-19

### Added

- Settings → General → **Check for updates** compares the running version with the latest
  published GitHub release, links to its release page, and never downloads or installs
  anything automatically.
- Launching peekr while it is already running now focuses the existing popover instead of
  starting a second tray process.
- The app writes a rotating log file (2 MiB) in the platform log directory, and Settings →
  General → **Open logs** reveals it.
- Removing an account now asks for confirmation. The dialog explains that saving removes
  the account and deletes its keychain token; a new, unsaved account is discarded instead.
- The popover is now a non-activating macOS panel: it opens over full-screen apps and on
  any desktop without switching Spaces, and keeps keyboard focus.
- `Esc` hides the popover. `/` still opens the search from anywhere.
- The popover closes when switching desktop, instead of following the user.
- Theme picker with System, Light and Dark plus the themes shared with sftply: One Dark,
  Gruvbox Dark, Gruvbox Light, Nord, Dracula, Solarized Light, Tokyo Night and Catppuccin
  Mocha. Each one carries its full palette and pins its own light/dark scheme.
- The first-run empty state now has an “Open settings” action, so connecting an account does
  not depend on finding the gear in the header.

### Changed

- The list and notifications use the same vocabulary for both providers: Bitbucket repos
  appear as `workspace/repo`, GitHub shows the profile name when set, CI is "checks" on
  GitHub and "pipelines" on Bitbucket, and Bitbucket's inferred ready state says "Looks
  ready to merge".
- When GitHub reports a PR as mergeable while non-required checks are failing or still
  running (`UNSTABLE`), the merge pill names that caveat instead of hiding the ready state.
- GitHub review requests come from a single paginated GraphQL search instead of four REST
  calls per PR, so a poll makes far fewer requests and no longer stops at 100 PRs.
- Bitbucket fans out with at most 8 requests at once, and a run of failed polls backs off
  instead of retrying at the normal interval.
- The webview now has a Content-Security-Policy and no longer exposes the global Tauri
  object the frontend does not use.
- The drafts filter uses an eye/eye-off icon and the draft row keeps full contrast; the
  “Draft” badge is the only signal. Truncated titles, branches and repo names show the full
  text on hover, and the header/refresh hit targets are 32px.
- All UI copy is in English now: settings sections, account form, provider hints, theme
  names/descriptions and the window title.
- Settings is now a larger window (820 × 660) with a header, a vertical sidebar for
  Cuentas, General and Apariencia, a section header and a full-width footer. The theme
  picker sits in a three-column grid; density and text size follow below.
- The tray icon is a template image, so macOS tints it from the real menu bar background.
  The red badge and the orange dev marker are drawn as layers on top and keep their colours.
- PR rows and repo groups are memoized, search terms are parsed once and filtering uses a
  deferred value; the list is no longer remounted every time the popover opens.
- Account forms are memoized and use stable callbacks, so typing only re-renders that account.
- The settings window and its CSS load in their own lazy chunk; the popover no longer ships
  that code.
- `objc 0.2` was replaced with the maintained `objc2`/`tauri-nspanel` crates.

### Fixed

- My PR notifications are announced once per PR update. A provider that briefly reports a
  different merge state between polls (for example Bitbucket's best-effort mergeability
  checks) no longer repeats the same "ready to merge" or "pipeline failed" notification.
- Network requests now really retry after a dropped connection. The retry path only covered
  response decoding, so a `connection closed before message completed` error from the
  transport bypassed it and surfaced on every poll.
- Two accounts for the same workspace are rejected, and a PR seen through more than one
  account is listed, counted and notified only once.
- Network errors carry the underlying cause (DNS, TLS, connection reset) and a transient
  request is retried once. Poll errors from both tabs are emitted together, without
  duplicate lines.
- Saving settings could delete the stored token of a removed account before the new config
  reached the disk, and a keychain error during the v1 migration started the app empty. The
  config is now written atomically, unreadable files are backed up, and tokens are only
  removed once the config no longer references them.
- An account with no token or no repository was skipped silently, so the popover looked
  empty. It is now reported in the error banner.
- A PR that GitHub's search dropped for a single poll was announced again when it came back.
  Each PR is now announced once.
- Validating a token in settings had no timeout and could spin forever.
- GitHub PRs count review comments again: the bubble uses `totalCommentsCount`, which adds
  issue and review comments as GitHub's own list does.
- Polling interval input could not be cleared and accepted out-of-range values.
- Listeners and timers are cleaned up on unmount, and `Esc` works with an empty list.
- The PR row shows the update time instead of the creation time.

### Accessibility

- Focus rings are outlines again on inputs and controls; the field rules were overriding the
  global `:focus-visible`.
- Small text on badges, pills and messages uses dedicated ink tokens that clear 4.5:1,
  including the amber palette and custom accents.
- `prefers-reduced-motion: reduce` now covers every animation, not just the list.
- Form labels, radio groups, tabs and icon-only buttons expose names and roles; tabs support
  arrow keys; errors and validation messages announce as alerts/status.
- The PR row uses valid phrasing markup inside the button, and decorative icons are hidden
  from assistive technology.
- Tracked-caps labels, middle-dot meta strings and the arrow glued to the token link are gone.

### Documentation

- Added the project documentation set: a rewritten README, a user guide, architecture,
  dependency evaluation, release plan, roadmap, progress log and seven ADRs under `docs/`.
- Added contribution, security and code-of-conduct documents, a manual QA checklist, and
  GitHub issue and pull-request templates.

### Tooling

- Added GitHub Actions for CI (frontend, Rust gates and a three-platform
  `cargo check` matrix), tagged releases (portable archives, installers,
  `SHA256SUMS` and a draft release) and weekly dependency audits.
- Added `bun run version:check` to keep `package.json`, `Cargo.toml`,
  `tauri.conf.json` and the release tag in sync.
- Added ESLint with the React hooks rules.
- Added Vitest + Testing Library with tests for search, drafts, themes and the confirmation
  dialog; `bun run check` now runs them.
- New scripts: `typecheck`, `test`, `rust:check`, `rust:test`, `rust:lint` and `check`.
- `rustfmt` applied across the crate; `cargo clippy -D warnings` is clean.
