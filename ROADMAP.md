# Roadmap

> **Current status (v0.3 era):** v0.2 is **tagged and released**. The backend
> is now entirely Rust — seven crates form a single Cargo workspace: `ai_core`
> (GGUF inference), `vcs_core` (SSH + remotes), `addons` (security core),
> `watcher` (VCS-aware file monitoring), `config` (settings/projects/themes,
> now Lua-backed), `git_cmd` (Git/Jujutsu CLI execution), and `app` (the new
> GPUI frontend, in development). The C++/Qt app in `src/` remains the shipped
> UI while `crates/app` reaches feature parity. **The active milestone is v0.3
> (Short Term), with the v1.0 GPUI migration under way.**

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
- [x] Meson/Ninja + Moon build system (with Cargo for the Rust crates)
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
- [x] **Configuration crate** (`config`) — INI settings plus Lua
      projects/themes/settings, no Qt dependency (32 tests passing; YAML
      persistence migrated to Lua in `484b683`)
- [x] **Git/Jujutsu CLI crate** (`git_cmd`) — Typed wrappers for `git status`,
      `git log`, `git branches`, `git diff`, and Jujutsu equivalents
      (7 tests passing)
- [x] **Addons security core** (`addons`) — Archive parsing, manifest
      validation, path security enforcement (77 tests passing)
- [x] **Cargo workspace** — All 7 Rust crates unified in a single workspace
- [x] **GPUI frontend crate** (`app`) — first native Rust UI port using
      `gpui-component` (window, sidebar, file tree with staging checkboxes,
      commit panel, diff placeholder); calls `git_cmd` directly
- [ ] **Switch C++ bridges to new crates** — superseded by the GPUI
      migration: `crates/app` already calls `git_cmd` (plus `config` and
      `watcher` deps), so the Qt app is retired rather than bridged
- [ ] **Remove obsolete C++ code** — Delete inline git/status parsing from
      `mainwindow.cpp`, remove QFileSystemWatcher setup, simplify
      `model_manager_bridge` / `vcs_bridge` / `addon_bridge` (do this once
      the GPUI frontend reaches feature parity)

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

- [x] **Unify packaging on the Meson build** — `debian/rules`, `PKGBUILD`,
      and `install/windows/build-msi.bat` all build with Meson/Ninja + Cargo
      (no build-system drift)

### Tooling & Developer Experience

- [x] **`.opencode/` validated for OpenCode V2** — agents/commands moved to
      the V2-preferred `agents/` + `commands/` dirs, plugins ported to the V2
      plugin API (`notify`, `gemini` image tools) with `@opencode/plugin`
      added to `.opencode/package.json`, `opencode.json(c)` config added, and
      stale OAC metadata repaired (verified: both plugins load cleanly)
- [ ] **Refresh OpenCode skills for GPUI** — replace the Qt/QML-era `qt-*`
      skills with GPUI/Rust equivalents as `crates/app` becomes the shipped
      UI

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
> frontend built on GPUI (the editor framework from Zed). The `crates/app`
> port is **in development** and already reuses the seven existing Rust
> crates directly — no FFI needed. Drops the C++/Meson build chain so the
> entire application builds with a single Cargo workspace.

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

- [x] Convert repository to a unified Cargo workspace (`crates/app`,
      `crates/ai_core`, `crates/vcs_core`, `crates/watcher`,
      `crates/config`, `crates/git_cmd`, `crates/addons`)
- [ ] Remove the Meson/Ninja C++ build, GCC/Clang dependencies, and C++23
      source directories (`src/`)
- [ ] Remove C-FFI layers (`cbindgen`, `mm_generate_commit_message`, raw C
      pointer marshalling, all `*_bridge.cpp` files)

### Frontend & UI Layer (GPUI)

- [x] Implement main window layout in GPUI using `gpui-component`
      panels: sidebar, changes tree, diff viewer (placeholder), and
      commit panel
- [ ] Build custom `.theme.lua` parser mapping directly to GPUI theme
      tokens — `.theme.lua` parsing already exists in the `config` crate;
      mapping to GPUI/`gpui-component` theme tokens is pending
- [x] Implement settings view (Appearance theme picker, Git identity,
      data locations) — hand-rolled on `gpui-component` elements; the
      `config` crate persists `appearance/theme`
- [ ] Settings view: SSH Key Manager (`vcs_core`) and AI configuration
      panels
- [x] Implement colorized diff viewer with line-number gutter, hunk
      coloring, and no-wrap monospace scroll — `diff_view.rs` (replaces
      the placeholder)
- [ ] Diff viewer: word-level syntax highlighting + inline image rendering
- [x] Implement file status tree with checkboxes (custom component)

### Core Engine & Async Pipeline

- [x] Replace `QProcess` Git CLI calls with direct `git_cmd` crate calls
      (done in the GPUI frontend via `crates/app/src/git_service.rs`)
- [ ] Replace `QFileSystemWatcher` with the `watcher` crate events
      (dep declared in `crates/app`; event wiring pending)
- [ ] Wire `ai_core`, `vcs_core`, `config`, and `git_cmd` directly into
      GPUI event loops via `tokio::mpsc` channels (`git_cmd`/`config` are
      deps; `ai_core`/`vcs_core` are not wired yet)
- [ ] Remove all Qt bridges (`model_manager_bridge`, `vcs_bridge`,
      `addon_bridge`)

### GPUI Frontend — Remaining Feature Work

Feature-parity items to port from the Qt app into `crates/app`:

- [x] Real diff viewer — colorized unified diffs with line-number gutter
      (port the Qt `diffviewer`; word-level syntax highlighting and inline
      images remain)
- [ ] Diff viewer: word-level syntax highlighting + inline image rendering
- [ ] Hunk-level staging — stage/unstage individual hunks from the diff view
- [x] Settings view — Appearance (theme picker, persisted to `config`
      INI), Git identity (`git config --global`), data locations
- [ ] Settings view — AI providers config, SSH key manager (`vcs_core`)
- [x] History view — commit list with per-commit diff (`git show`)
- [ ] History view — affected-files list with per-file drill-down
- [ ] Branch management UI — create / switch / delete / rename
- [ ] Push / fetch / pull toolbar actions wired to `git_cmd`
- [ ] Projects UI — recent-projects list, clone / init / load, folder
      scanning (port from the Qt app)
- [ ] AI commit messages in GPUI — wire `ai_core` (local GGUF) and the cloud
      providers (OpenRouter / OpenAI / Anthropic / Google AI Studio)
- [ ] Commit panel extras — co-author selector, amend toggle, skip-commit-
      hooks toggle
- [ ] Stash / reset / revert UI
- [ ] Full Jujutsu (jj) UI support in the GPUI status/commit flows
- [ ] Theming — map `.theme.lua` tokens from the `config` crate to GPUI /
      `gpui-component` theme tokens
- [ ] **Data migration** — migrate Qt-era persisted files (`projects.yaml`,
      `*.theme.yaml`) to the Lua files (`projects.lua`, `*.theme.lua`) that
      the `config` crate writes
- [ ] Auto-refresh — subscribe to `watcher` crate events (dep already
      declared)
- [ ] UX polish — keyboard shortcuts, multi-select staging, status filter,
      history search, tabbed multi-repo
- [ ] GPUI test coverage — unit + snapshot tests for `crates/app` (no tests
      today; only the backend crates are tested)
- [ ] Update `docs/` to describe the GPUI app as the shipped UI once the C++
      app is removed

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
