# Build from Source

LazyDesktop builds entirely with **Cargo** — the GPUI application
(`crates/app`) and the backend crates (`git_cmd`, `ai_core`, `vcs_core`,
`config`, `watcher`, `addons`). [Moon](https://moonrepo.dev) orchestrates the
workspace tasks.

## Requirements

- **Rust stable toolchain** — builds the app and all crates
- **moon** — task orchestration (`just setup` installs it via `cargo install
  moonrepo` if missing)
- **Git** — required at runtime (all Git operations use the CLI)
- **Linux build deps** — `cmake` and `ninja` are needed by C-code in the
  dependency tree (`aws-lc-sys`, `llama-cpp-2`); Ubuntu:
  `sudo apt-get install -y cmake ninja-build`

A convenient way to provision the toolchain is [`mise`](https://mise.jdx.dev)
with the pinned versions in [`mise.toml`](../../mise.toml):

```bash
mise install
```

## Configure and build

```bash
just setup           # install moon if needed, then moon sync
just build           # cargo build --workspace
```

Or run directly:

```bash
cargo build --workspace
```

The binary is written to `./target/debug/lazydesktop` (release:
`./target/release/lazydesktop`). Run it directly from the target directory —
no `make install` needed.

## Using the justfile

A [`justfile`](../../justfile) wraps the common workflows. With
[`just`](https://github.com/casey/just) installed:

| Command | What it does |
|---------|--------------|
| `just setup` | Install moon if needed, then `moon sync` |
| `just build` | Build debug (`cargo build --workspace`) |
| `just build-release` | Build release |
| `just run` / `just run-release` | Build and run |
| `just test` | Run the Rust test suites (`cargo test --workspace`) |
| `just test-crate <crate>` | Run tests for one crate |
| `just format` | Run `cargo fmt --all` |
| `just clippy` | Run `cargo clippy --workspace -- -D warnings` |
| `just audit` | Run `cargo audit` for security vulnerabilities |
| `just lint` | Run all pre-commit hooks |
| `just clean` | Remove packaging output |
| `just distclean` | Clean everything including Rust targets |

## Where your data lives

Your projects and settings survive rebuilds:

- Projects: `~/.config/lazydesktop/projects.yaml`
- Settings: `~/.config/lazydesktop/lazydesktop.conf`
- Custom themes: `~/.config/lazydesktop/themes/*.theme.yaml`
- Local AI models: `~/.config/lazydesktop/models/`

See [configuration](configuration.md) for the full reference.

> **Note:** the Rust `config` crate persists projects and themes.
> See the [roadmap](../../ROADMAP.md) for the persistence migration status.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `cargo` is not found during build | Install the Rust stable toolchain and ensure it is on `PATH` |
| Link errors about C deps (`aws-lc-sys`, `llama-cpp-2`) | Ensure `cmake` and `ninja` are installed, then rebuild |
| App fails to start in a headless shell | Use `xvfb-run -a target/debug/lazydesktop` or a real X/Wayland session |
| Missing runtime libraries on Linux | Install `libgl1` and `libxkbcommon-x11-0` |