# LazyDesktop

**A fast, native Git GUI for the KDE Plasma desktop.**

LazyDesktop is a lightweight alternative to GitHub Desktop — no Electron, no
web runtime, no bloat. It fits straight into your desktop, respects your
system theme, and gives you a clean, keyboard-friendly interface for everyday
Git work.

> Think GitHub Desktop, but native, fast, and built for the KDE ecosystem.

> **Status:** the UI is built with **Rust and GPUI** in [`crates/app`](crates/app)
> and calls the backend Rust crates (`git_cmd`, `ai_core`, `vcs_core`,
> `config`) directly — no FFI, no C++.

---

## Features

### Git, the essentials

- **Status list** — Flat file list with colored status indicators
  (modified = yellow, added = green, deleted = red, renamed = purple,
  untracked = gray).
- **Per-file checkboxes** — Pick exactly which files to stage and commit.
  A master **Select All** checkbox lives in the header bar.
- **Diff viewer** — Clean diff view with line numbers and syntax
  highlighting. Images (png, jpg, webp, gif, …) render inline; video files
  show a placeholder.
- **Commit** — Summary and optional description, committed in one click.
- **Skip pre-commit hooks** — A lightning-bolt toggle adds `--no-verify`.
- **Co-authors** — Scan the checked files' git history for authors and append
  `Co-authored-by:` trailers to the description.
- **Push / Fetch / Pull** — One smart button that cycles through push, fetch,
  and pull based on repository state.
- **Remote management** — Manage remotes (add, edit, rename, remove, copy URL)
  from the toolbar. Native and dependency-free: powered by the bundled
  `crates/vcs_core` Rust crate, no `git remote` subprocess.
- **SSH keys** — Generate Ed25519/RSA-4096 keypairs (optionally
  passphrase-encrypted), list existing public keys, copy or delete them, and
  test connections — all native, no `ssh-keygen`/`ssh` subprocess.
- **Branch management** — Switch, create, and delete branches from a dropdown.
- **Commit history** — Tabbed sidebar with a colored commit list. Click a
  commit to see its files; click a file to see its diff.

### Project management

- **Add project dropdown** — Clone a repository (`git clone`), create one
  (`git init`), or load an existing folder.
- **Recent projects** — Persistent history in `projects.lua`, grouped by
  remote owner/org, with a yellow dot on repos that have uncommitted changes.
- **Scan folder** — Bulk-import every Git repo inside a directory.
- **Remove / Clear all** — Remove one project or wipe the whole list, with
  confirmation dialogs.

### AI commit messages

- **Cloud providers** — Generate summary and description from the diffs of
  your checked files. Supports OpenRouter, OpenAI, Anthropic, and Google AI
  Studio. Requires an API key in Settings → AI.
- **Local inference** — Built-in GGUF model support with one-click downloads
  from HuggingFace, GPU acceleration toggle, and full model management. Local
  inference runs inside the bundled Rust crate (`crates/ai_core`, powered by
  `llama-cpp-2`).
- **Right-click** the AI button to switch providers; the **model name** is
  configurable in Settings → AI.

### AI editor skills

The repository ships **Conventional Commits** and **branch-creation** skills
so AI coding tools produce commit messages and branch names consistent with
the project's conventions. The two skills live in `.opencode/skills/`:

| Tool | Location | Skills |
|------|----------|--------|
| OpenCode | `.opencode/skills/` | `commit`, `create-branch` |

- **`commit`** detects Git vs Jujutsu, gathers the diff and recent history,
  and produces a Conventional Commits message (`type(scope): subject`),
  committing only after explicit approval.
- **`create-branch`** names branches using the `type/scope?/short-description`
  convention (for example `feat/vcs/jj-support`), for both Git and Jujutsu.

### Look and feel

- A shared palette token system (`crates/app/src/theme.rs`) drives every
  surface, interactive state, and text tier — no hardcoded UI colors.
- Built-in **Dark** and **Light** palettes, plus **System Default** which
  follows your desktop appearance.
- Custom **Lua themes** (`.theme.lua`) tune the mode via their background
  color; every other shade comes from the matching token set.
- System monospace font in the diff viewer; no custom painting for the file
  list.

#### Custom themes

Drop a `.theme.lua` file into `~/.config/lazydesktop/themes/` to add a new
theme option in Appearance settings:

```lua
return {
  name = "Ocean Night",
  colors = {
    background = "#0d1117",
    foreground = "#c9d1d9",
  }
}
```

The theme appears in Settings → Appearance after a restart or reopening the
settings dialog. Only `background` is required — its luminance selects the
Dark or Light token set. See [themes](docs/user-guide/themes.md).

### Settings

A categorized settings dialog covers:

- **Appearance** — System Default, Dark, Light, and any custom Lua themes.
- **Git** — Read and write global `user.name` / `user.email` via
  `git config --global`.
- **SSH Keys** — Generate keypairs, browse existing public keys (only public
  material is ever shown), copy them to the clipboard, and test connections.
- **AI** — Enable toggle, provider, API key, model name, system prompts, and
  local model downloads.

### Quality of life

- **Git bootstrapping** — If Git is missing, LazyDesktop offers to install it
  (`pkexec`/`sudo` on Linux, `xcode-select` on macOS, `winget` on Windows).
- **Credential handling** — `GIT_ASKPASS` integration with a credential
  dialog for remote authentication.
- **Auto-refresh** — A file watcher watches `.git/index` and
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

The binary runs directly from the target directory — no `make install`
needed. The build compiles the GPUI app plus the backend crates
(`git_cmd`, `ai_core`, `vcs_core`, `config`). Your data survives
rebuilds:

- Projects: `~/.config/lazydesktop/projects.lua`
- Settings (API key, theme, model, system prompt): `~/.config/lazydesktop/lazydesktop.conf`
- Custom themes: `~/.config/lazydesktop/themes/*.theme.lua`
- Local AI models: `~/.config/lazydesktop/models/`

See [build from source](docs/getting-started/build-from-source.md) for the
full guide, or use the included [`justfile`](justfile) recipes
(`just setup`, `just build`, `just run`, …).

## Run in Docker

The Docker image builds the GPUI app plus the bundled Rust crates (`ai_core`,
`vcs_core`), then ships only the runtime (no toolchain). The window either
connects to your host's X server or falls back to a browser-accessible VNC
session.

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
[Releases](https://github.com/itzzmateo/lazydesktop/releases) page, make it
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