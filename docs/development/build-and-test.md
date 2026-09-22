# Build & Test

## Building

The whole application — the GPUI frontend (`crates/app`) and the backend
crates (`git_cmd`, `ai_core`, `vcs_core`, `config`, `watcher`, `addons`) —
builds with **Cargo**:

```bash
cargo build --workspace
cargo build --workspace --release   # release
```

Or use the `justfile` recipes (`just setup`, `just build`, `just run`). See
[build from source](../getting-started/build-from-source.md) for details.

## Testing

Rust unit and integration tests cover the backend crates and the git command
layer:

```bash
just test
# equivalent:
cargo test --workspace
# or per crate:
cargo test -p ai_core
cargo test -p vcs_core
cargo test -p lazydesktop-git-cmd
```

## Static analysis & formatting

- **Rust** — `just format` (`cargo fmt --all`), `just clippy`
  (`cargo clippy --workspace --all-targets -- -D warnings`), `just audit`
  (`cargo audit`).
- **pre-commit** — `just lint` runs all hooks from
  `.pre-commit-config.yaml`:

  | Hook | Purpose |
  |------|---------|
  | `trailing-whitespace` | Remove trailing whitespace |
  | `end-of-file-fixer` | Ensure newline at end of files |
  | `check-yaml` | Validate YAML |
  | `check-added-large-files` | Block large files from being committed |
  | `check-merge-conflict` | Detect conflict markers |
  | `mixed-line-ending` | Normalize line endings to LF |

## Continuous integration

GitHub Actions runs on push/PR to `main` (`.github/workflows/ci.yml`):

- **Matrix**: `ubuntu-latest`, `windows-2022`, `macos-14`
- **Steps**: checkout → Rust toolchain → `cargo build --workspace` →
  `cargo test --workspace` → clippy/fmt/audit → pre-commit hooks on the
  changed files (Linux) → Xvfb smoke test (Linux) / background launch
  (macOS)
- Debug binaries are uploaded as artifacts on pushes to `main` (7-day
  retention).

A separate workflow (`.github/workflows/codeql.yml`) runs GitHub CodeQL
static analysis on the Rust sources, and Dependabot
(`.github/dependabot.yml`) keeps Actions and Rust dependencies up to date.

## Toolchain

`mise.toml` pins the recommended tool versions (moon, Rust, linuxdeploy,
appimagetool). Install with:

```bash
mise install
```