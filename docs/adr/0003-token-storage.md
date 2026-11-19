# ADR-0003: Access-token storage

- **Status:** Accepted
- **Date:** 2026-11-18

## Context

peekr stores Bitbucket and GitHub access tokens for every configured account.
Security requirements forbid plaintext secrets in configuration or logs, and the
webview must never be able to read a stored token back.

## Decision

- Store tokens **only** in the OS credential store through `keyring 4.2.0`:
  - macOS Keychain
  - Windows Credential Manager
  - Linux Secret Service (D-Bus)
- Use the service name `peekr` and one item per account: `token:<account-id>`.
- Keep only non-secret metadata in `config.json`: account id, provider,
  workspace and repositories.
- Never expose a "get token" command. `get_config` reports `has_token` per
  account; tokens are read into Rust memory only to build a provider.
- Accept a typed token for validation and save. An empty input keeps the stored
  token; removing an account deletes its keychain entry.
- Write tokens **before** the config that references them. If the config write
  fails, delete tokens for accounts that never reached the disk. Delete removed
  accounts' tokens only after the new config is on disk.
- Migrate the v1 single-token format from the old keychain entry or the old JSON
  field, store it under the new account id, and delete the legacy entry.
- If the keychain is unavailable, fail with a clear error. **No plaintext or
  obfuscated fallback.**
- Logs include account labels and provider errors, never tokens. Error bodies are
  reduced to one short line before they are shown or logged.

## Consequences

**Positive**

- Secrets are protected by OS access control and unlocked with the user's login.
- No custom cryptography or master password to lose.
- The webview never receives credentials after submission.
- Token/config ordering cannot leave a config that references a missing token.

**Negative / accepted**

- Linux needs a Secret Service provider; headless environments may not support
  saving tokens.
- Keychain access can prompt the user, especially after a binary update.
- The keychain can drift if the app crashes mid-save; orphan entries are
  harmless and can be cleaned manually.
- Rust cannot guarantee that every copied string is zeroized.

## Alternatives considered

- **Encrypted file with an app-managed key:** the key has to live somewhere;
  weaker than the OS store.
- **Plain config or base64:** unacceptable.
- **Session-only tokens:** poor usability for a background tray app; offered only
  as a consequence of an unavailable keychain, never as a silent fallback.
- **Stronghold vault with a master password:** viable as an explicit Linux
  fallback, but adds a second unlock UX. Deferred, not used silently.
