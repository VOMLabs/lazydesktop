# Architecture

This page describes how LazyDesktop is put together at a high level. For a
file-by-file tour, see [source layout](source-layout.md).

> **Note:** the shipped UI is the Qt 6 Widgets application described below. A
> pure-Rust frontend built on GPUI (`crates/app`) is in development and will
> replace the Qt UI; see [GPUI frontend](#gpui-frontend-cratesapp) at the end
> of this page.

## Overview

```
┌─────────────────────────────────────────────────────────┐
│                     MainWindow                          │
│  ┌──────────┐  ┌────────────────────┐  ┌─────────────┐ │
│  │  Toolbar  │  │   Sidebar Tabs     │  │   Viewer    │ │
│  │  - Branch │  │  ┌──────────────┐ │  │  Stack      │ │
│  │  - Push   │  │  │ Changes Tab  │ │  │  - Diff     │ │
│  │  - Project│  │  │  - File List │ │  │  - Image    │ │
│  │           │  │  │  - Commit    │ │  │  - Placeholder│ │
│  │           │  │  ├──────────────┤ │  │             │ │
│  │           │  │  │ History Tab  │ │  │             │ │
│  │           │  │  │  - Commits   │ │  │             │ │
│  │           │  │  │  - Files     │ │  │             │ │
│  │           │  │  └──────────────┘ │  │             │ │
│  └──────────┘  └────────────────────┘  └─────────────┘ │
├─────────────────────────────────────────────────────────┤
│  Git Process Layer (QProcess)     │  AI Layer           │
│  - status, diff, commit, push     │  - Cloud APIs       │
│  - branch, checkout, log          │  - Rust ai_core    │
│  - clone, init                    │    crate (FFI)     │
├─────────────────────────────────────────────────────────┤
│  Persistence Layer  │
│  - QSettings (INI)   │
│  - YAML (projects,   │
│    themes)           │
└──────────────────────┘
```

## Layers

### UI layer (`src/mainwindow.cpp`)

`MainWindow` owns all application logic: UI setup, git operations, AI
generation, settings, and project management. It is a single `QMainWindow`
with a `QStackedWidget` viewer (diff / placeholder / image), a tabbed sidebar
(Changes / History), and an overlay drawer for recent projects.

### Git process layer

Most Git operations shell out to the `git` CLI via `QProcess` — there is
**no libgit2**. Each operation type gets its own process handle
(`m_gitProcess`, `m_commitProcess`, `m_pushProcess`, `m_branchProcess`,
`m_checkoutProcess`, `m_createBranchProcess`, `m_logProcess`,
`m_commitDetailProcess`, `m_stageProcess`, …).

Two operations do **not** shell out: SSH key handling and `git remote`
config reads/writes run natively through the `vcs_core` Rust crate
(`ssh-key`, `russh`, `gix-config`) over a C ABI (`vcs_core.h`), wrapped in
Qt by `VcsBridge`. This removes the `ssh-keygen` / `ssh` / `git remote`
subprocess dependencies from the Settings SSH page and the Remotes dialog.

### AI layer

Two routes into AI:

- **Cloud** — `QNetworkAccessManager` HTTP calls to provider APIs
  (OpenRouter, OpenAI, Anthropic, Google AI Studio).
- **Local** — the Rust `ai_core` crate (`llama-cpp-2`) compiled as a
  `staticlib`, called through a small C ABI (`ai_core.h`). The C++
  `ModelManagerBridge` wraps the FFI in Qt signals so inference is
  non-blocking.

Model downloads are handled by a **detached worker process**
(`lazydesktop --background-dl …`) that survives UI restarts; progress is
tracked through JSON sidecar files.

### Persistence layer

- `QSettings` (INI) for application settings
- yaml-cpp for `projects.yaml` and custom themes (Qt UI)
- Raw GGUF files for local models

The bundled Rust `config` crate (used by the GPUI frontend) persists projects
and themes as Lua (`projects.lua`, `*.theme.lua`) via `mlua` and exposes the
same data over a C ABI (`config.h`).

## Key design decisions

| Decision | Rationale |
|----------|-----------|
| **Git CLI via QProcess, no libgit2** | Small binary, exact parity with command-line Git |
| **Qt Widgets, no Electron** | Native KDE integration, no web runtime |
| **No database** | `QSettings` + YAML cover configuration and project state |
| **Single-threaded UI, QThread inference** | Git ops are async via QProcess; blocking inference runs on a worker thread |
| **Rust for AI core** | `llama-cpp-2` is a mature Rust binding; exposed to C++ over a C ABI |
| **Detached background downloads** | Large model downloads must not block or die with the UI |

## Data flow examples

**Status refresh:**

```
.git/index or .git/HEAD changes
  → QFileSystemWatcher
  → onRepoDirChanged()
  → 2s debounce timer
  → startGitStatusQuery()  (git status --porcelain)
  → onGitProcessFinished() parses XY codes
  → tree widget updated
```

**Local AI commit message:**

```
checked files → diff
  → UI builds CommitContext JSON (diff, files, branch, recent messages)
  → mm_generate_commit_message(FFI)
  → ai_core builds prompt, runs llama-cpp-2 inference on a thread
  → tokens stream via on_token → UI updates live
  → normalized Conventional Commit message via on_finish
```

See the [implementation details](../../IMPLEMENTATION.md) for the full
technical deep-dive.

## GPUI frontend (`crates/app`)

The in-development frontend replaces the Qt UI with a pure-Rust application
built on **GPUI** (Zed's UI framework, via the `gpui-pre` crate). It shares
the same layered design:

```
┌─────────────────────────────────────────────────────────┐
│                     App (root view)                     │
│  ┌──────────┐  ┌──────────────┐  ┌───────────────────┐  │
│  │ Toolbar  │  │  Sidebar     │  │  Main content     │  │
│  │ - Branch │  │  - Changes   │  │  - File tree      │  │
│  │ - Push   │  │  - History   │  │  - Diff view      │  │
│  └──────────┘  └──────────────┘  └───────────────────┘  │
│  Commit panel (bottom strip)                            │
├─────────────────────────────────────────────────────────┤
│  Backend crates (Rust)                                  │
│  - git_cmd (git CLI)   - config (Lua persistence)       │
│  - watcher (fs watch)  - ai_core (local inference)      │
│  - vcs_core (SSH/remotes)                               │
└─────────────────────────────────────────────────────────┘
```

Key differences from the Qt UI:

- **No C ABI boundary** — the frontend consumes the backend crates
  (`config`, `git_cmd`, `watcher`, `ai_core`, `vcs_core`) directly as Rust
  libraries instead of through staticlib FFI.
- **GPUI idioms** — views are `Entity<T>` with `cx.notify()`-driven
  re-renders; inputs use the `InputState` pattern; styling follows a 4px
  spacing scale (`gap_1` … `gap_6`).
- **Same Git model** — Git operations still shell out to the `git` CLI
  (via `git_cmd`), with SSH/remote handling native in `vcs_core`.
