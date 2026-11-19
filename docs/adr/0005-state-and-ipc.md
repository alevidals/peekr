# ADR-0005: State ownership and IPC types

- **Status:** Accepted (codegen deferred)
- **Date:** 2026-11-19

## Context

The app mixes provider-derived data (PR lists), real-time pushed state (polls and
window events) and window-local UI state (tab, search, filters). Tauri commands
and events need a contract that both sides agree on. The command surface is
small, but it can drift silently when a Rust struct changes.

## Decision

1. **Rust owns authoritative state.**
   - `ConfigStore` holds an immutable `Snapshot` of config and in-memory tokens
     behind an `RwLock<Arc<…>>`.
   - `PrState` holds the two latest lists behind Tokio mutexes.
   - Commands are thin adapters; business rules stay in the modules.
2. **The frontend is window-local.** Each window loads config and PR state when
   it mounts. The popover re-reads config every time it opens, because Settings
   is a different window.
3. **One API module.** `src/api.ts` is the only place that knows command and
   event names. Components and hooks call it.
4. **One provider copy module.** `src/providers.tsx` owns labels, token scopes
   and token URLs. Rows render shared vocabulary through it.
5. **Wire conventions.**
   - Fields stay `snake_case`, matching the Rust structs.
   - Errors serialize as a single readable string with the source chain.
   - `get_config` returns `has_token`, never a token.
   - Events: `prs-updated`, `my-prs-updated`, `poll-error`,
     `popover-will-hide`, `popover-will-show`.
6. **Types are handwritten** in `src/types.ts` for now.
7. **Codegen is deferred** until one of these is true:
   - The command/event surface grows enough that manual review is error-prone.
   - A type-drift bug reaches a release.
   - A stable generator supports the Tauri 2 stack without a pre-release
     dependency.
8. **Pure logic is extracted.** Search parsing, draft conversion and theme
   resolution are plain functions, so they are ready for a test runner.

## Consequences

**Positive**

- No extra build step, no pre-release codegen dependency, no generated file to
  review.
- The contract is small enough to read in one sitting.
- Window-local state keeps the popover simple and avoids a global store.
- The provider copy module prevents vocabulary drift.

**Negative / accepted**

- `types.ts` is a manual mirror. A Rust change can compile while the frontend
  still expects the old shape.
- Runtime errors are strings, not structured codes, so the UI cannot branch on a
  stable error kind yet.
- The frontend test gap makes drift more likely than it would be with generated
  bindings plus tests.

## Alternatives considered

- **`tauri-specta` v2 RC:** the obvious candidate, but it is pre-release and adds
  a generation step. Revisit when it is stable.
- **Handwritten JSON Schema:** still a second source of truth and requires its own
  generator.
- **A global Zustand store:** unnecessary for two windows with independent
  lifetimes; adds subscriptions and rehydration concerns.
- **TanStack Query:** built for server state and caching. The poller already owns
  server state; the frontend receives the result.

## Validation

The provider and polling test suites protect the Rust side. The planned Vitest
suite covers search, drafts, merge labels and the event hook. If a type-drift bug
slips through, this ADR's deferral should be reconsidered first.
