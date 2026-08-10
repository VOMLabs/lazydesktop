# Build & Test

## Building

The primary build system is **XMake** (`xmake.lua`). `before_build` hooks
compile the bundled Rust crates (`ai_core`, `vcs_core`) automatically:

```bash
xmake f -m debug        # configure
xmake                   # build C++ + Rust
```

Or use the `justfile` recipes (`just setup`, `just build`, `just run`). See
[build from source](../getting-started/build-from-source.md) for details.

## Testing

There is no C++ test suite yet. The Rust crates have unit tests:

```bash
just test
# equivalent:
cargo test --workspace
# or per crate:
cargo test --manifest-path crates/ai_core/Cargo.toml
cargo test --manifest-path crates/vcs_core/Cargo.toml
```

## Static analysis & formatting

- **clang-format** — `just format` formats `src/*.cpp` and `src/*.h` using
  `.clang-format` (LLVM-based style).
- **clang-tidy** — `just tidy` runs clang-tidy over the sources with
  `-std=c++23` (`.clang-tidy`).
- **pre-commit** — `just lint` runs all hooks from
  `.pre-commit-config.yaml`:

  | Hook | Purpose |
  |------|---------|
  | `clang-format` | Format C/C++ sources |
  | `cmake-format` | Sort/format CMake files (legacy) |
  | `trailing-whitespace` | Remove trailing whitespace |
  | `end-of-file-fixer` | Ensure newline at end of files |
  | `check-yaml` | Validate YAML |
  | `check-added-large-files` | Block large files from being committed |
  | `check-merge-conflict` | Detect conflict markers |
  | `mixed-line-ending` | Normalize line endings to LF |

## Continuous integration

GitHub Actions runs on push/PR to `main` (`.github/workflows/ci.yml`):

- **Matrix**: `ubuntu-24.04`, `windows-2022`, `macos-14`
- **Steps**: checkout → Rust toolchain → xmake → Qt 6.7 (per OS) → yaml-cpp →
  `xmake f -m debug` → `xmake` → `cargo test --workspace` → pre-commit hooks
  on the changed files (Linux) → offscreen smoke test (Linux/macOS)
- Debug binaries are uploaded as artifacts on pushes to `main` (7-day
  retention).

A separate workflow (`.github/workflows/codeql.yml`) runs GitHub CodeQL
static analysis on the C++ sources, and Dependabot
(`.github/dependabot.yml`) keeps Actions and Rust dependencies up to date.

## Toolchain

`mise.toml` pins the recommended tool versions (just, xmake, ninja, meson,
Rust, GCC 14, linuxdeploy, appimagetool). Install with:

```bash
mise install
```
