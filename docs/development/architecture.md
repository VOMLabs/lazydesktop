# Architecture

This page describes how LazyDesktop is put together at a high level. For a
file-by-file tour, see [source layout](source-layout.md).

> **Note:** the shipped UI is a pure-Rust application built on **GPUI**
> (`crates/app`). It consumes the backend crates directly as Rust libraries —
> no C ABI, no C++.

## Overview

```
┌─────────────────────────────────────────────────────────┐
│                     App (root view)                     │
│  ┌──────────┐  ┌────────────────────┐  ┌─────────────┐ │
│  │  Toolbar  │  │  Sidebar          │  │  Main       │ │
│  │  - Branch │  │  - Branches       │  │  content    │ │
│  │  - Push   │  │  - History        │  │  - File tree│ │
│  │  - Fetch  │  │  - Projects       │  │  - Diff     │ │
│  │           │  ├───────────────────┤  │  - Image    │ │
│  │           │  │  Commit panel     │  │             │ │
│  └──────────┘  └────────────────────┘  └─────────────┘ │
├─────────────────────────────────────────────────────────┤
│  Backend crates (Rust)                                  │
│  - git_cmd (git CLI)      - config (persistence)       │
│  - watcher (fs watch)     - ai_core (cloud + local)    │
│  - vcs_core (SSH/remotes) - addons                      │
└─────────────────────────────────────────────────────────┘
```

## Layers

### UI layer (`crates/app`)

The GPUI application is a single-window app. The root `App` entity owns the
toolbar, sidebar, file tree, diff view, and commit panel; specialised views
(`Sidebar`, `CommitPanel`, `SettingsView`, `DiffView`) are separate
`Entity<T>` views that emit events back to the root and are driven by
`cx.notify()` re-renders.

Git state flows through `GitService`, a shared service that wraps the
`git_cmd` crate and keeps caches (status, branches, stash count, recent
subjects) that UIs read in their `render` methods.

### Git process layer

All Git operations shell out to the `git` CLI via the `git_cmd` crate —
there is **no libgit2**. Subsequent commands (status, diff, commit,
push/fetch/pull, branch ops, stash, reset, log) are exposed as
`std::process::Command` wrappers, and the app runs them on
`tokio::task::spawn_blocking` workers so the UI stays responsive.

Two operations do **not** shell out: SSH key handling and `git remote`
config reads/writes run natively through the `vcs_core` Rust crate
(`ssh-key`, `russh`, `gix-config`). This removes the `ssh-keygen` / `ssh` /
`git remote` subprocess dependencies from the Settings SSH page and the
Remotes dialog.

### AI layer

Two routes into AI (both in `crates/app/src/commit_panel.rs` and the
`ai_core` crate):

- **Cloud** — `ai_core::cloud` performs HTTP calls to provider APIs
  (OpenRouter, OpenAI, Anthropic, Google AI Studio).
- **Local** — `ai_core::commit_message::generate_commit_message` runs GGUF
  inference via `llama-cpp-2` on a worker thread.

The async pattern is: `cx.spawn` → `tokio::task::spawn_blocking(...)` → on
completion `WeakEntity::update(cx, ...)` to flush the result into the UI.

### Persistence layer

The `config` crate persists:

- `settings.rs` — application settings (INI, `~/.config/lazydesktop/lazydesktop.conf`)
- `projects.rs` — recent projects (`projects.yaml`)
- `themes.rs` — custom themes

Raw GGUF files store local models in `~/.config/lazydesktop/models/`.

## Key design decisions

| Decision | Rationale |
|----------|-----------|
| **Git CLI via `git_cmd`, no libgit2** | Small binary, exact parity with command-line Git |
| **Rust + GPUI, no Electron** | Native KDE integration, no web runtime |
| **No database** | `config` crate (settings + YAML) covers configuration and project state |
| **Async via `spawn_blocking`** | Git and inference run off the UI thread |
| **Rust for AI core** | `llama-cpp-2` is a mature Rust binding; cloud providers are plain HTTP |
| **Native SSH/remotes** | `vcs_core` removes `ssh-keygen` / `ssh` / `git remote` subprocesses |

## Data flow examples

**Status refresh:**

```
watcher → .git/index or .git/HEAD changes
  → GitService::refresh_all()
  → git status/branch/stash calls on a blocking worker
  → cx.notify() → render() reads cached state
```

**Local AI commit message:**

```
checked files → diff
  → build_ai_prompt() (diff, files, branch, recent messages)
  → cx.spawn + spawn_blocking(generate_commit_message)
  → WeakEntity::update(cx, ...) → flush into pending summary/description
  → render() shows a "Generating…" indicator meanwhile
```

See the [implementation details](../../IMPLEMENTATION.md) for the full
technical deep-dive.