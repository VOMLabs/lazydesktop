# Build from Source

LazyDesktop builds with **Meson + Ninja** for the C++ application and
**Cargo** for the bundled Rust crates (`ai_core`, `vcs_core`, `config`,
`git_cmd`, `watcher`). [Moon](https://moonrepo.dev) orchestrates the
workspace tasks.

## Requirements

- **Qt 6** (Core, Gui, Widgets, Network) — Qt 6.7+ recommended
- **Rust stable toolchain** — builds the bundled crates
- **meson + ninja** — the C++ build system
- **moon** — task orchestration (`just setup` installs it via `cargo install
  moonrepo` if missing)
- **yaml-cpp** — YAML parsing for projects and themes (Qt UI)
- **Git** — required at runtime (all Git operations use the CLI)
- **C++23 compiler** — GCC 14+ or Clang 18+ (only needed while the Qt UI
  ships)

A convenient way to provision the toolchain is [`mise`](https://mise.jdx.dev)
with the pinned versions in [`mise.toml`](../../mise.toml):

```bash
mise install
```

## Configure and build

```bash
just setup           # install moon if needed, then moon sync
just build           # cargo build --workspace + meson setup + ninja
```

Or run the steps directly:

```bash
cargo build --workspace
meson setup build --buildtype=debugoptimized --reconfigure
ninja -C build
```

The binary is written to `./build/lazydesktop`. Run it directly from the
build directory — no `make install` needed.

## Using the justfile

A [`justfile`](../../justfile) wraps the common workflows. With
[`just`](https://github.com/casey/just) installed:

| Command | What it does |
|---------|--------------|
| `just setup` | Install moon if needed, then `moon sync` |
| `just build` | Build debug (Rust crates + C++ app) |
| `just build-release` | Build release |
| `just build-rust` | Build only the Rust crates |
| `just run` / `just run-release` | Build and run |
| `just test` | Run the Rust test suites (`cargo test --workspace`) |
| `just test-crate <crate>` | Run tests for one crate |
| `just format` | Run clang-format on `src/*.cpp` and `src/*.h` |
| `just format-rust` | Run `cargo fmt --all` |
| `just tidy` | Run clang-tidy static analysis |
| `just clippy` | Run `cargo clippy --workspace -- -D warnings` |
| `just audit` | Run `cargo audit` for security vulnerabilities |
| `just lint` | Run all pre-commit hooks |
| `just compile-commands` | Generate `compile_commands.json` for clangd |
| `just clean` | Remove build artifacts and packaging output |
| `just distclean` | Clean everything including Rust targets |

## Where your data lives

Your projects and settings survive rebuilds:

- Projects: `~/.config/lazydesktop/projects.yaml`
- Settings: `~/.config/lazydesktop/lazydesktop.conf`
- Custom themes: `~/.config/lazydesktop/themes/*.theme.yaml`
- Local AI models: `~/.config/lazydesktop/models/`

See [configuration](configuration.md) for the full reference.

> **Note:** the bundled Rust `config` crate (used by the in-development GPUI
> frontend) persists projects and themes as Lua (`projects.lua`,
> `*.theme.lua`). The Qt UI still reads YAML directly.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| meson picked the wrong Qt | Point `PATH` at the Qt 6 bin dir and reconfigure (`meson setup build --reconfigure`) |
| `cargo` is not found during build | Install the Rust stable toolchain and ensure it is on `PATH` |
| Missing `yaml-cpp` headers | Install `libyaml-cpp-dev` (Debian/Ubuntu), `yaml-cpp` (Arch), or the equivalent |
| Link errors about the Rust crates | Run `just clean` and rebuild; the crates are rebuilt by `cargo build --workspace` |