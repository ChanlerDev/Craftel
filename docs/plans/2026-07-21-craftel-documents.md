# CRAFTEL Phase 2: Document Workspace Plan

> Execute only after the foundation is green. Follow `implement-spec`: test first, complete each checked slice, and commit at the stated boundaries.

**Goal:** Make agent-owned Markdown discoverable, searchable, revisioned, deletable, and recoverable on Linux and macOS without changing the Phase 1 ownership model.

**Authoritative behavior:** SQLite remains authoritative for task metadata and generated `TASK.md`; disk is authoritative for `INDEX.md` and agent-owned Markdown. The index and snapshots are application projections. Watch `<workDir>/craftel/**`, but revision only UTF-8 Markdown at `INDEX.md`, task-root `SPEC.md`, and lazy `decisions|discussions|notes|subtasks|plans|reviews/**/*.md`. Never ingest temporary files, symlinks, paths outside canonical `craftel`, or generated `TASK.md`. A startup scan and watcher use the same idempotent ingest operation.

## Locked contracts

- Store project-relative, slash-normalized paths; identity is `(project_id, relative_path)`. Reject absolute paths and `..`.
- `document_index`: one current row per identity with task ID when under a known immutable task directory, title (first Markdown heading or filename), body, content SHA-256, file mtime/size, `present`, and `indexed_at`. Search is project-scoped SQLite FTS5 over title/body and returns deterministic rank then path ordering.
- `document_snapshots`: immutable ID, identity, hash, complete bytes, `captured_at`, and cause `scan|watch|edit|restore`. Ingest skips a snapshot when its hash equals the latest snapshot; older equal content may recur, and restore always records a new row. Deletion changes the index to `present=false`; it does not create an empty snapshot and preserves the latest snapshot.
- One per-project coordinator serializes scans, watcher events, edits, restores, and pruning. Debounce a path until 250 ms quiet; rename is modeled as delete old plus ingest new. Read only after metadata is stable across two observations; retry bounded transient not-found/permission errors, then surface a document error without damaging prior index/snapshots.
- Initial scan reconciles every eligible disk file and marks missing indexed files deleted. Watch overflow/error triggers a full reconciliation. Unavailable projects retain history and report unavailable; registration removal follows existing cascading database ownership and never touches files.
- Editing and restoring use sibling-temp write, flush, atomic rename, then synchronous ingest. Restore requires a snapshot belonging to the same project/path, writes its bytes, and records a new `restore` snapshot even when restoring content seen before. Restore of a deleted file recreates parent directories only inside an existing known task workspace. Concurrent external change is rejected by expected-current-hash, not overwritten.
- Retention runs after ingest and on startup: delete snapshots older than 30 days, then retain only the newest 100 per file even if more were created within 30 days; always preserve the newest snapshot, including for deleted files. Timestamps are UTC RFC 3339. Pruning is transactional and never removes file content.
- APIs return typed `not_found`, `conflict`, `invalid_path`, `invalid_utf8`, `unavailable`, and `io` errors. Watch notifications carry only project/path/change; clients re-query durable state.

**Strict non-goals:** non-Markdown/binary history, generated `TASK.md` editing/history, Git integration, cloud sync, collaborative editing, configurable retention/search engines, worktrees, and UI beyond typed API seams (Phase 5).

## Slice 2.1 — Schema and deterministic ingestion

**Files:** add `002_documents.sql`, document domain/repository modules, repository integration tests; update migration registration and exports.

- [ ] Write failing file-backed SQLite tests for migration/reopen, FTS search scope/order, path validation, hash de-duplication, deletion, restore-cause duplication, retention boundaries, and project cascade.
- [ ] Add `document_index`, FTS triggers/table, and `document_snapshots`; keep migration additive and compatible with `001_foundation.sql`.
- [ ] Implement a repository transaction that upserts index + snapshot atomically and a deterministic retention query.
- [ ] Verify: `cargo test -p craftel-core --test document_repository`.
- [ ] Commit: `feat: persist document index and snapshots`.

## Slice 2.2 — Scanner, watcher, deletion, and recovery

**Files:** add `documents/path.rs`, `documents/indexer.rs`, `documents/watcher.rs`; update service lifecycle; add deterministic filesystem tests.

- [ ] Write failing tests using temporary projects and injectable clock/debounce: initial reconciliation, ignored paths/symlinks/temp files/`TASK.md`, burst coalescing, rename, delete, watcher overflow rescan, transient read retry, and independent project coordinators.
- [ ] Implement one shared ingest/reconcile path and Linux-capable notify watcher; ensure service shutdown joins watchers and startup never fails solely because a registered project is unavailable.
- [ ] Prove two service instances cannot corrupt state: SQLite uniqueness/transactions make duplicate events idempotent; document writes use expected hashes. MVP does not coordinate two live watchers beyond this.
- [ ] Verify: `cargo test -p craftel-core --test document_watcher -- --test-threads=1 && cargo test -p craftel-core documents`.
- [ ] Commit: `feat: watch and reconcile project documents`.

## Slice 2.3 — Service/API document operations

**Files:** extend core service, Tauri commands/state, and frontend `CraftelApi`/types; add Rust command/API contract tests.

- [ ] Write failing service tests for tree/list/read/search/edit, expected-hash conflict, deletion visibility, revision listing, restore after edit/deletion, cross-project snapshot rejection, and unavailable project errors.
- [ ] Expose `list_documents`, `read_document`, `search_documents`, `write_document`, `list_document_revisions`, and `restore_document_revision`. Return relative paths and hashes; never return host paths except existing project metadata.
- [ ] Emit one Tauri `document_changed` notification after durable ingest; no direct frontend filesystem access.
- [ ] Verify: `cargo test -p craftel-core --test document_service && cargo test -p craftel-desktop && pnpm --filter @craftel/desktop test && pnpm --filter @craftel/desktop build`.
- [ ] Commit: `feat: expose document workspace operations`.

## Phase verification and acceptance

- [ ] Fresh Linux gate: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm --filter @craftel/desktop test && pnpm --filter @craftel/desktop build && git diff --check`.
- [ ] Manual Linux check: edit, rapidly rewrite, rename, and delete `SPEC.md`; restart; verify search/current state/revisions; restore deletion and confirm a new revision.
- [ ] macOS pending check: run the same scenario in Tauri and verify FSEvents rename/burst behavior and atomic replacement. Record pending if no Mac is available; signing, packaging, notarization, and updater checks are excluded.

**Phase acceptance:** all authoritative behaviors above have automated evidence; restart rebuilds current projection without losing history; restore is conflict-safe; latest deleted content survives retention.
