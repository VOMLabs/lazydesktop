---
name: project-architecture
description: Guides codebase navigation, architecture decisions, and structural understanding of LazyDesktop. Trigger when the user asks how the project is structured, where to find things, or how components interact.
---

# Project Architecture Skill

## High-Level Overview

LazyDesktop is a native Git GUI client for KDE Plasma built with Qt 6 Widgets and C++23.

## Directory Layout

```
├── meson.build              # Root build definition
├── src/
│   ├── main.cpp             # Entry point
│   ├── mainwindow.h/.cpp    # Main window + all application logic
│   ├── diffviewer.h/.cpp    # Syntax-highlighted diff viewer
│   └── llamaai.h/.cpp       # Local AI inference (llama.cpp)
├── data/                    # Desktop files, icons
├── debian/                  # Debian packaging
├── install/                 # Installer scripts + WiX config
├── scripts/                 # Build helper scripts
├── assets/                  # Logos
├── .opencode/skills/        # OpenCode skills
└── subprojects/             # Meson subprojects (llama.cpp)
```

## Key Architecture Decisions

- **No libgit2** — all git operations via `QProcess` (git CLI)
- **No Electron** — pure Qt Widgets for native KDE integration
- **No database** — QSettings (INI) for config, YAML files for projects/themes
- **Single-threaded UI** — blocking git ops run via QProcess (async), AI inference on worker QThread

## Main Components

- `MainWindow` (3964 lines) — UI setup, git operations, AI, settings, project management
- `DiffViewer` — QPlainTextEdit subclass with line numbers + syntax highlighting
- `LlamaAI` — llama.cpp C API wrapper with worker thread for non-blocking inference

## Data Storage (~/.config/lazydesktop/)

| File | Format | Purpose |
|------|--------|---------|
| `lazydesktop.conf` | INI | App settings |
| `projects.yaml` | YAML | Recent projects |
| `themes/*.theme.yaml` | YAML | Custom themes |
| `models/` | GGUF | Local AI models |
