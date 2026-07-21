# CRAFTEL Phase 4: Workflow Automation Plan

> Depends on Phase 3 run persistence. This phase composes existing workflow commands with runs; it does not infer outcomes from agent prose or process exit status.

**Goal:** Start phase-appropriate autonomous prompts, require explicit `craftel pass`/`craftel fail` transitions, and enforce clean formal review sessions.

## Authoritative contract

- Explicit user `Start` is the only automation trigger. Drag/move/next/pass/fail never launches a process. Start is valid only in `defining`, `implementation`, or `reviewing`; Inbox/Done reject it.
- Prompt templates are built in and versioned (`prompt_version` stored on the run). They include task ID, absolute work directory, current stage, document ownership guidance, autonomous instruction to inspect required artifacts/code/tests, and the exact terminal command: `craftel pass <id> --project <project-id>` on success or `craftel fail ...` on failure. They state no push/PR/release/deploy and no `TASK.md` edits.
- Defining prompt uses the installed define-spec workflow but explicitly overrides its review-turn pause: proceed autonomously from approved design/context, write the spec, verify it, and invoke pass/fail. Implementation uses implement-spec and current agent-owned documents. Reviewing performs a fresh evidence-based review in a newly created review session and writes a review record; it never reuses implementation context.
- Every run records `stage_at_start`, `workflow_event_id_before`, prompt kind/version, and optional observed transition event ID. Workflow commands remain normal transactional service operations. A transition is attributable only when its workflow event occurs after launch, matches task and starting stage, and action is `pass` or `fail`; manual move/next does not satisfy it.
- At terminal process exit, reconcile attribution transactionally. Exit success plus matching pass/fail is normal. Any exit without one leaves stage unchanged except for independently executed commands and exposes `missing_transition=true`. Non-zero/stop/interruption never synthesizes `fail`. A matching command remains authoritative even if the process later exits non-zero.
- Start reserves the active run and snapshots stage in one immediate transaction; reject if task stage changes before spawn. During a run, workflow commands can execute from the child CLI through the shared SQLite database. Start and transition transactions serialize under existing busy timeout; no global GUI process is required.
- Review `pass` records approval and remains Reviewing; human `next` alone advances approved review to Done. Review `fail` records changes requested and returns to Implementation. The next implementation start reuses its implementation session. Every later formal review start creates another fresh review session; no review follow-up is used to fix implementation.
- Prompt construction failure/invalid stage creates no run. Spawn/recovery behavior remains Phase 3. Store notices durably on runs so restart yields the same status.

**Strict non-goals:** automatic start on stage movement, inferred prose outcomes, automatic Done/delivery, Git commit/push/PR, live steering, configurable columns/prompts, multi-agent, worktrees, cloud, implementation inside review sessions.

## Slice 4.1 — Prompt and attribution schema

- [ ] Add `004_automation.sql` fields/constraints and workflow-event lookup IDs without rewriting prior migrations.
- [ ] Write snapshot tests for each exact prompt kind/version, shell-safe CLI command text, required ownership/non-goal language, and no environment secrets.
- [ ] Implement pure prompt builder and run attribution types.
- [ ] Verify: `cargo test -p craftel-core automation::prompt && cargo test -p craftel-core --test automation_repository`.
- [ ] Commit: `feat: define autonomous phase prompts`.

## Slice 4.2 — Explicit transition reconciliation

- [ ] Write failing integration tests with the Phase 3 fake executable invoking a real isolated `craftel` CLI: pass/fail matrices for all three phases, missing transition, unrelated/manual transition, transition then non-zero exit, stop, concurrent stage change, and restart display.
- [ ] Compose start with stage-specific session policy and reconcile only matching workflow event IDs. Do not parse stdout for pass/fail.
- [ ] Verify: `cargo test -p craftel-core --test workflow_automation -- --test-threads=1 && cargo test -p craftel-cli --test cli`.
- [ ] Commit: `feat: reconcile explicit agent transitions`.

## Slice 4.3 — Review-cycle enforcement and API

- [ ] Test implementation→reviewing→fresh R1 pass→human next, R1 fail→implementation reuse→fresh R2, and manual moves clearing approval per the foundation contract.
- [ ] Expose `start_current_phase` and durable transition notice fields through Tauri/types; retain low-level Phase 3 start internally for tests, not as a UI bypass.
- [ ] Verify: `cargo test -p craftel-core --test review_cycle && cargo test -p craftel-desktop && pnpm --filter @craftel/desktop test`.
- [ ] Commit: `feat: automate phase and review cycles`.

## Phase verification

- [ ] Linux gate: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm --filter @craftel/desktop test && pnpm --filter @craftel/desktop build && git diff --check`.
- [ ] Manual fake run for each phase confirms no start on move, explicit commands alone transition, review pass waits for human Done, and R2 has a distinct external session.
- [ ] **Live Cursor pending:** authenticated run executes prompt and real shared-DB CLI command; confirm missing-transition notice with a prompt deliberately told not to transition.
- [ ] **macOS pending:** repeat automation/restart/review cycle in Tauri on Apple Silicon. Signing, updater, packaging, and delivery remain excluded.

**Commit boundaries:** schema/prompt, reconciliation, and review/API remain separate reversible commits; no GUI workspace work from Phase 5.
