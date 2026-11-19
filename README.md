<p align="center">
  <img src="public/app-icon.png" width="96" height="96" alt="peekr" />
</p>

<h1 align="center">peekr</h1>

<p align="center">
  A tray-first desktop client for the pull requests waiting on you.<br />
  Bitbucket Cloud and GitHub, native notifications, a live badge in the menu bar,
  and a popover that stays out of the way.
</p>

<p align="center">
  <a href="https://github.com/alevidals/peekr/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/alevidals/peekr/ci.yml?branch=main&style=flat-square&label=ci" /></a>
  <img alt="License" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square" />
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-macOS%20%7C%20Windows%20%7C%20Linux-64748b?style=flat-square" />
  <img alt="Stack" src="https://img.shields.io/badge/stack-Tauri%202%20%C2%B7%20React%2019-4f8cff?style=flat-square" />
</p>

<p align="center">
  <a href="docs/USER_GUIDE.md">User guide</a> ·
  <a href="docs/ARCHITECTURE.md">Architecture</a> ·
  <a href="CHANGELOG.md">Changelog</a> ·
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

## What it does

peekr lives in the tray. A badge shows how many pull requests are waiting for
your review, and a click opens a compact popover with the list. A notification
arrives once per new request, and clicking it opens the PR in your browser.

A second tab lists your own open PRs and tells you whether each one could be
merged now. GitHub supplies its own verdict. Bitbucket uses its mergeability
checks when the token can read them, and otherwise falls back to pipelines,
approvals, requested changes and open tasks.

## Highlights

|                                       |                                                                                                                                                     |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Two providers, many accounts**      | Bitbucket Cloud and GitHub, each with any number of workspaces or organisations. A PR seen through two accounts is listed, counted and notified once. |
| **An inbox, not a dashboard**         | Only PRs still waiting on you: Bitbucket hides the ones you already approved, and GitHub drops a PR from `review-requested:@me` once you review it.  |
| **My PRs with an honest verdict**     | GitHub's `mergeStateStatus` follows branch protection. Bitbucket uses mergeability checks when available and labels the fallback inference "Looks ready". |
| **Notifications that point somewhere** | One notification per new request or readiness change, and clicking it opens that exact PR. The first poll of an account never floods the user.        |
| **A native macOS popover**            | A non-activating panel that opens over full-screen apps and on any Space, keeps keyboard focus and closes on `Esc` or a click outside.              |
| **Themes that belong here**           | System, light and dark, plus One Dark, Gruvbox, Nord, Dracula, Solarized, Tokyo Night and Catppuccin, with density and text-size controls.          |
| **Tokens stay in the keychain**       | Access tokens live in the OS credential store. `config.json` and logs never contain them, and the webview never receives a stored token back.        |

## Status

`0.1.0` is released. The app also runs from source; CI, release and security
workflows are green. See the [roadmap](docs/ROADMAP.md) for what comes next.

## Download

[v0.1.0 release](https://github.com/alevidals/peekr/releases/tag/v0.1.0):

| Platform        | Portable                                    | Installer                                    |
| --------------- | ------------------------------------------- | -------------------------------------------- |
| macOS universal | `peekr_0.1.0_macos-universal_portable.zip`  | `peekr_0.1.0_macos-universal.dmg`            |
| Windows x64     | `peekr_0.1.0_windows-x64_portable.zip`      | `peekr_0.1.0_windows-x64-setup.exe`          |
| Linux x64       | `peekr_0.1.0_linux-x64.AppImage`            | `peekr_0.1.0_linux-x64.deb`                  |
| Linux ARM64     | `peekr_0.1.0_linux-arm64.AppImage`          | `peekr_0.1.0_linux-arm64.deb`                |

Every release includes `SHA256SUMS`.

> **Unsigned builds.** There is no Apple Developer ID or Windows code-signing
> certificate. macOS is ad-hoc signed and Gatekeeper asks on first launch;
> Windows shows a SmartScreen warning. [SECURITY.md](SECURITY.md) explains the
> trade-off, and [docs/RELEASE.md](docs/RELEASE.md) has the exact steps.

## Run it

Prerequisites: [Bun](https://bun.sh) 1.4.4+, Rust 1.99.1+ and the
[Tauri 2 platform dependencies](https://v2.tauri.app/start/prerequisites/).

```bash
bun install
bun run tauri dev      # desktop app with hot reload
```

On first launch, click the tray icon, open **Settings**, and add at least one
account. The [user guide](docs/USER_GUIDE.md#add-an-account) explains which
token scopes Bitbucket and GitHub need.

To build the bundles for the current platform:

```bash
bun run tauri build
```

Artifacts land in `src-tauri/target/release/bundle/`. Builds are not Developer ID
signed or notarized; the release workflow applies an ad-hoc macOS signature and
publishes checksums with every draft. Read [SECURITY.md](SECURITY.md) before
distributing one.

## Documentation

- [User guide](docs/USER_GUIDE.md): install, accounts, notifications and troubleshooting
- [Architecture](docs/ARCHITECTURE.md): layers, polling pipeline, IPC contract and security model
- [Contributing](CONTRIBUTING.md): setup, branches, checks and pull requests
- [Release and packaging](docs/RELEASE.md): CI, artifacts, versioning and signing
- [Dependency evaluation](docs/DEPENDENCIES.md): what is used and why
- [Architecture decision records](docs/adr/README.md): the decisions behind the stack
- [Roadmap](docs/ROADMAP.md) and [progress log](docs/PROGRESS.md)
- [Changelog](CHANGELOG.md): what changed in each release

The full index is in [docs/README.md](docs/README.md).

## Development

```bash
bun install
bun run tauri dev      # app with hot reload
bun run check          # typecheck, ESLint, Rust gates and production build
```

Individual gates:

```bash
bun run typecheck
bun run lint
bun run build
bun run rust:lint      # rustfmt + clippy with warnings as errors
bun run rust:test
```

The architecture and test strategy are described in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Manual acceptance steps live in
[QA-CHECKLIST.md](QA-CHECKLIST.md).

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) for the
development setup, branch workflow and checks. By participating, you agree to
follow the [Code of Conduct](CODE_OF_CONDUCT.md). Report security issues through
[SECURITY.md](SECURITY.md), never in a public issue.

## Stack

Tauri 2 · React 19 · TypeScript · Vite · Rust · Tokio · reqwest · keyring ·
`tauri-plugin-positioner` · `tauri-plugin-single-instance` · `tauri-plugin-log` ·
`tauri-nspanel` · `mac-notification-sys` / `notify-rust` · ESLint · Vitest

## License

MIT. See [LICENSE](LICENSE). Security reports: [SECURITY.md](SECURITY.md).

---

Made by [alevidals](https://github.com/alevidals).
