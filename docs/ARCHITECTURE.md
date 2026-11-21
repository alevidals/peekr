# peekr: Architecture

**Status:** Implemented. This document describes the current architecture.
**Date:** 2026-11-19
**Scope:** Layering, module boundaries, the provider and polling contracts, IPC,
the tray/popover model, persistence, security and the design system.

## 1. Starting point

peekr starts as a Tauri 2 + React + TypeScript template and grows into a
tray-only desktop client. The repository is intentionally small: one Rust crate,
one Vite project and two windows. There is no server, no database and no cloud
account.

`docs/PROGRESS.md` records how the current features were built. `docs/ROADMAP.md`
tracks what comes next, and the ADRs record the decisions that should not be
re-litigated by accident.

## 2. Product principles and non-goals

### Principles

1. **The tray is the product.** The main surface is a compact popover attached
   to the tray icon. Settings is a separate, ordinary window opened on demand.
2. **Only what needs attention.** The review tab lists PRs still waiting on the
   user. GitHub review searches and Bitbucket approval state both prune the list.
3. **Honest verdicts.** GitHub's merge state is shown as GitHub computes it.
   Bitbucket uses its mergeability checks when available, falls back to a clearly
   labelled inference when they are not, and never claims to detect conflicts in
   that fallback.
4. **No duplicate attention.** A PR seen through two accounts is counted and
   notified once. An announcement is made once and not repeated when a search
   result flickers.
5. **Secrets never leave the keychain.** Tokens are read into Rust memory to make
   requests. The webview receives `has_token`, never the token itself.
6. **Native where it matters.** A non-activating macOS panel, a template tray
   image and platform notifications. The web layer renders the content, not the
   chrome.
7. **A calm, dense interface.** One accent, hairline separators, small metadata
   and no decorative charts. The UI is a list, not a dashboard.

### Non-goals

- Reviewing, approving, commenting on or merging PRs from peekr.
- A full repository browser or code viewer.
- Analytics, team dashboards, or historical reporting.
- Mobile targets.
- A peekr account, telemetry or remote configuration.
- Cloud storage of any kind.

These are not architectural blockers, but they are not planned surface either.

## 3. System context

```text
┌──────────────────────────────────────────────────────────────────┐
│ Tauri (one process)                                              │
│                                                                  │
│  Popover window (380×500)          Settings window (820×660)     │
│  React 19 + TypeScript             React 19 + TypeScript         │
│  ┌───────────────────────┐         ┌───────────────────────┐     │
│  │ tabs, search, list    │         │ accounts, general,    │     │
│  │ filters, footer       │         │ appearance            │     │
│  └───────────┬───────────┘         └───────────┬───────────┘     │
│              │ typed invoke / events           │                 │
├──────────────┼─────────────────────────────────┼─────────────────┤
│ Rust         ▼                                 ▼                 │
│  commands (get/save config, validate, list, refresh, windows)    │
│      │                                                           │
│      ├── ConfigStore (Arc<Snapshot>: config + tokens + revision) │
│      ├── PrState (review list, own list)                         │
│      ├── PollTrigger (watch channel)                             │
│      │                                                           │
│  poller task ──► provider::build ──► Bitbucket / GitHub          │
│      │                 │                                         │
│      ├── notifier (new PRs, ready/failed transitions)            │
│      ├── tray (badge, tooltip, dev marker)                       │
│      └── events: prs-updated, my-prs-updated, poll-error         │
│                                                                  │
│  popover module: NSPanel + tray-edge placement + hide guard      │
│  secrets module: OS keychain, one entry per account              │
│  config module: atomic JSON, migrations, corrupt-file backup     │
└──────────────┬───────────────────────────────────────────────────┘
               ▼
   api.bitbucket.org (REST)   api.github.com (GraphQL)
```

### Process and threading model

- One Tauri process. Rust networking and polling run on Tokio's multi-thread
  runtime; the UI runs in the platform webview.
- `tauri-plugin-single-instance` enforces one tray process. Launching peekr again
  moves the existing popover under the tray and focuses it.
- `tracing` writes to stderr and to one file per UTC day (`peekr.log.YYYY-MM-DD`)
  in the platform log directory; the seven most recent files are kept. Settings →
  About → **Open logs** reveals it.
- Two windows share one Vite bundle. `App.tsx` reads the window label and
  lazy-loads `PopoverWindow` or `SettingsWindow`, so the popover does not ship the
  settings code.
- One `reqwest::Client` is created at startup, managed by Tauri and shared by the
  poller and token validation. It has a 30-second timeout, a single connection
  pool and the `peekr` user agent.
- The poller is a single task started in `setup`. It is woken by a `watch`
  channel when the user refreshes or saves settings, and otherwise sleeps for the
  configured interval.
- Network requests are async. Notification display blocks while waiting for a
  click, so each notification gets its own OS thread.
- No blocking I/O runs on the webview thread.

## 4. Layering and module boundaries

### Rust backend

```text
src-tauri/src/
  main.rs                    # binary entry point only
  lib.rs                     # composition root: plugins, state, poller, handlers
  commands.rs                # Tauri adapters; request/response DTOs
  config.rs                  # AppConfig, accounts, theme, migrations, atomic writes
  domain.rs                  # provider-agnostic PR, CI and merge types
  error.rs                   # AppError, serialized as a readable string
  http.rs                    # shared client, JSON decoding, one retry, body caps
  notifier.rs                # new-PR diff, My PR transitions, native notifications
  poller.rs                  # background loop, backoff, merge/dedupe, events
  popover.rs                 # show/hide, positioning, macOS NSPanel and Space observer
  update.rs                  # manual GitHub release check and version comparison
  provider/
    mod.rs                   # ReviewProvider trait, FetchReport, concurrency cap
    bitbucket.rs             # Bitbucket Cloud REST adapter
    github.rs                # GitHub GraphQL adapter
  secrets.rs                 # keychain read/write/delete; never returned to the UI
  state.rs                   # ConfigStore, Snapshot, PrState, PollTrigger
  tray/
    mod.rs                   # tray icon, menu, badge and tooltip updates
    badge.rs                 # template icon and runtime badge rendering
```

**Dependency direction:** `lib → commands / poller / tray / popover`, and those
depend on `config / domain / provider / secrets / state / error / http`. Tauri
adapters are concentrated in `lib.rs`, `commands.rs`, `popover.rs`, `notifier.rs`
and `tray/`. `domain` has no Tauri, network or filesystem dependency, and the
provider modules depend on `reqwest` and the domain, not on the frontend or the
tray.

This keeps the domain and provider logic testable with ordinary Rust tests and
no running webview.

### Frontend

```text
src/
  main.tsx                   # React root
  App.tsx                    # window label → lazy window component
  api.ts                     # the only place that knows command and event names
  types.ts                   # handwritten mirror of the Rust wire types
  providers.tsx              # per-provider copy, token scopes and token URLs
  search.ts                  # query parsing and PR matching
  drafts.ts                  # settings editing state ↔ AccountInput
  theme.ts                   # palette resolution, CSS variables, contrast math
  themes.ts                  # named theme definitions and previews
  App.css                    # the design system and window layout
  components/
    AccountForm.tsx          # one account's fields and validation
    AboutPanel.tsx           # identity, paths, credentials note and update check
    ConfirmDialog.tsx        # destructive-action confirmation
    EmptyState.tsx
    Icons.tsx
    PrItem.tsx               # one PR row
    PrList.tsx               # grouping, counts, provider labels
    Settings.tsx             # settings window body
    UpdateCheck.tsx          # manual GitHub release comparison
  hooks/
    useConfig.ts             # load config, apply appearance, isConfigured
    usePopoverAnimation.ts   # entrance/exit animation and show/hide events
    usePullRequests.ts       # initial fetch, event listeners, refresh
  windows/
    PopoverWindow.tsx        # tabs, search, filters, footer
    SettingsWindow.tsx       # settings shell and close behaviour
```

**Rules**

- Components never call `invoke` directly. They call `api.ts`.
- Provider-specific words live in `providers.tsx`, not in the row or list
  components. This is what keeps "checks" vs "pipelines" consistent.
- The wire types in `types.ts` mirror the Rust structs. They are handwritten
  today; see [ADR-0005](adr/0005-state-and-ipc.md) for the trade-off.
- Business decisions (what counts as ready, when to notify, when to hide an
  approved PR, backoff) live in Rust. The frontend renders the result.
- Appearance is applied by `applyAppearance` through CSS variables. A theme is a
  palette plus a resolved light/dark scheme.

## 5. Domain model

Every provider maps its payload to one provider-agnostic shape:

```rust
pub struct PullRequest {
    pub provider: ProviderKind,      // Bitbucket | Github
    pub account_id: String,          // configured account
    pub repo: String,                // slug, or owner/name for GitHub
    pub number: u64,                 // unique only within repo
    pub title: String,
    pub author_display_name: String,
    pub author_avatar_url: String,
    pub source_branch: String,
    pub destination_branch: String,
    pub created_on: String,
    pub updated_on: String,
    pub comment_count: u32,
    pub html_url: String,
    pub draft: bool,
    pub approvals: u32,
    pub ci_status: Option<CiStatus>, // None: no checks or unreadable
    pub merge: Option<MergeStatus>,  // only on the user's own PRs
}
```

`PullRequest::key()` is `"{account_id}/{repo}#{number}"`. Numbers repeat across
repositories and accounts, so this is the identity used by the list, the badge,
notifications and deduplication.

### CI status

`CiStatus` is `Success`, `Failed` or `Pending`. `CiStatus::combine` folds many
check results into one: any failure wins, then anything pending, and success only
when every check passed. No checks means `None`, which the UI renders as unknown
rather than green.

### Merge state

```rust
pub enum MergeState {
    Ready,
    Waiting,
    Blocked,
    Conflicts,
    Behind,
    Draft,
    Unknown,
}

pub struct MergeStatus {
    pub state: MergeState,
    pub blockers: Vec<String>,
}
```

For GitHub, `MergeState` is a direct mapping of GitHub's `mergeStateStatus`.
For Bitbucket, it comes from the PR's mergeability checks when the token can read
them; otherwise it is inferred client-side (see §6). `blockers` carries
human-readable reasons and never secrets.

## 6. Provider boundary

```rust
#[async_trait]
pub trait ReviewProvider: Send + Sync {
    async fn validate(&self) -> AppResult<String>;
    async fn fetch_review_prs(&self, repos: &[String]) -> AppResult<FetchReport>;
    async fn fetch_my_prs(&self, repos: &[String]) -> AppResult<FetchReport>;
}

pub struct FetchReport {
    pub prs: Vec<PullRequest>,
    pub failed_repos: Vec<(String, AppError)>,
}
```

`FetchReport` reports per-repository failures separately. One broken repository
must not hide the others, and its previously known PRs must not look closed.
`provider::build` selects the adapter from `ProviderKind` and returns
`Arc<dyn ReviewProvider>`.

### Bitbucket

- REST API at `https://api.bitbucket.org/2.0`.
- Lists open PRs where the authenticated account is a reviewer. The server-side
  `reviewers.account_id` filter is tried first; the adapter permanently falls
  back to client-side filtering the first time Bitbucket rejects the query for
  the token type.
- Resolves the token owner's account id once.
- Fetches participants, build statuses and task counts as needed to compute the
  CI state and the merge state.
- For the user's own PRs, tries `pullrequests/{id}/mergeability/checks` first.
  That endpoint covers Bitbucket's merge rules when the token can read them.
  When it fails, the adapter falls back to a client-side inference from build
  statuses, approvals, requested changes and open tasks. The fallback cannot see
  merge conflicts.
- Hides PRs the user already approved.
- Fans out with at most 8 requests in flight per account, enforced by a
  semaphore. This is a real cap across repositories and PRs, not just a buffering
  hint.

### GitHub

- GraphQL API at `https://api.github.com/graphql`.
- Review requests: one paginated search with `is:pr is:open review-requested:@me`,
  scoped to the account owner. Team review requests are included.
- Own PRs: `author:@me`, scoped the same way.
- The search node carries branches, comment count, reviews, the check rollup and
  `mergeStateStatus`, so no per-PR request is needed.
- The token owner's login is resolved once and cached in a `OnceCell`.
- Search returns at most 1,000 results; the adapter pages up to 10 × 100.

### Adding a provider

1. Implement `ReviewProvider` in `src-tauri/src/provider/<name>.rs` and map the
   payload to `PullRequest`.
2. Add the variant to `ProviderKind` and the arm in `provider::build`.
3. Add the settings copy, token scopes and token URL to `src/providers.tsx`.
4. Add the variant to `ProviderKind` in `src/types.ts`.
5. Add adapter tests for payload mapping, failures and the empty case.

The poller, notifier, tray and list UI should not need provider-specific
branches. If they do, the provider boundary is leaking.

## 7. Polling pipeline

```text
startup
  └─ poller::run
       ├─ snapshot = ConfigStore.snapshot()
       ├─ revision changed? → rebuild providers
       ├─ no active accounts? → clear lists, emit problems, wait 30 s
       ├─ join!(
       │    fetch_review_prs for every account,
       │    fetch_my_prs for every account
       │  )
       ├─ review batch → merge with stale PRs from failed sources → dedupe → sort
       │    ├─ notifier::newly_added → notify only seeded accounts and unseen keys
       │    ├─ emit "prs-updated"
       │    └─ tray::update_badge(count)
       ├─ own batch → merge with stale account failures → sort
       │    ├─ notifier::my_pr_events → notify BecameReady / ChecksFailed
       │    └─ emit "my-prs-updated"
       ├─ emit "poll-error" with deduplicated problems
       └─ wait for interval, a manual trigger, or a settings save
```

### Seeding and once-only announcements

- An account is **seeded** after its first successful poll. Only seeded accounts
  can notify, so the first poll never floods the user with every open PR.
- `notified` keeps the keys already announced. A search result is eventually
  consistent: a known PR can vanish for one poll and come back. The key remains
  in `notified`, so it is not announced twice.
- `newly_added` compares keys, not positions or titles. A PR edited between polls
  is not "new".

### Stale data and failures

- A whole-account failure keeps all of that account's previous PRs.
- A single-repository failure keeps only that repository's previous PRs in the
  review list. In My PRs, only whole-account failures are kept, because a
  partial own list can only be missing PRs.
- Fresh PRs from healthy sources replace stale ones. Deduplication keeps the
  first occurrence of a provider/repo/number.

### Backoff

`backoff_interval(base, failures)` multiplies the configured interval by
`2^failures`, capped at three doublings. `failures` increases only when a whole
account fails, not when one repository fails, so one broken repository does not
stretch the global cadence. A successful round resets it.

### Ordering

- Review requests sort by `created_on` descending.
- Own PRs sort by `updated_on` descending.

## 8. State and IPC contract

### Backend state

| State          | Type                       | Purpose                                                          |
| -------------- | -------------------------- | ---------------------------------------------------------------- |
| `ConfigStore`  | `RwLock<Arc<Snapshot>>`    | Config, tokens in memory, and a revision for provider rebuilds   |
| `Snapshot`     | struct                     | Immutable view: `config`, `tokens`, `revision`                   |
| `PrState`      | two `tokio::Mutex<Vec<…>>` | Latest review list and own list                                  |
| `poll trigger` | `watch::Sender<()>`        | Wakes the poller after a manual refresh or a settings save       |
| HTTP client    | `reqwest::Client`          | Shared pool and timeout for providers and token validation       |

`ConfigStore::replace` keeps the previous revision only when both the config and
the token map are unchanged; any actual change, including appearance, rebuilds
providers and resolves the token owner again.

### Commands

| Area     | Command               | Purpose                                                        |
| -------- | --------------------- | -------------------------------------------------------------- |
| App      | `app_info`            | Version, platform, architecture, config and log paths          |
| Config   | `get_config`          | Polling interval, theme, appearance and accounts (`has_token`) |
| Config   | `save_config`         | Validate, write tokens, write config, delete orphans, refresh  |
| Accounts | `validate_credentials`| Check a typed token or the stored token for an account         |
| PRs      | `get_pull_requests`   | Current review list                                            |
| PRs      | `get_my_pull_requests`| Current own list                                               |
| PRs      | `refresh_pull_requests`| Wake the poller                                               |
| Windows  | `open_settings_window`| Show or recreate Settings, hide the popover first              |
| Windows  | `hide_settings_window`| Hide Settings and return to accessory mode                     |
| Windows  | `hide_popover`        | Run the animated hide                                          |
| App      | `open_logs`           | Reveal the application log directory in the file manager       |
| App      | `check_for_updates`   | Compare the running version with the latest GitHub release     |

### Events

| Event              | Payload           | When                                      |
| ------------------ | ----------------- | ----------------------------------------- |
| `prs-updated`      | `PullRequest[]`   | After every review poll and when cleared  |
| `my-prs-updated`   | `PullRequest[]`   | After every own-PR poll and when cleared  |
| `poll-error`       | `string`          | Deduplicated problems from a round        |
| `popover-will-hide`| none              | Before the 150 ms hide animation          |
| `popover-will-show`| none              | Before the panel is shown                 |

### Wire conventions

- Field names are `snake_case` to match the Rust structs; there is no
  `rename_all` layer today.
- Commands return `Result<T, String>` on the wire: `AppError` serializes to the
  full cause chain so the UI can show DNS, TLS or HTTP detail.
- `get_config` returns `has_token`, never the token. There is no "get secret"
  command.
- `types.ts` is the handwritten mirror. When a command, event or DTO changes,
  update both sides. [ADR-0005](adr/0005-state-and-ipc.md) records why codegen is
  deferred and what would justify adding it.

## 9. Tray, popover and windows

### Tray

- One tray icon, ID `main`, with a Quit menu item.
- Left click toggles the popover. The menu does not open on left click.
- macOS uses a template image, so the system tints the eye for the current menu
  bar. The red badge and the orange dev marker are `CALayer` overlays on the
  status item button, not baked into the template.
- Windows/Linux redraw a 44×44 PNG with `image`/`imageproc`: the eye is
  normalised to black, with an optional orange dev dot and a red badge dot.
- The tooltip is `1 PR to review` or `N PRs to review`.

### macOS panel

`popover.rs` converts the configured window into an `NSPanel` with:

- `nonactivating_panel` so it can become key without activating the app or
  switching Spaces;
- a floating level;
- `full_screen_auxiliary` and `move_to_active_space` collection behavior;
- `hides_on_deactivate(false)`;
- `works_when_modal(true)`.

The panel is positioned before it becomes visible: `popover/geometry.rs` takes
the tray icon rectangle from the click event, finds the screen edge the icon is
docked to and opens the window towards the inside of the work area, flipping
above/below/left/right as needed and clamping to keep it fully on screen. A Space
observer hides the panel when the user switches desktop or enters/leaves full
screen, instead of joining the window to the new Space.

Closing is a two-step sequence: emit `popover-will-hide`, wait 150 ms, then hide.
A generation counter resolves the race between tray clicks and the blur handler;
a focus gain cancels an in-flight hide. `hide_popover` and tray toggle both use
the same path.

On Windows and Linux `configure_panel` is a no-op: the popover is a normal
undecorated, always-on-top, non-resizable window positioned next to the tray
icon. The same geometry handles a taskbar docked at the bottom, top, left or
right. Linux tray icons do not emit clicks through `libappindicator`, so the tray
menu also has an **Open peekr** item.

### Settings window

Settings is an ordinary decorated window. On macOS it uses the overlay title bar
with a hidden title so content extends under the traffic lights. Opening Settings
hides the popover, switches the activation policy to `Regular` (Dock icon), and
focuses the window. Closing it returns to `Accessory`. If the window was
destroyed, `open_settings_window` rebuilds it.

The settings sidebar has Accounts, General, Appearance and About. About shows the
app identity, version, platform, config/log paths, the keychain note and the
manual update check.

## 10. Notifications

- `notifier::newly_added` returns PRs whose key was not in the previous list.
- `notifier::my_pr_events` returns only transitions: `BecameReady` and
  `ChecksFailed`. A newly seen own PR raises nothing.
- Each event becomes its own notification so a click can open the right PR.
- A notification ledger keyed by PR key, event and `updated_on` suppresses repeats
  when a provider briefly reports a different merge state between polls. A new PR
  update can notify again.
- Titles use provider vocabulary:
  - Bitbucket: "Looks ready to merge", "Pipeline failed".
  - GitHub: "Ready to merge", "Checks failed".
- macOS uses `mac-notification-sys`. In development there is no app bundle, so
  notifications are attributed to `com.apple.Terminal`; packaged builds use the
  app identifier. Other platforms use `notify-rust`, with an app id on Windows
  and an explicit action on Linux.
- Showing a notification and waiting for a click blocks, so `send` spawns a
  thread per notification. A click opens `html_url` through
  `tauri-plugin-opener`.

## 11. Persistence and configuration

```text
<app config dir>/com.alevidals.peekr/
  config.json        # version, polling interval, accounts, theme, appearance
  config.json.bak    # previous unreadable file, if recovery happened
  config.json.tmp    # transient atomic-write file

<platform log dir>/com.alevidals.peekr/
  peekr.log.YYYY-MM-DD  # one file per UTC day; the seven most recent are kept
```

- `config.json` is written through a sibling temp file, `sync_all`, then rename.
  On Unix it is created with mode `0600`, so it is never world-readable.
- Load history:
  - v1 (implicit): `{ workspace, repos, polling_interval_secs }`, with a single
    Bitbucket token. The oldest files may still contain a `token` field.
  - v2: `{ version, polling_interval_secs, accounts: [...] }`, tokens keyed by
    account id.
  - `accounts` present → parse as v2. Otherwise migrate v1: read the legacy
    token from the file or the keychain, create one Bitbucket account, store the
    token, save, and delete the legacy keychain entry.
- Missing fields after v2 (`theme`, `appearance`) use serde defaults, so older
  files keep loading.
- The polling interval is clamped to 30–3600 seconds on load and on save.
- A corrupt file is renamed to `config.json.bak` and the app starts with
  defaults. It is never silently overwritten.

### Account validation

- Workspace must not be empty.
- Bitbucket accounts require at least one repository.
- The same provider + workspace (case-insensitive) cannot appear twice.
- The same account id cannot appear twice in one save.

### Token write ordering

Saving writes tokens **before** the config, because an orphan keychain entry is
harmless while a config that points at a missing token is not. If the config
write fails, tokens for accounts that never reached the disk are deleted. Removed
accounts lose their tokens only after the new config has been written.

## 12. Security model

| Threat                                   | Control                                                                                             |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Token theft at rest                      | OS keychain only; `config.json` has no secrets; no plaintext fallback                               |
| Token exposure to the webview            | No get-secret command; `get_config` exposes `has_token` only                                        |
| Token in logs or errors                  | Providers never log tokens; HTTP error bodies are trimmed to a short snippet                        |
| Token sent to the wrong host             | Each adapter uses a fixed API base; the opener only receives provider-supplied PR URLs and the fixed token pages in `providers.tsx` |
| Arbitrary local file access              | No Tauri `fs` or `shell` plugin; the app does not read user files                                   |
| Remote content execution                 | Strict CSP, `withGlobalTauri: false`, bundled assets only, no remote scripts                        |
| Webview capability escalation            | Minimal capability set in `src-tauri/capabilities/default.json`                                     |
| Clipboard leakage                        | The app copies no secrets; PR URLs are opened through the OS browser                                |
| Dependency compromise                    | Committed lockfiles; pinned direct versions; weekly audits in CI                                    |
| Malicious response body                  | JSON decoding with typed structs; error bodies capped; no HTML rendered as HTML                     |
| Notification spoofing or wrong target    | Notifications are built from typed PR data; the click target is the PR's `html_url`                 |
| Update check tampering                   | Fixed GitHub API URL and typed response; no download or install path exists. A private repo uses the configured GitHub token in Rust only |

### Tauri hardening

- `withGlobalTauri` is `false`; the frontend imports only what it uses.
- The production CSP is:
  `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src ipc: http://ipc.localhost ws://localhost:1420 http://localhost:1420; object-src 'none'; base-uri 'self'`.
  The localhost entries support Vite development; they stay in the policy but
  are unused by a release build.
- Plugins: opener, autostart, single-instance and log. No filesystem, shell or
  arbitrary HTTP plugin.
- `macOSPrivateApi` is enabled for the panel. This is a distribution choice: the
  app is not sandboxed and is distributed outside the Mac App Store.
- There is no updater yet. Updates will not be silently installed.

## 13. Visual language and theming

- **Dark, light and system.** System is the default and follows the OS; the base
  CSS starts light and switches under `prefers-color-scheme` or an explicit
  `data-theme`. Named themes pin their own scheme.
- **Tokens live in CSS variables.** `theme.ts` resolves a palette into `--bg`,
  `--surface`, `--text`, `--accent`, status colours and derived "ink" colours.
  `applyAppearance` writes them to the document root and sets the `dark` class.
- **Contrast is computed, not guessed.** Status inks are mixed toward black or
  white until they clear 4.5:1 on their soft background. `readableOn` picks the
  foreground for filled badges.
- **Surfaces:** near-black or paper base, subtle elevation, hairline borders. Only
  the popover shadow separates it from the desktop.
- **Rows:** compact metadata, truncated titles with the full text on hover, 32px
  hit targets for header actions.
- **Density and text size** are stored in the config and applied globally.
- **Motion:** a short entrance for the list and a 150 ms closing animation that
  matches the native hide. `prefers-reduced-motion` disables animations.
- **Accessibility:** focus rings are outlines, tabs support arrow keys, icon-only
  buttons have names, errors use `role="alert"`/status, and decorative icons are
  hidden from assistive technology. Small badge ink meets 4.5:1.

## 14. Error model

```rust
pub enum AppError {
    Config(String),
    Keychain(String),
    Provider(String),
    Http(reqwest::Error),
    HttpStatus { status: u16, url: String, message: String },
    Io(std::io::Error),
    Json(serde_json::Error),
    Tauri(tauri::Error),
}
```

`AppError::detail()` walks the `source()` chain and appends each cause, because
`reqwest` stores DNS, TLS and connection diagnostics there. The result is
serialized as a single string to the frontend, which shows it in the error
banner, grouped and deduplicated per round.

UX mapping:

- **First run:** an empty state with an explicit **Open settings** action.
- **Loading:** the popover shows a skeleton/spinner rather than an empty list.
- **Provider failure:** a persistent banner with the concrete cause; the last
  known list stays visible.
- **Token failure:** the message names the likely cause (expired token, missing
  scope, SSO) without echoing the token.
- **Notification failure:** logged; the app keeps polling.
- **Config failure:** corrupt files are backed up; keychain failures are logged
  and the account reports `has_token: false`.

## 15. Testing strategy

### Rust

- Unit tests live next to the code:
  - `domain`: CI folding.
  - `config`: clamping, repository cleanup, v2 roundtrip, theme defaults,
    appearance validation, camelCase palettes, v1 defaults.
  - `provider/bitbucket`: payload mapping, merge checks, inference, status and
    optional fields.
  - `provider/github`: GraphQL mapping, merge states, pagination edges, comment
    counting.
  - `poller`: backoff, failure coverage, diffing, deduplication, stale merging.
  - `notifier`: transition detection and provider vocabulary.
  - `popover`: hide-generation and spurious-blur logic.
  - `http`: retry and body snippet trimming.
  - `update`: version comparison, release parsing and updater edge cases.
- Run with `cargo test --manifest-path src-tauri/Cargo.toml`.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are part of
  `bun run rust:lint`.

### Frontend

- TypeScript is checked with `tsc --noEmit`.
- ESLint runs with the React hooks rules; `bun run check` includes it.
- Vitest + Testing Library cover search parsing, draft conversion, theme
  resolution and the confirmation dialog. `bun run test` runs them in jsdom.
- A production build is part of `bun run check`.
- The suite is intentionally small. UI flows that need a live Tauri window
  (tray, notifications, keychain) stay in the manual checklist.

### Manual

[QA-CHECKLIST.md](../QA-CHECKLIST.md) covers tray, popover, accounts, both
providers, notifications, settings, appearance, security, failure recovery,
performance and platform specifics. Automated checks cannot see the native
window, notifications or the real menu bar.

## 16. Known constraints

- **Handwritten IPC types.** `types.ts` can drift from Rust. Codegen is deferred
  until the command surface grows or a drift bug appears.
- **macOS-first native behaviour.** The non-activating panel and layer-based
  badge are macOS-specific. Windows and Linux use the cross-platform window and
  icon paths, which receive less runtime testing.
- **No automatic updates.** Settings → General → Check for updates performs one
  request to GitHub and opens the release page; it never downloads or installs
  anything. An in-app updater stays deferred until signing exists.
- **No Developer ID signing yet.** The release workflow uses an ad-hoc macOS
  signature and publishes checksums; Apple notarization and Windows signing are
  the future path documented in `RELEASE.md`.
- **Bitbucket fallback.** When mergeability checks are unavailable, readiness is
  inferred and conflicts are not visible through the public API.
- **Search consistency.** GitHub search and Bitbucket listing are eventually
  consistent; the notifier remembers announced keys so a flicker cannot
  re-notify.
- **One polling loop.** Accounts are polled in parallel, but there is no
  per-account schedule or manual pause yet.

## 17. What would change the architecture

- A second UI surface (for example a full list window) would justify a shared
  store instead of window-local state.
- More than two providers, or provider-specific actions, would argue for
  generated IPC and a command registry.
- Write actions (approve, comment, merge) would require a much stricter
  authorization model, audit logging and optimistic-update rules.
- A Linux distribution with a different tray story might require a separate
  status-icon strategy.

Until then, the current layering keeps the product small: a typed provider
boundary, one polling loop, two windows and no secrets outside the keychain.
