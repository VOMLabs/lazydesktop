# AGENTS.md

Guidance for AI coding agents working in this repository.

## Project overview

**LazyDesktop** is a fast, native Git GUI for the KDE Plasma desktop — a
lightweight alternative to GitHub Desktop. It is built with **Rust and GPUI**
(no Electron, no web runtime) and integrates with the system theme.

Core capabilities:

- Git essentials: status list, per-file staging, diff viewer (images render
  inline), commits, push/fetch/pull, branch management, commit history.
- Project management: clone / init / load repos, recent-projects list
  (`projects.lua`), folder scanning.
- AI commit messages: cloud providers (OpenRouter, OpenAI, Anthropic, Google
  AI Studio) or fully local GGUF inference via the bundled Rust `ai_core`
  crate (`llama-cpp-2`).
- Native SSH key management and Git remote config through the bundled Rust
  `vcs_core` crate (no `ssh-keygen` / `ssh` / `git remote` subprocesses).

## Tech stack

| Area | Technology |
|------|------------|
| UI | GPUI (`crates/app`), Rust |
| Build | Cargo + Just (`justfile`), moon for orchestration |
| Rust crates | `crates/ai_core` (GGUF inference), `crates/vcs_core` (SSH + remotes), `crates/git_cmd` (git CLI), `crates/config` (settings/projects) |
| Persistence | `config` crate (INI settings + `projects.lua`) — no database |
| Git operations | `git` CLI via `git_cmd` (no libgit2) |
| Task runner | `justfile` recipes (`just setup`, `just build`, `just run`, …) |
| Toolchain | Pinned in `mise.toml` (`mise install`) |

## Repository layout

- `crates/app/` — GPUI application (UI shell, commit panel, sidebar, settings, diff viewer)
- `crates/git_cmd/` — git CLI wrapper
- `crates/ai_core/` — Rust AI engine (cloud providers + local GGUF inference)
- `crates/vcs_core/` — Rust SSH/remote engine
- `crates/config/` — settings, paths, projects persistence
- `crates/watcher/`, `crates/addons/` — file watching and addons
- `docs/` — full documentation (see below)
- `data/`, `assets/`, `install/`, `debian/`, `scripts/` — packaging and resources
- `.github/workflows/` — CI (`ci.yml`), release (`release.yml`), CodeQL
- `.opencode/` — OpenCode agents, skills, context, tools
- `.moon/` — Moon workspace config (task orchestration)
- `ROADMAP.md`, `IMPLEMENTATION.md` — project direction and technical deep-dive

## Build, test, and quality

```bash
just setup           # install toolchain via mise
just build           # cargo build --workspace (Rust app + crates)
just run             # run the debug binary
just test            # cargo test --workspace
just format          # cargo fmt --all
just clippy          # cargo clippy --workspace --all-targets
just lint            # run all pre-commit hooks
```

CI runs on Ubuntu, Windows, and macOS (`.github/workflows/ci.yml`).

## Conventions

- **Commits**: Conventional Commits — `type(scope): subject` with
  `feat`, `fix`, `refactor`, `chore`, `docs`, `test`, `perf`, `build`, `ci`,
  `style`, `revert`. Subject is imperative, lowercase, ≤ 72 chars.
- **Branches**: `type/scope?/short-description` (e.g. `feat/settings/theme-picker`).
- **Skills**: the OpenCode skills in `.opencode/skills/` (`commit`,
  `create-branch`) encode these conventions — load them for those tasks.

## Documentation

Look into **`docs/`** for more information — the entry point is
[`docs/README.md`](docs/README.md):

- Getting started: installation, build from source, Docker, configuration
- User guide: git workflow, branches & history, projects, themes
- AI: overview, cloud providers, local models, editor skills
- Development: architecture, source layout, `ai_core`, build & test, packaging
- Project meta: contributing, FAQ, release process

Also see `ROADMAP.md` (direction and non-goals) and `IMPLEMENTATION.md`
(technical deep-dive).
