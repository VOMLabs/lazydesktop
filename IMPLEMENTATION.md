# LazyDesktop — Implementation Details

Comprehensive documentation of what is implemented in LazyDesktop, how it
works internally, and what each component does. This is the technical
companion to the user-facing guides in [`docs/`](docs/README.md).

---

## Part 1: High-Level Overview

### What is LazyDesktop?

LazyDesktop is a native Git GUI client for KDE Plasma, built as a lightweight
alternative to GitHub Desktop. It uses the same technology stack as KDE
itself (Qt 6, C++23) for the UI layer, with an increasingly Rust-based
backend for core services.

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     MainWindow (Qt/C++)                 │
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
│  - QSettings (INI)  │  - XMake (builds Rust + C++)     │
│  - YAML (projects,  │  - Cargo workspace               │
│    themes)          │  - justfile recipes               │
└─────────────────────┴───────────────────────────────────┘
```

### Technology Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| UI Framework | Qt 6 (Core, Gui, Widgets, Network) | All UI rendering |
| Language (UI) | C++23 | Application logic and UI |
| Language (Backend) | Rust | Core services, security, VCS |
| Build System | XMake + Cargo | Compilation and linking |
| Config Storage | Rust `config` crate | INI settings, YAML projects/themes |
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
| `projects.yaml` | YAML | Recent project paths |
| `themes/*.theme.yaml` | YAML | Custom theme definitions |
| `models/` | GGUF files | Downloaded local AI models |

---

## Part 2: Rust Backend Crates

### Workspace Structure

```
crates/
├── ai_core/      # GGUF inference, model management, commit messages
├── vcs_core/     # SSH key management, git remotes, connection tests
├── addons/       # Addon system security core (archive, manifest, path security)
├── watcher/      # VCS-aware file watcher with debouncing
├── config/       # Settings (INI), projects (YAML), themes (YAML)
└── git_cmd/      # Git/Jujutsu CLI command execution wrapper
```

### 2.1 `ai_core` — AI Inference Engine

**Purpose:** Local GGUF model inference and download, exposed over a C FFI.

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

**Purpose:** Typed access to settings, projects, and themes without Qt dependency.

| Module | Responsibility |
|--------|---------------|
| `settings.rs` | INI format settings (QSettings compatible) |
| `projects.rs` | YAML project list management |
| `themes.rs` | YAML theme definitions + scanning |
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

## Part 3: C++/Qt Layer (Remaining)

### 3.1 Source Files

| File | Responsibility | Migration Status |
|------|---------------|-----------------|
| `src/mainwindow.h/cpp` | All UI setup, git process management, settings dialog | **UI remains** — backend logic migratable |
| `src/diffviewer.h/cpp` | Custom QPlainTextEdit with line numbers and syntax highlighting | **UI remains** — tied to Qt widgets |
| `src/model_manager_bridge.h/cpp` | C++ wrapper around `ai_core` C FFI | **Bridge remains** — thin Qt signal wrapper |
| `src/vcs_bridge.h/cpp` | C++ wrapper around `vcs_core` C FFI | **Bridge remains** — thin Qt wrapper |
| `src/addon_bridge.h/cpp` | C++ wrapper around `addons` C FFI | **Bridge remains** — thin Qt wrapper |
| `src/background_download.h/cpp` | Detached model installer mode | **Migratable** — uses `ai_core` FFI |
| `src/main.cpp` | Entry point | **Minimal** — stays in C++ |

### 3.2 Qt Bridges

The C++ bridges (`model_manager_bridge`, `vcs_bridge`, `addon_bridge`) are
thin wrappers that:
1. Convert Qt types (`QString`, `QByteArray`) to C strings
2. Call Rust FFI functions
3. Convert results back to Qt types
4. Emit Qt signals for async results

These bridges will remain until the GPUI frontend replaces Qt. They are
intentionally thin — all business logic lives in Rust.

### 3.3 Still in C++/Qt

| Subsystem | Current Location | Why It Remains | Migration Path |
|-----------|-----------------|----------------|----------------|
| UI rendering | `mainwindow.cpp` | Qt widgets | GPUI frontend |
| Settings dialog | `mainwindow.cpp` | Qt widgets | GPUI frontend |
| Diff viewer | `diffviewer.cpp` | Qt widgets | GPUI frontend |
| Process management | `mainwindow.cpp` | QProcess for git | Already has Rust `git_cmd` — bridges can switch |
| File watching | `mainwindow.cpp` | QFileSystemWatcher | Already has Rust `watcher` — bridges can switch |
| Project management UI | `mainwindow.cpp` | Qt widgets | GPUI frontend |
| Theme application | `mainwindow.cpp` | Qt stylesheet | GPUI frontend |

---

## Part 4: Migration Roadmap

### Completed (Rust Backend)

| Component | Crate | Tests | Status |
|-----------|-------|-------|--------|
| AI inference | `ai_core` | — | Production |
| SSH/Remote | `vcs_core` | — | Production |
| Addon system | `addons` | 77 ✅ | Production |
| File watching | `watcher` | 10 ✅ | New — ready for integration |
| Configuration | `config` | 30 ✅ | New — ready for integration |
| Git/jj commands | `git_cmd` | 7 ✅ | New — ready for integration |

### In Progress

| Component | Status | Next Step |
|-----------|--------|-----------|
| C++ → Rust bridge switching | Bridges exist | Switch `mainwindow.cpp` to use new Rust crates |
| Background download | Still in C++ | Migrate to use `watcher` + `config` crates |

### Remaining C++/Qt Backend

| Component | What It Does | Location | Why It Exists | Migration Complexity |
|-----------|-------------|----------|---------------|---------------------|
| Process management | QProcess for git commands | `mainwindow.cpp` | UI integration | Low — `git_cmd` crate ready |
| File watcher integration | QFileSystemWatcher | `mainwindow.cpp` | UI integration | Low — `watcher` crate ready |
| Settings dialog | Qt widgets for settings | `mainwindow.cpp` | UI | Medium — needs GPUI |
| Project management UI | Qt widgets for project list | `mainwindow.cpp` | UI | Medium — needs GPUI |
| Diff viewer | QPlainTextEdit subclass | `diffviewer.cpp` | UI rendering | High — needs GPUI |
| Theme application | Qt stylesheet generation | `mainwindow.cpp` | UI | Medium — needs GPUI |

### GPUI Migration (Future)

The GPUI frontend will need:
1. **File watching:** Use `watcher` crate events directly
2. **Configuration:** Use `config` crate for all settings/projects/themes
3. **Git operations:** Use `git_cmd` crate for all git/jj commands
4. **AI inference:** Use `ai_core` crate directly (no C++ bridge needed)
5. **SSH/Remote:** Use `vcs_core` crate directly
6. **Addons:** Use `addons` crate directly

The Rust backend is now complete enough to support a GPUI frontend
without any C++ dependency for core services.

---

## Part 5: Build System

### XMake + Cargo

The build process:
1. XMake runs `cargo build --lib` for each Rust crate in `before_build`
2. XMake links the resulting static libraries
3. XMake compiles the C++ application with Qt
4. The C++ app links against all Rust static libraries

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
]
```

### Quality Checks

```bash
cargo check --workspace          # Type checking
cargo test --workspace           # All tests
cargo clippy --workspace         # Linting
just test                        # Rust tests via justfile
just build                       # Full build (C++ + Rust)
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

- [x] INI settings (QSettings compatible)
- [x] YAML project list
- [x] YAML theme management
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

- **Packaging build system drift** — The Debian rules and Arch PKGBUILD
  still reference the Meson/Ninja build from before the XMake migration.
  CI and release workflows use XMake.
- **AI quality depends on the model/provider** — Small local models produce
  usable but terse commit messages; larger models and cloud providers
  generally produce better summaries.
- **New crates not yet integrated** — The `watcher`, `config`, and `git_cmd`
  crates are complete and tested but the C++ `mainwindow.cpp` has not yet
  been switched to use them. Integration requires replacing QProcess/
  QFileSystemWatcher calls with FFI calls to the new crates.
