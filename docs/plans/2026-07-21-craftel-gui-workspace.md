# CRAFTEL Phase 5: GUI Workspace and Streaming Plan

> Depends on Phases 2–4 typed APIs. The React frontend presents durable core state and sends commands; it never reads files, SQLite, or child processes directly.

**Goal:** Complete the Linux-capable MVP GUI with task documents, persisted sessions/runs, streaming output/tool activity, explicit controls, revisions, and actionable errors.

## Locked UX/API behavior

- Opening a card routes to a task workspace while preserving project switcher/board return. Three responsive regions: document tree; Markdown editor/preview; session/run panel. Narrow layouts use tabs/drawers, not configurable columns.
- Tree shows only Phase 2 eligible documents, grouped by relative directory with deleted documents available through revision history. Selection is URL/app state by project/task/path. Read returns content + hash; Save supplies expected hash. Conflict preserves local edits, offers reload/copy, and never force-overwrites. Markdown preview is sanitized; no scripts/raw unsafe HTML.
- Session panel groups by phase/session, newest session first and runs chronological. Run view paginates durable events by sequence and then subscribes to notifications; on reconnect/sequence gap it fetches after the last sequence. Assistant text streams in order; tool start/completion pairs are collapsible; unknown/raw events have a safe diagnostic view. Never label content “reasoning”.
- Show state, duration, model, harness/version, final result, bounded stderr/error, missing-transition notice, and interrupted recovery message. Duration uses persisted timestamps plus a display timer only while running.
- `Start phase` is enabled only for eligible stage with no active task run. `Stop` is the only during-run interaction and is idempotent. Follow-up appears only after terminal Defining/Implementation runs with a resumable external session; submit creates a run and clears input only after acceptance. Formal Review uses Start and always creates a fresh session. No live steering.
- Revision browser lists timestamp/cause/hash newest first, previews selected bytes, clearly marks current/deleted state, and restores through Phase 2 expected-hash API. Restore refreshes editor/tree and displays the newly created revision.
- Board cards retain ID/title/excerpt and show current run indicator derived from durable active run state. Drag remains stage-only and never starts execution. Context menus mirror Open/Start/Stop/Edit where valid.
- All async screens have loading, empty, unavailable-project, recoverable error, and retry states. Tauri event payloads are hints only; every update is re-queried. Switching projects unsubscribes prior listeners and ignores late responses via request generation IDs.

**Strict non-goals:** PR/Git delivery, signing/notarization/updater, cloud/team sync, worktrees, multi-agent, live steering, configurable columns/workflows/prompts, raw hidden reasoning, Cursor IDE chat, general non-CRAFTEL file editor.

## Slice 5.1 — API client and workspace navigation

- [ ] Extend `CraftelApi`/Tauri adapter types for documents, snapshots, sessions, runs, events, automation notices, and subscriptions; add runtime-safe error mapping.
- [ ] Write failing injected-fake component tests for card open/back, project switching cancellation, tree loading/selection, sanitized preview, edit/save, hash conflict preserving input, unavailable/error/retry, and responsive tab semantics.
- [ ] Implement task route/layout, document tree, editor/preview. Keep server state in a small query cache; do not duplicate domain rules in React.
- [ ] Verify: `pnpm --filter @craftel/desktop test -- --run && pnpm --filter @craftel/desktop build`.
- [ ] Commit: `feat: add task document workspace`.

## Slice 5.2 — Session history and lossless streaming

- [ ] Write failing tests for history ordering, event pagination, subscribe-after-fetch race (second catch-up query), duplicate notification de-duplication, sequence-gap recovery, assistant/tool/unknown rendering, duration, stderr/final/error states, listener cleanup, and restart-loaded history.
- [ ] Implement session/run panel and event reducer keyed by `(run_id, sequence)`; notifications trigger fetches rather than carrying authoritative display content.
- [ ] Add Rust command serialization tests matching TypeScript fixtures.
- [ ] Verify: `pnpm --filter @craftel/desktop test -- --run && cargo test -p craftel-desktop`.
- [ ] Commit: `feat: stream persisted run activity`.

## Slice 5.3 — Controls and workflow notices

- [ ] Test start eligibility, no run on drag, one start on double click, Stop-only while active, stop idempotence, terminal follow-up eligibility, fresh Review labels/session, missing-transition callout, interrupted state, and context-menu parity.
- [ ] Implement controls exclusively through Phase 4 APIs and refresh task/run/session queries after durable changes. Disable pending actions and preserve follow-up text on rejection.
- [ ] Verify: `pnpm --filter @craftel/desktop test -- --run && cargo test -p craftel-desktop`.
- [ ] Commit: `feat: add run and workflow controls`.

## Slice 5.4 — Revisions and integrated error UX

- [ ] Test revision list/preview, deleted latest content, restore confirmation, conflict, successful restore refresh/new revision, malformed event diagnostics, missing directory, process launch/failed/stopped/interrupted messages, and keyboard/focus behavior.
- [ ] Implement revision browser and consolidated accessible alerts (`role=status|alert`, focus return, keyboard tree/control operation). Avoid exposing full host paths or raw secrets in user-facing errors.
- [ ] Verify: `pnpm --filter @craftel/desktop test -- --run && pnpm --filter @craftel/desktop build`.
- [ ] Commit: `feat: add document recovery and error UX`.

## Integrated acceptance and verification

- [ ] Linux full gate: `pnpm check && git diff --check` (includes fmt, Clippy, Rust workspace tests, frontend tests/build).
- [ ] Deterministic desktop-core scenario with fake Cursor: register/switch project; create/open task; edit/search document; start and watch assistant/tool events; stop; follow up; restart and inspect history; delete/restore document; run implementation then fresh review; approve and verify Reviewing persists until human Next.
- [ ] Browser visual review through development-only fake API at wide/narrow sizes: empty/loading/error, long Markdown, large event stream, active/stopped/failed/interrupted, conflict, deleted revision. Ensure fake mode cannot compile into production behavior.
- [ ] **Live Cursor pending:** authenticated end-to-end stream/stop/follow-up/review with real event variants; record unsupported variants as raw rather than blocking display.
- [ ] **macOS pending:** run native Tauri on Apple Silicon and repeat MVP scenario, file dialogs, watcher-driven refresh, keyboard controls, restart, and process stop. Packaging, signing, notarization, updater, Intel, Windows, and Linux release validation are excluded.

**Acceptance:** design criteria 1–12 are demonstrable across foundation plus these phases; all durable state survives restart; no UI action violates stage/run independence or document ownership.

**Commit boundary:** one commit per slice, then a final documentation-only acceptance record if required by the implementation workflow; never commit generated build artifacts.
