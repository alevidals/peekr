# ADR-0001: Tauri 2 + React tray app stack

- **Status:** Accepted
- **Date:** 2026-11-18
- **Deciders:** peekr maintainers

## Context

peekr needs to sit in the tray, show native notifications, store access tokens
in the OS credential store, and render a compact list. It must build for macOS,
Windows and Linux with a small footprint. The repository started from the Tauri
2 + React + TypeScript + Vite template.

## Decision

Use:

- **Tauri 2** for the desktop shell, tray, window management and Rust↔webview
  IPC.
- **React 19 + TypeScript (strict) + Vite** for the two windows.
- **Plain CSS with CSS variables** for the design system.
- **Rust** for networking, keychain access, polling, config persistence, tray
  rendering and notifications.
- **`reqwest` + rustls** behind one shared client.
- **`keyring` 4.2.0** for tokens.
- **`tauri-plugin-opener`** for PR URLs and token pages, and
  **`tauri-plugin-positioner`** for tray placement.
- **`tauri-plugin-autostart`** for launch at login.

## Consequences

**Positive**

- Small runtime and memory footprint compared with Electron.
- Tokens and network requests stay in Rust; the webview never gets a stored
  token.
- Native tray, panel and notification APIs are available without sidecars.
- One TypeScript bundle serves both windows; lazy loading keeps the settings
  chunk out of the popover.

**Negative / accepted**

- WebView differences across platforms require real-device testing.
- Tauri capability and CSP configuration are security-critical and reviewed.
- Rust compile time is higher than a pure web app.
- Handwritten IPC types can drift (see ADR-0005).

## Alternatives considered

| Alternative        | Why rejected                                                                 |
| ------------------ | ---------------------------------------------------------------------------- |
| Electron + Node    | Larger footprint; tokens and networking in JS; weaker native isolation       |
| Native Swift/WinUI | Not cross-platform; double the work for a small utility                      |
| Browser extension  | No tray, no keychain, provider tokens exposed to the browser context         |
| Menu-bar-only CLI  | No list, search, themes or notifications worth using                          |

## Notes

- No Tauri `fs`, `shell` or `http` plugins. The webview gets only opener,
  positioner, autostart and minimal core window capabilities.
- The app is distributed outside app stores, which is why the macOS private API
  used by the panel is acceptable.
