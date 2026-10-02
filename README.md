# peekr

Tray app (Tauri 2 + React) that shows the pull requests waiting for your review, with a badge on the tray icon and a notification when a new one arrives.

Supported providers: **Bitbucket Cloud** and **GitHub**. Several accounts of either can be configured.

Only PRs still waiting on you are listed: Bitbucket PRs you already approved are hidden, and GitHub drops a PR from `review-requested:@me` once you submit a review (requests to your teams are included).

## Development

```sh
pnpm install
pnpm tauri dev      # dev mode
pnpm tauri build    # production bundle
cargo test --manifest-path src-tauri/Cargo.toml
```

### My PRs

The "My PRs" tab lists your own open PRs and whether each could be merged now, and notifies you when one becomes ready or its checks fail.
- **GitHub**: uses GitHub's own verdict (`mergeStateStatus`), so required checks, required reviews, conflicts and "branch behind" follow the branch protection. The token must be able to read PRs and checks via GraphQL.
- **Bitbucket**: no merge verdict exists in the API and merge rules need admin access to read, so "Looks ready" is inferred from build statuses, approvals, requested changes and open tasks. Merge conflicts are not detected.

## Architecture

```
src-tauri/src/
  domain.rs      provider-agnostic PullRequest / ProviderKind
  provider/      ReviewProvider trait + one module per service (bitbucket.rs)
  config.rs      accounts + polling interval (JSON, with migrations)
  secrets.rs     one keychain entry per account
  state.rs       in-memory config/token snapshot, PR list, poll trigger
  poller.rs      background loop: fetch all accounts → state, event, badge, notifications
  notifier.rs    "what's new" diff + native notification
  popover.rs     show/hide behaviour of the tray popover
  tray/          tray icon, badge rendering
  commands.rs    Tauri commands exposed to the frontend
src/
  api.ts         the only place that knows command/event names
  providers.tsx  per-provider UI copy
  windows/       PopoverWindow, SettingsWindow
  components/ hooks/
```

### Adding a provider

1. Implement `ReviewProvider` in `src-tauri/src/provider/<name>.rs` (map API payloads to `PullRequest`).
2. Add it to `ProviderKind` and to the `match` in `provider::build`.
3. Add its entry to `src/providers.tsx` and the `ProviderKind` type.

Nothing else (poller, tray, notifications, list UI) needs to change. `provider/github.rs` is a compact example.

---

Made by [alevidals](https://github.com/alevidals).
