# LazyDesktop

**Git, the way your desktop intended.**

A fast, native Git client built with Rust and GPUI. No Electron. No web
runtime. No waiting — just a clean, keyboard-friendly application designed
to feel at home on your desktop.

> Think GitHub Desktop, reimagined as a lightweight native application.

**Built on Rust + GPUI.** The interface lives in [`crates/app`](crates/app)
and talks directly to the backend Rust crates (`git_cmd`, `ai_core`,
`vcs_core`, `config`) — no application-level FFI, no C++ UI layer, and no
web-based frontend.

---

## Why LazyDesktop?

- **Native performance** — Built with Rust and rendered with GPUI. A
  responsive interface designed to keep up with your workflow.
- **Desktop-first design** — A focused interface with keyboard-friendly
  workflows and adaptable appearance, without tying the application to a
  specific desktop environment.
- **Lightweight by design** — No Electron, no embedded web runtime, and no
  browser-sized application overhead.
- **Your data stays yours** — AI commit messages can run **100% locally**
  on your own GPU. No account, no cloud, no telemetry.
- **Keyboard-friendly** — Everyday Git work is only a few keystrokes away.

---

## Features

### Everyday Git, without the ceremony

- **See what changed at a glance** — Color-coded status list: modified =
  yellow, added = green, deleted = red, renamed = purple, untracked = gray.
- **Stage by choice** — Check exactly the files you want. A master
  **Select All** checkbox lives in the header bar.
- **Review diffs like a pro** — Clean diff view with line numbers and syntax
  highlighting. Images (png, jpg, webp, gif, …) render inline; videos get a
  tidy placeholder.
- **Commit in one click** — Summary plus optional description, done.
  A lightning-bolt toggle skips pre-commit hooks (`--no-verify`).
- **Give credit, automatically** — Co-author detection scans the checked
  files' Git history and appends `Co-authored-by:` trailers for you.
- **Push, fetch, pull — one smart button** — It cycles through push, fetch,
  and pull based on your repository's state. No menu hunting.
- **Manage remotes directly** — Add, edit, rename, remove, or copy a remote's
  URL straight from the toolbar. Powered by the bundled `crates/vcs_core`
  Rust crate — no `git remote` subprocess, no extra dependencies.
- **SSH keys, done right** — Generate Ed25519 or RSA-4096 keypairs (optionally
  passphrase-encrypted), list or copy your public keys, delete them, and test
  connections. All handled by the application — no `ssh-keygen` or `ssh`
  subprocess.
- **Branch and switch with ease** — Create, switch, and delete branches from
  one dropdown.
- **A history you can explore** — Tabbed sidebar with a color-coded commit
  list. Click a commit to see its files; click a file to see its diff.

### Projects, organized

- **Add a project in three ways** — Clone a repository, create one with
  `git init`, or load an existing folder.
- **Recent projects, always handy** — Persistent history in `projects.lua`,
  grouped by remote owner/org, with a yellow dot on repositories holding
  uncommitted changes.
- **Scan a folder** — Bulk-import every Git repository inside a directory.
- **Keep it tidy** — Remove one project or clear the whole list, with friendly
  confirmation dialogs.

### AI commit messages that write themselves

- **In the cloud** — Generate a summary and description from the diffs of your
  checked files. OpenRouter, OpenAI, Anthropic, and Google AI Studio are all
  supported. Add your API key in Settings → AI.
- **Or fully local** — Built-in GGUF model support with one-click downloads
  from HuggingFace, a GPU acceleration toggle, and complete model management.
  Inference runs inside the bundled Rust crate `crates/ai_core`
  (powered by `llama-cpp-2`) — your diffs never leave your machine.
- **Fast switching** — Right-click the AI button to switch providers; the
  model name is configurable in Settings → AI.

### AI coding tools that speak your conventions

The repository ships **Conventional Commits** and **branch-creation** skills so
AI editors produce commit messages and branch names that match your project's
rules — every time:

| Tool | Location | Skills |
|------|----------|--------|
| OpenCode | `.opencode/skills/` | `commit`, `create-branch` |

- **`commit`** detects Git vs Jujutsu, studies the diff and recent history, and
  drafts a Conventional Commits message (`type(scope): subject`) — committing
  only after you approve.
- **`create-branch`** names branches with the `type/scope?/short-description`
  convention (e.g. `feat/vcs/jj-support`), for both Git and Jujutsu.

### A look that adapts to you

- A shared palette token system (`crates/app/src/theme.rs`) drives every
  surface, interactive state, and text tier — keeping UI colors consistent.
- Built-in **Dark** and **Light** palettes, plus **System Default** that
  follows your desktop appearance.
- Custom **Lua themes** (`.theme.lua`) pick their mode from the background
  color; every other shade flows from the matching token set.
- The system monospace font powers the diff viewer; the file list needs no
  custom painting.

#### Custom themes

Drop a `.theme.lua` file into `~/.config/lazydesktop/themes/` and a new theme
option appears in Appearance settings:

```lua
return {
  name = "Ocean Night",
  colors = {
    background = "#0d1117",
    foreground = "#c9d1d9",
  }
}
