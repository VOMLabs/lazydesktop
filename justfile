# ─── Variables ───────────────────────────────────────────────────
project         := "lazydesktop"
builddir        := "build"
bindir          := builddir + "/linux/x86_64"
binary          := bindir + "/debug/" + project
binary_release  := bindir + "/release/" + project
appdir          := "AppDir"
ai_core_toml    := "crates/ai_core/Cargo.toml"
vcs_core_toml   := "crates/vcs_core/Cargo.toml"

# ─── Default ─────────────────────────────────────────────────────
# Alias for `just build`
default: build

# ─── Setup ───────────────────────────────────────────────────────
# Configure xmake for a debug build (Qt 6 must be on PATH)
setup:
    xmake f -m debug

# Configure xmake for a release build
setup-release:
    xmake f -m release

# Configure, build, and run the app
dev: setup build
    {{ binary }}

# ─── Build ───────────────────────────────────────────────────────
# Build debug (C++ UI + bundled Rust ai_core in one step)
build:
    xmake

# Build release
build-release:
    xmake -m release

# Build only the Rust ai_core crate
build-ai-core:
    cargo build --manifest-path {{ ai_core_toml }}

# Build only the Rust vcs_core crate
build-vcs-core:
    cargo build --manifest-path {{ vcs_core_toml }}

# Generate compile_commands.json for clangd
compile-commands:
    xmake project -k compile_commands --lsp=clangd

# ─── Test ────────────────────────────────────────────────────────
# Run the Rust test suites
test:
    cargo test --workspace

# Run only the ai_core Rust tests
test-ai-core:
    cargo test --manifest-path {{ ai_core_toml }}

# Run only the vcs_core Rust tests
test-vcs-core:
    cargo test --manifest-path {{ vcs_core_toml }}

# ─── Run ─────────────────────────────────────────────────────────
# Build and run the debug binary
run: build
    {{ binary }}

# Build and run the release binary
run-release: build-release
    {{ binary_release }}

# ─── Quality ─────────────────────────────────────────────────────
# Format C++ sources with clang-format
format:
    clang-format -i -style=file src/*.cpp src/*.h

# Run clang-tidy static analysis
tidy:
    clang-tidy src/*.cpp src/*.h -- -std=c++23

# Run all pre-commit hooks
lint:
    pre-commit run --all-files

# ─── Docker ──────────────────────────────────────────────────────
# Build the Docker image
docker-build:
    docker compose build

# Run the app in Docker (host X11, or VNC via http://localhost:6080)
docker-up:
    docker compose up -d

# Force browser/VNC mode
docker-up-vnc:
    VNC_MODE=1 docker compose up -d

# Stop and remove the container
docker-down:
    docker compose down

# Open a shell inside the running container
docker-shell:
    docker compose exec lazydesktop /bin/bash

# Tail container logs
docker-logs:
    docker compose logs -f

# ─── Release ─────────────────────────────────────────────────────
# Build all distribution artifacts
release: release-linux-appimage release-linux-deb release-linux-arch release-windows-msi

# Linux .AppImage
release-linux-appimage: build-release
    strip {{ binary_release }}
    linuxdeploy-x86_64.AppImage --appdir {{ appdir }} --executable {{ binary_release }} --plugin qt
    appimagetool-x86_64.AppImage {{ appdir }}
    mv {{ project }}*-x86_64.AppImage {{ project }}-x86_64.AppImage

# Linux .deb
release-linux-deb: build-release
    ./scripts/build-deb.sh

# Arch Linux .pkg.tar.zst
release-linux-arch: build-release
    ./scripts/build-arch.sh

# Windows .msi
release-windows-msi: build-release
    ./install/windows/build-msi.bat

# ─── Clean ───────────────────────────────────────────────────────
# Remove xmake artifacts and packaging output
clean:
    xmake clean --all
    rm -rf {{ builddir }} {{ appdir }} .xmake

# Remove xmake artifacts and all Rust targets
distclean: clean
    rm -rf crates/*/target .xmake
