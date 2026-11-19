# peekr: Implementation roadmap

**Status:** 0.1.0 is released. CI, release and security workflows, single
instance, file logging and the first frontend test suite are validated.
Cross-platform QA and the 0.3 polish work remain.
**Method:** Incremental milestones with explicit exit criteria. Each milestone
ends with a runnable app and a review gate. A later milestone does not start
while an earlier one has an unresolved reliability or security defect.

Sizes are relative (`S` ≈ small, `M` ≈ medium, `L` ≈ large), not calendar
commitments.

---

## Milestone 0.1: the tray client works

**Goal:** A real tray client for Bitbucket and GitHub that is usable every day.

| Area             | Deliverable                                                                                             |
| ---------------- | -------------------------------------------------------------------------------------------------------- |
| Tray             | One tray icon, click to toggle, Quit menu item, count tooltip                                            |
| Badge            | Template image and `CALayer` dot on macOS; runtime PNG badge on Windows/Linux; orange dev marker          |
| Popover          | Native `NSPanel` on macOS, positioned window elsewhere; `Esc`, click-outside and tray toggle               |
| Review requests  | Bitbucket and GitHub review lists, approved filtering, team requests, grouping, counts                    |
| My PRs           | Own PRs with GitHub merge verdict and Bitbucket mergeability checks plus fallback inference                |
| Notifications    | Once per new PR, once per ready/failed transition, click-through to the PR                                |
| Accounts         | Multiple accounts per provider, duplicate-workspace rejection, per-account tokens                         |
| Credentials      | OS keychain only, no get-secret command, orphan cleanup                                                   |
| Config           | Versioned JSON, v1 → v2 migration, atomic write, corrupt-file backup, interval clamping                   |
| Polling          | Parallel accounts, manual refresh, settings-save trigger, failure backoff                                 |
| Resilience       | Stale data kept per failed source, deduplication across accounts, once-only announcements                  |
| Appearance       | System/light/dark, eight named themes, accent, density, text size                                         |
| Accessibility    | Focus rings, named icon buttons, keyboard tabs, alerts/status, reduced motion                             |
| Security         | CSP, `withGlobalTauri: false`, minimal capabilities, no fs/shell plugins                                  |
| Tooling          | Typecheck, ESLint, rustfmt, clippy, Rust tests, production build, `bun run check`                         |

**Exit criteria**

- The app starts as a tray-only process and opens the popover from the tray.
- Review requests and own PRs are correct for both providers.
- A token is never readable by the webview or present in the config.
- `bun run check` passes on the development machine.
- [QA-CHECKLIST.md](../QA-CHECKLIST.md) has been walked once on the primary
  platform.

---

## Milestone 0.2: release infrastructure

**Goal:** Every push is checked and every tag produces artifacts someone else can
install.

**Status:** Implemented and validated. CI is green, a manual release run built
all four targets, and `v0.1.0` published with checksums.

| Area             | Deliverable                                                                                              |
| ---------------- | --------------------------------------------------------------------------------------------------------- |
| CI               | Frontend job (typecheck, ESLint, build) and Rust job (fmt, clippy, tests) on Ubuntu with Tauri system deps |
| Check matrix     | `cargo check --all-targets` on Ubuntu, macOS and Windows                                                   |
| Cache            | Rust target cache on `main`, no cache writes from pull requests, separate cache budget documented          |
| Version check    | One script validates `package.json`, `Cargo.toml`, `tauri.conf.json` and the tag                           |
| Release workflow | `v*` tags build macOS universal, Windows x64 and Linux x64/ARM64 bundles                                   |
| Artifacts        | Portable archives, installers and `SHA256SUMS`, attached to a draft release                                |
| Security workflow | `cargo audit` and `bun audit --production` weekly and on dependency changes                                |
| Docs             | `docs/RELEASE.md` matches the workflows that actually exist                                                |

**Exit criteria**

- CI is green on `main` and on a pull request.
- A manual release run produces artifacts but does not publish.
- A `v*` tag produces a draft release with checksums.
- The version check fails on a deliberate mismatch.
- A cache-cold release build succeeds on all three operating systems.

---

## Milestone 0.3: quality and polish

**Goal:** Close the gaps that automated checks cannot see, without adding product
surface.

**Status:** Started. Vitest and the single-instance guard are in; visual QA and
notification permission diagnostics remain.

| Area              | Deliverable                                                                                       |
| ----------------- | -------------------------------------------------------------------------------------------------- |
| Frontend tests    | Vitest + Testing Library for search, drafts, themes and the confirmation dialog; expand to filters and the event hook |
| Typed IPC         | Evaluate codegen (`tauri-specta` or a small generator) and remove handwritten drift                |
| visual QA         | Both themes, eight named palettes, 100/125/150/200% scaling on all platforms                        |
| macOS panel QA    | Full-screen apps, multiple Spaces, multiple displays, focus restoration, sleep/wake                 |
| Windows/Linux QA  | Tray placement, badge rendering, autostart, notifications and Secret Service availability           |
| Notifications     | Respect OS permission denial; add a diagnostics message when notifications are blocked              |
| Single instance   | Implemented: a second launch focuses the existing popover instead of starting another process        |
| Error copy        | Audit every provider and keychain message for a concrete next step                                  |
| Startup           | Measure cold start and first poll; keep the tray visible before network work begins                 |
| Docs              | User guide screenshots and a troubleshooting page that matches the UI                               |

**Exit criteria**

- The QA checklist passes on macOS, Windows and Linux.
- Frontend tests cover the pure logic and the event bridge.
- No handwritten type differs from its Rust struct.
- A second launch does not create a second tray icon.
- No placeholder copy or dead control remains.

---

## Milestone 0.4: provider breadth and PR context

**Goal:** More of the places where review requests live, without turning peekr
into a review client.

Order is deliberate. Each item ships only after the previous one is stable.

1. **GitLab** as a third provider, using the same `ReviewProvider` boundary.
2. **Provider-specific review filters** that stay honest: what each service can
   actually answer.
3. **Repository owner/name normalisation** so the list reads consistently across
   providers.
4. **Optional desktop notification actions** ("Open", "Snooze") where the
   platform supports them.
5. **Account health diagnostics** in Settings: last successful poll, last error,
   token scopes visible to the app.
6. **Configurable per-account polling** if multiple accounts make one global
   interval insufficient.

**Exit criteria**

- Each provider maps to the same domain type and the poller needs no provider
  branch.
- A provider can be disabled or removed without affecting the others.
- Diagnostics never reveal a token.

---

## Milestone 1.0: stable

**Goal:** A contract users can rely on, not a feature count.

| Area         | Deliverable                                                                                   |
| ------------ | ---------------------------------------------------------------------------------------------- |
| Security     | Threat-model review, dependency audits, secret-leak audit, CSP/capability review               |
| Reliability  | Soak test with the app running for days; sleep/wake, network changes and token expiry          |
| Notifications| Deduplication proven across search flicker, account reload and app restart                      |
| Packaging    | Signed/notarized macOS build when a certificate exists; Windows signing decision documented     |
| Updates      | Implemented: manual "Check for updates" and checksums; no silent installer or auto-download      |
| Docs         | User guide, architecture, ADRs and release notes reviewed against the shipped app               |
| Support      | Issue templates, a reproducible bug-report checklist and a security policy                     |

**Exit criteria**

- No known data-loss or token-leak path.
- A release has survived two weeks of daily use on each supported platform.
- The unsigned-build trade-off is documented everywhere a user might look.
- The roadmap after 1.0 contains only additive work.

---

## Post-1.0 backlog

- SSH/GitHub App authentication for organisations that disallow PATs.
- A compact "review requested" history or weekly summary, opt-in and local.
- Custom notification sounds or per-repository mute rules.
- Linux tray fallbacks for desktops without a status-notifier host.
- Theme import/export.

These are ideas, not commitments. They do not justify weakening the current
security or reliability guarantees.

---

## Cross-cutting definition of done

A change is done only when:

1. It is implemented in Rust or React with no mock path in production.
2. Provider limitations are respected and labelled, not hidden.
3. Errors have a user-safe message and a recovery action.
4. It is keyboard accessible and has accessible labels.
5. It works in light, dark and the named themes.
6. Rust logic has tests; important UI behavior has a manual QA step until the
   frontend test runner lands.
7. Logs contain no tokens; `config.json` contains no secrets.
8. Docs or an ADR are updated if behavior or architecture changed.
9. It does not regress startup, polling or list rendering.

---

## Sequencing and review gates

```text
0.1 tray client ──► gate: daily-use correct, keychain-only, QA walked once
0.2 release      ► gate: CI green, artifacts and checksums reproducible
0.3 quality      ► gate: QA matrix, frontend tests, no type drift
0.4 providers    ► gate: third provider through the same boundary
1.0 stable       ► gate: security + soak + packaging sign-off
```

**No milestone skipping.** In particular, release automation is not built on an
unverified app, and provider breadth is not added before the two existing
providers have a shared test story.
