# CRAFTEL

CRAFTEL is a local, project-oriented task board for AI-assisted software development. Phase 1 supplies a shared Rust core, the `craftel` CLI, and a Tauri 2/React five-column board. SQLite owns registrations and task metadata; project-visible Markdown provides the task workspace.

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

## Running CRAFTEL

Run the CLI (append commands such as `project list --json`):

```bash
cargo run -p craftel-cli -- --help
```

Run the browser-only Vite frontend with mock data:

```bash
pnpm --filter @craftel/desktop dev
```

In an orb, expose long-lived development servers with the orb service/portal mechanism rather than backgrounding a process. Run the native Tauri application on a machine with native GUI support:

```bash
pnpm --filter @craftel/desktop tauri dev
```

## Phase 1 limitations

Phase 1 includes project registration/switching, durable tasks and generated documents, CLI workflow commands, and board editing/dragging. It intentionally has no document watcher/search/snapshots/editor, agent or Cursor runs and sessions, streaming, automation, packaging, signing, notarization, or updater. `craftel/INDEX.md` is initialized but not synchronized. Debian 12 verifies core, CLI, web, and Linux compilation/tests; native macOS behavior and packaging require a Mac and remain pending for this repository acceptance.
