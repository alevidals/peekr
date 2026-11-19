# peekr: Dependency evaluation

**Status:** Implemented. Versions verified on 2026-11-19 against `Cargo.lock` and
`package.json`.
**Policy:** Pin direct versions where the API is sensitive, commit both
lockfiles, and review upgrades deliberately. Every dependency must earn its
place: it is maintained, permissively licensed, cross-platform where the app is,
and does not require a system binary at runtime.

peekr is deliberately small. It has no UI component library, no state manager,
no router and no database. That keeps the dependency surface close to the
platform and the APIs that matter: HTTP, the keychain, the tray and the two
windows.

---

## 1. Evaluation criteria

| Criterion      | What we check                                                                                   |
| -------------- | ----------------------------------------------------------------------------------------------- |
| Maintenance    | Recent releases, responsive issue tracker, release cadence                                       |
| Security       | Memory-safe where possible; TLS maturity; no unnecessary privilege                              |
| Capability fit | The exact APIs needed: GraphQL, REST, OS keychain, tray badge, native panel, notifications       |
| Async fit      | Works with Tokio without blocking the runtime; supports timeouts and cancellation                |
| Portability    | macOS, Windows and Linux; platform-specific code isolated behind `cfg`                           |
| License        | MIT / Apache-2.0 / BSD / ISC; compatible with an MIT desktop app                                 |
| Cost           | Binary size, compile time, transitive surface and runtime overhead                               |

---

## 2. Core stack

| Layer            | Choice                                | Why                                                                 |
| ---------------- | ------------------------------------- | ------------------------------------------------------------------- |
| Desktop shell    | Tauri 2                               | Small footprint, native tray/window APIs, Rust backend in one process |
| UI               | React 19 + TypeScript 6 + Vite 8      | Mature ecosystem; fast HMR; strict types at the IPC edge             |
| Styling          | Plain CSS with variables              | The UI is one list and two windows; a utility framework adds weight  |
| Networking       | `reqwest` with rustls                 | One client for HTTP/2, JSON, timeouts and a shared pool              |
| Async runtime    | Tokio                                 | Tauri already runs on it; the poller is a Tokio task                 |
| Credentials      | OS keychain via `keyring`             | No plaintext secrets and no custom cryptography                      |
| Native tray      | Tauri tray APIs + `objc2` on macOS    | Template image, layer badge and a non-activating NSPanel             |
| Notifications    | `mac-notification-sys` / `notify-rust`| Small platform wrappers with click handling                          |
| Config           | serde + JSON + atomic rename          | A few kilobytes of versioned settings; SQLite would be overkill      |

---

## 3. Rust backend dependencies

| Crate                       | Version   | Purpose                          | License        | Notes                                                                 |
| --------------------------- | --------- | -------------------------------- | -------------- | --------------------------------------------------------------------- |
| `tauri`                     | 2.12.7    | Desktop shell, tray, windows     | MIT/Apache-2.0 | Features: tray-icon, image-png, macOS private API                     |
| `tauri-build`               | 2.7.6     | Build script                     | MIT/Apache-2.0 | Generates context and permissions                                     |
| `tauri-plugin-opener`       | 2.7.3     | Open PR URLs and log folder      | MIT/Apache-2.0 | Only provider-supplied URLs                                           |
| `tauri-plugin-positioner`   | 2.4.0     | Position the popover under tray  | MIT/Apache-2.0 | `tray-icon` feature                                                   |
| `tauri-plugin-autostart`    | 2.7.0     | Launch at login                  | MIT/Apache-2.0 | LaunchAgent on macOS, registry on Windows, XDG entry on Linux          |
| `tauri-plugin-single-instance` | 2.5.5  | Keep one tray process            | MIT/Apache-2.0 | A second launch shows the existing popover                            |
| `tauri-plugin-log`          | 2.10.2    | File and stdout logging          | MIT/Apache-2.0 | Rotates one 2 MiB file in the platform log directory                  |
| `tokio`                     | 1.54.1    | Async runtime                    | MIT            | Features: rt, rt-multi-thread, time, sync, macros                     |
| `reqwest`                   | 0.12.28   | Bitbucket REST and GitHub GraphQL| MIT/Apache-2.0 | `json` + `rustls-tls`, default features off                            |
| `serde`                     | 1.0.229   | Serialization                    | MIT/Apache-2.0 | derive                                                                |
| `serde_json`                | 1.0.154   | Config and API payloads          | MIT/Apache-2.0 | Typed decoding; error-body snippet cap                                |
| `thiserror`                 | 2.0.21    | `AppError`                       | MIT/Apache-2.0 | Converted to a readable string at the IPC boundary                     |
| `keyring`                   | 4.2.0     | OS credential storage            | MIT/Apache-2.0 | Pinned exactly; macOS Keychain, Windows Credential Manager, Secret Service |
| `uuid`                      | 1.29.3    | Account ids                      | MIT/Apache-2.0 | v4                                                                    |
| `futures`                   | 0.3.34    | Concurrent provider fan-out      | MIT/Apache-2.0 | `join_all` and bounded streams                                         |
| `async-trait`               | 0.1.92    | Object-safe `ReviewProvider`     | MIT/Apache-2.0 | `Arc<dyn ReviewProvider>`                                             |
| `log`                       | 0.4.35    | Logging facade                   | MIT/Apache-2.0 | No token or full response logging                                      |

### macOS-only

| Crate                | Version | Purpose                                     | Notes                                                                    |
| -------------------- | ------- | ------------------------------------------- | ------------------------------------------------------------------------ |
| `tauri-nspanel`      | 2.1.1   | Convert the popover into an `NSPanel`       | Non-activating panel, collection behavior                                 |
| `objc2`              | 0.6     | Objective-C runtime access                  | Replaces the unmaintained `objc` 0.2                                      |
| `objc2-app-kit`      | 0.3     | `NSWorkspace`, `NSStatusItem`, `NSWindow`   | Space observer, status item button, layer host                            |
| `objc2-foundation`   | 0.3     | `NSString`, `NSObject`, notifications       |                                                                          |
| `objc2-quartz-core`  | 0.3     | `CALayer` badge overlays                    | Draws the red dot and dev marker without baking them into the image       |
| `objc2-core-foundation` | 0.3  | `CGPoint`, `CGRect`, `CGSize`               |                                                                          |
| `objc2-core-graphics`| 0.3     | `CGColor`                                   |                                                                          |
| `mac-notification-sys`| 0.6.15 | Native macOS notifications                  | Waits for a click; uses the app identifier in packaged builds              |

### Windows/Linux

| Crate         | Version | Purpose                        | Notes                                                    |
| ------------- | ------- | ------------------------------ | -------------------------------------------------------- |
| `notify-rust` | 4.19.0  | Native notifications           | App id on Windows, explicit action on Linux              |
| `image`       | 0.25.10 | Decode the tray PNG            | PNG only; no codec zoo                                   |
| `imageproc`   | 0.25.1  | Draw the runtime badge dot     | Circles only                                             |

`image` and `imageproc` are deliberately excluded on macOS, where the template
image and `CALayer` overlays do the work.

**Dev dependencies:** none beyond the standard Rust toolchain. Tests use
`#[cfg(test)]` modules and serde fixtures; there is no mocking framework.

---

## 4. Frontend dependencies

| Package                          | Version   | Purpose                     | Notes                                                       |
| -------------------------------- | --------- | --------------------------- | ----------------------------------------------------------- |
| `react`, `react-dom`             | ^19.1.0   | UI runtime                  | Lazy-loaded windows                                          |
| `typescript`                     | ~6.0.3    | Language                    | Strict checking at build time                                |
| `vite`                           | ^8.0.16   | Build and dev server        | Tauri dev URL on `localhost:1420`                            |
| `@vitejs/plugin-react`           | ^6.0.2    | React transform             |                                                              |
| `@tauri-apps/api`                | ^2        | IPC, events, windows        | `invoke`, `listen`, window label and focus                   |
| `@tauri-apps/plugin-opener`      | ^2        | Open URLs                   | Settings token pages and PR links                            |
| `@tauri-apps/plugin-autostart`   | ^2.6.0    | Launch at login             | Settings toggle                                              |
| `eslint`                         | ^10.15.0  | Lint                        | Flat config                                                  |
| `@eslint/js`                     | ^10.0.1   | Recommended JS rules        |                                                              |
| `typescript-eslint`              | ^8.73.0   | Type-aware TypeScript rules |                                                              |
| `eslint-plugin-react-hooks`      | ^7.1.1    | Hooks rules                 | Catches stale closures and dependency mistakes               |
| `globals`                        | ^17.14.0  | Browser globals             |                                                              |
| `@types/node`                    | ^26.6.6   | Vite config types           |                                                              |
| `@types/react`, `@types/react-dom` | ^19.1.8 / ^19.1.6 | React types      |                                                              |
| `@tauri-apps/cli`                | ^2        | `tauri dev`, `tauri build`  |                                                              |
| `vitest`                         | ^5.1.1    | Test runner                 | jsdom environment                                            |
| `jsdom`                          | ^31.0.1   | DOM environment             | For Testing Library                                          |
| `@testing-library/react`         | ^16.3.3   | Component rendering         | Dialog and list tests                                        |
| `@testing-library/jest-dom`      | ^8.0.0    | DOM matchers                | `toBeInTheDocument`, `toHaveFocus`                           |
| `@testing-library/user-event`    | ^14.6.13  | Interaction tests           | Keyboard and click flows                                     |

**Not included intentionally:**

| Candidate               | Why not                                                                                 |
| ----------------------- | --------------------------------------------------------------------------------------- |
| Tailwind CSS            | A small set of CSS files with variables is smaller and easier to reason about for two windows |
| shadcn/ui / Radix       | The app has no dialogs, comboboxes or menus that would justify a component layer          |
| Zustand / TanStack Query| State is window-local and event-driven; `useState` + `usePullRequests` covers it          |
| React Router            | Two fixed windows, selected by label                                                     |
| date-fns / Intl wrapper | The list shows provider timestamps and a relative "last updated"; `Intl` is enough       |
| Icon packs              | The handful of icons are inline SVG components                                           |

---

## 5. Rejected, deferred and platform choices

| Candidate                    | Decision | Reason                                                                                                     |
| ---------------------------- | -------- | ---------------------------------------------------------------------------------------------------------- |
| `tauri-plugin-store`         | Rejected | No schema versioning, migrations or atomic write control. peekr needs a validated `config.json`.            |
| `tauri-plugin-http`          | Rejected | Would expose request building to the webview. Native `reqwest` keeps tokens in Rust.                        |
| `tauri-plugin-fs` / `shell`  | Rejected | peekr does not read files or run commands. No reason to grant the capability.                               |
| `tauri-specta` / codegen     | Deferred | The command surface is small and stable. Adding codegen introduces a pre-release dependency and a build step. |
| `rusqlite`                   | Deferred | The app stores a few kilobytes of settings and no history. JSON is enough.                                  |
| `keyring` older majors       | Rejected | `4.2.0` is pinned because the API changed; upgrades need a deliberate pass.                                 |
| `objc` 0.2                   | Replaced | Unmaintained; `objc2`/`tauri-nspanel` are the maintained path.                                              |
| `native-tls`                 | Rejected | rustls gives a consistent cross-platform TLS story and avoids an OpenSSL dependency.                        |
| `openssl` / system binaries  | Rejected | No mandatory system binaries; portable builds and predictable behaviour.                                    |
| Custom updater               | Deferred | No signed artifacts yet. Shipping an updater before signing would weaken the security story.                |
| Analytics / crash reporting  | Rejected | No telemetry, no remote config, no peekr account.                                                           |

---

## 6. Licensing and supply chain

- All direct dependencies are MIT, Apache-2.0, BSD, ISC or equivalent.
- `Cargo.lock` and `bun.lock` are committed. CI and local builds use the
  lockfiles.
- `keyring` and the Tauri crates are the highest-value upgrade targets; review
  their release notes before bumping.
- `cargo audit` and `bun audit --production` run weekly in
  `.github/workflows/security.yml` and when a lockfile or the audit configuration
  changes.
- No dependency runs a postinstall script that the project has not reviewed.
- No telemetry SDK, analytics SDK or remote configuration library is present.

---

## 7. Upgrade policy

1. Read the changelog and migration notes.
2. Bump the version in `Cargo.toml` or `package.json`, then let the lockfile
   update.
3. Run `bun run check` and `bun run tauri dev`.
4. Exercise the affected surface in [QA-CHECKLIST.md](../QA-CHECKLIST.md). For a
   Tauri or `keyring` upgrade that means the tray, both windows, the keychain and
   notifications.
5. Record non-obvious decisions in an ADR or the changelog.

Protocol-adjacent behaviour (Bitbucket/GitHub payloads) changes more often than
crate versions. Provider mapping tests are the guard there, not the dependency
version.
