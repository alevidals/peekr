# peekr user guide

peekr is a tray app that keeps the pull requests waiting on you in one place.
It supports Bitbucket Cloud and GitHub, several accounts of either provider, a
badge in the menu bar or system tray, and native notifications.

This guide covers installation, account setup and the behaviour you should
expect. For the internals, read [ARCHITECTURE.md](ARCHITECTURE.md).

## Install

peekr currently runs from source or from the unsigned builds produced by the
release workflow. See [ROADMAP.md](ROADMAP.md) and [RELEASE.md](RELEASE.md).

Prerequisites:

- [Bun](https://bun.sh) 1.4.4 or newer.
- Rust 1.99.1 or newer; the repository pins this in `rust-toolchain.toml`.
- The [Tauri 2 system dependencies](https://v2.tauri.app/start/prerequisites/)
  for your platform.

```bash
git clone https://github.com/alevidals/peekr.git
cd peekr
bun install
bun run tauri dev
```

To build a bundle for the current platform:

```bash
bun run tauri build
```

Artifacts are written under `src-tauri/target/release/bundle/`. Builds are not
Developer ID signed or notarized; the release workflow applies an ad-hoc macOS
signature and attaches `SHA256SUMS` to the draft release. Unsigned macOS builds
are ad-hoc signed at best, and Windows builds trigger SmartScreen. Read
[SECURITY.md](../SECURITY.md) before distributing a build.

### First launch

1. The app starts without a Dock or taskbar icon. Look for the eye in the menu
   bar (macOS) or the system tray (Windows/Linux).
2. Click the icon to open the popover.
3. With no accounts, the popover offers **Open settings**.
4. Add an account, save, and wait for the first poll.

The tray icon shows a red dot when at least one PR is waiting. A debug build
also shows a small orange marker so you can tell it apart from an installed
build.

## Add an account

Open **Settings → Accounts → Add account** and choose the provider. Each account
has a workspace, an optional repository list and an access token.

### Bitbucket Cloud

| Field        | What to enter                                                                 |
| ------------ | ----------------------------------------------------------------------------- |
| Workspace    | The slug in `bitbucket.org/<workspace>/repo`                                  |
| Repository   | At least one repository slug, one per line                                    |
| Access token | A workspace or repository access token from Atlassian's token page            |

Required scopes:

- `read:pullrequest:bitbucket`
- `read:user:bitbucket`
- `read:repository:bitbucket`

Bitbucket accounts require at least one repository. peekr hides PRs you have
already approved, but it still needs to see them to know that.

### GitHub

| Field      | What to enter                                                                                  |
| ---------- | ---------------------------------------------------------------------------------------------- |
| Owner      | The organisation or user in `github.com/<owner>/repo`                                          |
| Repository | Optional. Leave empty to watch every repository. One name or `owner/name` per line             |
| Token      | A fine-grained or classic personal access token                                               |

Fine-grained tokens need:

- **Pull requests (Read)**
- **Checks (Read)** and **Commit statuses (Read)** for CI state

Classic tokens need the `repo` scope. If the organisation enforces SAML SSO,
authorise the token for that organisation or GitHub will answer with a 403.

### Several accounts

You can add any number of Bitbucket and GitHub accounts. Two accounts for the
same provider and workspace are rejected, because they would see the same PRs.
The same PR visible through two different accounts is listed, counted and
notified once.

Tokens are stored in the OS keychain, one entry per account, under the service
name `peekr`. They are never written to `config.json`, logs or exports.

## The popover

Click the tray icon to open the popover. It closes with `Esc`, with a click
outside, or with the tray icon again.

### Review requests

The first tab lists open PRs where you are a requested reviewer:

- **GitHub** uses the search `is:pr is:open review-requested:@me`, which includes
  requests made to teams you belong to. This is a **pending review request**, not
  an approval filter: submitting any review clears it, so the PR leaves the list
  after **Approve** and after **Request changes**. The ball is then with the
  author; it returns when someone requests your review again. With **General →
  Show approved pull requests** enabled, peekr also searches
  `is:pr is:open reviewed-by:@me review:approved` and keeps only PRs whose latest
  review by you is an approval. A PR that is pending again stays pending even if
  an older review was an approval.
- **Bitbucket** lists open PRs where you are a reviewer and, by default, hides
  the ones you have already approved. A PR where you requested changes stays
  visible, because Bitbucket keeps you as a reviewer. Enable **General →
  Show approved pull requests** to keep your approvals in the list.

Rows show the title, author, source and destination branch, comment count,
approval count and CI state. Approved rows say **Approved** instead of the author
and sit after the pending ones. Clicking a row opens the PR in your browser.

The repo header groups the list by account and repository. When more than one
provider is configured, each group also shows its provider.

### Search and filters

- Press `/` anywhere in the popover to search.
- Every term must appear somewhere in the title, author, repository, branches or
  `#number`. Searching `api fix` matches a PR whose title contains both words.
- The eye button hides draft PRs.
- The check button hides PRs whose checks are not green.
- Closing search clears the query immediately.

### Refresh

The refresh button asks the poller to run now instead of waiting for the next
interval. The footer shows when the list was last updated.

## My PRs

The second tab lists your own open PRs and whether each one could be merged.

### GitHub

GitHub computes `mergeStateStatus` itself, so required checks, required reviews,
conflicts and a behind branch follow the branch protection of the repository.
peekr renders that verdict. When GitHub reports `UNSTABLE` (mergeable, but
non-required checks are failing or still running), the row says so instead of
hiding the caveat.

### Bitbucket

peekr first asks Bitbucket for the PR's mergeability checks, which cover the
merge rules the token can read (approvals, builds, conflicts and so on). When
that endpoint is unavailable for the token, it falls back to a client-side
inference from build statuses, approvals, requested changes and open tasks. The
fallback cannot detect merge conflicts.

The row says "Looks ready" instead of "Ready", so the inference remains visible
even when the mergeability checks supplied the verdict.

### Notifications

peekr notifies you when one of your PRs:

- becomes ready to merge, or
- starts failing its checks or pipeline.

The first time an account is polled, its existing PRs are not announced. Each
transition is notified once; a PR that disappears from a search result for one
poll and comes back is not announced again. For My PRs, the same readiness or
failure event is announced once per PR update even if the provider briefly
reports a different merge state between polls; new activity can notify again.

Clicking a notification opens that PR. On macOS, a packaged build is attributed
to peekr; a development build is attributed to Terminal because it has no bundle
of its own.

Each notification kind can be turned off in **Settings → General →
Notifications**. Turning one off keeps the event out of your desktop but does not
replay it if you turn it back on later.

## Settings

Settings is a normal window with a Dock or taskbar entry while it is open. When
it closes, peekr returns to tray-only mode.

### Accounts

Add, edit and remove accounts. Saving without typing a token keeps the stored
token. Removing an account asks for confirmation; the account and its token are
deleted when you save. If the account has not been saved yet, the dialog discards
it instead. Cancelling the confirmation keeps the account untouched.

### General

**Polling**

- **Polling interval**: 30 to 3600 seconds. The default is 120 seconds.

**Startup**

- **Launch at login**: registers peekr with the OS autostart mechanism.

**Review list**

- **Show approved pull requests**: keeps the PRs you already approved in the
  review tab. They are marked **Approved**, sorted after the pending ones, and
  never count towards the tray badge or a "new review request" notification.
  Off by default.

**Notifications**

- **New review requests**: a PR starts waiting on your review. On by default.
- **Ready to merge**: one of your PRs becomes mergeable. On by default.
- **Failing checks**: checks or pipelines on one of your PRs fail. On by default.

If several polls fail in a row, peekr backs off up to three doublings of the
configured interval, so a service that is down or throttling is not hammered.

### About

- App name, icon and version.
- Platform and architecture.
- Config and log directories, with **Open logs** to reveal the log folder.
- A reminder that credentials live in the OS keychain.
- **Check for updates**: asks GitHub for the latest published release and
  compares it with the running version. It only shows the result and opens the
  release page in the browser; nothing is downloaded or installed automatically.
  On a private repository it reuses the first configured GitHub account's token
  (in Rust only) so the release can be read.

### Appearance

- **Theme**: System, Light or Dark.
- **Named themes**: One Dark, Gruvbox Dark, Gruvbox Light, Nord, Dracula,
  Solarized Light, Tokyo Night and Catppuccin Mocha. Each one pins its own light
  or dark scheme.
- **Accent**: an optional `#rrggbb` override. An invalid value is discarded.
- **Density**: comfortable or compact.
- **Text size**: small, medium or large.

Themes are applied to both windows and persist in `config.json`.

## Where data lives

| Item         | Location                                                                 |
| ------------ | ------------------------------------------------------------------------ |
| Config       | The platform app-config directory for `com.alevidals.peekr`/`config.json` |
| Tokens       | The OS credential store, service `peekr`, one entry per account          |
| Logs         | The platform log directory for `com.alevidals.peekr` (one file per UTC day, last 7 kept) |
| Window state | Not persisted yet; the popover is positioned under the tray on each open  |

`config.json` contains the schema version, polling interval, accounts, theme and
appearance. It is written atomically with owner-only permissions where the
platform supports it. An unreadable file is renamed to `config.json.bak` and the
app starts with defaults instead of overwriting it.

## Troubleshooting

### The list is empty

- Check that the account has a token (`has_token` in Settings) and, for
  Bitbucket, at least one repository.
- Confirm the token can see the workspace or organisation; a missing scope
  usually produces a specific 401/403 message.
- Press refresh and read the error banner. A 403 from an SSO organisation means
  the token must be authorised for that organisation.
- Remember that GitHub removes a PR from the review search after you submit a
  review, and Bitbucket hides PRs you approved.

### The badge does not change

The badge counts the review tab only, not My PRs. If the list is empty, the
badge disappears. A debug build also shows an orange dot, which is the dev
marker, not a count.

### Notifications do not appear

- Check the OS notification permission for peekr (or Terminal, in development).
- On macOS, Focus mode can suppress them.
- The first successful poll of an account never notifies; only later additions
  and transitions do.
- My PRs only notify when a PR becomes ready or its checks fail, not on every
  update.

### Diagnostics

Open **Settings → About → Open logs** to reveal the application log in Finder,
Explorer or the system file manager. The log goes to stderr in development and to
the platform log directory in a packaged build. It writes one file per UTC day as
`peekr.log.YYYY-MM-DD` and keeps the seven most recent; there is no per-file size
cap. It does not contain tokens.

If a notification does not appear, check the OS permission for peekr (or Terminal
in development) and the log for a `[notification] failed` line.

### The update check says the app is current

The check uses GitHub's `/releases/latest` endpoint. That endpoint ignores draft
releases and pre-releases, so a release marked as a pre-release does not count as
an update. A network or rate-limit error is shown in Settings instead of being
treated as “up to date”.

### The popover does not open over a full-screen app

This is a macOS-specific feature. If the panel does not appear, check that the
app is built with the standard Tauri macOS private API configuration and that
the window was converted to an `NSPanel`. On Windows and Linux the popover is a
regular positioned window and cannot float over another app's full-screen space.

### A network error appears

peekr reports the underlying cause (DNS, TLS, connection reset) and retries a
transient request once. A run of failed polls backs off. Recovery clears the
banner on the next successful poll. The previously known review list is kept
when an account fails, so a temporary outage does not look like every PR was
closed.

### Settings says the keychain is unavailable

On Linux, token storage needs a Secret Service provider (GNOME Keyring, KWallet
or similar). In a headless environment there may be no keychain; peekr reports
the failure instead of falling back to plaintext.

### Reset the app

Quit peekr, remove `config.json` from the app-config directory, and delete the
`peekr` entries from the OS credential store. The next launch starts at the
first-run state.

## Privacy

peekr talks only to:

- `api.bitbucket.org` for Bitbucket accounts.
- `api.github.com` for GitHub accounts.
- The token creation page when you click the token link in Settings.
- The PR URL you click, through the OS browser.

There is no telemetry, analytics, remote config or peekr account. The only
stored values are the configuration above and the access tokens in the OS
credential store.
