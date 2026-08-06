# Roadmap

> **Current status (v0.1/0.2 era):** the build system has moved from Meson to
> XMake, and local AI inference moved from a bundled llama.cpp subproject into
> the Rust `ai_core` crate (`crates/ai_core`), exposed to the UI over a C FFI
> with a dedicated Conventional Commits message API. The repo also ships
> `commit` and `create-branch` skills for OpenCode, Claude Code, Gemini CLI,
> and Antigravity in `.opencode/skills/`, `.claude/skills/`, `.gemini/skills/`,
> and `.agents/skills/`.

## v0.2 — Current (GitHub Desktop Lite)

- [x] Git status tree with per-file checkboxes
- [x] Diff viewer with syntax highlighting and line numbers
- [x] Commit with summary + description
- [x] Skip pre-commit hooks toggle (`--no-verify`)
- [x] Co-author selection from git history
- [x] AI commit message generation (OpenRouter, OpenAI, Anthropic, Google AI Studio, and local GGUF via the Rust `ai_core` crate)
- [x] Push / Fetch / Pull
- [x] Branch management (switch, create, delete)
- [x] Commit history with file-level drill-down
- [x] Recent projects drawer (persistent YAML storage)
- [x] Add project dropdown (clone / create / load)
- [x] Scan folder for Git repos
- [x] Projects grouped by remote owner
- [x] Yellow-dot dirty repo indicator
- [x] Image preview (png, jpg, webp, gif, etc.)
- [x] Custom YAML themes
- [x] Dark theme
- [x] Settings dialog (Appearance, Git config, AI)
- [x] Credential helper dialog
- [x] Auto-refresh via file system watcher
- [x] Git bootstrapping (install missing Git)

## v0.3 — Short Term

- [ ] **Staging area UI** — Show staged vs unstaged files separately; allow partial staging (hunk-by-hunk)
- [ ] **Merge conflict resolver** — Inline conflict markers with side-by-side ours/theirs picker
- [ ] **Submodule support** — Recurse into submodules for status, commit, and diff
- [ ] **Stash management** — Stash/pop/drop UI with stash list
- [ ] **Rebase / cherry-pick UI** — Interactive rebase and cherry-pick via context menus on commit history
- [ ] **SSH key management** — Generate, load, and test SSH keys from settings
- [ ] **Tabbed multi-repo** — Open several repos in tabs; per-tab sidebar state
- [ ] **Remote management** — Add / remove remotes, edit remote URLs from UI

## v0.4 — Medium Term

- [ ] **Git LFS support** — Track, fetch, and push LFS files
- [ ] **GitHub / GitLab / Gitea integration** — PR/MR creation, issue linking, code review comments
- [ ] **CI status badges** — Show CI pipeline status per branch (GitHub Actions, GitLab CI)
- [ ] **Diff editor** — Edit file content inline and save changes (not just discard)
- [ ] **Patch workflow** — Create, apply, and export patches
- [ ] **Hooks editor** — View, edit, and manage local git hooks from UI
- [ ] **Bisect UI** — GUI for `git bisect` with visual commit marking
- [ ] **File history / blame** — Per-file annotation view with blame

## v0.5 — Long Term

- [ ] **Performance mode** — Virtual file system for monorepos; lazy-load commit graph
- [ ] **Visual commit graph** — DAG render of branches with drag-to-rebase
- [ ] **Side-by-side diff** — Split-view editor for staged/unstaged comparison
- [ ] **Cross-platform Windows/macOS packaging** — MSI installer, macOS .app bundle
- [ ] **Internationalisation** — i18n via Qt Linguist `.ts` files
