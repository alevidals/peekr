# ADR-0002: Provider boundary and API strategy

- **Status:** Accepted
- **Date:** 2026-11-18

## Context

Bitbucket Cloud and GitHub expose different APIs, different review models and
different CI concepts. Bitbucket calls them pipelines and has no single merge
verdict; GitHub calls them checks and computes `mergeStateStatus`. The pill, the
list and the notifier should not know which service produced a row.

## Decision

1. Define one async `ReviewProvider` trait:
   - `validate()` checks a token and returns an identity.
   - `fetch_review_prs(repos)` returns PRs waiting on the user.
   - `fetch_my_prs(repos)` returns the user's own PRs with a merge status.
   - Both fetch methods return a `FetchReport` with per-repository failures.
2. Map every payload to one `PullRequest` domain type and one `MergeStatus`.
3. Select the adapter in `provider::build` from `ProviderKind`; the poller holds
   `Arc<dyn ReviewProvider>`.
4. Keep provider-specific words in `src/providers.tsx`, not in the row component.
5. Use each API the way it is best at:
   - Bitbucket: REST, server-side reviewer filter with a client-side fallback,
     and the mergeability checks endpoint when the token can read it.
   - GitHub: GraphQL, one search per side, including team review requests, and
     GitHub's own merge verdict.
6. Cap concurrent requests per account (8 for Bitbucket) with a semaphore.
7. Never emulate a capability the provider does not expose. Label the Bitbucket
   fallback as an inference and state that it cannot detect conflicts.

## Consequences

**Positive**

- The poller, notifier, tray and list stay provider-agnostic.
- Adding a provider is one module, one enum arm and one settings entry.
- Per-repository failures are visible without discarding healthy data.
- Provider wording (checks vs pipelines, ready vs looks ready) stays consistent.

**Negative / accepted**

- The domain type is a lowest common denominator; provider-only fields need a
  deliberate extension.
- Bitbucket uses more requests because reviewer, participants, statuses and merge
  checks are separate resources.
- GitHub's search is eventually consistent and caps at 1,000 results.
- The trait must be extended carefully if write actions ever arrive.

## Alternatives considered

- **One UI branch per provider:** rejected; it duplicates list, badge and
  notification logic and makes wording inconsistent.
- **Normalise everything in the frontend:** rejected; business rules belong in
  Rust and the webview should not parse provider payloads.
- **A third-party unified PR library:** none covers these two services with the
  required review, CI and merge semantics.
- **GitHub REST instead of GraphQL:** rejected; it needs several requests per PR
  and loses GitHub's own verdict at a glance.
