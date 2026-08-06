# LazyDesktop

**The KDE Native GitHub Desktop Alternative.**

LazyDesktop is a fast, native Git GUI client for Linux. It uses Qt 6 and C++23, the same frameworks KDE Plasma is built on, so it fits right into your desktop without needing Electron or any web runtime.

Think GitHub Desktop, but native, fast, and built for the KDE ecosystem.

## Features

### Git

- **Git status list** Flat file list with colored status indicators. Modified files show yellow, added files show green, deleted files show red, renamed files show purple, and untracked files show gray.
- **Per-file checkboxes** Pick which files to stage and commit with checkboxes. There is a master "Select All" checkbox in the header bar.
- **Diff viewer** A clean diff view with line numbers and syntax highlighting. Image files like png, jpg, webp, and gif render inline. Video files show a placeholder message.
- **Commit** Write a summary and optional description, then commit with one click.
- **Skip pre-commit hooks** A lightning icon toggle adds `--no-verify` to your commit command.
- **Co-authors** Click the people icon to scan the checked files' git history for authors. Pick your co-authors from the dialog and `Co-authored-by:` trailers get appended to the description.
- **Push / Fetch / Pull** A single smart button that cycles through push, fetch, and pull.
- **Branch management** Switch branches, create new ones, and delete old ones from a dropdown.
- **Commit history** A tabbed sidebar shows a colored commit list. Click a commit to see what files changed. Click a file to see its diff.
- **Add project dropdown** A drawer button gives you three options: Clone Repository (enter a URL and destination, it runs `git clone`), Create Repository (pick a directory, it runs `git init`), and Load Existing (folder picker).
- **Recent projects** A sidebar overlay keeps your project history in `~/.config/lazydesktop/projects.yaml`. Repos with uncommitted changes show a yellow dot.
- **Projects grouped by remote owner** The recent list groups projects under headers based on the GitHub owner or org from the remote origin URL.
- **Scan Folder for Projects** Bulk import all Git repos from a folder's subdirectories through the drawer button.
- **Remove All** Clear the entire recent projects list with a confirmation dialog.

### AI Commit Messages

- **Cloud providers** Click the AI button to generate a summary and description from the diffs of your checked files. Supports OpenRouter, OpenAI, Anthropic, and Google AI Studio. You need an API key in Settings > AI.
- **Local inference** Built-in GGUF model support with one-click downloads from HuggingFace. GPU acceleration toggle and full model management included. Local inference runs inside a bundled Rust crate (`crates/ai_core`, powered by `llama-cpp-2`) exposed to the UI over a C FFI.
- **Right-click** the AI button to switch providers.
- **Model name** is configurable in Settings > AI. Fields are disabled while generating.

### AI Editor Skills

The repository ships Conventional Commits and branch-creation skills for AI coding tools, so an agent in the repo can write commit messages and branch names consistently with the project's conventions. The same two skills are installed in four locations:

| Tool | Location | Skill |
|------|----------|-------|
| OpenCode | `.opencode/skills/` | `commit`, `create-branch` |
| Claude Code | `.claude/skills/` | `commit`, `create-branch` |
| Gemini CLI | `.gemini/skills/` | `commit`, `create-branch` |
| Antigravity (IDE / CLI) | `.agents/skills/` | `commit`, `create-branch` |

- **`commit`** detects whether the repo uses Git or Jujutsu, gathers the diff and recent history, and produces a Conventional Commits message (`type(scope): subject`), committing only after explicit approval.
- **`create-branch`** creates branches using the `type/scope?/short-description` convention (for example `feat/vcs/jj-support`), for both Git and Jujutsu.

### Look and Feel

- Uses system palette colors throughout. Respects your desktop theme and a dark theme is available.
- System monospace font for the diff viewer.
- Standard Qt widget rendering, no custom painting for the file list.
- Colored status squares match the system icon size.
- Dark theme and custom YAML themes apply Qt stylesheets at startup.

### Custom YAML Themes

Drop a `.theme.yaml` file into `~/.config/lazydesktop/themes/` to add a new theme option in your Appearance settings. Here is an example:

```yaml
name: "Ocean Night"
colors:
  background: "#0d1117"
  foreground: "#c9d1d9"
  widget_background: "#161b22"
  input_background: "#21262d"
  input_foreground: "#c9d1d9"
  button_background: "#1f6feb"
  button_foreground: "#ffffff"
  tooltip_background: "#21262d"
  tooltip_foreground: "#c9d1d9"
  selection: "#1f6feb"
```

The theme shows up in Settings > Appearance after you restart the app or reopen the settings dialog.

### Settings

A categorized settings dialog covers:

- **Appearance** System Default, Dark, and any custom YAML themes you added.
- **Git** Read and write global `user.name` and `user.email` via `git config --global`.
- **AI** Enable toggle, provider selection, API key, model name, system prompt, and local model downloads.

### Quality of Life

- **Git bootstrapping** If Git is missing at startup, LazyDesktop detects it and offers to install it. Uses `pkexec` or `sudo` on Linux, `xcode-select` on macOS, and `winget` on Windows.
- **Credential handling** GIT_ASKPASS integration with a credential dialog for remote authentication.
- **Auto-refresh** A `QFileSystemWatcher` watches `.git/index` and `.git/HEAD`. Changes trigger a debounced 2-second status refresh. Your selection and diff state are preserved during refreshes and skipped during active commits.
- **Files menu** Open in Editor (kate), Open in File Manager, Open in Terminal (konsole), View on GitHub (opens the remote URL in your browser).
- **View menu** Toggle Commit Panel and Commit Files panel visibility.
- **Right-click context menu** Discard changes on modified files or delete untracked files.

## Build and Run (without installing)

```bash
xmake f -m debug          # configure (release: xmake f -m release)
xmake                     # builds C++ and the Rust ai_core crate
./build/linux/x86_64/debug/lazydesktop
```

The binary runs directly from the build directory. No `make install` needed. The `xmake` build automatically runs `cargo build` for the `ai_core` Rust crate and links it in. Your projects and settings survive rebuilds:

- Projects: `~/.config/lazydesktop/projects.yaml`
- Settings (API key, theme, model, system prompt): `~/.config/lazydesktop/lazydesktop.conf`
- Custom themes: `~/.config/lazydesktop/themes/*.theme.yaml`
- Local AI models: `~/.config/lazydesktop/models/`

> **Note on Qt versions:** if both Qt 5 and Qt 6 are installed, xmake may pick Qt 5 from your `PATH`. Point it at Qt 6 before configuring, for example `PATH=/usr/lib/qt6/bin:$PATH xmake f -c -m debug`.

## Installation

### Arch Linux

```bash
makepkg -si
```

### Debian / Ubuntu

```bash
sudo dpkg -i lazydesktop_*.deb
```

### From source

See [Build and Run](#build-and-run-without-installing) above.

## Requirements

- Qt 6 (Core, Gui, Widgets, Network) — Qt 6.7+ recommended
- xmake (build system)
- Rust stable toolchain (builds the bundled `ai_core` crate automatically)
- yaml-cpp
- Git
- A C++23 compiler (GCC 14+ or Clang 18+)
