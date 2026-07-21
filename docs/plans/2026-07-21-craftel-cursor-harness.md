# CRAFTEL Phase 3: Cursor Harness Plan

> Depends on Phase 2 migrations/service conventions. Implement against a deterministic fake executable; live Cursor evidence is explicitly pending when unavailable.

**Goal:** Persist observable phase sessions, runs, and ordered Cursor stream-json events; explicitly start, stop, and follow up while recovering stale runs safely.

## Locked behavior and state contracts

- `phase_sessions`: UUID, project/task, phase `defining|implementation|reviewing`, harness, external session ID (nullable until observed), created/updated timestamps. Defining and Implementation each reuse their latest session; every Review start creates a fresh session. Phase 4 owns prompt policy, not this rule.
- `runs`: UUID, session/task, sequence, state `queued|running|succeeded|failed|stopped|interrupted`, prompt, harness/version/model/work_dir, request ID, PID plus per-launch random ownership token, started/finished timestamps, exit code, stderr, final result, stop-requested timestamp, and error. State transitions are `queued→running→terminal`; terminal is immutable. At most one `queued|running` run per task via a partial unique index and immediate transaction.
- `run_events`: `(run_id, sequence)` identity, normalized kind `user|assistant|tool_start|tool_complete|result|system|unknown`, event timestamp, display text/tool metadata, and complete raw JSON. Preserve arrival order; append each complete NDJSON line transactionally. Partial stdout is buffered only in memory until newline; malformed complete lines become `unknown` events and do not abort the run.
- Harness seam is `start`, `resume`, `stop`, stream events, final result. Cursor command is `agent -p --force [--resume=<id>] --output-format stream-json --stream-partial-output <prompt>`, with `current_dir=project.work_dir`; use argv, never a shell. Capture installed executable/version at launch. Do not read Cursor databases or claim hidden reasoning.
- Initial run starts/resumes only after the active-run transaction succeeds. Follow-up is rejected while active, is allowed only for Defining or Implementation after terminal state, creates a new run in that same session, and requires its external Cursor session ID. Reviewing rejects follow-up; another formal review is a new session.
- One in-process run supervisor owns children. Stop marks intent, sends graceful termination, waits a bounded 5 seconds, then kills; observed termination becomes `stopped` only when stop was requested, otherwise non-zero is `failed`. Zero exit is `succeeded`, regardless of workflow transition (Phase 4).
- Startup recovery never auto-resumes. For every persisted `running`, verify PID *and ownership token* through an injectable process inspector. Since a restarted process cannot be safely reattached to pipes, a matching live child is terminated then marked `interrupted`; missing/mismatched process is marked `interrupted`. Queued records left before spawn also become `interrupted`. Preserve all events/stderr.
- Event subscribers receive `(run_id, last_persisted_sequence)` after commit and re-query pages; slow/disconnected clients cannot block or lose durable events. Stderr is bounded to 1 MiB with an explicit truncation marker. No prompt or raw payload is logged outside the database.

**Strict non-goals:** Amp harness, concurrent task agents, worktrees, live steering/input during runs, Cursor IDE chat sync, hidden reasoning, workflow transitions/skill prompts (Phase 4), GUI rendering (Phase 5), Git commits/pushes/PRs, cloud execution.

## Slice 3.1 — Persistent sessions/runs/events

- [ ] Add `003_harness.sql`, domain/repository modules, and failing file-backed tests for all states, unique active task constraint across two connections, session/run sequences, ordered raw events, terminal immutability, and reopen.
- [ ] Implement typed repository operations with immediate launch reservation and atomic terminal updates.
- [ ] Verify: `cargo test -p craftel-core --test run_repository`.
- [ ] Commit: `feat: persist harness sessions and runs`.

## Slice 3.2 — Cursor parser and deterministic fake

- [ ] Add test fixture executable under `crates/craftel-core/tests/fixtures/` (source/script, no binary) controlled by argv/environment to emit fragmented valid NDJSON, unknown/malformed lines, stderr, delays, session/request IDs, and selected exit codes; it records argv/cwd for assertions.
- [ ] Write parser tests from captured representative Cursor shapes while tolerating additive unknown fields. Normalize known user/assistant/tool/result events and retain every raw object.
- [ ] Implement `Harness`/`CursorHarness` and process factory seams; prove exact initial/resume argv, no shell interpolation, project cwd, partial-line buffering, bounded stderr, and exit classification.
- [ ] Verify: `cargo test -p craftel-core --test cursor_harness -- --test-threads=1`.
- [ ] Commit: `feat: stream Cursor harness events`.

## Slice 3.3 — Supervisor, stop, follow-up, recovery

- [ ] Write failing service tests for one active run/task, independent tasks, launch failure, stop race/idempotence/escalation, follow-up session reuse, mandatory fresh review session, subscriber catch-up, and all stale recovery branches using fake process/clock inspectors.
- [ ] Implement supervisor and APIs: `start_phase_run`, `stop_run`, `follow_up`, `get_session`, `list_sessions`, `list_runs`, `get_run`, `list_run_events(after_sequence, limit)`.
- [ ] Expose equivalent Tauri commands plus post-commit `run_event`/`run_changed` notifications. Keep child ownership in Rust core; frontend cannot spawn processes.
- [ ] Verify: `cargo test -p craftel-core --test run_service -- --test-threads=1 && cargo test -p craftel-desktop`.
- [ ] Commit: `feat: supervise resumable Cursor runs`.

## Phase verification and acceptance

- [ ] Linux gate: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm --filter @craftel/desktop build && git diff --check`.
- [ ] Run fake executable end-to-end: initial stream, persisted restart history, follow-up with exact resume ID, stop, failed exit, and startup interruption.
- [ ] **Live Cursor pending:** on Linux with authenticated Cursor Agent CLI, verify version discovery and one harmless temporary-project initial/follow-up stream. Never make this a deterministic CI requirement.
- [ ] **macOS pending:** repeat fake and live checks in Tauri on Apple Silicon, including stop escalation and app restart. No signing/updater/package validation.

**Commit boundary:** each slice commit contains only its schema/core/API/tests. Phase 4 must not be folded into this phase.
