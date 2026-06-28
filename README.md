# LazyDesktop

**The KDE-Native GitHub Desktop Alternative.**

Built with Qt 6 and C++23 — the same frameworks KDE Plasma ships with, requiring no extra runtimes.

## Features

- **Git status tree** — Modified, added, deleted, renamed, and untracked files shown with colored status indicators
- **Diff viewer** — Click any changed file to see a syntax-highlighted diff with line numbers
- **Commit** — Write a summary and optional description, then commit with a single click
- **Push / Fetch / Pull** — Smart button cycles through push → fetch → pull
- **Branch management** — Switch, create, and delete branches from a dropdown
- **Recent projects** — Sidebar overlay with persistent project history (stored in `~/vomlabs/lazydesktop/projects.yaml`)
- **Git bootstrapping** — Detects missing Git at startup and offers to install it
- **Credential handling** — GIT_ASKPASS integration with a credential dialog for remote auth

## Build

```bash
meson setup build
ninja -C build
```

## Requirements

- Qt 6 (Core, Gui, Widgets)
- yaml-cpp
- Git
