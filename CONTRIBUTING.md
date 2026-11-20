# Contributing to peekr

Thanks for taking the time to improve peekr. Bug reports, fixes, tests,
documentation, provider integrations and theme work are welcome.

This project follows the [Code of Conduct](CODE_OF_CONDUCT.md). By
participating, you agree to uphold it.

## Before you open a pull request

- Open an issue first for a bug fix that touches more than a few lines, and for
  any new feature or provider. Agreeing on the scope avoids work that cannot be
  merged.
- Report security issues through GitHub's private vulnerability reporting, as
  described in [SECURITY.md](SECURITY.md). Do not open a public issue.
- Typo fixes and small documentation corrections can go straight to a pull
  request.

## Set up the project

Use the versions pinned in `.bun-version` and `rust-toolchain.toml`. You also
need the platform dependencies listed in the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```bash
git clone https://github.com/alevidals/peekr.git
cd peekr
bun install
bun run tauri dev
```

The app is a tray application: after it starts, click the tray icon and open
**Settings** to add an account. The [user guide](docs/USER_GUIDE.md) has the
token scopes each provider needs.

## Branches

`main` is the stable line. Fixes and features branch from `main` and target
`main`:

```bash
git switch -c fix/notification-dedupe
```

Keep a pull request focused. Do not mix a bug fix, a refactor and a new provider
in the same change.

## Checks

Run the checks that cover your change before you open a pull request:

```bash
bun run check          # typecheck, ESLint, Rust gates and production build
```

Individual gates:

```bash
bun run typecheck
bun run lint
bun run build
bun run rust:check
bun run rust:lint      # rustfmt + clippy with warnings as errors
bun run rust:test
bun run version:check  # package.json, Cargo.toml and tauri.conf.json
```

`bun run check` already runs the Rust lint and test gates. CI runs the same
frontend and Rust gates, plus a `cargo check --all-targets` matrix on Ubuntu,
macOS and Windows. If you changed the native layer, also launch the app with
`bun run tauri dev` and walk the relevant part of
[QA-CHECKLIST.md](QA-CHECKLIST.md).

The popover can be inspected outside the desktop shell:

```bash
bun run dev:gallery   # theme gallery at http://localhost:1421/preview.html
```

The landing page is a separate Bun project under `web/`:

```bash
bun run web:dev      # local Astro dev server
bun run web:check    # astro check
bun run web:build    # static output in web/dist
```

It is built and deployed by `.github/workflows/web.yml`; changes under `web/` do
not run the desktop gates.

## Code style

- TypeScript and TSX are formatted and linted by ESLint with the React hooks
  rules. Rust uses rustfmt and clippy with warnings as errors.
- Keep changes focused. A pull request that fixes a bug should not also rename
  unrelated files.
- UI changes must work in the light, dark and named themes, keep a visible focus
  ring, and remain reachable by keyboard.
- Add a test for behavior changes. Rust domain, provider, polling and popover
  logic belongs in a `#[cfg(test)]` module next to the code. Bug fixes should
  include a test that fails before the fix when the behavior can be tested.
- Do not log access tokens or response bodies that may contain credentials. The
  HTTP layer already trims error bodies to a short, safe snippet.
- Use conventional commit prefixes such as `feat:`, `fix:`, `docs:`, `test:` and
  `chore:`.

## Provider changes

Every provider implements `ReviewProvider` in `src-tauri/src/provider/` and
maps its payloads to the shared `PullRequest` domain type. Adding or changing a
provider usually touches:

1. The provider module (`provider/<name>.rs`) and its unit tests.
2. `ProviderKind` in `domain.rs` and the `match` in `provider::build`.
3. `src/providers.tsx` for the settings copy and token scopes.
4. `src/types.ts` if the wire contract changes.

The poller, tray, notifier and list UI should not need provider-specific
branches.

## Pull requests

- Describe the problem and the behavior change, not only the files touched.
- Include a screenshot or recording for UI changes, in light and dark mode.
- List the checks you ran.
- Keep the pull request in draft until the checks pass if you want early
  feedback.

Maintainers review for correctness, security and scope. If a change is out of
scope, they will explain why.

## Reporting bugs

Include the peekr version, operating system, provider (Bitbucket or GitHub), the
account setup involved and the steps to reproduce. Redact tokens, workspace
names, repository names and private paths.

## License

By contributing, you agree that your contribution is licensed under the MIT
license in [LICENSE](LICENSE).
