# ADR-0004: Polling and notification pipeline

- **Status:** Accepted
- **Date:** 2026-11-18

## Context

The app must notice new review requests and transitions on the user's own PRs
without hammering the APIs. Both services are eventually consistent, both have
rate limits, and a first poll must not announce every open PR as new.

## Decision

1. **One polling loop.** A single Tokio task polls every active account in
   parallel, for review requests and own PRs, then sleeps for the configured
   interval.
2. **Two scopes.** Review requests and own PRs are fetched, merged, sorted and
   published independently. The badge counts only review requests.
3. **Seeding.** An account becomes eligible for notifications only after its
   first successful poll. The first poll never floods the user.
4. **Once-only announcements.** A `notified` set keeps PR keys that have been
   announced. A search result that disappears for one poll and returns does not
   notify again.
5. **Transitions, not snapshots.** Own PRs notify only on `BecameReady` and
   `ChecksFailed`. A newly seen own PR raises nothing.
6. **Stale data rules.** A whole-account failure keeps all of that account's
   previous PRs. A single-repository failure keeps only that repository's review
   PRs. Own PRs keep only whole-account failures, because a partial own list can
   only be missing PRs.
7. **Deduplication.** A PR seen through two accounts is identified by
   `provider/repo/number` and kept once.
8. **Backoff.** A failed round multiplies the interval by `2^failures`, capped at
   three doublings. Only whole-account failures count, so one bad repository does
   not slow every account.
9. **Manual wake.** A `watch` channel wakes the loop after a manual refresh or a
   settings save. It does not bypass backoff semantics for failures.
10. **Provider rebuilds are revision-based.** A save that leaves both the config
    and the token map unchanged keeps the revision; any actual change rebuilds
    providers.

## Consequences

**Positive**

- No notification flood on first setup.
- Temporary provider outages do not look like PRs were closed.
- A flickering search result cannot cause repeated notifications.
- One throttled or broken account does not stall the others.

**Negative / accepted**

- Search consistency is handled with memory, not a durable event log; a restart
  after a PR appears can notify once for a PR seen before the restart.
- The global interval applies to every account. Per-account scheduling is future
  work.
- The badge and list are eventually consistent by design.

## Alternatives considered

- **Webhooks:** impossible for a desktop app without a public endpoint or a relay
  service.
- **Per-account intervals from day one:** more state and UI for little benefit
  with a handful of accounts.
- **Notifying on every list change:** noisy and wrong for edits, reordering and
  search flicker.
- **Clearing the list on any failure:** misrepresents a network problem as PRs
  being closed.
