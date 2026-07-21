# CRAFTEL Foundation Specification

Task metadata: [TASK.md](./TASK.md)

## Product contract

Phase 1 registers, switches, and safely removes local project registrations; creates and edits durable tasks; displays five workflow columns; and permits explicit CLI or GUI movement. Movement never launches an agent. Project removal never deletes project files.

## Domain contract

SQLite is authoritative for project registrations, task fields, stage, review approval, and workflow events. Stages are Inbox, Defining, Implementation, Reviewing, and Done. Manual moves are unconstrained; `next`, `pass`, and `fail` follow the approved transition matrix, and an approved review remains in Reviewing until explicit human advancement.

## Document contract

Tasks use stable project-local IDs and directories. CRAFTEL atomically projects metadata to `TASK.md`; `SPEC.md` is initialized once and remains agent-owned. Dirty/missing task projections are repairable without recreating a deleted `SPEC.md`. `INDEX.md` is static in Phase 1.

## Technical contract

The Cargo workspace shares `craftel-core` between a Clap CLI and Tauri 2 commands. The desktop is React/TypeScript/Vite behind a typed API boundary. Both applications resolve the same SQLite path, including `CRAFTEL_DB_PATH`. Phase 1 excludes watchers, document history/search/editing, process execution, run/session storage, and delivery automation.
