# Build from Source

LazyDesktop builds with **XMake**, which automatically compiles the bundled
Rust `ai_core` crate and links it into the C++ binary.

## Requirements

- **Qt 6** (Core, Gui, Widgets, Network) — Qt 6.7+ recommended
- **xmake** — the build system
- **Rust stable toolchain** — builds `crates/ai_core` automatically
- **yaml-cpp** — YAML parsing for projects and themes
- **Git** — required at runtime (all Git operations use the CLI)
- **C++23 compiler** — GCC 14+ or Clang 18+

A convenient way to provision the toolchain is [`mise`](https://mise.jdx.dev)
with the pinned versions in [`mise.toml`](../../mise.toml):

```bash
mise install
```

## Configure and build

```bash
xmake f -m debug          # configure (release: xmake f -m release)
xmake                     # builds C++ and the Rust ai_core crate
```

The binary is written to:

```bash
./build/linux/x86_64/debug/lazydesktop      # debug
./build/linux/x86_64/release/lazydesktop    # release
```

Run it directly from the build directory — no `make install` needed.

> **Note on Qt versions:** if both Qt 5 and Qt 6 are installed, xmake may
> pick Qt 5 from your `PATH`. Point it at Qt 6 before configuring, for
> example:
>
> ```bash
> PATH=/usr/lib/qt6/bin:$PATH xmake f -c -m debug
> ```

## Using the justfile

A [`justfile`](../../justfile) wraps the common workflows. With
[`just`](https://github.com/casey/just) installed:

| Command | What it does |
|---------|--------------|
| `just setup` | Configure xmake for a debug build |
| `just setup-release` | Configure xmake for a release build |
| `just build` | Build debug (C++ + Rust) |
| `just build-release` | Build release |
| `just run` / `just run-release` | Build and run |
| `just dev` | Configure, build, and run |
| `just test` | Run the Rust `ai_core` test suite (`cargo test`) |
| `just format` | Run clang-format on `src/*.cpp` and `src/*.h` |
| `just tidy` | Run clang-tidy static analysis |
| `just lint` | Run all pre-commit hooks |
| `just compile-commands` | Generate `compile_commands.json` for clangd |
| `just clean` | Remove xmake artifacts and packaging output |
| `just distclean` | Clean everything including Rust targets |

## Where your data lives

Your projects and settings survive rebuilds:

- Projects: `~/.config/lazydesktop/projects.yaml`
- Settings: `~/.config/lazydesktop/lazydesktop.conf`
- Custom themes: `~/.config/lazydesktop/themes/*.theme.yaml`
- Local AI models: `~/.config/lazydesktop/models/`

See [configuration](configuration.md) for the full reference.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| xmake picked the wrong Qt | Point `PATH` at the Qt 6 bin dir and reconfigure (`xmake f -c`) |
| `cargo` is not found during build | Install the Rust stable toolchain and ensure it is on `PATH` |
| Missing `yaml-cpp` headers | Install `libyaml-cpp-dev` (Debian/Ubuntu), `yaml-cpp` (Arch), or the equivalent |
| Link errors about `ai_core` | Run `just clean` and rebuild; the crate is rebuilt by the `before_build` hook |
