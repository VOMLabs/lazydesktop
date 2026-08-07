# Source Layout

A quick tour of where everything lives in the repository.

```
├── src/                        # C++ application
│   ├── main.cpp                # Entry point (QApplication + MainWindow)
│   ├── mainwindow.h/.cpp       # Main window + all application logic
│   ├── diffviewer.h/.cpp       # Syntax-highlighted diff viewer
│   ├── model_manager_bridge.h/.cpp  # C++ wrapper around the ai_core C FFI
│   └── background_download.h/.cpp   # Detached background model downloads
├── crates/
│   └── ai_core/                # Rust AI engine (staticlib)
│       ├── ai_core.h           # C ABI header consumed by C++
│       ├── model_manager.h     # Rust-side manager header
│       └── src/
│           ├── lib.rs          # Crate root, module wiring
│           ├── ffi.rs          # C FFI exports
│           ├── inference.rs    # GGUF model loading + streaming inference
│           ├── download.rs     # HuggingFace downloads
│           ├── discovery.rs    # Model discovery + ggml backend detection
│           ├── commit_message.rs  # Conventional Commits generation
│           ├── types.rs        # Shared types / CommitContext JSON
│           └── error.rs        # Error types
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
├── xmake.lua                   # XMake build definition
├── justfile                    # Task runner recipes
├── mise.toml                   # Toolchain versions (mise)
├── Cargo.toml                  # Rust workspace
├── .github/
│   ├── dependabot.yml             # Dependabot config (Actions + Cargo)
│   └── workflows/
│       ├── ci.yml                 # 3-OS CI matrix
│       ├── release.yml            # Tag-driven release artifacts
│       └── codeql.yml             # CodeQL static analysis
├── .opencode/                  # OpenCode agents, skills, context, tools
├── .claude/  .gemini/  .agents/  # AI tool skills (commit, create-branch)
└── docs/                       # This documentation
```

## Notes

- The **build system is XMake** (`xmake.lua`). The CI and release workflows
  build with XMake. The legacy packaging files used by local scripts
  (`debian/rules`, `PKGBUILD`) still reference the removed Meson build; see
  [packaging](packaging.md).
- The `ai_core` crate is built automatically by an xmake `before_build` hook
  (`cargo build --lib`) and linked as a static library.
- The data layout at runtime is documented in
  [configuration](../getting-started/configuration.md).
