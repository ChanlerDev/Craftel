# CRAFTEL Product and System Design

**Date:** 2026-07-21  
**Status:** Approved  
**Initial platform:** macOS on Apple Silicon  
**Initial agent harness:** Cursor Agent CLI

## 1. Product Summary

CRAFTEL is a project-oriented task board for AI-assisted software development. A project is a local working directory. Each project has a visible, Markdown-based task workspace that agents can read and edit with their native file tools, while CRAFTEL maintains board metadata, agent runs, session history, and document recovery in SQLite.

The core workflow is:

```text
Inbox -> Defining -> Implementation -> Reviewing -> Done
                          ^                |
                          +-- review fail -+
```

Moving a task between columns changes only its workflow stage. It never starts an agent. A run starts only from an explicit GUI action, context-menu action, or `craftel` CLI command.

## 2. Product Principles

1. **Local projects are the unit of work.** Each CRAFTEL project points to one `workDir`.
2. **Documents remain normal Markdown.** Specs, decisions, plans, and reviews are edited by agents through native Read/Write/Edit tools.
3. **Board metadata has one writer.** CRAFTEL owns fixed task fields and projects them into `TASK.md`.
4. **Workflow stage and execution state are separate.** A task can be in Implementation without an agent running.
5. **Agent runs are observable and recoverable.** CRAFTEL stores prompts, structured events, tool calls, results, errors, and external session IDs.
6. **Human delivery remains explicit.** A successful automated review does not mark a task Done.
7. **Harness integration is replaceable.** Cursor is first; Amp local runners and orbs can be added without changing the task model.

## 3. Project and Document Layout

CRAFTEL can register multiple projects and switch between them in the GUI. Each project records a display name and an absolute `workDir`. Removing a project from the application does not delete files.

The project-owned workspace is visible and uses the existing CRAFTEL skill convention:

```text
<workDir>/
├── craftel/
│   ├── INDEX.md
│   └── tasks/
│       └── T0001-refresh-authentication-flow/
│           ├── TASK.md
│           ├── SPEC.md
│           ├── decisions/       # created when needed
│           ├── discussions/     # created when needed
│           ├── notes/           # created when needed
│           ├── subtasks/        # created when needed
│           ├── plans/           # created when needed
│           └── reviews/         # created when needed
└── <normal project files>
```

Only `TASK.md` and `SPEC.md` are created for every task. Supporting directories are created lazily.

### 3.1 Task identity

- CRAFTEL allocates a sequential local ID such as `T0001`.
- The directory is `<id>-<initial-slug>`.
- The ID is authoritative; the slug is only for human readability.
- Renaming a task does not rename its directory.
- CLI operations address tasks by ID.

## 4. Document Ownership

### 4.1 CRAFTEL-owned `TASK.md`

`TASK.md` is a generated projection of fixed fields stored in SQLite. Agents must use the `craftel` CLI rather than directly editing it.

```markdown
---
id: T0001
title: Refresh authentication flow
status: inbox
created_at: 2026-07-21T10:30:00Z
updated_at: 2026-07-21T10:30:00Z
---

# Refresh authentication flow

## Content

Current tokens expire without a refresh mechanism.

## Artifacts

- Specification: [SPEC.md](./SPEC.md)
- Latest plan: Not created
- Latest review: Not created

> This file is managed by CRAFTEL. Use the `craftel` CLI to update task
> metadata and workflow state. Put agent-authored details in `SPEC.md` and
> supporting document directories.
```

The required creation fields are `title` and `content`. Content is the durable task description or initial context, not merely a generated one-line summary.

### 4.2 Agent-owned documents

Agents may use native file tools to edit:

- `SPEC.md`
- `decisions/*.md`
- `discussions/*.md`
- `notes/*.md`
- `subtasks/*.md`
- `plans/*.md`
- `reviews/*.md`

Project guidance and CRAFTEL skills must state that `TASK.md` is generated and that workflow transitions use CRAFTEL commands.

## 5. Workflow Model

The MVP has five board stages:

```text
inbox
defining
implementation
reviewing
done
```

Users may drag a task to any column to correct or reorganize the board. Automated actors follow normal transitions.

### 5.1 CLI transitions

```bash
craftel next T0001
craftel pass T0001
craftel fail T0001
```

`craftel next` performs an explicit forward transition:

| Current stage | Next stage |
| --- | --- |
| Inbox | Defining |
| Defining | Implementation |
| Implementation | Reviewing |
| Reviewing with an approved review | Done |
| Done | No transition |

`craftel pass` reports successful phase completion:

| Current stage | Result |
| --- | --- |
| Defining | Move to Implementation |
| Implementation | Move to Reviewing |
| Reviewing | Record Approved; remain in Reviewing for human delivery |

`craftel fail` reports unsuccessful phase completion:

| Current stage | Result |
| --- | --- |
| Defining | Remain in Defining and record failure |
| Implementation | Remain in Implementation and record failure |
| Reviewing | Record Changes Requested and return to Implementation |

An agent's phase prompt tells it to invoke `craftel pass` or `craftel fail`. CRAFTEL does not infer workflow results from prose.

## 6. Execution Model

Workflow stage and run state are independent.

Run states are stored only in SQLite:

```text
queued
running
succeeded
failed
stopped
interrupted
```

They are not written into task YAML because they are transient, can change frequently, and belong to individual attempts rather than the task itself.

### 6.1 Sessions and runs

A task can have multiple phase sessions, and each session can have multiple runs:

```text
Task
├── Defining session
│   ├── Initial run
│   └── Follow-up run
├── Implementation session
│   ├── Implementation run
│   └── Fix run
└── Reviewing session R1
    └── Review run
```

Default policy:

- Defining reuses one resumable session.
- Implementation reuses one resumable session, including fixes.
- Every formal review starts a fresh session with clean context.
- A user can start additional sessions later, but this is not an MVP requirement.
- A task can have at most one active run at a time.
- During a run, MVP interaction is limited to Stop.
- Follow-up messages are sent after the current run ends and resume its session.

## 7. Cursor Agent Integration

The first harness invokes Cursor in headless mode:

```bash
agent -p --force \
  --output-format stream-json \
  --stream-partial-output \
  "<phase prompt>"
```

Follow-up runs resume the external session:

```bash
agent -p --force \
  --resume="<cursor-session-id>" \
  --output-format stream-json \
  --stream-partial-output \
  "<follow-up prompt>"
```

CRAFTEL consumes NDJSON from stdout and stores:

- Harness name and version
- External session ID and request ID
- Model and working directory
- Prompt
- User and assistant events
- Tool-call start and completion events
- Final result
- Raw event payloads
- stderr, process exit code, and timing

The GUI renders observable conversation history and tool activity. Cursor print mode does not expose hidden model reasoning, so CRAFTEL does not claim to show it.

CRAFTEL must not read or modify Cursor's internal chat database. Its own event stream is the stable integration boundary.

## 8. Persistence and Synchronization

The application database lives under the macOS application data directory, for example:

```text
~/Library/Application Support/CRAFTEL/craftel.sqlite3
```

Initial logical tables:

```text
projects
tasks
phase_sessions
runs
run_events
document_index
document_snapshots
```

### 8.1 Sources of truth

| Data | Source of truth |
| --- | --- |
| Registered projects and work directories | SQLite |
| Fixed task fields | SQLite |
| `TASK.md` | Generated projection from SQLite |
| Specs, decisions, discussions, plans, and reviews | Markdown files |
| Document search index | Rebuildable SQLite projection |
| Document revision history | SQLite snapshots |
| Agent sessions, runs, events, and logs | SQLite |

### 8.2 Document watcher

CRAFTEL watches `<workDir>/craftel/**`. For agent-owned documents it:

1. Debounces partial or repeated file-system events.
2. Reads the complete file.
3. Computes a content hash.
4. Stores a snapshot only when content changed.
5. Updates the search index.
6. Notifies the GUI.

Initial retention policy:

- Keep revisions for 30 days.
- Keep at most 100 revisions per file.
- Preserve the latest snapshot when a file is deleted.
- Restoring a revision creates a new revision instead of erasing history.

## 9. GUI Scope

### 9.1 Project switcher

- Open a local directory as a project.
- Switch among recent projects.
- Detect missing or moved work directories.
- Remove a project registration without deleting files.

### 9.2 Board

- Five workflow columns.
- Task cards with ID, title, content excerpt, and current run indicator.
- Drag tasks to any stage without starting an agent.
- Create, rename, run, stop, and open tasks.
- Context-menu alternatives for primary actions.

### 9.3 Task workspace

- Document tree for the task directory.
- Markdown preview and editing surface.
- Side panel with phase sessions and runs.
- Streaming assistant output.
- Collapsible tool calls and results.
- Run duration, state, model, and final result.
- Stop while running and follow up after completion.
- Document revision browsing and restoration.

## 10. Application Architecture

The selected stack is:

```text
Tauri 2
├── React + TypeScript + Vite frontend
├── Rust application core
├── SQLite persistence
└── Rust `craftel` CLI using the same core library
```

The Rust core owns:

- Project registry
- Task IDs and generated task documents
- Workflow transitions
- SQLite access and migrations
- Document watching and snapshots
- Harness process lifecycle
- Cursor NDJSON parsing
- Run cancellation and recovery

The React frontend owns presentation and user interaction. It does not directly write project files or launch agents.

The CLI links to the same core behavior and does not require the GUI process to be running.

## 11. Failure and Recovery Behavior

- A non-zero Cursor exit marks the run Failed and preserves stderr and events.
- A successful process without `craftel pass` or `craftel fail` leaves the task in its current stage and shows a missing-transition notice.
- On application startup, stale Running records are reconciled with live child processes. Missing processes become Interrupted.
- Interrupted runs are never resumed automatically because the working tree may be in an unknown state.
- Task-document writes use atomic replacement.
- SQLite updates that affect task metadata and transition history use transactions.
- Deleting or moving a project directory does not silently delete the project registration or historical run data.

## 12. Development and Delivery Environment

Amp orbs run Debian 12. They can develop and verify the React frontend, Rust core, CLI, SQLite behavior, Cursor adapter, and Linux Tauri integration. A Vite development server can be exposed through an Amp portal for browser-based UI review.

Orbs cannot build or validate a real macOS application because they do not provide Xcode, AppKit, SwiftUI, macOS SDKs, signing, or notarization. macOS Tauri packaging and native GUI verification must run on a Mac, preferably through a local Amp runner.

The MVP distribution target is:

- macOS on Apple Silicon
- Local development builds
- No Intel package
- No Developer ID signing or notarization
- No automatic updater

## 13. Future Harnesses

The harness boundary exposes only the capabilities CRAFTEL needs:

```text
start
resume
stop
stream events
report final result
```

An Amp adapter can later map a phase session to an Amp thread and choose among local execution, a macOS runner, or an orb. Shared sessions, parallel agents, and worktree isolation are later features, not MVP requirements.

## 14. MVP Acceptance Criteria

The MVP is complete when a user can:

1. Register and switch between local projects.
2. Create a task with required title and content through GUI or CLI.
3. See the generated task directory, `TASK.md`, and `SPEC.md`.
4. Move a task without starting an agent.
5. Explicitly start and stop a Cursor run for the current phase.
6. Watch assistant and tool events in the GUI while the run executes.
7. Reopen the project and view persisted session and run history.
8. Resume Defining or Implementation with a follow-up message.
9. Start Review in a fresh session.
10. Use `craftel pass`, `craftel fail`, and `craftel next` to perform the documented transitions.
11. Keep an approved task in Reviewing until a human advances it to Done.
12. Edit agent-owned Markdown normally and recover a previous snapshot after modification or deletion.

## 15. Explicit Non-Goals for the MVP

- GitHub or GitLab PR/MR creation
- Git commits or pushes performed by workflow automation
- Multi-agent execution on one task
- Git worktree isolation
- Live steering of a running Cursor process
- Cursor IDE chat synchronization
- Configurable workflow columns
- Team or cloud synchronization of the CRAFTEL database
- Intel macOS, Windows, or Linux product releases
- Signing, notarization, and automatic updates
