# Security policy

## Supported versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |

The current development line receives fixes. There is no supported release older
than 0.1.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting: **Security → Report a
vulnerability** in this repository. Do not open a public issue for a security
problem.

Please include the affected version, the platform, the provider involved
(Bitbucket or GitHub) and reproduction steps. You can expect an acknowledgement
within a few days.

## Scope

In scope:

- Access-token handling and OS keychain storage.
- The IPC boundary between the Rust backend and the webview.
- Provider requests, URL handling and anything that could send a token to the
  wrong host or leak it into logs, errors or crash output.
- Notification handling and opener calls that follow URLs from an API payload.
- The Content-Security-Policy and Tauri capability set.

Out of scope:

- The absence of an OS code-signing certificate (see below).
- Vulnerabilities in Bitbucket, GitHub or the user's network.
- Anything that requires an already-compromised machine.

## Unsigned builds

There is no Apple Developer ID certificate and no Windows code-signing
certificate yet. As a result:

- macOS builds are ad-hoc signed at best, so Apple Silicon runs them, but
  Gatekeeper asks for confirmation on first launch.
- Windows builds trigger SmartScreen until the file build gains reputation.

Release infrastructure will publish checksums alongside the artifacts. Until
then, build from source or verify a binary you received against the source tree
before running it.

## Secrets

Access tokens are stored in the OS credential store, one entry per account.
`config.json` contains only workspace names, repository lists, polling settings
and appearance. The frontend never receives a stored token: `get_config`
reports `has_token` per account instead.

Logs include account labels and provider errors, never tokens. Error bodies from
Bitbucket and GitHub are truncated to a short snippet before they reach the UI
or the log.

If you find a path that violates this, treat it as a security report.

## Dependency audits

`cargo audit` and `bun audit --production` run weekly and when a lockfile or the
audit configuration changes (`.github/workflows/security.yml`). Accepted
advisories are documented in `src-tauri/.cargo/audit.toml` with the reason and
the condition for removing them.
