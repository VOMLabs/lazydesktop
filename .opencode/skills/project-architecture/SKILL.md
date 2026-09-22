---
name: project-architecture
description: Guides codebase navigation, architecture decisions, and structural understanding of LazyDesktop. Trigger when the user asks how the project is structured, where to find things, or how components interact.
---

# Project Architecture Skill

## High-Level Overview

LazyDesktop is a native Git GUI client for KDE Plasma built entirely with Rust:
the UI uses **GPUI** (`crates/app`) and the backend crates form a single Cargo
workspace. There is no C++/Qt code anymore — the `src/` Qt app, `meson.build`,
and the C ABI headers were removed.

## Directory Layout

```
├── Cargo.toml                  # Root workspace definition (7 crates)
├── crates/
│   ├── app/                    # GPUI frontend — the shipped UI
│   ├── ai_core/                # GGUF inference + cloud providers
│   ├── vcs_core/               # SSH keys, git remotes
│   ├── git_cmd/                # Git/jj CLI wrapper
│   ├── config/                 # Settings (INI), projects, themes
│   ├── watcher/                # VCS-aware file watching
│   └── addons/                 # Addon security core
├── data/                       # Desktop files, icons
├── debian/                     # Debian packaging
├── install/                    # Installer scripts + WiX config
├── scripts/                    # Build helper scripts
├── assets/                     # Logos
├── .opencode/skills/           # OpenCode skills
└── docs/                       # User + developer documentation
```

## Key Architecture Decisions

- **No libgit2** — all git operations via the `git` CLI through the `git_cmd`
  crate
- **No Electron** — pure Rust + GPUI for native KDE integration
- **No database** — `config` crate: INI settings, YAML projects/themes
- **Async UI** — `cx.spawn` + `tokio::task::spawn_blocking` for git ops and
  AI inference; results flushed into the UI via `WeakEntity::update`
- **No C ABI in the app** — `crates/app` calls backend crates directly as
  Rust libraries; `ffi.rs` modules remain only for external C++ hosts

## Main Components

- `crates/app/src/app.rs` — root view: toolbar, sidebar, file tree, diff, commit panel
- `crates/app/src/sidebar.rs` — branches / history / projects navigation
- `crates/app/src/commit_panel.rs` — commit UI, AI message generation, stash/reset actions
- `crates/app/src/git_service.rs` — async Git CLI wrapper
- `crates/app/src/settings_view.rs` — settings dialog (AI, SSH, remotes, appearance)

## Data Storage (~/.config/lazydesktop/)

| File | Format | Purpose |
|------|--------|---------|
| `lazydesktop.conf` | INI | App settings |
| `projects.yaml` | YAML | Recent projects |
| `themes/*.theme.yaml` | YAML | Custom themes |
| `models/` | GGUF | Local AI models |