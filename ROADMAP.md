# Roadmap

> **Current status (v0.3 era):** v0.2 is **tagged and released** (`v0.2`;
> `v1.0.0-ALPHA` was the earlier pre-release). The build system runs on XMake,
> and local AI inference lives in the Rust `ai_core` crate (`crates/ai_core`),
> exposed to the UI over a C FFI with a dedicated Conventional Commits message
> API. SSH key management and Git remote operations live in a second native
> Rust crate, `crates/vcs_core`, also exposed over a C FFI — no `ssh-keygen`,
> `ssh`, or `git remote` subprocesses. Model downloads run as detached
> background workers so the UI stays responsive. The repo also ships `commit`
> and `create-branch` skills for OpenCode in `.opencode/skills/`. Releases are
> tag-driven and build `.deb`, `.AppImage`,
> `.pkg.tar.zst`, and `.msi` artifacts on GitHub Actions. **The active
> milestone is v0.3 (Short Term).**

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

- [ ] **Staging area UI** — Show staged vs unstaged files separately; allow
      partial staging (hunk-by-hunk)
- [ ] **Merge conflict resolver** — Inline conflict markers with a
      side-by-side ours/theirs picker
- [ ] **Submodule support** — Recurse into submodules for status, commit, and
      diff
- [ ] **Stash management** — Stash / pop / drop UI with a stash list
- [ ] **Rebase / cherry-pick UI** — Interactive rebase and cherry-pick via
      context menus on commit history
- [ ] **Tabbed multi-repo** — Open several repos in tabs; per-tab sidebar state
- [ ] **Fix packaging build-system drift** — Migrate `debian/rules`,
      `PKGBUILD`, and `install/windows/build-msi.bat` from the legacy
      Meson/Ninja build to XMake (currently only CI builds with XMake)
- [ ] **Full Jujutsu (jj) UI support** — Today only the AI commit-message path
      understands `jj`; extend the status tree, commit, branch, and
      push/fetch/pull flows to jj repos
- [ ] **Amend last commit** — Edit the summary/description of `HEAD`
      (`git commit --amend`) with an amend toggle on the commit panel
- [ ] **Reset / revert** — Soft / mixed / hard reset with confirmation, plus
      `git reflog`-based undo after destructive operations
- [ ] **Tag management** — Create, annotate, delete, and push tags from the UI
- [ ] **Status filter + history search** — Filter the Changes list by path or
      status, and search commit history by message or hash
- [ ] **Multi-select files** — Shift/Ctrl+click for batch stage, discard, and
      selection toggling
- [ ] **Keyboard shortcuts** — Ctrl+Enter to commit, Ctrl+A select all, and
      discoverable shortcut hints

## v0.4 — Medium Term

- [ ] **Git LFS support** — Track, fetch, and push LFS files
- [ ] **GitHub / GitLab / Gitea integration** — PR/MR creation, issue linking,
      code review comments
- [ ] **CI status badges** — Show CI pipeline status per branch (GitHub
      Actions, GitLab CI)
- [ ] **Diff editor** — Edit file content inline and save changes (not just
      discard)
- [ ] **Patch workflow** — Create, apply, and export patches
- [ ] **Hooks editor** — View, edit, and manage local Git hooks from the UI
- [ ] **Bisect UI** — GUI for `git bisect` with visual commit marking
- [ ] **File history / blame** — Per-file annotation view with blame
- [ ] **Compare branches / commits** — Diff any two refs or commits without
      checking them out
- [ ] **Cloud auth via device flow** — GitHub / GitLab / Gitea OAuth with
      tokens stored in the system keyring instead of pasted PATs
- [ ] **Per-repo settings** — Repo-local overrides (default branch, remote,
      hooks toggle, AI provider) stored per project
- [ ] **Git worktrees** — Create, list, switch, and prune worktrees
- [ ] **Reflog viewer** — Browse `git reflog` and restore lost commits
- [ ] **System tray presence** — Background status indicator with quick
      commit/push actions
- [ ] **C++ / UI test suite** — Qt Test coverage for the widget layer
      (currently only the Rust `ai_core` crate has automated tests)

## v0.5 — Long Term

- [ ] **Performance mode** — Virtual file system for monorepos; lazy-load
      commit graph
- [ ] **Visual commit graph** — DAG render of branches with drag-to-rebase
- [ ] **Side-by-side diff** — Split-view editor for staged/unstaged comparison
- [ ] **macOS packaging** — `.app` bundle and notarized DMG (Windows MSI
      shipped in v0.2)
- [ ] **Flatpak packaging** — Sandboxed distribution alongside the existing
      `.deb`, `.AppImage`, `.pkg.tar.zst`, and `.msi`
- [ ] **Command palette** — Fuzzy launcher for actions, file jumping, and
      refs (Ctrl+P style)
- [ ] **Internationalisation** — i18n via Qt Linguist `.ts` files
- [ ] **Accessibility pass** — Screen-reader labels, full keyboard
      navigation, and high-contrast theme support

## v1.0 — Pure Rust Architecture & Slint UI

> **Status:** Replaces the Qt Widgets C++23 UI (v0.1–v0.5) with a native Rust
> frontend built on Slint. Reuses the existing `ai_core` and `vcs_core` crates,
> ports the Git/`QProcess` layer to Rust, and drops the XMake/GCC/Clang build
> chain so the entire application builds with a single Cargo workspace.

### Workspace & Toolchain Consolidation

- [ ] Convert repository to a unified Cargo workspace (`crates/app`,
      `crates/ai_core`, `crates/vcs_core`)
- [ ] Remove `xmake` build system, GCC/Clang dependencies, and C++23 source
      directories (`src/`)
- [ ] Remove C-FFI layers (`cbindgen`, `mm_generate_commit_message`, raw C
      pointer marshalling)

### Frontend & UI Layer (Slint)

- [ ] Implement main window layout in Slint (`.slint`): sidebar, changes
      tree, diff viewer, and commit panel
- [ ] Build custom `.theme.yaml` parser mapping directly to Slint global
      design tokens
- [ ] Implement settings dialog tabs (Appearance, Git config, SSH Key
      Manager, AI configuration)
- [ ] Implement syntax-highlighted diff viewer with inline media
      placeholders

### Core Engine & Async Pipeline

- [ ] Replace `QProcess` Git CLI calls with `tokio::process::Command` (or
      native `gix`/`git2`)
- [ ] Replace `QFileSystemWatcher` with the `notify` crate and debounced
      `tokio` timers
- [ ] Wire `ai_core` and `vcs_core` directly into Slint event loops via
      `tokio::mpsc` channels

### Packaging & CI Overhaul

- [ ] Replace Qt-bundled packaging scripts (`linuxdeploy`, WiX Qt runtime
      deployment) with lightweight single-binary builders
- [ ] Update GitHub Actions CI workflows to use standard `cargo build` and
      `cargo-packager`

---

## Non-goals (for now)

- **libgit2** — Most Git operations stay on the `git` CLI via `QProcess` for
  behavioral parity with the command line. Two exceptions are native Rust
  crates: `ai_core` (local GGUF inference) and `vcs_core` (SSH key handling +
  `git remote` config reads/writes).
- **Web tech / Electron** — LazyDesktop is intentionally native Qt Widgets.
- **A database** — `QSettings` (INI) plus YAML files cover configuration and
  project state.
