# Source Layout

A quick tour of where everything lives in the repository.

```
├── crates/
│   ├── app/                    # GPUI application (Rust)
│   │   └── src/
│   │       ├── main.rs         # Entry point (GPUI app)
│   │       ├── app.rs          # Root view: toolbar, sidebar, file tree, diff, commit panel
│   │       ├── sidebar.rs      # Branches / History / Projects navigation
│   │       ├── file_tree.rs    # Status file list with staging checkboxes
│   │       ├── diff_view.rs    # Diff viewer (syntax highlighting, inline images)
│   │       ├── diff.rs         # Diff parsing helpers
│   │       ├── commit_panel.rs # Commit summary/description + AI + stash/reset actions
│   │       ├── settings_view.rs# Settings dialog (AI, SSH keys, remotes, appearance)
│   │       ├── git_service.rs  # Git CLI wrapper (async, for the GPUI app)
│   │       ├── color.rs        # Theme/color helpers
│   │       └── lib.rs          # Crate root, module wiring
│   ├── ai_core/                # Rust AI engine
│   │   └── src/
│   │       ├── lib.rs          # Crate root, module wiring
│   │       ├── ffi.rs          # C FFI exports (kept; C++ consumer removed)
│   │       ├── cloud.rs        # Cloud providers (OpenRouter, OpenAI, Anthropic, Google)
│   │       ├── inference.rs    # GGUF model loading + streaming inference
│   │       ├── download.rs     # HuggingFace downloads
│   │       ├── discovery.rs    # Model discovery + ggml backend detection
│   │       ├── commit_message.rs  # Conventional Commits generation
│   │       ├── types.rs        # Shared types / CommitContext JSON
│   │       └── error.rs        # Error types
│   ├── vcs_core/               # Rust VCS/SSH engine
│   │   └── src/
│   │       ├── lib.rs          # Crate root, module wiring
│   │       ├── ffi.rs          # C FFI exports (kept; C++ consumer removed)
│   │       ├── ssh.rs          # SSH key generation/listing/fingerprinting
│   │       ├── connect.rs      # SSH connection testing (russh, no ssh CLI)
│   │       ├── remote.rs       # Git remote read/write (gix-config)
│   │       └── error.rs        # Error types
│   ├── config/                 # Rust config crate
│   │   └── src/
│   │       ├── lib.rs          # Crate root
│   │       ├── ffi.rs          # C FFI exports (kept; C++ consumer removed)
│   │       ├── paths.rs        # Data path resolution
│   │       ├── projects.rs     # Recent projects
│   │       ├── settings.rs     # Application settings
│   │       └── themes.rs       # Custom themes
│   ├── git_cmd/                # Rust git CLI wrapper
│   │   └── src/
│   │       ├── lib.rs          # Crate root
│   │       ├── ffi.rs          # C FFI exports (kept; C++ consumer removed)
│   │       ├── git.rs          # Git command execution
│   │       ├── jj.rs           # Jujutsu support
│   │       └── types.rs        # Shared types
│   ├── watcher/                # Rust filesystem watcher
│   │   └── src/
│   │       ├── lib.rs          # Crate root
│   │       └── ffi.rs          # C FFI exports (kept; C++ consumer removed)
│   └── addons/                 # Rust addon manager
│       └── src/
│           ├── lib.rs          # Crate root
│           ├── ffi.rs          # C FFI exports (kept; C++ consumer removed)
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
├── justfile                    # Task runner recipes
├── mise.toml                   # Toolchain versions (mise)
├── Cargo.toml                  # Rust workspace
├── moon.yml                    # App project config (moon)
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

- The **whole application builds with Cargo** (`cargo build --workspace`):
  the GPUI frontend (`crates/app`) consumes the backend crates (`config`,
  `git_cmd`, `watcher`, `ai_core`, `vcs_core`, `addons`) directly as Rust
  libraries. Moon (`.moon/` + `moon.yml`) orchestrates workspace tasks; the
  `justfile` wraps the common workflows.
- The `ai_core`, `vcs_core`, and other crates still ship `ffi.rs` modules and
  `staticlib` crate-types for backwards compatibility with external C++ hosts,
  but the bundled app no longer uses the C ABI (the old C++ UI in `src/` and
  the `include/*.h` headers were removed).
- The data layout at runtime is documented in
  [configuration](../getting-started/configuration.md).