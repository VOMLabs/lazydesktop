---
name: release-engineering
description: Manages build, packaging, and release workflows for LazyDesktop. Trigger when the user asks to build, package, create a release, configure CI/CD, or produce distribution artifacts (AppImage, .deb, .pkg.tar.zst, .msi).
---

# Release Engineering Skill

This skill covers the build and release pipeline for LazyDesktop, a Qt 6 / C++23 / Meson project.

## Build System

- **Build tool**: Meson + Ninja
- **Build types**: `debugoptimized` (dev), `release` (distribution)
- **Compiler**: GCC 14+ or Clang 18+ (C++23 required)
- **Qt 6 modules**: Core, Gui, Widgets, Network

## Release Artifacts

| Format | Target | Tooling |
|--------|--------|---------|
| .AppImage | Linux (universal) | linuxdeploy + linuxdeploy-plugin-qt + appimagetool |
| .deb | Debian/Ubuntu | dpkg-buildpackage (via debian/) |
| .pkg.tar.zst | Arch Linux | makepkg (via PKGBUILD) |
| .msi | Windows | WiX Toolset (via install/windows/lazydesktop.wxs) |

## Workflow

1. `just setup-release` — configure Meson with release flags (`-Dbuildtype=release -Dwarning_level=0 -Db_strip=true -Db_lto=true`)
2. `just build-release` — compile with Ninja
3. `just release` — runs all artifact targets sequentially
4. Individual targets: `release-linux-appimage`, `release-linux-deb`, `release-linux-arch`, `release-windows-msi`

## Dependencies

- Qt 6 development packages
- yaml-cpp
- meson >= 1.0.0, ninja
- git
- Packaging: linuxdeploy, appimagetool, debhelper, WiX Toolset
