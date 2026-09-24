# LazyDesktop — Implementation Details

Comprehensive documentation of what is implemented in LazyDesktop, how it
works internally, and what each component does. This is the technical
companion to the user-facing guides in [`docs/`](docs/README.md).

**Current status:** LazyDesktop is a **pure Rust** application. The UI is
built with **GPUI** (the editor framework from Zed) in `crates/app`, and six
backend crates form a single Cargo workspace. The Qt/C++ UI, the Meson build,
and the C ABI headers were removed when the GPUI frontend reached feature
parity — the repository contains no C++ code today. The active milestone is
v0.3; see [`ROADMAP.md`](ROADMAP.md) for what is still open.

---

## Part 1: High-Level Overview

### What is LazyDesktop?

LazyDesktop is a native Git GUI client for the KDE Plasma desktop, built as a
lightweight alternative to GitHub Desktop. It is built entirely with **Rust**:
the UI uses **GPUI**, and seven crates form a single Cargo workspace for the
frontend and the backend services.

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     App (GPUI/Rust)                    │
│  ┌──────────┐  ┌────────────────────┐  ┌─────────────┐ │
│  │  Toolbar  │  │  Sidebar          │  │  Main       │ │
│  │  - Branch │  │  - Branches       │  │  content    │ │
│  │  - Push   │  │  - History        │  │  - File tree│ │
│  │  - Project│  │  - Projects       │  │  - Diff     │ │
│  │           │  ├───────────────────┤  │             │ │
│  │           │  │  Commit panel     │  │             │ │
│  └──────────┘  └────────────────────┘  └─────────────┘ │
├─────────────────────────────────────────────────────────┤
│              Rust Backend Crates                         │
│  ┌─────────────┐ ┌─────────────┐ ┌────────────────────┐ │
│  │ ai_core     │ │ vcs_core    │ │ addons             │ │
│  │ - GGUF      │ │ - SSH keys  │ │ - Archive parsing  │ │
│  │ - Cloud     │ │ - Remotes   │ │ - Manifests        │ │
│  │ - Inference │ │ - Connect   │ │ - Path security    │ │
│  └─────────────┘ └─────────────┘ └────────────────────┘ │
│  ┌─────────────┐ ┌─────────────┐ ┌────────────────────┐ │
│  │ watcher     │ │ config      │ │ git_cmd            │ │
│  │ - File watch│ │ - Settings  │ │ - Git CLI          │ │
│  │ - Events    │ │ - Projects  │ │ - Jujutsu CLI      │ │
│  │ - Debounce  │ │ - Themes    │ │ - Status/Log/Diff  │ │
│  └─────────────┘ └─────────────┘ └────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│  Persistence Layer  │  Build System                     │
│  - INI settings     │  - Cargo workspace (single)       │
│  - Lua (projects,   │  - justfile recipes               │
│    themes)          │  - Moon task orchestration        │
└─────────────────────┴───────────────────────────────────┘
```

### Technology Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| UI Framework | GPUI (`crates/app`) | All UI rendering |
| Language | Rust | Application logic, UI, backend |
| Build System | Cargo workspace + Moon + just | Compilation and linking |
| Config Storage | Rust `config` crate | INI settings, Lua projects/themes |
| Local AI | Rust `ai_core` crate | GGUF model inference |
| Cloud AI | Rust `ai_core::cloud` | OpenRouter / OpenAI / Anthropic / Google AI Studio |
| Native VCS/SSH | Rust `vcs_core` crate | SSH keys, git remotes |
| Addon System | Rust `addons` crate | Security core, package parsing |
| File Watching | Rust `watcher` crate | VCS state monitoring |
| Git Commands | Rust `git_cmd` crate | Git/jj CLI execution |
| Git operations | `git` CLI via Rust subprocess | No libgit2 dependency |

### Data Storage

All persistent data lives under `~/.config/lazydesktop/`:

| File | Format | Purpose |
|------|--------|---------|
| `lazydesktop.conf` | INI | App settings (AI provider, API key, theme, model) |
| `projects.lua` | Lua | Recent project paths |
| `themes/*.theme.lua` | Lua | Custom theme definitions |
| `models/` | GGUF files | Downloaded local AI models |

---

## Part 2: Rust Backend Crates

### Workspace Structure

```
crates/
├── app/          # GPUI frontend (the shipped UI)
├── ai_core/      # GGUF inference, cloud providers, model management
├── vcs_core/     # SSH key management, git remotes, connection tests
├── addons/       # Addon system security core (archive, manifest, path security)
├── watcher/      # VCS-aware file watcher with debouncing
├── config/       # Settings (INI), projects (Lua), themes (Lua)
└── git_cmd/      # Git/Jujutsu CLI command execution wrapper
```

> **Note on FFI:** each backend crate still ships an `ffi.rs` module and
> builds as `staticlib` for **external consumer projects**. The bundled GPUI
> app (`app`) calls the crates directly as Rust libraries and does not use
> the C ABI. The `include/*.h` headers that declared that ABI were deleted
> together with the Qt app.

### 2.1 `ai_core` — AI Inference Engine

**Purpose:** Local GGUF model inference + downloads, and cloud provider calls
(`ai_core::cloud` for OpenRouter / OpenAI / Anthropic / Google AI Studio).

| Module | Responsibility |
|--------|---------------|
| `inference.rs` | GGUF model loading, token-by-token streaming |
| `cloud/` | Cloud provider clients (OpenRouter, OpenAI, Anthropic, Google AI Studio) |
| `download.rs` | HuggingFace model downloads with progress |
| `discovery.rs` | CPU/GPU backend discovery |
| `commit_message.rs` | Conventional Commits message generation |
| `ffi.rs` | C ABI (`mm_*` functions) |
| `error.rs` | Error taxonomy |

Key functions consumed by the app: `ai_core::commit_message::generate` and
`ai_core::cloud::generate` back the **AI button** in the commit panel; the
provider, model, and system prompt come from `lazydesktop.conf`.

### 2.2 `vcs_core` — SSH and Remote Management

**Purpose:** Native SSH key management and git remote configuration via Rust crates.

| Module | Responsibility |
|--------|---------------|
| `ssh.rs` | SSH key listing, generation, fingerprinting |
| `connect.rs` | SSH connection testing via `russh` |
| `remote.rs` | Git remote read/write via `gix-config` |
| `jj.rs` | Jujutsu subprocess orchestration |
| `ffi.rs` | C ABI (`vcs_*` functions) |

The Settings view uses `vcs_core::ssh` (list/generate/copy public keys) and
`vcs_core::remote` (list/add/remove remotes) directly. No `ssh-keygen`, `ssh`,
or `git remote` subprocesses are needed.

### 2.3 `addons` — Addon System Security Core

**Purpose:** Rust trust boundary for addon package parsing, validation, and resource access.

| Module | Responsibility |
|--------|---------------|
| `archive.rs` | `.zip`/`.lzd` streaming extraction with limits |
| `manifest.rs` | TOML manifest schema + validation |
| `ignore.rs` | `.lzdignore` parsing + gitignore matching |
| `pathsec.rs` | Path normalization, traversal rejection |
| `package.rs` | `AddonDescriptor`, `LoadedPackage`, `AssetReader` |
| `provider/` | `AddonProvider` trait, `LocalProvider`, `RemoteProvider` stub |
| `registry.rs` | Provider aggregation, snapshot index |
| `ffi.rs` | C ABI (`lda_*` functions) |

This crate is a self-contained security core; it is not yet surfaced in the
GPUI app.

### 2.4 `watcher` — File Watcher

**Purpose:** VCS-aware filesystem monitoring with typed events and debouncing.

| Module | Responsibility |
|--------|---------------|
| `lib.rs` | `RepoWatcher`, `FileEvent`, `WatcherConfig` |
| `ffi.rs` | C ABI (`watcher_*` functions) |

**Key types:**
```rust
pub enum FileEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
    Rescan,
}
```

**Watches:** `.git/HEAD`, `.git/index` (Git) or `.jj/working_copy`, `.jj/repo` (Jujutsu).

> **Status:** implemented and tested on its own; **not wired into the GPUI
> app yet** — `lazydesktop-watcher` is declared in `crates/app/Cargo.toml`
> but no code consumes it. Auto-refresh via watcher events is open work.

### 2.5 `config` — Configuration Management

**Purpose:** Typed access to settings, projects, and themes.

| Module | Responsibility |
|--------|---------------|
| `settings.rs` | INI format settings (section/key = value) |
| `projects.rs` | Lua project list management |
| `themes.rs` | Lua theme definitions + scanning + **builtin Dark/Light palettes** |
| `paths.rs` | Platform-appropriate path resolution |
| `ffi.rs` | C ABI (`config_*` functions) |

`themes.rs` is the source of the builtin theme definitions and the
`.theme.lua` parser used by the palette system (see Part 3).

### 2.6 `git_cmd` — Git/Jujutsu CLI Wrapper

**Purpose:** Typed, synchronous API for git and jj CLI commands.

| Module | Responsibility |
|--------|---------------|
| `git.rs` | Git command execution + output parsing |
| `jj.rs` | Jujutsu command execution + output parsing |
| `types.rs` | Shared types (`FileStatus`, `CommitEntry`, `BranchEntry`) |
| `ffi.rs` | C ABI (`vcs_git_*`, `vcs_jj_*` functions) |

The app talks to git through `crates/app/src/git_service.rs`, which wraps
these calls and runs them off the UI thread.

---

## Part 3: Frontend Layer (Rust + GPUI)

### 3.1 Source Files

| File | Responsibility |
|------|---------------|
| `crates/app/src/main.rs` | Entry point (GPUI app) |
| `crates/app/src/lib.rs` | Crate root with shared types |
| `crates/app/src/app.rs` | Root view: toolbar, sidebar, content split; git-op feedback |
| `crates/app/src/sidebar.rs` | Branches / History / Projects navigation |
| `crates/app/src/file_tree.rs` | Status file list with staging checkboxes |
| `crates/app/src/diff_view.rs` | Diff viewer (colorized lines, gutter, source/file/commit modes) |
| `crates/app/src/diff.rs` | Diff parsing + color mapping (unit-tested) |
| `crates/app/src/commit_panel.rs` | Commit summary/description, AI generation, skip-hooks toggle, stash/reset actions |
| `crates/app/src/settings_view.rs` | Settings dialog (appearance, git identity, AI, SSH keys, remotes, data locations) |
| `crates/app/src/git_service.rs` | Async Git CLI wrapper for the app |
| `crates/app/src/color.rs` | Palette component colors (unit-tested) |
| `crates/app/src/theme.rs` | **Palette token system** — single source of truth for UI colors |

### 3.2 Theme System

Theming was consolidated in `crates/app/src/theme.rs` plus
`crates/config/src/themes.rs`:

- **`Palette`** — a `Copy` struct of 37 color tokens (background, surface,
  foreground, text tiers, accent, status colors, diff colors, …). Every view
  renders exclusively from a `Palette`; there are no hardcoded UI colors.
- **Builtin palettes** — Dark and Light token sets defined in
  `crates/config/src/themes.rs`.
- **Custom themes** — a `.theme.lua` file (`return { name = …, colors = { background = …, foreground = … } }`)
  is parsed by `config::themes`; the `background` luminance selects the Dark
  or Light token set, and every other shade flows from the matching palette.
- **Selection** — System Default / Dark / Light / custom, persisted under
  `appearance/theme` in `lazydesktop.conf`, resolved at startup in `main.rs`.
- **Propagation** — a `ThemeChanged(Option<Palette>)` event is emitted on
  theme change and applied across sidebar, file tree, diff view, commit
  panel, settings view, and app.
- The system monospace font powers the diff viewer; the file list needs no
  custom painting.

### 3.3 C FFI Modules

`ffi.rs` modules remain in `ai_core`, `vcs_core`, `config`, `git_cmd`,
`watcher`, and `addons`, and those crates build as `staticlib` for external
consumer projects. The bundled app does **not** use them — it calls the crates
directly as Rust libraries.

---

## Part 4: Implemented Features (current)

What the shipped `crates/app` UI does today, and where:

### Git operations — `git_service.rs` + `git_cmd`

- Status list with per-file staging checkboxes (`file_tree.rs`)
- Commit with summary + description, **skip-hooks toggle** (`--no-verify`)
- **Toolbar:** branch badge, **Fetch / Pull / Push** buttons with result
  feedback (`app.rs::run_git_op`)
- Branch management: **create, switch, delete, rename** (`sidebar.rs`)
- **Stash pop** with live count, **unstage all** (mixed reset)
  (`commit_panel.rs`)
- Reset index to HEAD (unstage), preserving working-tree changes
- Jujutsu detection (`.jj` directory) in the git service
- Remote management via `vcs_core::remote` in Settings

### AI commit messages — `commit_panel.rs` + `ai_core`

- Cloud providers (OpenRouter / OpenAI / Anthropic / Google AI Studio) and
  local GGUF inference, both via `ai_core`
- Provider/model/API key/system prompt configured in Settings → AI
- Local model downloads from HuggingFace with GPU toggle

### Diff viewer — `diff_view.rs` + `diff.rs`

- Colorized unified diff with line-number gutter and hunk coloring
- File / commit / source modes; binary files shown as "Binary files differ"
- **Not yet:** word-level syntax highlighting, inline image rendering

### History — `sidebar.rs` + `diff_view.rs`

- Commit list; clicking a commit shows its full diff (`git show`)

### Projects — `sidebar.rs` + `config::projects`

- Recent-projects list (persisted in `projects.lua`), load / init / clone
  controls

### Settings — `settings_view.rs`

- Appearance: theme picker (System / Dark / Light / custom Lua)
- Git identity via `git config --global`
- AI: enable toggle, provider, model, API key, local GGUF path, GPU toggle
- SSH keys via `vcs_core::ssh`
- Remotes via `vcs_core::remote`
- Data locations (read-only)

---

## Part 5: Not Yet Implemented

Open items tracked in [`ROADMAP.md`](ROADMAP.md) — the short list:

- Diff viewer: word-level syntax highlighting, inline image rendering,
  hunk-level staging
- History view: affected-files list with per-file drill-down
- Commit panel: co-author selector, amend toggle
- Stash list / drop UI; reset / revert UI
- Tag management; rebase / cherry-pick UI; merge conflict resolver;
  submodules; staging-area UI (staged vs unstaged)
- Full Jujutsu (jj) UI support in status/commit/branch flows
- Wire the `watcher` crate into the app for auto-refresh; scan folder for
  project import; data migration from Qt-era YAML
- UX: keyboard shortcuts, multi-select, status filter, history search,
  tabbed multi-repo
- Expanded GPUI test coverage (unit tests exist for `color`, `diff`,
  `theme`; no UI/snapshot tests yet)
- Refresh `.opencode` skills: replace Qt/QML-era `qt-*` skills with
  GPUI/Rust equivalents

---

## Part 6: Build System

### Cargo (single workspace)

The whole application — GPUI frontend (`app`) plus the backend crates —
builds with Cargo:

1. `cargo build --workspace` builds every crate (including the `app` binary)
2. The `app` crate links the backend crates directly as Rust libraries
3. Release builds use LTO + `opt-level = "z"` + strip (see `Cargo.toml`)

### Workspace Configuration

```toml
[workspace]
members = [
    "crates/ai_core",
    "crates/vcs_core",
    "crates/addons",
    "crates/watcher",
    "crates/config",
    "crates/git_cmd",
    "crates/app",
]
```

### Quality Checks

```bash
cargo check --workspace          # Type checking
cargo test --workspace           # All tests
cargo clippy --workspace -D warnings  # Linting (gate)
cargo fmt --all                  # Formatting
cargo audit                      # Security audit
just test                        # Rust tests via justfile
just lint                        # All pre-commit hooks
```

CI runs on Ubuntu, Windows, and macOS (`.github/workflows/ci.yml`). Release
artifacts (`.deb`, `.AppImage`, `.pkg.tar.zst`, `.msi`) are built with
`cargo build --workspace --release` by the tag-driven release workflow —
no build-system drift between packaging scripts and CI.

---

## Part 7: Test Status

| Crate | Tests | Status |
|-------|-------|--------|
| `lazydesktop-addons` | 77 | ✅ All passing |
| `lazydesktop-config` | 33 | ✅ All passing |
| `lazydesktop-vcs-core` | 61 | ✅ All passing |
| `lazydesktop-git-cmd` | 12 | ✅ All passing |
| `lazydesktop-watcher` | 10 | ✅ All passing |
| `lazydesktop-ai-core` | 4 | ✅ All passing |
| `lazydesktop-app` | 16 | ✅ All passing (color 5, diff 4, theme 7) |
| **Total** | **213** | **✅ All passing** |

---

## Known Notes / Caveats

- **Packaging build system** — Debian rules, Arch PKGBUILD, and Windows MSI
  script all build with `cargo build --workspace --release`, matching CI and
  the release workflows (no build-system drift).
- **AI quality depends on the model/provider** — Small local models produce
  usable but terse commit messages; larger models and cloud providers
  generally produce better summaries.
- **`staticlib` + `ffi.rs` kept for external hosts** — The bundled app calls
  the crates directly; the C ABI modules remain only for external consumer
  projects and are inert in this repository (headers removed).
- **Watcher crate not wired** — `lazydesktop-watcher` is tested standalone
  but status auto-refresh in the app is not yet event-driven.
- **Remaining UI gaps** — see [Part 5](#part-5-not-yet-implemented) and
  [ROADMAP.md](ROADMAP.md).