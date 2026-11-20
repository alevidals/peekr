# Release and packaging

peekr ships as a Tauri 2 desktop app. Bundles are produced per operating system;
Tauri does not cross-compile installers.

**Status:** Implemented and validated. CI is green, the manual release run
produced the expected artifacts, and `v0.2.0` is published. macOS builds are
ad-hoc signed; there is no Developer ID or notarization.

## Continuous integration

The CI workflow (`.github/workflows/ci.yml`) runs on every push to `main`, every
pull request and on demand:

- **Frontend**: version consistency, typecheck, ESLint, Vitest and a production
  build.
- **Rust (Linux)**: `rustfmt`, clippy with `-D warnings`, `cargo test`, and a
  check that no extra binary target exists (Tauri's universal macOS bundler only
  `lipo`s the main binary). The Tauri system dependencies are installed first.
- **Rust check matrix**: `cargo check --all-targets` on Ubuntu, macOS and
  Windows. This catches platform-specific code and dependency problems without
  bundling anything.
- **Version consistency**: `bun run version:check` reads `package.json`,
  `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`; on a tag it also
  compares the tag to the app version.

Provider mapping and polling logic are covered by Rust tests, not by network
calls in CI. Frontend tests run in jsdom; native window, tray, keychain and
notification behaviour stay in the manual checklist.

`.github/workflows/security.yml` runs `bun audit --production` and `cargo audit`
weekly and when a lockfile or the audit configuration changes. Accepted Rust
advisories live in `src-tauri/.cargo/audit.toml`.

## Cache budget

GitHub gives a repository 10 GB of Actions cache and evicts the least recently
used entries above that. The Rust toolchain cache stores the whole `target`
directory:

- Release builds do not use the Rust cache. They run once per tag and start
  cold.
- CI restores the cache on `main`, but pull requests never write it.
- If the budget looks wrong, delete the caches and let the next run rebuild.

## Downloadable builds

The release workflow builds on every `v*` tag. A manual dispatch builds the same
artifacts without publishing a release, which is useful for validating the
pipeline.

Every build job runs `bun run version:check`, so the tag matches:

1. `package.json`
2. `src-tauri/Cargo.toml`
3. `src-tauri/tauri.conf.json`

| System          | Portable (no install)                          | Installer         |
| --------------- | ---------------------------------------------- | ----------------- |
| macOS universal | Zipped `.app`                                  | `.dmg`            |
| Windows x64     | Raw `.exe`                                     | NSIS `-setup.exe` |
| Linux x64       | `.AppImage`                                    | `.deb`            |
| Linux ARM64     | `.AppImage`                                    | `.deb`            |

A final job writes `SHA256SUMS` and creates a **draft** GitHub release with all
artifacts. Review it and publish manually.

Portable caveats:

- **Windows**: the raw `.exe` needs the WebView2 runtime, which ships with
  Windows 10 (1803+) and Windows 11. It does not register an uninstaller.
- **macOS**: the `.app` is ad-hoc signed, so Apple Silicon runs it, but Gatekeeper
  asks the user to allow an app from an unknown developer. Real signing and
  notarization need a Developer ID certificate.
- **Linux**: the AppImage may need FUSE 2 (or `--appimage-extract-and-run`) and
  the WebKitGTK libraries provided by the distribution.

## Troubleshooting first launch

### macOS: "cannot be opened because the developer cannot be verified"

The app is not notarized, so Gatekeeper quarantines the download. Pick either
path:

1. **Finder**: right-click `peekr.app` → **Open** → **Open**. macOS remembers the
   choice.
2. **Settings**: System Settings → Privacy & Security → scroll to the blocked-app
   notice → **Open Anyway**.
3. **Terminal** after copying the app to `/Applications`:

   ```sh
   xattr -dr com.apple.quarantine /Applications/peekr.app
   ```

"The app is damaged and should be moved to the Trash" is the same quarantine
flag, not a corrupt binary; the `xattr` command fixes it.

### Windows: the portable `.exe` opens a blank window

Install the Microsoft Edge WebView2 Runtime (already present on Windows 10 1803+
and Windows 11). The NSIS setup installs it when missing.

### Linux: the AppImage does not start

Install FUSE 2 (`libfuse2` on Debian/Ubuntu, `fuse2` on Fedora) or run the image
with `--appimage-extract-and-run`. WebKitGTK 4.1 must be installed by the
distribution; a missing `libwebkit2gtk-4.1.so.0` is the usual cause.

## Unsigned (ad-hoc) distribution

There is no Apple Developer ID certificate, no Apple Developer account and no
Windows code-signing certificate, so **no release is signed for distribution**
yet. This is a constraint of the current project, not a bug.

The macOS workflow applies an **ad-hoc** signature (`APPLE_SIGNING_IDENTITY:
"-"`). That requires no Apple account and lets Apple Silicon run the binary, but
it is not Developer ID signing and does not notarize the app. Gatekeeper still
warns on first launch. The same distinction applies to Windows: the installer is
not code-signed.

- `README.md` explains that builds are not Developer ID signed or notarized.
- `SECURITY.md` states the trade-off and points at checksums.
- Release notes repeat it.

What users see:

| Platform | Effect                                                                         | Workaround                                          |
| -------- | ------------------------------------------------------------------------------ | --------------------------------------------------- |
| macOS    | Gatekeeper blocks the first launch; an ad-hoc signature keeps Apple Silicon happy | Right-click → Open, or clear `com.apple.quarantine` |
| Windows  | SmartScreen warns until the download gains reputation                          | "More info" → "Run anyway"                          |

Integrity comes from `SHA256SUMS`, generated per release. The signing sections
below describe the procedure for when a certificate exists; they are not used by
the current release workflow, which applies an ad-hoc signature only.

## Version bump

Update the version in all three files, move the `[Unreleased]` section of
`CHANGELOG.md` under the new version, then verify:

1. `package.json`
2. `src-tauri/Cargo.toml`
3. `src-tauri/tauri.conf.json`
4. `CHANGELOG.md`

```bash
bun run typecheck
bun run lint
bun run rust:test
bun run tauri build    # local smoke test
```

The version-check script fails when one file disagrees. Run it before tagging:

```bash
bun run version:check
```

## Branching and maintenance

`main` is the stable line. Fixes branch from `main`, target `main`, and each
release is tagged `v0.x.y`. A future major line can live on a long-lived branch;
until then, keep `main` releasable.

- Do not mix a release commit with a feature.
- Merge dependency updates deliberately and run `bun run check`.
- Move release notes from `Unreleased` before tagging.

## Local unsigned build

```bash
bun run check                 # typecheck, ESLint, Rust gates, production build
bun run tauri build --bundles app   # macOS .app only, fastest smoke test
bun run tauri build                 # every bundle target for this OS
```

Artifacts land in `src-tauri/target/release/bundle/`:

- macOS: `macos/peekr.app`, `dmg/peekr_<version>_<arch>.dmg`
- Linux: `deb/`, `rpm/`, `appimage/`
- Windows: `msi/`, `nsis/`

An unsigned macOS build runs locally but Gatekeeper blocks it on other machines.
Treat it as a smoke test, not a release.

## Optional: macOS signing and notarization (not active)

> **Not active.** This requires a paid Apple Developer account, a Developer ID
> Application certificate and an app-specific password. peekr does not have
> them today, so the release workflow keeps the ad-hoc signature. The steps below
> are the procedure for when that changes; they are not part of the current
> release.

Prerequisites: an Apple Developer ID Application certificate in the login
keychain and an app-specific password for the Apple ID.

```sh
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
export APPLE_ID="you@example.com"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="TEAMID"

bun run tauri build
```

Tauri signs the app and submits the DMG for notarization. Enable the hardened
runtime in the Tauri configuration when signing is set up. Keychain access for
saved tokens needs no extra entitlement because the app is not sandboxed; if
sandboxing is added later, add `keychain-access-groups` to an entitlements file
and reference it from `bundle.macOS.entitlements`.

For CI, the equivalent inputs are repository secrets. Tauri imports a base64
`.p12` certificate and uses the identity and notarization credentials:

| Secret                         | Purpose                                      |
| ------------------------------ | -------------------------------------------- |
| `APPLE_CERTIFICATE`            | Base64-encoded Developer ID `.p12`           |
| `APPLE_CERTIFICATE_PASSWORD`   | Password for that `.p12`                     |
| `APPLE_SIGNING_IDENTITY`       | `Developer ID Application: … (TEAMID)`       |
| `APPLE_ID`                     | Apple ID email                               |
| `APPLE_PASSWORD`               | App-specific password                        |
| `APPLE_TEAM_ID`                | Apple Developer team id                      |

The macOS build step must receive those secrets and stop hardcoding `-` before
notarization can run in CI.

Verify a finished artifact:

```sh
codesign --verify --deep --strict --verbose=2 "src-tauri/target/release/bundle/macos/peekr.app"
spctl --assess --type execute --verbose=2 "src-tauri/target/release/bundle/macos/peekr.app"
xcrun stapler validate "src-tauri/target/release/bundle/dmg/peekr_<version>_<arch>.dmg"
```

## Optional: Windows signing (not active)

> **Not active.** This requires a Windows code-signing certificate. peekr does
> not have one today.

Set a certificate thumbprint in `bundle.windows.certificateThumbprint`, or use a
custom `bundle.windows.signCommand`. The default WebView2 install mode is
`downloadBootstrapper`, so the installer fetches the runtime when it is missing.
Build on Windows; the MSI and NSIS installers come from the same command.

## Landing page

`web/` holds the static Astro landing page. It is a separate Bun project with
its own `bun.lock`, outside the desktop dependency tree, so `tauri build` never
sees it.

```sh
bun run web:dev      # local dev server
bun run web:check    # astro check
bun run web:build    # static output in web/dist
```

The newest published release is resolved at build time. Set `GITHUB_TOKEN` to
read a private repository; without it the page still builds and links to the
`/releases/latest` redirect. For public download links the repository and its
release assets must be public.

`SITE_URL` and `SITE_BASE` make the same build deployable anywhere:

| Host                                | `SITE_URL`                    | `SITE_BASE` |
| ----------------------------------- | ----------------------------- | ----------- |
| GitHub Pages project site (default) | `https://alevidals.github.io` | `/peekr`    |
| Custom domain, Vercel, Cloudflare   | `https://peekr.app`           | `/`         |

### GitHub Pages

`.github/workflows/web.yml` builds on changes under `web/`, on a published
release and on demand, then deploys with `actions/deploy-pages`. One-time setup:
**Settings → Pages → Build and deployment → Source: GitHub Actions**. For a
custom domain, add it there and set `SITE_BASE=/` plus `SITE_URL` to the final
domain.

The `github-pages` environment must also allow **tag** deployments, not only
`main`: publishing a release triggers a rebuild so the download links move to
the new version. Add a tag rule (`v*`) in
**Settings → Environments → github-pages → Deployment branches and tags**.

Without that rule the build succeeds and the deploy fails with
`Tag "vX.Y.Z" is not allowed to deploy to github-pages`.

## Linux packages

Build on the target distribution. The Debian package is filed under the `devel`
section; Tauri derives runtime dependencies automatically. AppImage and RPM
targets need their usual host tooling (`linuxdeploy`, `rpmbuild`) available.

## Release checklist

- [ ] Version bumped in the three files and `CHANGELOG.md` updated.
- [ ] `bun run check` passes.
- [ ] [QA-CHECKLIST.md](../QA-CHECKLIST.md) walked on macOS, Windows and Linux.
- [ ] Installer built on each target OS.
- [ ] Artifact starts, shows the tray icon and polls a real account.
- [ ] A token survives a restart through the OS keychain.
- [ ] `config.json` and logs inspected for secrets.
- [ ] macOS artifact signed and notarized if a certificate exists.
- [ ] `SHA256SUMS` generated and attached.
- [ ] Release notes repeat the unsigned-build caveat.
