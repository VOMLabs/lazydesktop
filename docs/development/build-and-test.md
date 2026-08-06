# Build & Test

## Building

The primary build system is **XMake** (`xmake.lua`). A `before_build` hook
compiles the Rust `ai_core` crate automatically:

```bash
xmake f -m debug        # configure
xmake                   # build C++ + Rust
```

Or use the `justfile` recipes (`just setup`, `just build`, `just run`). See
[build from source](../getting-started/build-from-source.md) for details.

## Testing

There is no C++ test suite yet. The Rust crate has unit tests:

```bash
just test
# equivalent:
cargo test --manifest-path crates/ai_core/Cargo.toml
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

- **Matrix**: `ubuntu-latest`, `windows-latest`, `macos-latest`
- **Steps**: checkout → Rust toolchain → xmake → Qt 6.7 (per OS) → yaml-cpp →
  `xmake f -m debug` → `xmake` → offscreen smoke test (Linux/macOS)

> **Note:** the CI build exercises the XMake path. The release workflow
> (`.github/workflows/release.yml`) still uses Meson for packaging; see
> [packaging](packaging.md).

## Toolchain

`mise.toml` pins the recommended tool versions (just, xmake, ninja, meson,
Rust, GCC 14, linuxdeploy, appimagetool). Install with:

```bash
mise install
```
