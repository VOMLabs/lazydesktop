# ─── Variables ───────────────────────────────────────────────────
project         := "lazydesktop"
builddir        := "build"
binary          := builddir + "/lazydesktop"
binary_release  := builddir + "/lazydesktop"
appdir          := "AppDir"

# ─── Default ─────────────────────────────────────────────────────
default: build

# ─── Setup ───────────────────────────────────────────────────────
# Install moon if not present, then run moon setup
setup:
    @command -v moon >/dev/null 2>&1 || cargo install moonrepo
    moon sync

# ─── Build ───────────────────────────────────────────────────────
# Build Rust crates + C++ app (debug)
build:
    cargo build --workspace
    meson setup {{ builddir }} --buildtype=debugoptimized --reconfigure 2>/dev/null || true
    ninja -C {{ builddir }}

# Build Rust crates + C++ app (release)
build-release:
    cargo build --workspace --release
    meson setup {{ builddir }} --buildtype=release --reconfigure 2>/dev/null || true
    ninja -C {{ builddir }}

# Build only Rust crates
build-rust:
    cargo build --workspace

# Build only Rust crates (release)
build-rust-release:
    cargo build --workspace --release

# Generate compile_commands.json for clangd
compile-commands:
    meson setup {{ builddir }} --reconfigure 2>/dev/null || true

# ─── Test ────────────────────────────────────────────────────────
# Run all Rust test suites
test:
    cargo test --workspace

# Run tests for a specific crate
test-crate crate:
    cargo test -p {{ crate }}

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

# Format Rust sources
format-rust:
    cargo fmt --all

# Check Rust formatting
check-rust-fmt:
    cargo fmt --all -- --check

# Run clang-tidy static analysis
tidy:
    clang-tidy src/*.cpp src/*.h -- -std=c++23

# Run Rust clippy lints
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Run cargo audit for security vulnerabilities
audit:
    cargo audit

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
# Remove build artifacts
clean:
    rm -rf {{ builddir }} {{ appdir }} builddir

# Remove all build artifacts including Rust targets
distclean: clean
    rm -rf crates/*/target target
