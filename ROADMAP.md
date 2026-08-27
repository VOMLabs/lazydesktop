# Roadmap

> **Current status (v0.3 era):** v0.2 is **tagged and released**. The backend
> is now mostly Rust — six crates form a single Cargo workspace: `ai_core`
> (GGUF inference), `vcs_core` (SSH + remotes), `addons` (security core),
> `watcher` (VCS-aware file monitoring), `config` (settings/projects/themes),
> and `git_cmd` (Git/Jujutsu CLI execution). The C++/Qt layer is thinning —
> only the UI shell and signal routing remain in `mainwindow.cpp`. **The active
> milestone is v0.3 (Short Term).**

---

## v0.1 — Shipped (Foundation)

- [x] Native Qt 6 Widgets UI (C++23)
- [x] Git status tree with per-file checkboxes
- [x] Diff viewer with syntax highlighting and line numbers
- [x] Commit with summary + description
- [x] Push / Fetch / Pull
- [x] Branch management (switch, create, delete)
- [x] Recent projects drawer (persistent YAML storage)
- [x] Settings dialog (Appearance, Git config, AI)
- [x] Debian and Arch packaging

## v0.2 — Shipped (GitHub Desktop Lite)

- [x] Skip pre-commit hooks toggle (`--no-verify`)
- [x] Co-author selection from git history
- [x] AI commit message generation (OpenRouter, OpenAI, Anthropic, Google AI
      Studio, and local GGUF via the Rust `ai_core` crate)
- [x] Dedicated commit-message FFI (`mm_generate_commit_message`) that
      normalizes output to Conventional Commits
- [x] Background model downloads (detached worker with progress sidecar)
- [x] Commit history with file-level drill-down
- [x] Add project dropdown (clone / create / load)
- [x] Scan folder for Git repos
- [x] Projects grouped by remote owner
- [x] Yellow-dot dirty repo indicator
- [x] Image preview (png, jpg, webp, gif, etc.)
- [x] Custom YAML themes
- [x] Dark theme
- [x] Credential helper dialog
- [x] Auto-refresh via file system watcher
- [x] Git bootstrapping (install missing Git)
- [x] XMake build system (replaces Meson)
- [x] Docker packaging (X11 or VNC/noVNC)
- [x] AI editor skills (`commit`, `create-branch`)
- [x] AI description generation with its own configurable prompt
- [x] `<diff>` prompt placeholder for custom system prompts
- [x] AI "thinking" overlay with expandable raw output
- [x] Jujutsu (jj) support in the AI commit-message path (diff, status,
      change id, recent history)
- [x] Rebuilt AI settings page and theme picker
- [x] AppImage packaging (linuxdeploy + Qt plugin)
- [x] Windows MSI packaging (WiX toolset, deployed Qt runtime)
- [x] Tag-driven GitHub Actions release workflow (`.deb`, `.AppImage`,
      `.pkg.tar.zst`, `.msi`, `checksums.txt`)
- [x] GitHub Actions CI matrix (Ubuntu / Windows / macOS)
- [x] CodeQL static analysis and Dependabot config
- [x] Full documentation tree under `docs/`
- [x] Staging checkbox fix — checkboxes toggle with the selection (Qt
      `editorEvent` handling in `FileTreeDelegate`)
- [x] "View on GitHub" opens the remote web URL, even for SSH-style remotes
- [x] Native SSH key management (generate / list / fingerprint / copy / test)
      in Settings via the `vcs_core` Rust crate
- [x] Remote management dialog (add / edit / rename / remove / copy URL)
      via the `vcs_core` Rust crate

## v0.3 — Short Term

### Rust Backend Integration (current focus)

- [x] **File watcher crate** (`watcher`) — VCS-aware filesystem monitoring
      with typed events and debouncing (10 tests passing)
- [x] **Configuration crate** (`config`) — INI settings, YAML projects/themes
      without Qt dependency (30 tests passing)
- [x] **Git/Jujutsu CLI crate** (`git_cmd`) — Typed wrappers for `git status`,
      `git log`, `git branches`, `git diff`, and Jujutsu equivalents
      (7 tests passing)
- [x] **Addons security core** (`addons`) — Archive parsing, manifest
      validation, path security enforcement (77 tests passing)
- [x] **Cargo workspace** — All 6 Rust crates unified in a single workspace
- [ ] **Switch C++ bridges to new crates** — Replace QProcess calls in
      `mainwindow.cpp` with `git_cmd` FFI, replace QFileSystemWatcher with
      `watcher` FFI, replace settings load/save with `config` FFI
- [ ] **Remove obsolete C++ code** — Delete inline git/status parsing from
      `mainwindow.cpp`, remove QFileSystemWatcher setup, simplify
      `model_manager_bridge` / `vcs_bridge` / `addon_bridge`

### Git Features

- [ ] **Staging area UI** — Show staged vs unstaged files separately; allow
      partial staging (hunk-by-hunk)
- [ ] **Merge conflict resolver** — Inline conflict markers with a
      side-by-side ours/theirs picker
- [ ] **Submodule support** — Recurse into submodules for status, commit, and
      diff
- [ ] **Stash management** — Stash / pop / drop UI with a stash list
- [ ] **Rebase / cherry-pick UI** — Interactive rebase and cherry-pick via
      context menus on commit history
- [ ] **Amend last commit** — Edit the summary/description of `HEAD`
      (`git commit --amend`) with an amend toggle on the commit panel
- [ ] **Reset / revert** — Soft / mixed / hard reset with confirmation, plus
      `git reflog`-based undo after destructive operations
- [ ] **Tag management** — Create, annotate, delete, and push tags from the UI

### UX Improvements

- [ ] **Tabbed multi-repo** — Open several repos in tabs; per-tab sidebar state
- [ ] **Full Jujutsu (jj) UI support** — Extend the status tree, commit,
      branch, and push/fetch/pull flows to jj repos
- [ ] **Status filter + history search** — Filter the Changes list by path or
      status, and search commit history by message or hash
- [ ] **Multi-select files** — Shift/Ctrl+click for batch stage, discard, and
      selection toggling
- [ ] **Keyboard shortcuts** — Ctrl+Enter to commit, Ctrl+A select all, and
      discoverable shortcut hints

### Packaging & Build

- [ ] **Fix packaging build-system drift** — Migrate `debian/rules`,
      `PKGBUILD`, and `install/windows/build-msi.bat` from the legacy
      Meson/Ninja build to XMake (currently only CI builds with XMake)

## v0.4 — Medium Term

### Platform Integration

- [ ] **GitHub / GitLab / Gitea integration** — PR/MR creation, issue linking,
      code review comments
- [ ] **Cloud auth via device flow** — GitHub / GitLab / Gitea OAuth with
      tokens stored in the system keyring instead of pasted PATs
- [ ] **CI status badges** — Show CI pipeline status per branch (GitHub
      Actions, GitLab CI)

### Advanced Git

- [ ] **Git LFS support** — Track, fetch, and push LFS files
- [ ] **Diff editor** — Edit file content inline and save changes (not just
      discard)
- [ ] **Patch workflow** — Create, apply, and export patches
- [ ] **Hooks editor** — View, edit, and manage local Git hooks from the UI
- [ ] **Bisect UI** — GUI for `git bisect` with visual commit marking
- [ ] **File history / blame** — Per-file annotation view with blame
- [ ] **Compare branches / commits** — Diff any two refs or commits without
      checking them out
- [ ] **Git worktrees** — Create, list, switch, and prune worktrees
- [ ] **Reflog viewer** — Browse `git reflog` and restore lost commits

### Project Management

- [ ] **Per-repo settings** — Repo-local overrides (default branch, remote,
      hooks toggle, AI provider) stored per project
- [ ] **System tray presence** — Background status indicator with quick
      commit/push actions

### Quality

- [ ] **C++ / UI test suite** — Qt Test coverage for the widget layer
      (currently only the Rust crates have automated tests)

## v0.5 — Long Term

### Performance

- [ ] **Performance mode** — Virtual file system for monorepos; lazy-load
      commit graph

### Visual

- [ ] **Visual commit graph** — DAG render of branches with drag-to-rebase
- [ ] **Side-by-side diff** — Split-view editor for staged/unstaged comparison

### Packaging

- [ ] **macOS packaging** — `.app` bundle and notarized DMG (Windows MSI
      shipped in v0.2)
- [ ] **Flatpak packaging** — Sandboxed distribution alongside the existing
      `.deb`, `.AppImage`, `.pkg.tar.zst`, and `.msi`

### UX Polish

- [ ] **Command palette** — Fuzzy launcher for actions, file jumping, and
      refs (Ctrl+P style)
- [ ] **Internationalisation** — i18n via Qt Linguist `.ts` files
- [ ] **Accessibility pass** — Screen-reader labels, full keyboard
      navigation, and high-contrast theme support

---

## v1.0 — Pure Rust + GPUI Frontend

> **Status:** Replace the Qt Widgets C++23 UI entirely with a native Rust
> frontend built on GPUI (the editor framework from Zed). Reuses all six
> existing Rust backend crates directly — no FFI needed. Drops the XMake
> build chain so the entire application builds with a single Cargo workspace.

### Why GPUI over Slint

GPUI provides a GPU-accelerated, retained-mode UI with built-in text
rendering, theming, and editor primitives. It is the framework behind Zed
editor and has proven performance for code-heavy UIs. Unlike Slint, it
does not require a separate markup language — layouts are built in Rust.

### Leverage gpui-component for Standard Widgets

Rather than building every UI element from scratch, use GPUI's built-in
components for standard widgets:

| Use gpui-component for | Build custom for |
|------------------------|------------------|
| Buttons, inputs, checkboxes | Diff viewer (syntax-highlighted, inline images) |
| Lists, scroll views, panels | Status tree (file checkboxes, staging) |
| Tooltips, popovers, modals | Commit graph (DAG visualization) |
| Tab bars, dividers, spacing | Branch manager (combo + context menus) |
| Text rendering, theming | Settings dialog (complex form layout) |
| Dialogs, confirmations | Co-author selector (history-based) |

This keeps the v1.0 scope focused on Git-specific components while
reusing battle-tested primitives for everything else.

### Architecture

```
┌─────────────────────────────────────────────────┐
│              GPUI Application (Rust)            │
│  ┌──────────┐  ┌───────────┐  ┌──────────────┐ │
│  │ Sidebar   │  │ Changes   │  │ Diff Viewer  │ │
│  │ - Branch  │  │ - Files   │  │ - Inline     │ │
│  │ - History │  │ - Status  │  │ - Images     │ │
│  │ - Projects│  │ - Commit  │  │              │ │
│  └──────────┘  └───────────┘  └──────────────┘ │
├─────────────────────────────────────────────────┤
│           Direct Rust Crate Calls               │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌──────────┐ │
│  │ git_cmd│ │ watcher│ │ config │ │ addons   │ │
│  └────────┘ └────────┘ └────────┘ └──────────┘ │
│  ┌────────┐ ┌────────┐                         │
│  │ai_core │ │vcs_core│                         │
│  └────────┘ └────────┘                         │
└─────────────────────────────────────────────────┘
```

### Workspace & Toolchain Consolidation

- [ ] Convert repository to a unified Cargo workspace (`crates/app`,
      `crates/ai_core`, `crates/vcs_core`, `crates/watcher`,
      `crates/config`, `crates/git_cmd`, `crates/addons`)
- [ ] Remove `xmake` build system, GCC/Clang dependencies, and C++23 source
      directories (`src/`)
- [ ] Remove C-FFI layers (`cbindgen`, `mm_generate_commit_message`, raw C
      pointer marshalling, all `*_bridge.cpp` files)

### Frontend & UI Layer (GPUI)

- [ ] Implement main window layout in GPUI using `gpui-component`
      panels: sidebar, changes tree, diff viewer, and commit panel
- [ ] Build custom `.theme.yaml` parser mapping directly to GPUI theme
      tokens
- [ ] Implement settings view using `gpui-component` forms: Appearance,
      Git config, SSH Key Manager, AI configuration
- [ ] Implement syntax-highlighted diff viewer with inline media
      placeholders (custom component)
- [ ] Implement file status tree with checkboxes (custom component)

### Core Engine & Async Pipeline

- [ ] Replace `QProcess` Git CLI calls with direct `git_cmd` crate calls
      (already built)
- [ ] Replace `QFileSystemWatcher` with the `watcher` crate events
      (already built)
- [ ] Wire `ai_core`, `vcs_core`, `config`, and `git_cmd` directly into
      GPUI event loops via `tokio::mpsc` channels
- [ ] Remove all Qt bridges (`model_manager_bridge`, `vcs_bridge`,
      `addon_bridge`)

### Packaging & CI Overhaul

- [ ] Replace Qt-bundled packaging scripts (`linuxdeploy`, WiX Qt runtime
      deployment) with lightweight single-binary builders
- [ ] Update GitHub Actions CI workflows to use standard `cargo build` and
      `cargo-packager`
- [ ] Single-binary release artifacts (no bundled Qt runtime)

---

## Non-goals (for now)

- **libgit2** — Git operations use the `git` CLI via the `git_cmd` crate
  for behavioral parity with the command line.
- **Web tech / Electron** — LazyDesktop is intentionally native Rust.
- **A database** — INI settings plus YAML files cover configuration and
  project state.
