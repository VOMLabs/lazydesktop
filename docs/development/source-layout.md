# Source Layout

A quick tour of where everything lives in the repository.

```
├── src/                        # C++ application (Qt 6 Widgets)
│   ├── main.cpp                # Entry point (QApplication + MainWindow)
│   ├── mainwindow.h/.cpp       # Main window + all application logic
│   ├── diffviewer.h/.cpp       # Syntax-highlighted diff viewer
│   ├── model_manager_bridge.h/.cpp  # C++ wrapper around the ai_core C FFI
│   ├── vcs_bridge.h/.cpp       # C++ wrapper around the vcs_core C FFI
│   └── background_download.h/.cpp   # Detached background model downloads
├── crates/
│   ├── app/                    # GPUI frontend (in development, replaces src/)
│   │   └── src/
│   │       ├── main.rs         # Entry point (GPUI app)
│   │       ├── app.rs          # Root view: toolbar, sidebar, file tree, diff, commit panel
│   │       ├── sidebar.rs      # Changes / History navigation
│   │       ├── file_tree.rs    # Status file list with staging checkboxes
│   │       ├── diff_view.rs    # Diff viewer (placeholder)
│   │       ├── commit_panel.rs # Commit summary/description + actions
│   │       └── git_service.rs  # Git CLI wrapper (QProcess-style, async)
│   ├── ai_core/                # Rust AI engine (staticlib)
│   │   ├── ai_core.h           # C ABI header consumed by C++
│   │   ├── model_manager.h     # Rust-side manager header
│   │   └── src/
│   │       ├── lib.rs          # Crate root, module wiring
│   │       ├── ffi.rs          # C FFI exports
│   │       ├── inference.rs    # GGUF model loading + streaming inference
│   │       ├── download.rs     # HuggingFace downloads
│   │       ├── discovery.rs    # Model discovery + ggml backend detection
│   │       ├── commit_message.rs  # Conventional Commits generation
│   │       ├── types.rs        # Shared types / CommitContext JSON
│   │       └── error.rs        # Error types
│   ├── vcs_core/               # Rust VCS/SSH engine (staticlib)
│   │   ├── vcs_core.h          # C ABI header consumed by C++
│   │   └── src/
│   │       ├── lib.rs          # Crate root, module wiring
│   │       ├── ffi.rs          # C FFI exports
│   │       ├── ssh.rs          # SSH key generation/listing/fingerprinting
│   │       ├── connect.rs      # SSH connection testing (russh, no ssh CLI)
│   │       ├── remote.rs       # Git remote read/write (gix-config)
│   │       └── error.rs        # Error types
│   ├── config/                 # Rust config crate (staticlib + rlib)
│   │   ├── include/config.h    # C ABI header
│   │   └── src/
│   │       ├── lib.rs          # Crate root
│   │       ├── ffi.rs          # C FFI exports
│   │       ├── paths.rs        # Data path resolution
│   │       ├── projects.rs     # Recent projects (Lua: projects.lua)
│   │       ├── settings.rs     # Application settings
│   │       └── themes.rs       # Custom themes (Lua: *.theme.lua)
│   ├── git_cmd/                # Rust git CLI wrapper (staticlib + rlib)
│   │   └── src/
│   │       ├── lib.rs          # Crate root
│   │       ├── ffi.rs          # C FFI exports
│   │       ├── git.rs          # Git command execution
│   │       ├── jj.rs           # Jujutsu support
│   │       └── types.rs        # Shared types
│   ├── watcher/                # Rust filesystem watcher (staticlib + rlib)
│   │   └── src/
│   │       ├── lib.rs          # Crate root
│   │       └── ffi.rs          # C FFI exports
│   └── addons/                 # Rust addon manager (staticlib + rlib)
│       └── src/
│           ├── lib.rs          # Crate root
│           ├── ffi.rs          # C FFI exports
│           ├── archive.rs      # Addon archive handling
│           ├── ignore.rs       # Ignore rules
│           ├── manifest.rs     # Addon manifests
│           ├── package.rs      # Addon packaging
│           ├── pathsec.rs      # Path security
│           ├── provider/       # Addon providers
│           ├── registry.rs     # Addon registry
│           └── runtime.rs      # Addon runtime
├── data/
│   ├── lazydesktop.desktop    # Desktop entry
│   └── icons/hicolor/         # Application icons
├── assets/                     # Logos
├── docker/                     # Docker entrypoint script
│   └── entrypoint.sh
├── Dockerfile                  # Container build
├── docker-compose.yml          # Container orchestration
├── install/
│   └── windows/                # WiX MSI config + build script
├── scripts/
│   ├── build-deb.sh            # .deb packaging helper
│   └── build-arch.sh           # Arch packaging helper
├── debian/                     # Debian packaging metadata
├── PKGBUILD                    # Arch packaging
├── meson.build                 # Meson build definition (C++ app)
├── justfile                    # Task runner recipes
├── mise.toml                   # Toolchain versions (mise)
├── Cargo.toml                  # Rust workspace
├── .moon/                      # Moon workspace config (task orchestration)
├── .github/
│   ├── dependabot.yml             # Dependabot config (Actions + Cargo)
│   └── workflows/
│       ├── ci.yml                 # 3-OS CI matrix
│       ├── release.yml            # Tag-driven release artifacts
│       └── codeql.yml             # CodeQL static analysis
├── .opencode/                  # OpenCode agents, skills, context, tools
└── docs/                       # This documentation
```

## Notes

- The **C++ app builds with Meson + Ninja** (`meson.build`); the **Rust
  crates build with Cargo** (`cargo build --workspace`) and are linked into
  the C++ binary as static libraries. Moon (`.moon/`) orchestrates workspace
  tasks; the `justfile` wraps the common workflows.
- The **GPUI frontend** (`crates/app`) is the in-development replacement for
  the Qt UI (`src/`). It is a pure-Rust application built on `gpui-pre`
  (Zed's UI framework) and consumes the backend crates (`config`, `git_cmd`,
  `watcher`, `ai_core`, `vcs_core`) directly as Rust libraries.
- The `ai_core` and `vcs_core` crates are built automatically by
  `cargo build --workspace` and linked as static libraries. Because each Rust
  staticlib embeds its own copy of Rust's std, the Linux link adds
  `-Wl,--allow-multiple-definition` to keep the first (identical) definition.
- The data layout at runtime is documented in
  [configuration](../getting-started/configuration.md).