# CRAFTEL

CRAFTEL is a local, project-oriented workspace for AI-assisted software development. The Linux-capable MVP combines a shared Rust core, the `craftel` CLI, and a Tauri 2/React desktop UI. It provides the five-stage task board, indexed and revisioned Markdown documents, persisted Cursor Agent sessions/runs/events, explicit phase automation, streamed activity, follow-ups, and human-controlled review completion. SQLite owns application state while project-visible Markdown remains the task workspace.

## Prerequisites

- Rust stable (the repository's `rust-toolchain.toml` selects it)
- Node.js 20.19+ or 22.12+ and Corepack/pnpm 9
- Platform prerequisites for [Tauri 2](https://v2.tauri.app/start/prerequisites/)

## Development setup

In an Amp orb, run the idempotent setup script from the repository root:

```bash
.agents/setup
```

It installs Debian 12 Tauri build libraries, installs stable Rust when absent, provides a compatible verified Node LTS runtime when needed, enables Corepack, and installs the frozen pnpm lockfile. It does not start a persistent service or alter macOS hosts.

On macOS, install Xcode Command Line Tools and the WebKit/Tauri prerequisites, install a compatible Node.js and Corepack, then run:

```bash
rustup toolchain install stable
corepack enable
pnpm install --frozen-lockfile
```

Run the complete repository check with:

```bash
pnpm check
```

This runs Rust formatting, Clippy with warnings denied, all workspace tests, frontend tests, TypeScript, and the production Vite build.

## Running CRAFTEL

Run the CLI (append commands such as `project list --json`):

```bash
cargo run -p craftel-cli -- --help
```

Run the browser-only Vite frontend:

```bash
pnpm --filter @craftel/desktop dev
```

The normal browser page cannot use native Tauri commands. For visual development only, append `?mock=1` to use deterministic sample projects, documents, revisions, and run activity. Mock mode is guarded by `import.meta.env.DEV`, dynamically imported, and unavailable in production builds. In an Amp orb, expose the Vite server through the supervised orb service/portal mechanism rather than backgrounding it.

Run the native application on a Linux or macOS machine with GUI support:

```bash
pnpm --filter @craftel/desktop tauri dev
```

Useful CLI commands include:

```bash
cargo run -p craftel-cli -- project list --json
cargo run -p craftel-cli -- create --title "Task" --content "Details"
cargo run -p craftel-cli -- next T0001
cargo run -p craftel-cli -- pass T0001
cargo run -p craftel-cli -- fail T0001
```

## MVP boundaries and native checks

Debian 12 verifies the Rust core and CLI, SQLite persistence, Linux document watcher, Cursor adapter through a deterministic fake executable, Tauri command layer, React UI, and production web compilation. Authenticated live-Cursor behavior and native Apple Silicon launch, file-dialog, FSEvents, keyboard/drag, restart, and stop behavior remain pending manual checks. The MVP does not include Git delivery, cloud/team sync, worktrees, multi-agent operation, live steering, Cursor IDE chat synchronization, hidden reasoning, configurable workflows, or product release packaging. See [`docs/acceptance/2026-07-21-craftel-linux-mvp.md`](docs/acceptance/2026-07-21-craftel-linux-mvp.md) for exact evidence and the native morning checklist.
