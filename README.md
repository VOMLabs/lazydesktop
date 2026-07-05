# LazyDesktop

**The KDE-Native GitHub Desktop Alternative.**

Built with Qt 6 and C++23 — the same frameworks KDE Plasma ships with, requiring no extra runtimes.

## Features

- **Git status list** — Flat file list with colored status square indicators for modified, added, deleted, renamed, and untracked files
- **Per-file checkboxes** — Select individual files to stage and commit via checkboxes; master "Select All" checkbox in the header bar
- **Diff / media viewer** — `QPlainTextEdit`-based diff viewer with line numbers and syntax highlighting. Image files (png, jpg, webp, gif, etc.) render inline; video files show a placeholder message
- **Commit** — Write a summary and optional description, then commit with a single click
- **Skip pre-commit hooks** — Toggle button (lightning icon) adds `--no-verify` to the commit command
- **Co-authors** — Click the people icon to scan checked files' git history for authors, select co-authors from a dialog, and append `Co-authored-by:` trailers to the description
- **AI commit message generation** — Click the AI button to generate a summary + description from the diffs of checked files. Supports multiple providers:
  - **OpenRouter**, **OpenAI**, **Anthropic**, **Gemini**, **Google AI Studio** (require an API key in Settings → AI)
  - **Ollama**, **LMStudio** (local, no key needed — auto-detected on startup)
  - Right-click the AI button to switch providers
  - Model name is configurable in Settings → AI; fields are disabled while generating
- **Push / Fetch / Pull** — Smart button cycles through push → fetch → pull
- **Branch management** — Switch, create, and delete branches from a dropdown
- **Commit history** — Tabbed sidebar with colored commit list; click a commit to see changed files; click a file to see its diff
- **Add project dropdown** — Drawer button shows a dropdown with **Clone Repository** (prompt for URL + destination, runs `git clone`), **Create Repository** (prompts for directory, runs `git init`), and **Load Existing** (folder picker)
- **Recent projects** — Sidebar overlay with persistent project history (stored in `~/.config/lazydesktop/projects.yaml`); yellow dot indicates dirty repos
- **Projects grouped by remote owner** — Recent list organizes projects under category headers (GitHub owner/org extracted from `remote.origin.url`)
- **Scan Folder for Projects** — Bulk-import all Git repos from a folder's subdirectories via the drawer button
- **Remove All** — Clears the entire recent projects list with confirmation
- **Settings dialog** — Categorized settings:
  - **Appearance**: System Default / Dark / custom YAML themes
  - **Git**: Global `user.name` and `user.email` read/written via `git config --global`
  - **AI**: OpenRouter API key, model name, system prompt
- **Custom YAML themes** — Place `.theme.yaml` files in `~/.config/lazydesktop/themes/` to add new theme options in Appearance settings. See example themes at that path after first run
- **Git bootstrapping** — Detects missing Git at startup and offers to install it via `pkexec`/`sudo` (Linux), `xcode-select` (macOS), or `winget` (Windows)
- **Credential handling** — GIT_ASKPASS integration with a credential dialog for remote auth
- **Auto-refresh** — `QFileSystemWatcher` on `.git/index` and `.git/HEAD` triggers debounced (2s) status refresh; selection and diff state are preserved across refreshes and skipped during active commits
- **Files menu** — Open in Editor (kate), Open in File Manager, Open in Terminal (konsole), View on GitHub (opens remote URL in browser)
- **View menu** — Toggle Commit Panel and Commit Files panel visibility
- **Right-click context menu** — Discard changes (modified files) or delete file (untracked files)

## Native look

- Uses system palette colors throughout (respects desktop theme; dark theme available)
- System monospace font for the diff viewer
- Standard Qt widget rendering (no custom painting for the file list)
- Colored status squares match the system icon size
- Dark theme and custom YAML themes apply Qt stylesheets at startup

## Build & Run (without installing)

```bash
meson setup build
ninja -C build
./build/src/lazydesktop
```

The binary runs directly from the build directory — no `make install` needed.
Your projects and settings persist across rebuilds:
- Projects: `~/.config/lazydesktop/projects.yaml`
- Settings (API key, theme, model, system prompt): `~/.config/lazydesktop/lazydesktop.conf`
- Custom themes: `~/.config/lazydesktop/themes/*.theme.yaml`

## Custom Themes

Create a YAML file in `~/.config/lazydesktop/themes/` with a `.theme.yaml` extension:

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

The theme appears in Settings → Appearance after the next launch or when the settings dialog is re-opened.

## Requirements

- Qt 6 (Core, Gui, Widgets, Network)
- yaml-cpp
- Git
- A C++23 compiler (GCC 14+, Clang 18+)
