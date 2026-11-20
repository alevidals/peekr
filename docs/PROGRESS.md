# peekr — progress log & continuation notes

> **Historical record.** This log covers development through 0.1.0. See
> [ROADMAP.md](ROADMAP.md) for current plans.

**Purpose:** Working log for the 0.1 line. It records what was built, verified
and left open.

- **Last updated:** 2026-11-19
- **Status:** 0.1.0 released. CI, release and security workflows are validated;
  cross-platform visual QA remains.
- **Primary platform:** macOS, with the Windows/Linux code paths kept buildable.

---

## Provisional decisions in use

| Topic              | Default in use                                      | Notes                                                            |
| ------------------ | --------------------------------------------------- | ---------------------------------------------------------------- |
| Product name       | `peekr`                                             | Identifier `com.alevidals.peekr`                                 |
| Desktop stack      | Tauri 2 + React 19 + TypeScript + Vite              | One Rust crate, one Vite project, two windows                    |
| Styling            | Plain CSS with CSS variables                        | No Tailwind or component library                                 |
| Networking         | `reqwest` + rustls, one shared client               | 30 s timeout, one retry on transport/502/503/504                 |
| Credentials        | `keyring` 4.2.0, service `peekr`                     | One entry per account; no plaintext fallback                     |
| Config             | Versioned JSON, atomic write                         | v1 → v2 migration; corrupt files backed up                       |
| IPC types          | Handwritten `src/types.ts`                          | Codegen deferred; see ADR-0005                                   |
| Lint/format        | ESLint + rustfmt + clippy                           | `bun run check` runs the whole gate                              |
| Platform priority  | macOS-first native panel and badge                  | Windows/Linux use cross-platform window and icon paths           |
| Git                | Conventional commits, focused changes               | `main` is the stable line                                        |

---

## Current status

### Completed

- [x] Tray app with a live review count and tooltip
- [x] Popover with review and My PRs tabs
- [x] Bitbucket Cloud provider (REST)
- [x] GitHub provider (GraphQL)
- [x] Multiple accounts, duplicate detection and cross-account deduplication
- [x] Keychain token storage and config migration
- [x] Notifications for new requests and readiness/failure transitions
- [x] Non-activating macOS panel, Space handling and click-outside close
- [x] Template tray icon with layer badge; runtime badge on Windows/Linux
- [x] Themes, named palettes, accent, density and text size
- [x] CSP, no global Tauri object, minimal capabilities
- [x] ESLint, typecheck, Rust tests and the production build gate
- [x] CI, release and security workflows plus the version-consistency script
- [x] Single-instance guard and a rotating application log with an Open logs action
- [x] Manual Check for updates against the latest GitHub release
- [x] About section with app identity, paths, keychain note and updates
- [x] Vitest + Testing Library for search, drafts, themes and the confirmation dialog
- [x] CI on Ubuntu/macOS/Windows, security audits and a published `v0.1.0` release
- [x] Architecture, user, dependency, release, roadmap and ADR docs

### In progress

- [ ] Cross-platform visual QA on real Windows and Linux machines
- [ ] Codegen evaluation for the IPC types

### Known issues / blockers

- `src/types.ts` is handwritten. A Rust DTO change can compile without the
  frontend noticing until runtime. The test gap is documented, not hidden.
- The frontend suite covers pure logic and the confirmation dialog, not the full
  popover event flow.
- The macOS panel path is the most exercised. Windows/Linux tray placement,
  badge rendering and notification click behaviour need a real-machine pass.
- Bitbucket's mergeability endpoint is best effort. When it is unavailable, the
  fallback inference cannot detect conflicts.
- No signed artifacts and no updater yet. The release workflow produces unsigned
  bundles and a draft release; distribution policy waits for signing/notarization.
- GitHub search is eventually consistent. The notifier keeps announced keys so a
  flicker cannot cause a second notification, but the list itself can briefly
  miss a PR.

---

## Session log

### 2026-11-18 — Session 1: tray client foundation

**Done**

- Replaced the template UI with a tray-first app: popover window, settings
  window and a Quit menu item.
- Added the Bitbucket provider over REST: review requests, approved filtering,
  comments, approvals and pipeline status.
- Added the tray badge, count tooltip and dev marker.
- Added keychain token storage and the first JSON config.
- Added native notifications for new review requests.
- Added polling with a manual refresh trigger.

**Verified**

- `bun run typecheck`, `bun run build`.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.

**Next**

- Multi-account support, config migration and the GitHub provider.

### 2026-11-18 — Session 2: accounts and configuration

**Done**

- Replaced the single workspace/token pair with a list of accounts. Each account
  has an id, provider, workspace and repositories.
- Added config v1 → v2 migration. The legacy token is read from the file or the
  keychain, moved to the new account, and the legacy entry is deleted.
- Made config writes atomic: temp file, `sync_all`, rename, mode `0600` on Unix.
- Added corrupt-file recovery: unreadable configs are renamed to
  `config.json.bak`, and the app starts with defaults.
- Added token write ordering: tokens first, then config; orphan tokens are
  cleaned up if the config write fails, and removed accounts lose their tokens
  only after the new config is on disk.
- Added duplicate-workspace rejection and account validation.
- Added configurable polling with a 30 s floor and 1 h ceiling.

**Verified**

- Rust config tests: clamping, repository cleanup, v2 roundtrip, theme defaults,
  appearance validation, v1 defaults.
- `cargo test`, `cargo clippy`, `bun run typecheck`.

**Next**

- GitHub and the My PRs tab.

### 2026-11-18 — Session 3: GitHub and own PRs

**Done**

- Added the GitHub GraphQL provider. One paginated search per side:
  `review-requested:@me` and `author:@me`, both including team review requests.
- Mapped the search node to `PullRequest` without per-PR requests: branches,
  comments, reviews, check rollup and `mergeStateStatus`.
- Added the My PRs tab and provider-aware merge labels.
- Added GitHub merge state mapping: ready, waiting, blocked, conflicts, behind,
  draft, unknown.
- Added the `UNSTABLE` caveat when GitHub reports mergeable while non-required
  checks are failing or running.
- Added Bitbucket mergeability checks with a client-side fallback and explicit
  "Looks ready" wording.
- Added readiness/failure notifications for My PRs.
- Added cross-account deduplication by provider/repo/number.

**Verified**

- Provider mapping tests for both services.
- Notifier transition tests.
- `cargo test`, `cargo clippy`, `bun run typecheck`, `bun run build`.

**Next**

- Native popover polish, themes and performance.

### 2026-11-19 — Session 4: native panel and attention to detail

**Done**

- Converted the macOS popover into a non-activating `NSPanel`: it opens over
  full-screen apps, on any Space, and keeps keyboard focus.
- Added a Space observer so switching desktop hides the panel instead of dragging
  it along.
- Added the hide generation counter to resolve tray-click vs blur races.
- Added `Esc` to close and `/` to search from anywhere.
- Added eight named themes, plus a custom accent.
- Added density and text-size settings.
- Added contrast-computed status inks and `prefers-reduced-motion` coverage.
- Added memoization for rows and repo groups, deferred search parsing and lazy
  settings chunks.
- Moved all UI copy to English and widened Settings to 820 × 660 with a sidebar.

**Verified**

- `bun run typecheck`, `bun run lint`, `bun run build`.
- `cargo test` for popover hide logic.
- Manual macOS pass for Spaces, full-screen, focus and theme switching.

**Next**

- Accessibility and security sweep, then documentation.

### 2026-11-19 — Session 5: accessibility, security and reliability

**Done**

- CSP added; `withGlobalTauri` disabled.
- Capabilities reduced to `core:default`, window dragging, opener, positioner
  and autostart.
- Focus rings restored as outlines; field rules no longer override the global
  `:focus-visible`.
- Badge and pill inks meet 4.5:1, including amber and custom accents.
- Form labels, radio groups, tabs and icon-only buttons expose names and roles.
- Errors and validation messages announce as alerts/status.
- Valid token checks got a timeout; settings validation can no longer spin.
- Network errors carry DNS/TLS/reset causes and retry once.
- Polling backs off after failed rounds; Bitbucket concurrency is a real cap.
- Failed repositories keep their previous PRs instead of looking closed.
- PRs seen through two accounts are listed, counted and notified once.
- Search flicker no longer re-announces a PR.
- Token removal happens only after the config no longer references the account.

**Verified**

- `bun run check`.
- Rust tests for http retry/body trimming, poller failures/diff/dedupe, notifier
  transitions and provider mapping.

**Next**

- Release infrastructure and frontend tests.

### 2026-11-19 — Session 6: tooling and documentation

**Done**

- Added ESLint with the React hooks rules.
- Added `typecheck`, `rust:check`, `rust:test`, `rust:lint` and `check`.
- Applied rustfmt across the crate; clippy is clean with `-D warnings`.
- Added this documentation set: README, user guide, architecture, dependencies,
  release plan, roadmap, progress log and ADRs.
- Added contribution, security and code-of-conduct documents.

**Verified**

- `bun run check` (typecheck, ESLint, rustfmt, clippy, Rust tests, build).
- Documentation reviewed for links and stale claims.

**Next**

- Add CI, release and security workflows.
- Add Vitest and component tests for the popover logic.

### 2026-11-19 — Session 7: release infrastructure

**Done**

- Added `.github/workflows/ci.yml`: frontend (version check, typecheck, ESLint,
  build), Rust gates on Linux (format, extra-binary check, clippy, tests) and a
  `cargo check --all-targets` matrix on Ubuntu, macOS and Windows.
- Added `.github/workflows/release.yml`: universal macOS, Linux x64/ARM64 and
  Windows x64 builds on `v*` tags, portable archives plus installers,
  `SHA256SUMS` and a draft GitHub release. Manual runs validate builds without
  publishing.
- Added `.github/workflows/security.yml`: weekly `bun audit --production` and
  `cargo audit`, also triggered by lockfile and audit-configuration changes.
- Added `scripts/check-versions.ts` and `bun run version:check`; the script
  compares `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`
  and the tag.
- Added `src-tauri/.cargo/audit.toml` for accepted advisories.
- Updated the release, roadmap, dependency, security and progress documents to
  match the workflows that now exist.

**Verified**

- YAML parsed for all issue templates and workflows.
- `bun run version:check`, `bun run lint`, `bun run typecheck`.
- `bun run check` (39 Rust tests, clippy, ESLint, production build).

**Next**

- Push `main` and confirm the first CI run.
- Tag the first release and review the draft artifacts.

### 2026-11-19 — Session 8: account removal and notification dedupe

**Done**

- Added a confirmation dialog before removing an account. Saved accounts explain
  that saving deletes the account and its keychain token; unsaved accounts are
  discarded. The dialog traps focus, closes on `Escape` or a backdrop click, and
  returns focus to the account section.
- Added a notification ledger for My PRs keyed by PR, event and `updated_on`.
  A provider that briefly reports a different merge state between polls no longer
  repeats the same ready/failure notification.
- Documented the GitHub review rule: `review-requested:@me` is a pending request,
  not an approval filter, so submitting either approval or request-changes clears
  the request.

**Verified**

- `bun run check` (40 Rust tests, clippy, ESLint, production build).

**Next**

- Push `main` and confirm the first CI run.

### 2026-11-19 — Session 9: v1 hardening and 0.1.0

**Done**

- Added `tauri-plugin-single-instance`: a second launch shows and focuses the
  existing popover instead of starting another tray process.
- Added `tauri-plugin-log`: stdout plus one rotating 2 MiB file in the platform
  log directory. Added `open_logs` and **Settings → General → Open logs**.
- Added Vitest + Testing Library with jsdom, `@testing-library/jest-dom` and
  `user-event`; 12 tests cover search, drafts, themes and `ConfirmDialog`.
- Added the frontend test step to CI and to `bun run check`.
- Prepared the `0.1.0` changelog entry. macOS remains ad-hoc signed; no Apple
  Developer account is used.

**Verified**

- `bun run check`: 12 frontend tests and 40 Rust tests pass, plus clippy, ESLint,
  typecheck and production build.
- `bun run version:check` reports `0.1.0` consistently.

**Next**

- Push `main`, confirm CI, tag `v0.1.0` and review the draft release.

### 2026-11-19 — Session 10: 0.1.0 released

**Done**

- Pushed `main`; the first CI run found a macOS-only helper that was dead code
  on Linux and Windows. Gated `hide_now` behind `cfg(target_os = "macos")`.
- CI is green on Ubuntu, macOS and Windows (frontend tests, Rust gates and the
  three-platform `cargo check` matrix).
- Ran the release workflow manually: macOS universal, Linux x64/ARM64 and
  Windows x64 built successfully.
- Tagged `v0.1.0`; the tag run produced the draft release with all artifacts and
  `SHA256SUMS`. Published it as **peekr 0.1.0**.
- Added the download table and unsigned-build caveat to the README.

**Verified**

- CI: green on `main`.
- Security: `bun audit` and `cargo audit` passed.
- Manual release run: all four build jobs succeeded.
- Release: nine assets, including `SHA256SUMS`.

**Next**

- Cross-platform visual QA on real Windows and Linux machines.
- Expand frontend tests and decide on IPC codegen.

### 2026-11-19 — Session 11: manual update check

**Done**

- Added `update.rs`: compares the running version with the latest published
  GitHub release using the shared HTTP client. One request, no background polling.
  A private repository falls back to the first configured GitHub token in Rust.
- Added `check_for_updates` and **Settings → General → Check for updates**. The
  result is display-only; the release page opens in the browser.
- Supports `v` prefixes, numeric ordering and pre-release ordering; unit tests
  cover newer, current, equal and draft responses.
- Added frontend tests for the update-available and up-to-date states.

**Verified**

- `cargo test update::tests` (5 tests).
- `bun run test` (14 tests) and `bun run typecheck`.

**Next**

- Cross-platform visual QA on real Windows and Linux machines.

### 2026-11-20 — Session 12: About panel

**Done**

- Added `app_info` (version, platform, arch, config dir, log dir) and the
  `AppInfo` wire type.
- Added **Settings → About** with the app icon, name, tagline, version, platform,
  config/log paths, keychain note, **Open logs** and the update check.
- Moved logs and updates out of General so General only holds polling and launch
  at login.
- Styled About as a distinct sidebar entry with a larger brand block and a
  monospaced metadata grid.

**Verified**

- `bun run check` (15 frontend tests and 46 Rust tests, plus clippy, ESLint,
  typecheck and production build).

**Next**

- Cross-platform visual QA on real Windows and Linux machines.

---

## Verification

```bash
# Full gate
bun run check

# Individual gates
bun run typecheck
bun run lint
bun run test
bun run build
bun run rust:lint
bun run rust:test
```

**Last verification run:** 2026-11-19.

| Gate                  | Result                                              |
| --------------------- | --------------------------------------------------- |
| `bun run typecheck`   | pass                                                |
| `bun run lint`        | pass                                                |
| `bun run test`        | 15 tests pass in 6 files                            |
| `bun run build`       | pass                                                |
| `cargo fmt --check`   | pass                                                |
| `cargo clippy`        | pass with `-D warnings`                             |
| `cargo test`          | 46 tests pass                                       |

No frontend test runner yet. Manual acceptance steps are in
[QA-CHECKLIST.md](../QA-CHECKLIST.md).
