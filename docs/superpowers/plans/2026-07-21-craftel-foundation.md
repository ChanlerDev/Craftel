# CRAFTEL Foundation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the first working CRAFTEL vertical slice: register and switch local projects, create durable tasks, transition them through the five-stage workflow from CLI or GUI, and generate the project-owned Markdown workspace.

**Architecture:** Use a Cargo workspace with a reusable `craftel-core` crate, a thin `craftel` CLI, and a Tauri 2 desktop application. SQLite is authoritative for project registrations and fixed task fields; `TASK.md` is generated atomically, while `SPEC.md` is initialized once and left agent-editable. The React frontend uses a small API boundary so domain behavior remains in Rust.

**Tech Stack:** Rust stable, Tauri 2, React, TypeScript, Vite, pnpm, SQLite via `rusqlite`, Clap, Serde, Vitest, Testing Library

**Design:** `docs/plans/2026-07-21-craftel-design.md`

---

## Delivery Roadmap

This plan implements only Phase 1. Later phases receive separate plans after this foundation is merged and validated.

| Phase | Outcome | Included here |
| --- | --- | --- |
| 1. Foundation | Project registry, task model, generated documents, CLI, five-column board | Yes |
| 2. Documents | File watcher, Markdown workspace, search, snapshots, restore | No |
| 3. Cursor Harness | Phase sessions, runs, NDJSON streaming, stop, follow-up | No |
| 4. Automation | Skill prompts, pass/fail-driven execution, review loop, crash recovery | No |
| 5. macOS Delivery | Native validation, packaging, signing/updater decisions | No |

## Chunk 1: Repository and Domain Foundation

### Task 1: Scaffold the workspace

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `crates/craftel-core/Cargo.toml`
- Create: `crates/craftel-core/src/lib.rs`
- Create: `crates/craftel-cli/Cargo.toml`
- Create: `crates/craftel-cli/src/main.rs`
- Create: `package.json`
- Create: `pnpm-workspace.yaml`
- Create: `.gitignore`
- Create: `.agents/setup`

- [ ] **Step 1: Create the Rust workspace**

Use a root virtual workspace with resolver 2 and only the members that exist at this point: `crates/craftel-core` and `crates/craftel-cli`. Task 6 adds `apps/desktop/src-tauri` to the workspace when that crate is created. Pin stable Rust in `rust-toolchain.toml`.

- [ ] **Step 2: Create minimal core and CLI crates**

`craftel-core` must expose a public `VERSION` equal to `env!("CARGO_PKG_VERSION")`. The CLI must use Clap and provide `craftel --version`; do not add product commands yet.

- [ ] **Step 3: Create the pnpm workspace**

The root `package.json` must be private and provide these scripts:

```json
{
  "scripts": {
    "check": "pnpm check:rust && pnpm check:web",
    "check:rust": "cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace",
    "check:web": "pnpm --filter @craftel/desktop test && pnpm --filter @craftel/desktop build"
  }
}
```

- [ ] **Step 4: Add Orb setup**

`.agents/setup` must be executable, idempotent, install pnpm dependencies with a frozen lockfile when present, and install Debian packages required to compile Tauri 2 on Debian 12. Keep macOS-specific work out of this script.

- [ ] **Step 5: Verify the empty workspace**

Run:

```bash
cargo fmt --check
cargo test --workspace
cargo run -p craftel-cli -- --version
```

Expected: all commands succeed and the last command prints the package version.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml rust-toolchain.toml crates package.json pnpm-workspace.yaml .gitignore .agents/setup
git commit -m "chore: scaffold CRAFTEL workspace"
```

### Task 2: Define project, task, and workflow domain types

**Files:**
- Create: `crates/craftel-core/src/domain/mod.rs`
- Create: `crates/craftel-core/src/domain/project.rs`
- Create: `crates/craftel-core/src/domain/task.rs`
- Create: `crates/craftel-core/src/domain/workflow.rs`
- Modify: `crates/craftel-core/src/lib.rs`
- Test: inline unit tests in each domain module

- [ ] **Step 1: Write failing workflow tests**

Cover the complete action-by-stage matrix, including invalid actions:

```rust
assert_eq!(Stage::Inbox.next(false), Ok(Stage::Defining));
assert_eq!(Stage::Defining.pass(), Ok(Transition::Move(Stage::Implementation)));
assert_eq!(Stage::Implementation.pass(), Ok(Transition::Move(Stage::Reviewing)));
assert_eq!(Stage::Reviewing.pass(), Ok(Transition::ReviewApproved));
assert_eq!(Stage::Reviewing.fail(), Ok(Transition::Move(Stage::Implementation)));
assert!(Stage::Reviewing.next(false).is_err());
assert_eq!(Stage::Reviewing.next(true), Ok(Stage::Done));
```

Also cover:

- `next` from Defining and Implementation;
- `next` from Done is rejected;
- `pass` in Reviewing records approval without moving;
- `fail` in Defining and Implementation stays in place but emits a failed workflow event;
- automated `pass` and `fail` are rejected in Inbox and Done;
- manual `move` accepts every valid target stage and never starts an external process;
- leaving Reviewing or moving back to Implementation clears stale review approval;
- lowercase Serde values and `Display` output for all five stages.

- [ ] **Step 2: Run the tests and verify failure**

Run: `cargo test -p craftel-core domain::workflow`

Expected: FAIL because the domain types do not exist.

- [ ] **Step 3: Implement the domain types**

Required public shapes:

```rust
pub enum Stage { Inbox, Defining, Implementation, Reviewing, Done }
pub enum Transition { Move(Stage), Stay, ReviewApproved }
pub enum WorkflowAction { Move(Stage), Next, Pass, Fail }
pub enum WorkflowOutcome { Moved, Stayed, ReviewApproved, PhaseFailed }

pub struct Project {
    pub id: String,
    pub name: String,
    pub work_dir: PathBuf,
    pub available: bool,
    pub created_at: DateTime<Utc>,
    pub last_opened_at: DateTime<Utc>,
}

pub struct Task {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub content: String,
    pub stage: Stage,
    pub relative_dir: PathBuf,
    pub review_approved: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Use typed errors for invalid transitions. Do not encode execution/run states in `Task`.
Every successful workflow action produces a workflow event containing task ID, action, from stage, to stage, outcome, and timestamp. Implement the tests as a table covering every `WorkflowAction × Stage` combination, including manual self-moves. Manual `Move` is unconstrained, while `Next`, `Pass`, and `Fail` use the approved transition table. Review approval is valid only while the task remains in the same Reviewing cycle; clear it whenever the task leaves Reviewing or any manual move enters Reviewing, including a Reviewing-to-Reviewing self-move.

- [ ] **Step 4: Run domain tests**

Run: `cargo test -p craftel-core domain`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/craftel-core
git commit -m "feat: define project and task workflow domain"
```

### Task 3: Add SQLite storage and migrations

**Files:**
- Modify: `crates/craftel-core/Cargo.toml`
- Create: `crates/craftel-core/migrations/001_foundation.sql`
- Create: `crates/craftel-core/src/storage/mod.rs`
- Create: `crates/craftel-core/src/storage/sqlite.rs`
- Create: `crates/craftel-core/src/storage/error.rs`
- Create: `crates/craftel-core/src/app_paths.rs`
- Modify: `crates/craftel-core/src/lib.rs`
- Test: `crates/craftel-core/tests/sqlite_repository.rs`

- [ ] **Step 1: Write failing repository tests**

Use a temporary SQLite file, not an in-memory database, so connection reopening is tested. Cover:

- migrations apply to an empty database;
- registering a project normalizes and persists its absolute path;
- the same work directory cannot be registered twice;
- a missing registered directory remains listable with `available: false`;
- removing a registration deletes database rows but never project files;
- projects are ordered by `last_opened_at` descending;
- allocating task IDs produces `T0001`, then `T0002` within one project;
- two independently opened repository connections cannot allocate the same task number;
- foreign keys remain enabled after closing and reopening the repository;
- task title, content, stage, path, and review approval survive reopen;
- transition and task update happen in one transaction;
- CLI and desktop default to the exact same database path, including a shared `CRAFTEL_DB_PATH` override.

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p craftel-core --test sqlite_repository`

Expected: FAIL because the repository is not implemented.

- [ ] **Step 3: Add the migration**

Create `projects`, `task_counters`, `tasks`, and `workflow_events`. Add uniqueness constraints for `projects.work_dir` and `(tasks.project_id, tasks.task_number)`. Add `projection_dirty` to tasks so failed `TASK.md` projection writes are recoverable. Store timestamps as RFC 3339 text and stages as lowercase text with a database check constraint.

Enable and verify `PRAGMA foreign_keys = ON` on every `SqliteRepository::open`; a migration-time pragma is insufficient. Allocate task numbers by incrementing the per-project counter inside an immediate write transaction. Gaps after compensated failures are acceptable; duplicate IDs are not.

Put default application database path resolution and parent-directory creation in `craftel-core::app_paths`. Both CLI and desktop must call this function and honor `CRAFTEL_DB_PATH` identically.

- [ ] **Step 4: Implement `SqliteRepository`**

Required operations:

```rust
pub fn open(path: &Path) -> Result<Self, StorageError>;
pub fn register_project(&mut self, name: &str, work_dir: &Path) -> Result<Project, StorageError>;
pub fn list_projects(&self) -> Result<Vec<Project>, StorageError>;
pub fn touch_project(&mut self, id: &str) -> Result<Project, StorageError>;
pub fn remove_project(&mut self, id: &str) -> Result<(), StorageError>;
pub fn create_task(&mut self, input: NewTask) -> Result<Task, StorageError>;
pub fn get_task(&self, project_id: &str, task_id: &str) -> Result<Task, StorageError>;
pub fn list_tasks(&self, project_id: &str) -> Result<Vec<Task>, StorageError>;
pub fn update_task(&mut self, input: UpdateTask) -> Result<Task, StorageError>;
pub fn apply_transition(&mut self, project_id: &str, task_id: &str, action: WorkflowAction) -> Result<Task, StorageError>;
pub fn mark_projection_clean(&mut self, project_id: &str, task_id: &str) -> Result<(), StorageError>;
pub fn delete_new_task_after_projection_failure(&mut self, project_id: &str, task_id: &str) -> Result<(), StorageError>;
```

Keep SQL and row mapping inside the storage module. Do not leak `rusqlite` errors through the public API. Project path registration requires an existing directory and stores its canonical absolute path; duplicate symlink aliases are rejected. `Project.available` is computed when projects are returned and must appear consistently in core values, CLI JSON, Tauri responses, and TypeScript types. Use deterministic secondary ordering by ID when timestamps are equal.

- [ ] **Step 5: Run repository tests**

Run: `cargo test -p craftel-core --test sqlite_repository`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/craftel-core
git commit -m "feat: persist projects and tasks in SQLite"
```

## Chunk 2: Project Documents and CLI

### Task 4: Generate the project workspace and task documents

**Files:**
- Modify: `crates/craftel-core/Cargo.toml`
- Create: `crates/craftel-core/src/documents/mod.rs`
- Create: `crates/craftel-core/src/documents/slug.rs`
- Create: `crates/craftel-core/src/documents/task_document.rs`
- Create: `crates/craftel-core/src/service.rs`
- Modify: `crates/craftel-core/src/lib.rs`
- Test: `crates/craftel-core/tests/task_service.rs`

- [ ] **Step 1: Write failing service tests**

Cover:

- title and content reject blank values after trimming;
- Unicode titles produce a non-empty stable slug, with `task` as final fallback;
- creating a task initializes `craftel/INDEX.md` if absent;
- task directory is `craftel/tasks/T0001-<slug>`;
- `TASK.md` contains parseable YAML with ID, title, stage, and timestamps;
- `TASK.md` body contains full content and the managed-file notice;
- `SPEC.md` is initialized with a link to `TASK.md` and is never overwritten by metadata updates;
- metadata updates atomically regenerate `TASK.md`;
- a filesystem failure does not leave an apparently healthy task row without its required files;
- failed metadata and workflow projection writes leave SQLite authoritative, mark the projection dirty, and repair on the next service open or task read;
- repair never overwrites an existing agent-authored `SPEC.md`;
- cleanup never removes a task directory that existed before the attempted operation.

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p craftel-core --test task_service`

Expected: FAIL because `CraftelService` does not exist.

- [ ] **Step 3: Implement slug generation and templates**

Slug rules:

- trim and lowercase;
- transliterate when the chosen small dependency supports it;
- replace non-alphanumeric runs with one hyphen;
- trim hyphens;
- limit to 48 characters without leaving a trailing hyphen;
- fall back to `task`.

Render YAML using a serializer rather than string concatenation. Render the Markdown body separately. Write to a sibling temporary file, flush, and rename over `TASK.md`.

`INDEX.md` is initialized once in Phase 1 and is not regenerated or synchronized. Automatic index maintenance belongs to the later document phase.

- [ ] **Step 4: Implement `CraftelService`**

The service coordinates repository transactions and filesystem effects. Expose project registration/listing/removal, task creation/listing/update, and workflow actions. SQLite and the filesystem cannot share one transaction, so implement this explicit protocol:

1. **Create:** allocate and insert the task with `projection_dirty = true` in an immediate SQLite transaction; create a uniquely named temporary task directory; write and flush `TASK.md` and `SPEC.md`; rename the temporary directory to its final path; mark the projection clean. On failure, compensate by deleting the new row and only the temporary/final directory proven to have been created by this operation. Never remove a pre-existing directory. A newly initialized project-level `craftel/INDEX.md` may remain after a failed first task creation.
2. **Update or workflow action:** commit authoritative SQLite state with `projection_dirty = true`; atomically rewrite only `TASK.md`; mark clean after the rename succeeds. If projection fails, return a projection error while retaining the valid database update.
3. **Repair:** on service open and task read, regenerate missing/dirty `TASK.md` from SQLite and mark clean. Service-open repair skips unavailable project directories, leaves their projections dirty, and must not prevent the shared database from opening or those registrations from being listed or removed. Create `SPEC.md` only when the entire task is first created; repair must never overwrite or recreate a deliberately deleted agent-owned `SPEC.md` in Phase 1.

Add focused failure-injection tests for create, update, and workflow actions. A test-only filesystem adapter is acceptable when needed for deterministic failure points.

- [ ] **Step 5: Run service tests**

Run: `cargo test -p craftel-core --test task_service`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/craftel-core
git commit -m "feat: generate durable task documents"
```

### Task 5: Implement the `craftel` CLI

**Files:**
- Modify: `crates/craftel-cli/Cargo.toml`
- Create: `crates/craftel-cli/src/args.rs`
- Create: `crates/craftel-cli/src/output.rs`
- Modify: `crates/craftel-cli/src/main.rs`
- Test: `crates/craftel-cli/tests/cli.rs`

- [ ] **Step 1: Write failing CLI tests**

Use `assert_cmd` and isolated temporary values for both the application database and project directory. Cover:

```text
craftel project add <path> --name <name>
craftel project list --json
craftel project remove <project-id>
craftel create --project <project-id> --title <title> --content <content>
craftel task list --project <project-id> --json
craftel task update T0001 --project <project-id> --title <title> --content <content>
craftel move T0001 <stage> --project <project-id>
craftel next T0001 --project <project-id>
craftel pass T0001 --project <project-id>
craftel fail T0001 --project <project-id>
```

Also test that commands infer the registered project whose canonical work directory is the longest ancestor of the canonical current directory, explicit `--project` wins, missing directories are reported without being silently removed, project removal leaves all files untouched, and invalid transitions return non-zero with a concise stderr message.

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p craftel-cli --test cli`

Expected: FAIL because commands are missing.

- [ ] **Step 3: Implement command parsing and database selection**

Call the shared `craftel-core::app_paths` resolver. Support `CRAFTEL_DB_PATH` for tests and advanced local use. Human output goes to stdout, errors to stderr, and `--json` returns stable serialized objects suitable for agent use.

- [ ] **Step 4: Implement CLI commands as thin service calls**

Do not duplicate validation, transitions, task ID generation, or document rendering in the CLI crate.

- [ ] **Step 5: Run CLI and workspace tests**

Run:

```bash
cargo test -p craftel-cli --test cli
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 6: Manually exercise the CLI**

Run against a temporary directory and verify that `TASK.md` and `SPEC.md` are created, a move does not run any external process, and `pass/fail/next` follow the approved transition table.

- [ ] **Step 7: Commit**

```bash
git add crates/craftel-cli
git commit -m "feat: add project and task CLI workflows"
```

## Chunk 3: Desktop Vertical Slice

### Task 6: Scaffold Tauri and expose typed commands

**Files:**
- Create: `apps/desktop/package.json`
- Create: `apps/desktop/tsconfig.json`
- Create: `apps/desktop/vite.config.ts`
- Create: `apps/desktop/index.html`
- Create: `apps/desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/src-tauri/build.rs`
- Create: `apps/desktop/src-tauri/tauri.conf.json`
- Create: `apps/desktop/src-tauri/capabilities/default.json`
- Create: `apps/desktop/src-tauri/src/lib.rs`
- Create: `apps/desktop/src-tauri/src/main.rs`
- Create: `apps/desktop/src-tauri/src/commands.rs`
- Create: `apps/desktop/src-tauri/src/state.rs`
- Modify: `Cargo.toml`
- Test: inline Rust command tests where practical

- [ ] **Step 1: Scaffold a minimal Tauri 2 application**

Use the existing workspace rather than creating a nested Git repository, then add `apps/desktop/src-tauri` to the root Cargo workspace. Configure the bundle identifier as `dev.chanler.craftel`. Add the official Tauri 2 dialog plugin and grant only its directory-open permission plus required core window permissions.

- [ ] **Step 2: Add desktop state**

Open the database through the shared `craftel-core::app_paths` resolver used by the CLI. Keep mutable access behind a Tauri-managed mutex. Errors returned across IPC must be serializable messages, not panics or raw database internals.

- [ ] **Step 3: Expose commands**

Required Tauri commands:

```text
register_project
list_projects
open_project
remove_project
create_task
list_tasks
update_task
move_task
next_task
pass_task
fail_task
```

All commands delegate to `CraftelService`.

- [ ] **Step 4: Verify Rust integration**

Run:

```bash
cargo check -p craftel-desktop
cargo test -p craftel-desktop
```

Expected: PASS on Debian after Orb setup and on macOS locally.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri apps/desktop/package.json apps/desktop/tsconfig.json apps/desktop/vite.config.ts apps/desktop/index.html
git commit -m "feat: expose CRAFTEL core through Tauri"
```

### Task 7: Build the project switcher and five-column board

**Files:**
- Create: `apps/desktop/src/main.tsx`
- Create: `apps/desktop/src/App.tsx`
- Create: `apps/desktop/src/api/types.ts`
- Create: `apps/desktop/src/api/craftel.ts`
- Create: `apps/desktop/src/api/tauri.ts`
- Create: `apps/desktop/src/components/ProjectSwitcher.tsx`
- Create: `apps/desktop/src/components/CreateTaskDialog.tsx`
- Create: `apps/desktop/src/components/EditTaskDialog.tsx`
- Create: `apps/desktop/src/components/Board.tsx`
- Create: `apps/desktop/src/components/BoardColumn.tsx`
- Create: `apps/desktop/src/components/TaskCard.tsx`
- Create: `apps/desktop/src/styles.css`
- Create: `apps/desktop/src/test/setup.ts`
- Test: `apps/desktop/src/components/ProjectSwitcher.test.tsx`
- Test: `apps/desktop/src/components/CreateTaskDialog.test.tsx`
- Test: `apps/desktop/src/components/EditTaskDialog.test.tsx`
- Test: `apps/desktop/src/components/Board.test.tsx`

- [ ] **Step 1: Write failing component tests**

Cover:

- loading and switching registered projects;
- selecting a directory and registering it;
- displaying a missing-directory state and removing its registration without deleting files;
- requiring non-blank title and content;
- rendering all five columns in approved order;
- grouping tasks by stage;
- rendering ID, title, and content excerpt;
- dragging a task calls `moveTask` exactly once and does not call any run operation;
- failed moves restore the card to its previous column and display an error;
- task creation refreshes the board;
- editing title/content updates the card while preserving task path and `SPEC.md`.

Use an injected `CraftelApi` fake. Do not mock Tauri internals in component tests.

- [ ] **Step 2: Run and verify failure**

Run: `pnpm --filter @craftel/desktop test`

Expected: FAIL because components do not exist.

- [ ] **Step 3: Implement the API boundary**

`CraftelApi` mirrors the Tauri command surface with camelCase TypeScript methods. Keep Tauri `invoke` calls in `api/tauri.ts`; components receive the interface through context or props.

- [ ] **Step 4: Implement project and task creation flows**

Keep the first-run empty state useful: explain that a CRAFTEL project is a local working directory and provide an Open Project button. Preserve the last selected project in SQLite through `open_project`, not browser local storage. Missing projects remain visible with Locate-later guidance and a Remove Registration action that explicitly states files are untouched.

- [ ] **Step 5: Implement minimal task editing**

Provide an Edit action for title and content. A successful update refreshes the card while retaining the immutable task directory; errors keep the dialog open. Do not add a general Markdown editor.

- [ ] **Step 6: Implement the board**

Use a maintained drag-and-drop library compatible with React. Keyboard movement must be possible through the library's accessible sensors. Do not implement run buttons or Markdown editing in Phase 1.

- [ ] **Step 7: Run frontend checks**

Run:

```bash
pnpm --filter @craftel/desktop test
pnpm --filter @craftel/desktop build
```

Expected: PASS.

- [ ] **Step 8: Run the web UI for Orb review**

Start the Vite server with a mock/dev API mode containing representative tasks in all columns. Expose it through an Amp portal and visually verify the empty state, populated board, task dialog, long titles, long content, and narrow window behavior. The mock mode must be development-only and impossible to enable in a production build.

- [ ] **Step 9: Commit**

```bash
git add apps/desktop/src apps/desktop/package.json pnpm-lock.yaml
git commit -m "feat: add project switcher and task board"
```

### Task 8: Add project guidance and complete Phase 1 verification

**Files:**
- Create: `README.md`
- Create: `AGENTS.md`
- Create: `craftel/INDEX.md`
- Create: `craftel/tasks/T0001-craftel-foundation/TASK.md`
- Create: `craftel/tasks/T0001-craftel-foundation/SPEC.md`
- Create: `craftel/tasks/T0001-craftel-foundation/reviews/implementation-acceptance.md`
- Modify: `.agents/setup`

- [ ] **Step 1: Document developer setup**

README must explain prerequisites, Orb setup, macOS local setup, `pnpm check`, running the CLI, running Vite, running Tauri, and the Phase 1 limitations.

- [ ] **Step 2: Add agent guidance**

AGENTS.md must state:

- `TASK.md` files are CRAFTEL-generated and must not be edited directly;
- agent-authored details belong in `SPEC.md` and supporting directories;
- use `craftel next/pass/fail` for workflow changes;
- moving a task never starts an agent;
- no Git push, PR, release, or delivery action is implicit.

- [ ] **Step 3: Record the foundation task**

Create static repository documentation only; this step does not run a skill, create an agent session, or perform a workflow transition. `TASK.md` contains YAML `id: T0001` and the valid workflow `status: done`, the approved foundation Goal, artifact links, and chronological history. `SPEC.md` links to `TASK.md` and records the Phase 1 product/domain/technical contracts. `reviews/implementation-acceptance.md` contains exactly these sections:

```markdown
# Implementation Acceptance

## Change Summary
## Specification Deviations
## Acceptance Evidence
| Criterion | Evidence | Result |
## Verification Commands
| Command or check | Result | Notes |
## Remaining Risks
```

The acceptance record maps every Phase 1 criterion to concrete automated or manual evidence. It does not add run/session tables, review automation, or later-phase behavior.

- [ ] **Step 4: Run complete verification**

Run:

```bash
pnpm check
git diff --check
```

Expected: all checks pass.

- [ ] **Step 5: Validate on macOS**

On a live macOS runner or local machine, run the Tauri application and verify:

1. Register a temporary project.
2. Create a task with title and content.
3. Confirm generated `TASK.md` and `SPEC.md`.
4. Drag the card through columns without spawning a process.
5. Restart the app and confirm persisted state.
6. Run equivalent `craftel` CLI transitions and confirm the GUI reflects them after refresh.

If a macOS runner is unavailable, record this step as unverified rather than claiming success.

- [ ] **Step 6: Commit**

```bash
git add README.md AGENTS.md craftel .agents/setup
git commit -m "docs: document and accept CRAFTEL foundation"
```

## Phase 1 Definition of Done

- Rust core tests prove project registration, task persistence, document generation, and every workflow transition.
- CLI integration tests prove GUI-independent project and task operations, missing-directory reporting, and safe registration removal.
- React tests prove project switching, missing-directory behavior, required creation fields, task editing, board grouping, and drag behavior.
- The web production build succeeds.
- The Rust workspace passes formatting, Clippy, and tests.
- The Orb provides a portal for reviewing the board UI.
- macOS native verification is completed on a runner or explicitly reported as pending.
- No Cursor process, run/session schema, file watcher, or document editor is added prematurely.
