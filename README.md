# LazyDesktop

**Git, the way your desktop intended.**

A fast, native Git client for the KDE Plasma desktop. No Electron. No web
runtime. No waiting — just a clean, keyboard-friendly window that feels like it
belongs on your machine, because it does.

> Think GitHub Desktop, but native, fast, and built for the KDE ecosystem.

**Built on Rust + GPUI.** The interface lives in [`crates/app`](crates/app) and
talks directly to the backend Rust crates (`git_cmd`, `ai_core`, `vcs_core`,
`config`) — no FFI, no C++, no hidden slowdowns.

---

## Why LazyDesktop?

- **Native speed** — Compiled Rust, rendered with GPUI. The UI keeps up with
  your fingers, not the other way around.
- **KDE-native** — Wears your Plasma theme like a second skin. System fonts,
  system colors, zero effort.
- **Zero bloat** — No Electron, no web runtime, no 200 MB tax. A Git client
  that feels as light as git itself.
- **Your data stays yours** — AI commit messages can run **100% locally** on
  your own GPU. No account, no cloud, no telemetry.
- **Keyboard-friendly** — Everyday Git work is a few keystrokes away.

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
  files' git history and appends `Co-authored-by:` trailers for you.
- **Push, fetch, pull — one smart button** — It cycles through push, fetch,
  and pull based on your repository's state. No menu hunting.
- **Manage remotes natively** — Add, edit, rename, remove, or copy a remote's
  URL straight from the toolbar. Powered by the bundled `crates/vcs_core`
  Rust crate — no `git remote` subprocess, no dependencies.
- **SSH keys, done right** — Generate Ed25519 or RSA-4096 keypairs (optionally
  passphrase-encrypted), list or copy your public keys, delete them, and test
  connections. All native — no `ssh-keygen`, no `ssh` subprocess.
- **Branch and switch with ease** — Create, switch, and delete branches from
  one dropdown.
- **A history you can explore** — Tabbed sidebar with a color-coded commit
  list. Click a commit to see its files; click a file to see its diff.

### Projects, organized

- **Add a project in three ways** — Clone a repository, create one with
  `git init`, or load an existing folder.
- **Recent projects, always handy** — Persistent history in `projects.lua`,
  grouped by remote owner/org, with a yellow dot on repos holding uncommitted
  changes.
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
  surface, interactive state, and text tier — no hardcoded UI colors anywhere.
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
```

Only `background` is required — its luminance selects the Dark or Light token
set. See [themes](docs/user-guide/themes.md).

### Settings, when you need them

A categorized settings dialog covers:

- **Appearance** — System Default, Dark, Light, and any custom Lua themes.
- **Git** — Read and write your global `user.name` / `user.email` via
  `git config --global`.
- **SSH Keys** — Generate keypairs, browse existing public keys (only public
  material is ever shown), copy them to the clipboard, and test connections.
- **AI** — Enable toggle, provider, API key, model name, system prompts, and
  local model downloads.

### Small touches, big difference

- **Git bootstrapping** — If Git is missing, LazyDesktop offers to install it
  (`pkexec`/`sudo` on Linux, `xcode-select` on macOS, `winget` on Windows).
- **Credential handling** — `GIT_ASKPASS` integration with a credential dialog
  for remote authentication.
- **Auto-refresh** — A file watcher keeps an eye on `.git/index` and
  `.git/HEAD`; changes trigger a debounced status refresh that preserves your
  selection and diff state.
- **Files menu** — Open in Editor (kate), File Manager, Terminal (konsole),
  or View on GitHub.
- **Right-click context menu** — Discard changes on modified files or delete
  untracked files.

---

## Quick start (build and run)

```bash
just setup           # install the pinned toolchain via mise
just build           # cargo build --workspace (Rust app + crates)
just run             # run the debug binary
```

No `make install` needed — the binary runs straight from the target
directory. Your data survives rebuilds:

- Projects: `~/.config/lazydesktop/projects.lua`
- Settings (API key, theme, model, system prompt): `~/.config/lazydesktop/lazydesktop.conf`
- Custom themes: `~/.config/lazydesktop/themes/*.theme.lua`
- Local AI models: `~/.config/lazydesktop/models/`

See [build from source](docs/getting-started/build-from-source.md) for the
full guide, or browse the [`justfile`](justfile) recipes
(`just setup`, `just build`, `just run`, …).

## Run in Docker

Build the GPUI app plus the bundled Rust crates, then ship only the runtime —
the container connects to your host's X server, or falls back to a
browser-accessible VNC session:

```bash
docker compose build            # build the image

# Linux host with an X server -> native window
docker compose up -d

# Any OS (macOS/Windows) or headless -> open http://localhost:6080 in a browser
VNC_MODE=1 docker compose up -d
```

By default the container mounts:

- `/tmp/.X11-unix` — host X socket for the native window
- `lazydesktop-config` (named volume) — settings, projects, themes, models
- `./repos` → `/workspace` — bind-mount your Git repositories, e.g.
  `REPO_DIR=~/code/myrepo docker compose up -d`

Set `PUID`/`PGID` to your host UID/GID if the mounted config is owned by
root. Shortcuts: `just docker-build`, `just docker-up`, `just docker-shell`,
`just docker-logs`. Full details in
[docker](docs/getting-started/docker.md).

## Installation

### Arch Linux

```bash
makepkg -si
```

### Debian / Ubuntu

```bash
sudo dpkg -i lazydesktop_*.deb
```

### AppImage

Grab the latest `lazydesktop-*-x86_64.AppImage` from the
[Releases](https://github.com/VOMLabs/lazydesktop/releases) page, make it
executable, and run it.

### From source

See [build from source](docs/getting-started/build-from-source.md).

## AI setup (one minute)

1. Open **Settings → AI**.
2. Toggle **Enable AI**.
3. Choose a provider and paste an API key, **or** pick a local GGUF model and
   let LazyDesktop download it from HuggingFace.
4. Click the AI button next to the commit panel to generate a message.

See the [AI documentation](docs/ai/overview.md) for details.

## Requirements

- Rust stable toolchain (builds the GPUI app and bundled crates)
- moon (build orchestration)
- Git
- Linux: `libgl1`, `libxkbcommon-x11-0` (X11/GL runtime), plus standard
  build deps (`cmake`, `ninja`) for the bundled C-dependent crates

## Documentation

The full documentation lives in [`docs/`](docs/README.md):

| Area | Guide |
|------|-------|
| Getting started | [Installation](docs/getting-started/installation.md) · [Build from source](docs/getting-started/build-from-source.md) · [Docker](docs/getting-started/docker.md) · [Configuration](docs/getting-started/configuration.md) |
| User guide | [Git workflow](docs/user-guide/git-workflow.md) · [Branches & history](docs/user-guide/branches-and-history.md) · [Projects](docs/user-guide/projects.md) · [Themes](docs/user-guide/themes.md) |
| AI | [Overview](docs/ai/overview.md) · [Cloud providers](docs/ai/cloud-providers.md) · [Local models](docs/ai/local-models.md) · [Editor skills](docs/ai/editor-skills.md) |
| Development | [Architecture](docs/development/architecture.md) · [Source layout](docs/development/source-layout.md) · [`ai_core`](docs/development/ai-core.md) · [Build & test](docs/development/build-and-test.md) · [Packaging](docs/development/packaging.md) |

Other resources: [Roadmap](ROADMAP.md) · [Implementation details](IMPLEMENTATION.md) ·
[Contributing](docs/contributing.md) · [FAQ](docs/faq.md) · [Release process](docs/release-process.md)

## License

MIT — see the repository license file for details.