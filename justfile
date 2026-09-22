# ─── Variables ───────────────────────────────────────────────────
project         := "lazydesktop"
binary          := "target/debug/lazydesktop"
binary_release  := "target/release/lazydesktop"
appdir          := "AppDir"

# ─── Default ─────────────────────────────────────────────────────
default: build

# ─── Setup ───────────────────────────────────────────────────────
# Install moon if not present, then run moon sync
setup:
    @command -v moon >/dev/null 2>&1 || cargo install moonrepo
    moon sync

# ─── Build ───────────────────────────────────────────────────────
# Build the Rust workspace (debug)
build:
    cargo build --workspace

# Build the Rust workspace (release)
build-release:
    cargo build --workspace --release

# Generate rust-analyzer project metadata
compile-commands:
    cargo metadata --format-version 1

# ─── Test ────────────────────────────────────────────────────────
# Run all Rust test suites
test:
    cargo test --workspace

# Run tests for a specific crate
test-crate crate:
    cargo test -p {{ crate }}

# ─── Run ─────────────────────────────────────────────────────────
# Build and run the GPUI app (debug)
run:
    cargo run -p lazydesktop-app

# Build and run the GPUI app (release)
run-release:
    cargo run -p lazydesktop-app --release

# ─── Quality ─────────────────────────────────────────────────────
# Format Rust sources
format:
    cargo fmt --all

# Check Rust formatting
check-rust-fmt:
    cargo fmt --all -- --check

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
    linuxdeploy-x86_64.AppImage --appdir {{ appdir }} --executable {{ binary_release }}
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
    rm -rf {{ appdir }}

# Remove all build artifacts including Rust targets
distclean: clean
    rm -rf target