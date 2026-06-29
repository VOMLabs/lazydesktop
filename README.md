# LazyDesktop

**The KDE-Native GitHub Desktop Alternative.**

Built with Qt 6 and C++23 — the same frameworks KDE Plasma ships with, requiring no extra runtimes.

## Features

- **Git status list** — Flat file list with colored status square indicators for modified, added, deleted, renamed, and untracked files
- **Per-file checkboxes** — Select individual files to stage and commit via checkboxes; master "Select All" checkbox in the header bar
- **Diff viewer** — Native `QPlainTextEdit`-based diff viewer with line numbers and syntax highlighting (green for additions, red for deletions, blue for hunk headers)
- **Commit** — Write a summary and optional description, then commit with a single click
- **Right-click context menu** — Discard changes (modified files) or delete file (untracked files)
- **Push / Fetch / Pull** — Smart button cycles through push → fetch → pull
- **Branch management** — Switch, create, and delete branches from a dropdown
- **Commit history** — Tabbed sidebar with colored commit list; click a commit to see changed files; click a file to see its diff
- **Recent projects** — Sidebar overlay with persistent project history (stored in `~/vomlabs/lazydesktop/projects.yaml`)
- **Git bootstrapping** — Detects missing Git at startup and offers to install it via `pkexec`/`sudo` (Linux), `xcode-select` (macOS), or `winget` (Windows)
- **Credential handling** — GIT_ASKPASS integration with a credential dialog for remote auth
- **Auto-refresh** — `QFileSystemWatcher` on `.git/index` and `.git/HEAD` triggers debounced status refresh
- **Files menu** — Open in Editor (kate), Open in File Manager, Open in Terminal (konsole), View on GitHub (opens remote URL in browser)
- **View menu** — Toggle Commit Panel and Commit Files panel visibility with X close buttons

## Native look

- Uses system palette colors throughout (respects desktop theme)
- System monospace font for the diff viewer
- Standard Qt widget rendering (no custom painting for the file list)
- Colored status squares match the system icon size

## Build

```bash
meson setup build
ninja -C build
```

## Requirements

- Qt 6 (Core, Gui, Widgets)
- yaml-cpp
- Git
