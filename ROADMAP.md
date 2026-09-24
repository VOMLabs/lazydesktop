# Roadmap

> **Current status (v0.3 era):** v0.2 is tagged and released
> (`v0.2`, plus `v1.0.0-ALPHA`). The application is now **entirely Rust** —
> the Qt/C++ UI in `src/` and the Meson build have been **removed**, and the
> GPUI frontend in `crates/app` is the shipped UI. Seven crates form a single
> Cargo workspace: `app` (GPUI frontend), `ai_core` (GGUF inference + cloud
> providers), `vcs_core` (SSH + remotes), `addons` (security core),
> `watcher` (VCS-aware file monitoring), `config`
> (settings/projects/themes), and `git_cmd` (Git/Jujutsu CLI execution).
> The palette theme system is done (Dark / Light / custom Lua themes).
>
> **Everything still open is listed first, in [Remaining Work](#remaining-work-not-yet-done).**
> Shipped milestones follow below.

---

## Remaining Work (not yet done)

### Diff & Review

- [ ] **Word-level syntax highlighting** in the diff viewer
- [ ] **Inline image rendering** in the diff viewer (png, jpg, webp, gif, …)
- [ ] **Staging area UI** — staged vs unstaged sections with partial
      (hunk-by-hunk) staging from the diff view
- [ ] **History drill-down** — affected-files list per commit with per-file
      diff
- [ ] **Side-by-side diff** — split-view comparison of two refs/versions

### Commit & History Tools

- [ ] **Co-author selector** on the commit panel (from checked files' history)
- [ ] **Amend toggle** — edit `HEAD` summary/description (`git commit --amend`)
- [ ] **Stash list / drop UI** (stash push/pop already ship in the commit
      panel; a list with drop is missing)
- [ ] **Reset / revert UI** — soft / mixed / hard reset with confirmation,
      plus `git reflog`-based undo after destructive operations
- [ ] **Rebase / cherry-pick UI** — via context menus on the commit history
- [ ] **Merge conflict resolver** — inline conflict markers with a
      side-by-side ours/theirs picker
- [ ] **Tag management** — create, annotate, delete, and push tags
- [ ] **Reflog viewer** — browse `git reflog` and restore lost commits

### Git Features

- [ ] **Submodule support** — recurse into submodules for status, commit, diff
- [ ] **Git LFS support** — track, fetch, and push LFS files
- [ ] **Diff editor** — edit file content inline and save (not just discard)
- [ ] **Patch workflow** — create, apply, and export patches
- [ ] **Hooks editor** — view, edit, and manage local Git hooks
- [ ] **Bisect UI** — GUI for `git bisect` with visual commit marking
- [ ] **File history / blame** — per-file annotation view
- [ ] **Compare branches / commits** — diff any two refs without checkout
- [ ] **Git worktrees** — create, list, switch, and prune worktrees

### Frontend Wiring

- [ ] **Auto-refresh via the `watcher` crate** — `lazydesktop-watcher` is
      declared in `crates/app` but unwired; subscribe to `.git/index` +
      `.git/HEAD` events with a debounced status refresh
- [ ] **Full Jujutsu (jj) UI support** — extend status tree, commit, branch,
      and push/fetch/pull flows to jj repos (only `.jj` detection exists)
- [ ] **Scan folder for Git repos** — bulk-import projects from a directory
- [ ] **Data migration** — import Qt-era `projects.yaml` / `*.theme.yaml` for
      existing installs (new installs already write `projects.lua` /
      `*.theme.lua`)

### UX

- [ ] **Tabbed multi-repo** — several repos in tabs; per-tab sidebar state
- [ ] **Status filter + history search** — filter the Changes list by path or
      status; search history by message or hash
- [ ] **Multi-select files** — Shift/Ctrl+click for batch stage and discard
- [ ] **Keyboard shortcuts** — Ctrl+Enter to commit, Ctrl+A select all,
      discoverable shortcut hints
- [ ] **Command palette** — fuzzy launcher for actions, files, and refs
- [ ] **System tray presence** — background status with quick commit/push
- [ ] **Per-repo settings** — repo-local overrides (default branch, remote,
      hooks toggle, AI provider)
- [ ] **Internationalisation** — i18n via gettext-rs (or similar) `.po` files
- [ ] **Accessibility pass** — screen-reader labels, full keyboard
      navigation, high-contrast theme support

### Platform & Cloud

- [ ] **GitHub / GitLab / Gitea integration** — PR/MR creation, issue linking,
      code review comments
- [ ] **Cloud auth via device flow** — GitHub / GitLab / Gitea OAuth with
      tokens in the system keyring instead of pasted PATs
- [ ] **CI status badges** — per-branch pipeline status (GitHub Actions,
      GitLab CI)

### Performance & Visual

- [ ] **Performance mode** — virtual file system for monorepos; lazy-load the
      commit graph
- [ ] **Visual commit graph** — DAG render of branches with drag-to-rebase

### Packaging

- [ ] **macOS packaging** — `.app` bundle and notarized DMG (Windows MSI
      shipped in v0.2)
- [ ] **Flatpak packaging** — sandboxed distribution alongside `.deb`,
      `.AppImage`, `.pkg.tar.zst`, `.msi`

### Quality & Tooling

- [ ] **Expand GPUI test coverage** — unit tests exist for `color`, `diff`,
      and `theme`; add UI/snapshot and integration tests for `crates/app`
- [ ] **Refresh OpenCode skills for GPUI** — replace the Qt/QML-era `qt-*`
      skills with GPUI/Rust equivalents now that `crates/app` is the shipped
      UI

---

## Shipped

### v0.1 — Foundation (shipped)

- [x] Native Qt 6 Widgets UI (C++23) — *later replaced by GPUI in v0.3+*
- [x] Git status tree with per-file checkboxes
- [x] Diff viewer with syntax highlighting and line numbers
- [x] Commit with summary + description
- [x] Push / Fetch / Pull
- [x] Branch management (switch, create, delete)
- [x] Recent projects drawer
- [x] Settings dialog (Appearance, Git config, AI)
- [x] Debian and Arch packaging

### v0.2 — GitHub Desktop Lite (shipped)

- [x] Skip pre-commit hooks toggle (`--no-verify`)
- [x] Co-author selection from git history (Qt era; GPUI re-add is open)
- [x] AI commit message generation (OpenRouter, OpenAI, Anthropic, Google AI
      Studio, and local GGUF via the Rust `ai_core` crate)
- [x] Dedicated commit-message FFI (`mm_generate_commit_message`) that
      normalizes output to Conventional Commits
- [x] Background model downloads (detached worker with progress sidecar)
- [x] Commit history with file-level drill-down (Qt era; GPUI re-add is open)
- [x] Add project dropdown (clone / create / load)
- [x] Scan folder for Git repos (Qt era; GPUI re-add is open)
- [x] Projects grouped by remote owner
- [x] Yellow-dot dirty repo indicator
- [x] Image preview (png, jpg, webp, gif, etc.) — Qt era; GPUI re-add is open
- [x] Custom YAML themes (Qt era; replaced by Lua `.theme.lua` in v0.3)
- [x] Dark theme
- [x] Credential helper dialog
- [x] Auto-refresh via file system watcher (Qt era; GPUI watcher wiring is open)
- [x] Git bootstrapping (install missing Git)
- [x] Meson/Ninja + Moon build system — *later replaced by Cargo-only in v0.3*
- [x] Docker packaging (X11 or VNC/noVNC)
- [x] AI editor skills (`commit`, `create-branch`)
- [x] AI description generation with its own configurable prompt
- [x] `<diff>` prompt placeholder for custom system prompts
- [x] AI "thinking" overlay with expandable raw output
- [x] Jujutsu (jj) support in the AI commit-message path (diff, status,
      change id, recent history)
- [x] Rebuilt AI settings page and theme picker
- [x] AppImage packaging (linuxdeploy + Qt plugin) — *later single-binary*
- [x] Windows MSI packaging (WiX toolset, deployed Qt runtime) — *later single-binary*
- [x] Tag-driven GitHub Actions release workflow (`.deb`, `.AppImage`,
      `.pkg.tar.zst`, `.msi`, `checksums.txt`)
- [x] GitHub Actions CI matrix (Ubuntu / Windows / macOS)
- [x] CodeQL static analysis and Dependabot config
- [x] Full documentation tree under `docs/`
- [x] "View on GitHub" opens the remote web URL, even for SSH-style remotes
- [x] Native SSH key management (generate / list / fingerprint / copy / test)
      in Settings via the `vcs_core` Rust crate
- [x] Remote management dialog (add / edit / rename / remove / copy URL)
      via the `vcs_core` Rust crate

### v0.3 — Short Term (shipped parts)

#### Rust Backend Integration (complete)

- [x] **File watcher crate** (`watcher`) — VCS-aware filesystem monitoring
      with typed events and debouncing
- [x] **Configuration crate** (`config`) — INI settings plus Lua
      projects/themes; YAML persistence migrated to Lua
- [x] **Git/Jujutsu CLI crate** (`git_cmd`) — typed wrappers for `git status`,
      `git log`, `git branches`, `git diff`, and Jujutsu equivalents
- [x] **Addons security core** (`addons`) — archive parsing, manifest
      validation, path security enforcement
- [x] **Cargo workspace** — all 7 Rust crates unified in a single workspace

#### GPUI Frontend (shipped UI)

- [x] **GPUI frontend crate** (`app`) — native Rust UI using
      `gpui-component` (window, sidebar, file tree with staging checkboxes,
      commit panel, diff viewer); calls the backend crates directly
- [x] **Remove obsolete C++ code** — `src/` (Qt 6 mainwindow, diffviewer,
      bridges), `meson.build`, and the `include/*.h` C ABI headers deleted;
      `ffi.rs` modules remain in the crates for external staticlib consumers

#### Theming (complete)

- [x] **Palette token system** — `crates/app/src/theme.rs`: 37 color tokens,
      every UI surface renders from the palette (no hardcoded colors)
- [x] **Builtin Dark/Light palettes** + **System Default** following the
      desktop appearance (`crates/config/src/themes.rs`)
- [x] **Custom Lua themes** — `.theme.lua` parsing; background luminance
      selects the Dark or Light token set
- [x] **`ThemeChanged(Option<Palette>)`** propagation across sidebar, file
      tree, diff viewer, commit panel, settings view, and app; persisted
      under `appearance/theme` and applied at startup

#### Packaging & Build (complete)

- [x] **Unify packaging on Cargo** — `debian/rules`, `PKGBUILD`, and
      `install/windows/build-msi.bat` all build with `cargo build
      --workspace --release` (no build-system drift; Meson removed)
- [x] Replace Qt-bundled packaging scripts (linuxdeploy Qt plugin, WiX Qt
      runtime deployment, windeployqt) with lightweight single-binary
      builders; CI/release workflows updated

#### Tooling & Docs (complete)

- [x] **`.opencode/` validated for OpenCode V2** — agents/commands moved to
      the V2-preferred `agents/` + `commands/` dirs, plugins ported to the V2
      plugin API (`notify`, `gemini` image tools), `opencode.json`
      simplified to `$schema`-only with plugin auto-discovery, stale OAC
      metadata repaired (verified: both plugins load cleanly, zero ERROR/WARN)
- [x] **Docs refresh** — `docs/`, README, ROADMAP, IMPLEMENTATION updated for
      the pure-Rust GPUI UI and palette theming

### v1.0 — Pure Rust + GPUI Frontend (shipped)

> **Status:** the Qt Widgets C++23 UI has been **fully replaced** with a
> native Rust frontend built on GPUI. The `crates/app` port is **the shipped
> UI** and reuses the seven existing Rust crates directly — no FFI needed.
> The C++/Meson build chain is gone; the entire application builds with a
> single Cargo workspace. Open items that remain from the original v1.0
> checklist are listed under [Remaining Work](#remaining-work-not-yet-done).

#### Why GPUI over Slint

GPUI provides a GPU-accelerated, retained-mode UI with built-in text
rendering, theming, and editor primitives. It is the framework behind Zed
editor and has proven performance for code-heavy UIs. Unlike Slint, it does
not require a separate markup language — layouts are built in Rust.

#### Leverage gpui-component for Standard Widgets

Rather than building every UI element from scratch, use GPUI's built-in
components for standard widgets:

| Use gpui-component for | Build custom for |
|------------------------|------------------|
| Buttons, inputs, checkboxes | Diff viewer (colorized lines, hunk coloring) |
| Lists, scroll views, panels | Status tree (file checkboxes, staging) |
| Tooltips, popovers, modals | Commit graph (DAG visualization) |
| Tab bars, dividers, spacing | Branch manager (combo + context menus) |
| Text rendering, theming | Settings dialog (complex form layout) |
| Dialogs, confirmations | Co-author selector (history-based) |

#### Architecture

```
┌─────────────────────────────────────────────────┐
│              GPUI Application (Rust)            │
│  ┌──────────┐  ┌───────────┐  ┌──────────────┐ │
│  │ Sidebar   │  │ Changes   │  │ Diff Viewer  │ │
│  │ - Branch  │  │ - Files   │  │ - Unified    │ │
│  │ - History │  │ - Status  │  │ - Gutter     │ │
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

#### Workspace & Toolchain Consolidation (complete)

- [x] Convert repository to a unified Cargo workspace (`crates/app`,
      `crates/ai_core`, `crates/vcs_core`, `crates/watcher`,
      `crates/config`, `crates/git_cmd`, `crates/addons`)
- [x] Remove the Meson/Ninja C++ build, GCC/Clang dependencies, and C++23
      source directories (`src/`)
- [x] Remove C-FFI layers (`cbindgen`, raw C pointer marshalling, all
      `*_bridge.cpp` files) — the bundled app calls the crates directly;
      `ffi.rs` modules are kept inert in the crates for external staticlib
      consumers

#### Frontend & UI Layer (complete)

- [x] Implement main window layout in GPUI using `gpui-component` panels:
      sidebar, changes tree, diff viewer, and commit panel
- [x] Build custom `.theme.lua` parser mapping directly to GPUI theme tokens
      — `Palette::from_theme` (luminance → dark/light) plus the
      `gpui-component` `ThemeSet` projection in `crates/app/src/theme.rs`
- [x] Implement settings view (Appearance theme picker, Git identity, data
      locations) — the `config` crate persists `appearance/theme`
- [x] Settings view: SSH Key Manager (`vcs_core`) and AI configuration
      panels — plus Remotes management (`vcs_core::remote`)
- [x] Implement colorized diff viewer with line-number gutter, hunk
      coloring, and no-wrap monospace scroll — `diff_view.rs`
- [x] Implement file status tree with checkboxes (custom component)

#### Core Engine & Async Pipeline (complete)

- [x] Replace `QProcess` Git CLI calls with direct `git_cmd` crate calls via
      `crates/app/src/git_service.rs` (async UI via `cx.spawn` +
      `tokio::task::spawn_blocking`)
- [x] Wire `ai_core`, `vcs_core`, `config`, and `git_cmd` directly into
      `crates/app` — dependency and direct-call wiring done
- [x] Remove all Qt bridges (`model_manager_bridge`, `vcs_bridge`,
      `addon_bridge`) — deleted with `src/`

#### Feature Parity Reached (complete)

- [x] Real diff viewer — colorized unified diffs with line-number gutter
      (binary files handled; word-level highlighting and inline images
      remain open)
- [x] Settings view — Appearance (theme picker), Git identity
      (`git config --global`), data locations
- [x] Settings view — AI providers config, SSH key manager (`vcs_core`),
      Remotes (`vcs_core::remote`)
- [x] History view — commit list with per-commit diff (`git show`)
- [x] Branch management UI — create / switch / delete / rename
- [x] Push / fetch / pull toolbar actions wired to `git_cmd`
- [x] Projects UI — recent-projects list, clone / init / load
- [x] AI commit messages in GPUI — `ai_core` local GGUF and the cloud
      providers (OpenRouter / OpenAI / Anthropic / Google AI Studio)
- [x] Commit panel extras — skip pre-commit hooks toggle, stash / stash pop,
      unstage all
- [x] Theming — shared `Palette` maps `.theme.lua` tokens to GPUI /
      `gpui-component` tokens (System / Dark / Light / custom, persisted and
      applied at startup)
- [x] Update `docs/` to describe the GPUI app as the shipped UI

#### Packaging & CI Overhaul (complete)

- [x] Replace Qt-bundled packaging scripts with lightweight single-binary
      builders — `just`/CI/release workflows updated
- [x] Update GitHub Actions CI workflows to use standard `cargo build`
- [x] Single-binary release artifacts (no bundled Qt runtime)

---

## Non-goals (for now)

- **libgit2** — Git operations use the `git` CLI via the `git_cmd` crate
  for behavioral parity with the command line.
- **Web tech / Electron** — LazyDesktop is intentionally native Rust.
- **A database** — INI settings plus Lua files cover configuration and
  project state.