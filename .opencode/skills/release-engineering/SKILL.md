---
name: release-engineering
description: Manages build, packaging, and release workflows for LazyDesktop. Trigger when the user asks to build, package, create a release, configure CI/CD, or produce distribution artifacts (AppImage, .deb, .pkg.tar.zst, .msi).
---

# Release Engineering Skill

This skill covers the build and release pipeline for LazyDesktop, a **Rust +
GPUI** project built entirely with Cargo (no C++, no Meson).

## Build System

- **Build tool**: Cargo (single workspace, 7 crates including `crates/app`)
- **Profiles**: `debug` (dev), `release` (distribution; LTO + opt-level=z + strip)
- **Task runner**: just (`justfile`) + Moon for orchestration
- **Toolchain**: pinned in `mise.toml` (`mise install`)

## Release Artifacts

| Format | Target | Tooling |
|--------|--------|---------|
| .AppImage | Linux (universal) | linuxdeploy + appimagetool |
| .deb | Debian/Ubuntu | dpkg-buildpackage (via debian/, `cargo build --release`) |
| .pkg.tar.zst | Arch Linux | makepkg (via PKGBUILD, `cargo build --release`) |
| .msi | Windows | WiX Toolset (via install/windows/lazydesktop.wxs) |

## Workflow

1. `just build-release` — `cargo build --workspace --release`
2. `just release` — runs all artifact targets sequentially
3. Individual targets: `release-linux-appimage`, `release-linux-deb`, `release-linux-arch`, `release-windows-msi`

Tagging the repo with `v*` triggers `.github/workflows/release.yml`, which
builds the release binary with Cargo and packages every artifact.

## Dependencies

- Rust stable toolchain (`cargo`, `rustc`)
- cmake + ninja (for C code in the dep tree: `aws-lc-sys`, `llama-cpp-2`)
- Packaging: linuxdeploy, appimagetool, debhelper, WiX Toolset
- Runtime deps on Linux: `git`, `libgl1`, `libxkbcommon-x11-0`, `shared-mime-info`