# CRAFTEL Linux MVP Acceptance — 2026-07-21

## Delivered architecture

- **Foundation:** a shared Rust domain/service layer, additive SQLite migrations, project-local task IDs, generated `TASK.md` projection, one-time `SPEC.md`, an independent `craftel` CLI, typed Tauri commands, and a React five-column board. Stage movement and process execution remain independent.
- **Documents:** disk-authoritative, eligible UTF-8 Markdown is scanned and watched under `craftel/`; SQLite holds the searchable index and immutable snapshots. Project/path validation, hash-based optimistic writes, atomic replacement, deletion history, restore, retention, durable errors, and post-commit notifications are implemented. Generated `TASK.md` is never ingested or edited.
- **Cursor harness:** replaceable process and inspector seams launch Cursor by argv (never a shell), persist phase sessions, runs, ordered normalized plus raw NDJSON events, bounded stderr, identifiers, and terminal state. The supervisor supports stop, resumable Defining/Implementation follow-up, fresh Review sessions, durable history, and conservative interrupted-run recovery.
- **Automation:** only explicit Start launches a phase. Versioned prompts require an exact shared-database `craftel pass` or `craftel fail`; attribution uses workflow event IDs, not prose or exit status. Review pass remains Reviewing until human `next`; review fail returns to Implementation.
- **GUI:** the Tauri/React boundary re-queries durable state after event hints. The board and responsive task workspace provide document tree/search/edit/sanitized preview/revisions/restore, persisted session and run history, paged event streaming, status/error notices, start/stop/follow-up controls, and stale-request/listener cleanup.

## Acceptance criteria 1–12

| # | Criterion and evidence | Linux result |
| --- | --- | --- |
| 1 | Register and switch local projects: SQLite/CLI integration tests and `ProjectSwitcher.test.tsx`, including missing registrations and stale loads. | Pass |
| 2 | Create a required-title/content task through GUI or CLI: CLI workflow and `CreateTaskDialog.test.tsx`. | Pass |
| 3 | Generate task directory, `TASK.md`, and `SPEC.md`: `task_service` creation, compensation, repair, and preservation tests. | Pass |
| 4 | Move without starting an agent: exhaustive workflow/CLI tests and board drag/API-boundary tests. | Pass |
| 5 | Explicitly start and stop a phase run: run-service fake end-to-end tests plus board/workspace control tests. | Pass with fake Cursor |
| 6 | Watch assistant/tool events in the GUI: ordered persistence and pagination/subscription catch-up, gap, deduplication, unknown-event, and activity rendering tests. | Pass with fake events |
| 7 | Reopen persisted sessions/runs/history: file-backed run repository, fake restart/resume, stale recovery, and restart-loaded UI tests. | Pass |
| 8 | Follow up in Defining/Implementation: fake end-to-end resume asserts the exact external session ID; eligibility and input preservation are component-tested. | Pass with fake Cursor |
| 9 | Start Review in a fresh session: review-cycle and automation matrix tests plus fresh-review GUI controls. | Pass with fake Cursor |
| 10 | Perform documented `pass`, `fail`, and `next` transitions: exhaustive domain matrix and real isolated CLI automation integration. | Pass |
| 11 | Keep approved review in Reviewing until human advance: `review_cycle` and full CLI session-cycle tests. | Pass |
| 12 | Edit agent-owned Markdown and recover modified/deleted snapshots: repository/service/watcher tests and workspace conflict/revision/restore tests. | Pass |

## Final verification

At current HEAD before this documentation commit, `pnpm check` passed in the Debian 12 orb on 2026-07-21:

- `cargo fmt --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo test --workspace`: **44 passed, 0 failed, 0 ignored** across Rust unit/integration targets; doc-test targets contained 0 tests.
- `vitest run`: **7 files passed, 31 tests passed**.
- `tsc && vite build`: pass; Vite 7.3.6 transformed 214 modules and emitted a 380.12 kB JavaScript bundle (119.74 kB gzip), completing in 1.37 s.
- Overall `pnpm check`: exit 0. No test was skipped or filtered.

Vitest still emitted **12 React `act(...)` warnings across three tests**: four in the board double-activation test, one in the dirty-draft document-hint test, and seven in the active Stop/idempotence test. Assertions pass, but these warnings can hide timing defects and remain cleanup risk.

## Deterministic fake Cursor scope

Linux acceptance does not claim an authenticated Cursor run. The source fixtures exercise version discovery, exact initial/resume argv, project cwd and no-shell interpolation, fragmented/unknown/malformed NDJSON handling, session/request IDs, assistant and tool events, bounded stderr, success/failure, stop, persisted restart history, follow-up resume, explicit CLI pass/fail attribution, missing transitions, review cycles, and stale-run interruption. They do not validate Cursor authentication, actual model behavior, future live event variants, service/network failures, or real process stop timing.

The browser mock is separate synthetic UI data. It is enabled only by `?mock=1` while Vite's `import.meta.env.DEV` is true and is dynamically excluded from normal production behavior. A portal/mock demonstration is development evidence, not a native or live-Cursor acceptance run.

## Portal visual review

Development-only portal review covered a populated five-column board and document/activity workspace at wide and narrow sizes, with long Markdown, revisions, running/succeeded/failed/stopped/interrupted histories, unknown tools, and sanitized unsafe markup represented by fixtures.

- Wide board: `.amp/in/artifacts/craftel-board-wide.png`
- Narrow board: `.amp/in/artifacts/craftel-board-narrow.png`
- Wide workspace: `.amp/in/artifacts/craftel-workspace-wide.png`
- Narrow workspace: `.amp/in/artifacts/craftel-workspace-narrow.png`
- Rich wide workspace: `.amp/in/artifacts/craftel-workspace-rich-wide.png`
- Rich narrow workspace: `.amp/in/artifacts/craftel-workspace-rich-narrow.png`

The wide layouts are coherent and expose all major controls and activity states. The narrow workspace stacks navigation and provides Documents/Activity switching without page-level horizontal overflow. The board intentionally remains a horizontally scrolling Kanban surface; the narrow capture shows the next column clipped at the viewport edge, so discoverability of horizontal scrolling remains a usability risk. Minor density/alignment issues remain in card actions and run metadata, and unknown tool output is visibly diagnostic rather than silently discarded. These are not native Tauri screenshots.

## Strict exclusions and residual risks

Excluded: Git commits/pushes/PRs or delivery automation; cloud/team sync; concurrent agents; worktrees; live steering; Cursor IDE chat/database synchronization; hidden reasoning; configurable columns/workflows/prompts; general file editing outside CRAFTEL-owned Markdown; Intel macOS, Windows, or Linux product-release validation; packaging, signing, notarization, and updater work.

Residual risks are the pending live-Cursor and macOS checks below; the React `act(...)` warnings; future Cursor stream variants (preserved as raw unknown events but not necessarily richly rendered); platform differences between Linux inotify/WebKit and macOS FSEvents/WebKit; process termination races on real Cursor; narrow-board horizontal-scroll discoverability; and untested production packaging. Linux automated tests cannot prove native file-dialog, accessibility, drag, suspend/restart, or filesystem event behavior on Apple Silicon.

## macOS/live-Cursor morning checklist

Use an Apple Silicon Mac with Xcode Command Line Tools, Tauri 2 prerequisites, compatible Node/Corepack/pnpm, stable Rust, and an authenticated Cursor Agent CLI. Do not package, sign, notarize, configure an updater, push, or use a valuable project.

1. Clone/open the same commit, run `pnpm install --frozen-lockfile`, then `pnpm check`; record macOS/Rust/Node/pnpm/Cursor versions and all counts/warnings.
2. Run `pnpm --filter @craftel/desktop tauri dev`. Register two disposable local projects through the native directory dialog, switch between them, remove one registration, and confirm files are untouched.
3. Create a task in GUI and one via `cargo run -p craftel-cli -- create ...`; verify board refresh and exact generated task directory, `TASK.md`, and `SPEC.md`. Drag/move a card and confirm no Cursor process starts; verify keyboard board/workspace controls.
4. Edit, rapidly rewrite, rename, and delete eligible Markdown externally. Confirm FSEvents refresh/search, stable current content and revision order; restart the app, restore the deleted revision, and confirm a new restore snapshot. Trigger a stale edit and verify the draft survives conflict.
5. In Defining, explicitly Start authenticated Cursor. Confirm version/model, assistant/tool/unknown events stream in order and persist. Stop one run and verify graceful termination (or five-second escalation), terminal status, bounded stderr, and no orphan process.
6. Start again, allow the exact `craftel pass` command, verify attribution and transition. Send a follow-up and confirm a new run resumes the same external Cursor session. Restart the app and verify history and no automatic resume.
7. Repeat for Implementation. Exercise one deliberate missing-transition run and one non-zero exit; confirm neither synthesizes `fail`, while an explicit transition remains authoritative even if Cursor later exits non-zero.
8. Start formal Review and record its new external session ID. Verify it differs from Implementation and prior reviews. Run `craftel pass`; confirm the task stays Reviewing until a human invokes `craftel next`. Also test review `fail` returning to Implementation and a second Review creating another fresh session.
9. Recheck narrow/wide resizing, sanitized Markdown, file dialogs, focus/keyboard behavior, drag, project switching during loads, app restart, and quit during an active run. Save logs/screenshots without secrets and update this acceptance record with results and any newly observed live event shapes.
