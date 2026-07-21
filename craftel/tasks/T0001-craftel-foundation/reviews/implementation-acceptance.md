# Implementation Acceptance

## Change Summary

Phase 1 delivers the Rust domain and SQLite repository, durable Markdown projection service, independent CLI, Tauri command boundary, and React project switcher/five-stage task board. Repository guidance and setup document the supported Debian development path and pending macOS validation.

## Specification Deviations

No approved Phase 1 behavior is intentionally omitted. Native macOS validation is not available in the Debian orb and is explicitly pending; packaging is a later phase. The static foundation task is an acceptance record, not a runtime-created database task or agent session.

## Acceptance Evidence

| Criterion | Evidence | Result |
| --- | --- | --- |
| Register, list, switch, and safely remove projects, including unavailable paths | SQLite repository and CLI integration tests; `ProjectSwitcher.test.tsx` | Pass on Debian |
| Create a task with required title and content through CLI or GUI | CLI integration tests and `CreateTaskDialog.test.tsx` | Pass on Debian |
| Generate durable task directory, `TASK.md`, and one-time `SPEC.md` | `task_service` tests cover creation, atomic projection, repair, and preservation | Pass on Debian |
| Persist task metadata and allocate unique project-local IDs | File-backed SQLite repository tests, including reopen and concurrent allocation | Pass on Debian |
| Apply the complete five-stage transition contract | Exhaustive Rust workflow tests and CLI transition integration tests | Pass on Debian |
| Move tasks without starting an external process | CLI tests plus board API boundary/drag tests; Phase 1 has no process runner | Pass on Debian |
| Switch, create, edit, group, and drag cards in the board | React component tests, including optimistic rollback and keyboard-accessible handles | Pass on Debian |
| Compile the Rust/Tauri workspace and production web application | Root `pnpm check` runs fmt, Clippy, Rust tests, Vitest, TypeScript, and Vite build | Pass on Debian |
| Reopen native app and verify persisted GUI/CLI interoperability | Six-step macOS manual procedure in the implementation plan | Pending macOS |
| Exclude later document, harness, automation, and delivery phases | Schema/code inspection and documented Phase 1 limitations | Pass |

## Verification Commands

| Command or check | Result | Notes |
| --- | --- | --- |
| `pnpm check` | Pass | Debian orb: Rust format/Clippy/tests, web tests, and production build all succeeded. |
| `git diff --check` | Pass | No whitespace errors. |
| macOS native six-step acceptance | Pending | No live macOS runner was available; no native-app claim is made. |

## Remaining Risks

- Native macOS launch, drag behavior, restart persistence, and CLI-to-GUI refresh remain unverified until run on Apple Silicon.
- macOS packaging, signing, notarization, and updates are explicitly outside Phase 1.
- Linux/WebKit compilation and browser tests do not prove AppKit/WebKit behavior on macOS.
- Document synchronization, history, agent sessions/runs, and automation are intentionally deferred to later phases.
