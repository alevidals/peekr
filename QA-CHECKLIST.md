# peekr manual QA checklist

Use this list before tagging a release or after changing the tray, popover,
poller, notifications or provider code. Mark `[x]` for a pass. When something
fails, record the OS, version, provider, account setup, steps and expected
result.

**This round**

- Version tested:
- Operating system:
- Display scaling:
- Providers:
- Date:

---

## 1. Build and first launch

- [ ] `bun run check` passes on a clean checkout.
- [ ] `bun run tauri build` produces a bundle for the current platform.
- [ ] The app starts with no console panic and no missing-icon warning.
- [ ] No Dock/taskbar entry appears until Settings is opened (macOS accessory
      policy).
- [ ] The tray icon appears in the menu bar or system tray.
- [ ] A debug build shows the orange dev marker; a release build does not.
- [ ] Clicking the tray icon opens the popover under the icon.
- [ ] Clicking it again closes the popover.
- [ ] Launching peekr a second time focuses the existing popover instead of
      creating a second tray icon or a second polling process.
- [ ] With no accounts, the popover shows a first-run state with an **Open
      settings** action.

## 2. Accounts

### Bitbucket

- [ ] Add a workspace and one repository: the account saves and polls.
- [ ] Add several repositories, one per line: all are polled.
- [ ] Leaving the repository list empty is rejected with a clear message.
- [ ] A wrong token fails validation with an actionable error.
- [ ] A token without the required scopes produces a readable error, not a
      silent empty list.
- [ ] Saving without typing a token keeps the stored token.
- [ ] Removing a saved account asks for confirmation and explains that Save deletes its
      token; cancelling the dialog keeps the account.
- [ ] Removing an unsaved account asks to discard it, without mentioning a keychain token.
- [ ] Removing the account removes its keychain entry.

### GitHub

- [ ] Add an owner with no repositories: every repository is searched.
- [ ] Add explicit repositories: only those are polled.
- [ ] A fine-grained token with Pull requests (Read) works.
- [ ] CI status requires Checks (Read) and Commit statuses (Read); without them
      the PR still lists and CI is unknown rather than wrong.
- [ ] An organisation that enforces SSO shows a clear error until the token is
      authorised.
- [ ] Saving without typing a token keeps the stored token.

### Multiple accounts

- [ ] Two accounts of different providers coexist in the popover.
- [ ] Two accounts for the same workspace and provider are rejected.
- [ ] The same PR visible through two accounts is listed once and counted once.
- [ ] Deleting one account does not touch another account's token.

## 3. Tray and popover

- [ ] The badge count matches the number of PRs in the review tab.
- [ ] The tooltip says `1 PR to review` or `N PRs to review`.
- [ ] Hiding and re-opening the popover refreshes the list.
- [ ] `Esc` closes the popover.
- [ ] Clicking outside closes the popover.
- [ ] `/` opens search from anywhere in the popover.
- [ ] The popover does not steal focus from the previous app after closing.
- [ ] macOS: the popover opens over a full-screen app without switching Spaces.
- [ ] macOS: switching desktop hides the popover instead of dragging it along.
- [ ] macOS: the popover still receives keyboard input while the app stays in
      the background.
- [ ] Windows/Linux: the popover positions under the tray icon and stays on top
      of the app window only while open.

## 4. Review list

- [ ] Bitbucket PRs you already approved are hidden.
- [ ] GitHub PRs disappear after you submit a review.
- [ ] Team review requests appear when the token can see them.
- [ ] Repo groups are ordered consistently and show per-repo counts.
- [ ] Multiple providers show the provider label in each group.
- [ ] The provider icon matches the account.
- [ ] Approval count, comment count and CI state render for both providers.
- [ ] Draft PRs show the badge and can be hidden with the drafts filter.
- [ ] The "only passing" filter hides PRs whose checks are not green.
- [ ] Search matches title, author, repo, source branch, destination branch and
      `#number`; multiple terms are ANDed.
- [ ] Closing search clears the query immediately.
- [ ] A long list scrolls smoothly and typing stays responsive.
- [ ] Hovering a truncated title, branch or repo reveals the full text.
- [ ] The refresh button starts a poll and shows feedback while it runs.
- [ ] The footer shows the last update time.

## 5. My PRs

- [ ] The tab lists only the authenticated user's own open PRs.
- [ ] GitHub shows the provider's own merge verdict.
- [ ] GitHub marks `UNSTABLE` as ready with the non-required-checks caveat.
- [ ] GitHub shows conflicts, behind branch, draft, blocked and waiting states.
- [ ] Bitbucket infers "Looks ready to merge" from pipelines, approvals,
      requested changes and open tasks.
- [ ] Bitbucket states that merge conflicts are not detected.
- [ ] Blockers are listed and readable.
- [ ] A PR that becomes ready produces one notification.
- [ ] A provider that briefly flips the merge state between polls does not repeat
      the same notification.
- [ ] New activity on the PR allows a new readiness notification.
- [ ] A PR whose checks fail produces one notification.
- [ ] The same transition is not notified again on the next poll.

## 6. Notifications

- [ ] A new review request produces one notification per PR.
- [ ] The first successful poll of an account produces no notification flood.
- [ ] Clicking a notification opens the correct PR in the browser.
- [ ] Notification text uses provider vocabulary: checks vs pipelines.
- [ ] A PR that disappears for one poll and returns is not announced twice.
- [ ] Notifications appear with the app in the background.
- [ ] macOS: a packaged build is attributed to peekr, not Terminal.
- [ ] If notifications are denied by the OS, the app still works and the list
      updates.

## 7. Settings

- [ ] Settings opens as a normal window with a Dock/taskbar entry.
- [ ] Closing Settings returns the app to tray-only mode.
- [ ] Polling interval accepts 30–3600 seconds and clamps out-of-range input.
- [ ] Clearing the polling field does not save an invalid value.
- [ ] Launch at login can be enabled and disabled, and the change survives a
      restart.
- [ ] **Open logs** in About reveals the log directory and the current log is readable.
- [ ] The log contains no tokens and has a bounded size.
- [ ] **Check for updates** in About with the current version says it is up to date.
- [ ] With an older build than the latest release, it reports the newer version
      and the **Release page** button opens that release.
- [ ] A network failure shows an error and leaves the button enabled for retry.
- [ ] Theme, palette, accent, density and text size save.
- [ ] Tall forms scroll inside the window without pushing the footer off screen.
- [ ] Validation errors are visible and announced.
- [ ] Save reports success or a specific failure without closing the window.
- [ ] Re-opening the popover picks up the new settings.

## 8. Appearance

- [ ] System, light and dark apply without a flash of the wrong theme.
- [ ] One Dark, Gruvbox Dark, Gruvbox Light, Nord, Dracula, Solarized Light,
      Tokyo Night and Catppuccin Mocha all render readable text.
- [ ] A custom `#rrggbb` accent applies; an invalid value is discarded.
- [ ] Focus rings are visible on every theme with keyboard-only use.
- [ ] Comfortable and compact density do not clip rows.
- [ ] Small, medium and large text do not overflow badges or buttons.
- [ ] Long names, deep repository names and non-ASCII text truncate cleanly.
- [ ] `prefers-reduced-motion` disables animations.
- [ ] Icon-only buttons expose accessible names; tabs support arrow keys.
- [ ] Error banners and validation messages are announced as alerts/status.

## 9. Security and privacy

- [ ] `config.json` contains no token, password or secret.
- [ ] The keychain contains one entry per account under the `peekr` service.
- [ ] Deleting an account deletes its keychain entry.
- [ ] Logs contain no tokens, Authorization headers or full response bodies.
- [ ] The webview has no global Tauri object.
- [ ] The CSP blocks remote scripts.
- [ ] A provider error shows a short body snippet, not an entire HTML page.
- [ ] The opener only receives URLs that came from a provider payload.

## 10. Failure and recovery

- [ ] No network: the popover reports a readable network error and keeps the
      last known list.
- [ ] DNS failure, TLS failure and connection reset each produce a message that
      names the cause.
- [ ] A 401/403 explains that the token may be expired or missing scopes.
- [ ] A 429 or 5xx response backs off instead of polling at the normal rate.
- [ ] One failing repository does not hide the PRs from healthy repositories.
- [ ] A failed account keeps its previously known review PRs instead of showing
      them as closed.
- [ ] Recovering the network clears the error on the next successful poll.
- [ ] A corrupt `config.json` is backed up and the app starts with defaults.
- [ ] An unreadable keychain entry is reported in the log and does not crash.

## 11. Performance and stability

- [ ] Cold start feels immediate; the tray appears before the first poll ends.
- [ ] The first poll completes within the HTTP timeout (30 s).
- [ ] 50+ PRs across several repositories scroll and filter smoothly.
- [ ] Re-opening the popover does not remount the list unnecessarily.
- [ ] Typing in search with a long list does not drop characters.
- [ ] Leaving the app running for an hour keeps polling on schedule.
- [ ] Waking the laptop from sleep recovers polling without restarting.
- [ ] Memory stays flat after repeated open/close and poll cycles.
- [ ] Polling stops cleanly when every account is removed.

## 12. Platform specifics

### macOS

- [ ] Apple Silicon and Intel (or a universal build) launch.
- [ ] The tray icon is tinted by the menu bar and stays visible on light and
      dark wallpapers.
- [ ] The red badge and orange dev marker keep their colours.
- [ ] The app stays out of the Dock while Settings is closed.
- [ ] Keychain entries appear under the expected service name.
- [ ] Notifications are attributed to peekr in a packaged build.

### Windows

- [ ] Windows 10 22H2 and Windows 11 launch the app.
- [ ] 125% and 150% scaling keep the popover readable.
- [ ] The tray icon shows and hides the badge.
- [ ] Autostart uses the expected registry entry.
- [ ] Credential Manager contains one entry per account.

### Linux

- [ ] Wayland and X11 launch the app.
- [ ] GNOME and KDE show the tray icon (extension availability noted).
- [ ] Secret Service stores and retrieves tokens; a missing provider produces a
      clear error.
- [ ] Notifications work with `notify-rust` and open the PR when clicked.

## 13. Regression list

- [ ] Polling backoff grows after consecutive failures and resets on success.
- [ ] PR keys use `account/repo#number`, so the same number in two repos is not
      conflated.
- [ ] Config v1 files migrate to the multi-account format.
- [ ] Saving settings with no changes keeps the provider revision; changing an
      account or a token rebuilds providers.
- [ ] Tokens are written before the config that references them, and removed
      only after the config no longer references the account.
- [ ] Duplicate errors are collapsed into one banner line.
- [ ] The popover closes when switching desktop on macOS.

---

## Report template

```text
OS:                 macOS 15.x / Windows 11 / Ubuntu 24.04
Version:            0.1.0
Provider:           Bitbucket / GitHub
Account setup:      workspace + N repos (redact names)
Scaling:            100% / 150%
Steps:
  1.
  2.
Expected:
Actual:
Reproducible:       always / sometimes / once
Screenshot or log:
```
