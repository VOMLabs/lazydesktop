# LazyDesktop — Implementation Details

Comprehensive documentation of what is implemented in LazyDesktop, how it
works internally, and what each component does. This is the technical
companion to the user-facing guides in [`docs/`](docs/README.md).

---

## Part 1: High-Level Overview

### What is LazyDesktop?

LazyDesktop is a native Git GUI client for KDE Plasma, built as a lightweight
alternative to GitHub Desktop. It is built entirely with **Rust**: the UI uses
**GPUI** (the editor framework from Zed), and seven crates form a single Cargo
workspace for the frontend and the backend services.

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     App (GPUI/Rust)                    │
│  ┌──────────┐  ┌────────────────────┐  ┌─────────────┐ │
│  │  Toolbar  │  │  Sidebar          │  │  Main       │ │
│  │  - Branch │  │  - Branches       │  │  content    │ │
│  │  - Push   │  │  - History        │  │  - File tree│ │
│  │  - Project│  │  - Projects       │  │  - Diff     │ │
│  │           │  ├───────────────────┤  │  - Image    │ │
│  │           │  │  Commit panel     │  │             │ │
│  └──────────┘  └────────────────────┘  └─────────────┘ │
├─────────────────────────────────────────────────────────┤
│              Rust Backend Crates                         │
│  ┌─────────────┐ ┌─────────────┐ ┌────────────────────┐ │
│  │ ai_core     │ │ vcs_core    │ │ addons             │ │
│  │ - GGUF      │ │ - SSH keys  │ │ - Archive parsing  │ │
│  │ - Inference │ │ - Remotes   │ │ - Manifests        │ │
│  │ - Downloads │ │ - Connect   │ │ - Path security    │ │
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

> **Note on FFI:** every crate still ships an `ffi.rs` module and builds as
> `staticlib` for **external consumer projects**. The bundled GPUI app (`app`)
> calls the crates directly as Rust libraries and does not use the C ABI.

### 2.1 `ai_core` — AI Inference Engine

**Purpose:** Local GGUF model inference + downloads, and cloud provider calls
(`ai_core::cloud` for OpenRouter / OpenAI / Anthropic / Google AI Studio).

| Module | Responsibility |
|--------|---------------|
| `inference.rs` | GGUF model loading, token-by-token streaming |
| `download.rs` | HuggingFace model downloads with progress |
| `discovery.rs` | CPU/GPU backend discovery |
| `commit_message.rs` | Conventional Commits message generation |
| `ffi.rs` | C ABI (`mm_*` functions) |
| `error.rs` | Error taxonomy |

**Key FFI functions:**
- `mm_init` / `mm_destroy` — Manager lifecycle
- `mm_stream_inference` — Generic completion
- `mm_generate_commit_message` — Commit-message generation
- `mm_download_model` / `mm_cancel_download` — Model downloads
- `mm_list_local_models` / `mm_discover_models` — Model listing

### 2.2 `vcs_core` — SSH and Remote Management

**Purpose:** Native SSH key management and git remote configuration via Rust crates.

| Module | Responsibility |
|--------|---------------|
| `ssh.rs` | SSH key listing, generation, fingerprinting |
| `connect.rs` | SSH connection testing via `russh` |
| `remote.rs` | Git remote read/write via `gix-config` |
| `jj.rs` | Jujutsu subprocess orchestration |
| `ffi.rs` | C ABI (`vcs_*` functions) |

**Key FFI functions:**
- `vcs_ssh_list_public_keys` — List public keys
- `vcs_ssh_generate_key` — Generate keypairs
- `vcs_ssh_test_connection` — Test SSH connections
- `vcs_remote_list` / `vcs_remote_add` / `vcs_remote_set_url` — Remote management

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

**Test status:** 77 tests passing (archive, manifest, ignore, pathsec, registry, provider, runtime, error).

### 2.4 `watcher` — File Watcher (NEW)

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

**Key FFI functions:**
- `watcher_create` — Start watching a repository
- `watcher_poll` — Get next event with timeout
- `watcher_destroy` — Stop watching and join threads

**Watches:** `.git/HEAD`, `.git/index` (Git) or `.jj/working_copy`, `.jj/repo` (Jujutsu).

**Test status:** 10 tests passing.

### 2.5 `config` — Configuration Management (NEW)

**Purpose:** Typed access to settings, projects, and themes.

| Module | Responsibility |
|--------|---------------|
| `settings.rs` | INI format settings (section/key = value) |
| `projects.rs` | Lua project list management |
| `themes.rs` | Lua theme definitions + scanning |
| `paths.rs` | Platform-appropriate path resolution |
| `ffi.rs` | C ABI (`config_*` functions) |

**Key FFI functions:**
- `config_settings_load` / `config_settings_get` — Settings access
- `config_projects_load` / `config_projects_add` / `config_projects_remove` — Project management

**Test status:** 30 tests passing.

### 2.6 `git_cmd` — Git/Jujutsu CLI Wrapper (NEW)

**Purpose:** Typed, synchronous API for git and jj CLI commands.

| Module | Responsibility |
|--------|---------------|
| `git.rs` | Git command execution + output parsing |
| `jj.rs` | Jujutsu command execution + output parsing |
| `types.rs` | Shared types (`FileStatus`, `CommitEntry`, `BranchEntry`) |
| `ffi.rs` | C ABI (`vcs_git_*`, `vcs_jj_*` functions) |

**Key FFI functions:**
- `vcs_git_status` / `vcs_git_log` / `vcs_git_branches` — Git queries
- `vcs_git_diff_file` / `vcs_git_is_dirty` — File status
- `vcs_jj_status` / `vcs_jj_diff_file` — Jujutsu queries
- `vcs_is_git_available` / `vcs_is_jj_available` — Detection

**Test status:** 7 tests passing.

---

## Part 3: Frontend Layer (Rust + GPUI)

### 3.1 Source Files

The Qt/C++ application in `src/` (plus `meson.build` and the `include/*.h`
C ABI headers) was **removed** when the GPUI frontend reached feature
parity. The whole repository is now a single Rust Cargo workspace.

| File | Responsibility |
|------|---------------|
| `crates/app/src/main.rs` | Entry point (GPUI app) |
| `crates/app/src/app.rs` | Root view: toolbar, sidebar, file tree, diff, commit panel |
| `crates/app/src/sidebar.rs` | Branches / History / Projects navigation |
| `crates/app/src/file_tree.rs` | Status file list with staging checkboxes |
| `crates/app/src/diff_view.rs` | Diff viewer (syntax highlighting, inline images) |
| `crates/app/src/commit_panel.rs` | Commit summary/description, AI generation, stash/reset actions |
| `crates/app/src/settings_view.rs` | Settings dialog (AI, SSH keys, remotes, appearance) |
| `crates/app/src/git_service.rs` | Async Git CLI wrapper for the app |

### 3.2 C FFI modules

`ffi.rs` modules remain in `ai_core`, `vcs_core`, `config`, `git_cmd`,
`watcher`, and `addons`, and those crates still build as `staticlib` for
external consumer projects. The bundled app does **not** use them — it calls the
crates directly as Rust libraries. The `include/*.h` headers that declared
the C ABI were deleted together with the Qt app.

---

## Part 4: Migration Roadmap

### Completed (Rust Backend)

| Component | Crate | Tests | Status |
|-----------|-------|-------|--------|
| AI inference + cloud providers | `ai_core` | ✅ | Production |
| SSH/Remote | `vcs_core` | ✅ | Production |
| Addon system | `addons` | 77 ✅ | Production |
| File watching | `watcher` | 10 ✅ | Production |
| Configuration | `config` | 30 ✅ | Production |
| Git/jj commands | `git_cmd` | 7 ✅ | Production |
| **GPUI frontend** | `app` | — | **Shipped UI** |

### Completed (Frontend)

| Component | Status | Notes |
|-----------|--------|-------|
| GPUI app replaces Qt `mainwindow` | ✅ | `crates/app` is the shipped UI |
| Git ops switched to `git_cmd` crate | ✅ | `crates/app/src/git_service.rs`, async via `spawn_blocking` |
| File watching via `watcher` crate | ✅ | .git/index + .git/HEAD debounce |
| Settings via `config` crate | ✅ | INI settings, `projects.lua`, themes |
| AI wired directly (`ai_core`) | ✅ | Local GGUF + cloud providers (`ai_core::cloud`) |
| SSH/remotes via `vcs_core` | ✅ | No `ssh-keygen` / `ssh` / `git remote` subprocesses |
| Diff viewer | ✅ | Colorized diffs, line-number gutter; word-level highlighting + inline images remaining |
| Branch/stash/reset UI | ✅ | Create/switch/delete/rename branches, stash push/pop, reset modes |

### Remaining Frontend Work

| Item | Status |
|------|--------|
| Diff viewer: word-level syntax highlighting + inline image rendering | In progress |
| Co-author selector, amend toggle in commit panel | Planned |
| Stash list/drop UI, revert UI | Planned |
| Theming — map `config` crate tokens to `gpui-component` theme tokens | Planned |
| Data migration — Qt-era YAML persistence to `config` crate Lua files | Planned |
| Full Jujutsu (jj) UI support in the status/commit flows | Planned |
| UI test coverage for `crates/app` | Planned |

---

## Part 5: Build System

### Cargo (single workspace)

The whole application — GPUI frontend (`app`) plus the backend crates —
builds with Cargo:

1. `cargo build --workspace` builds every crate (including the `app` bin)
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
cargo clippy --workspace         # Linting
cargo fmt --all                  # Formatting
cargo audit                      # Security audit
just test                        # Rust tests via justfile
just build                       # Full build (Rust workspace)
```

---

## Part 6: Feature Checklist

### Git Operations (via `git_cmd` crate)

- [x] `git status --porcelain` parsing
- [x] Branch listing
- [x] Current branch detection
- [x] Commit log
- [x] File diff
- [x] Dirty repository detection
- [x] Jujutsu support (`jj status`, `jj diff`)

### AI System (via `ai_core` crate)

- [x] OpenRouter/OpenAI/Anthropic/Google AI Studio providers
- [x] Local GGUF inference
- [x] Token-by-token streaming
- [x] Background model downloads
- [x] Model management (list, discover, delete)

### SSH/Remote (via `vcs_core` crate)

- [x] SSH key generation
- [x] SSH connection testing
- [x] Git remote management

### Addon System (via `addons` crate)

- [x] Package parsing (zip, lzd, directory)
- [x] Manifest validation
- [x] Path security enforcement
- [x] Ignore system (.lzdignore)
- [x] Provider architecture

### File Watching (via `watcher` crate)

- [x] VCS-aware monitoring (.git, .jj)
- [x] Typed events (Created, Modified, Removed, Renamed)
- [x] Debouncing
- [x] Cross-platform (Linux inotify, macOS FSEvents, Windows ReadDirectoryChanges)

### Configuration (via `config` crate)

- [x] INI settings
- [x] Lua project list
- [x] Lua theme management
- [x] Platform-appropriate paths

---

## Part 7: Test Results

| Crate | Tests | Status |
|-------|-------|--------|
| `lazydesktop-addons` | 77 | ✅ All passing |
| `lazydesktop-config` | 30 | ✅ All passing |
| `lazydesktop-watcher` | 10 | ✅ All passing |
| `lazydesktop-git-cmd` | 7 | ✅ All passing |
| **Total** | **124** | **✅ All passing** |

---

## Known Notes / Caveats

- **Packaging build system** — The Debian rules, Arch PKGBUILD, and Windows
  MSI script all build with `cargo build --workspace --release`, matching CI
  and the release workflows (no build-system drift).
- **AI quality depends on the model/provider** — Small local models produce
  usable but terse commit messages; larger models and cloud providers
  generally produce better summaries.
- **`staticlib` + `ffi.rs` kept for external hosts** — The bundled app calls
  the crates directly; the C ABI modules remain only for external consumer
  projects and are inert in this repository (headers removed).
- **Remaining UI gaps** — Word-level diff highlighting, inline image
  rendering, co-author selector, amend toggle, stash list/drop, revert UI,
  full jj UI support, and `crates/app` unit tests are still open (see
  [ROADMAP.md](ROADMAP.md)).
