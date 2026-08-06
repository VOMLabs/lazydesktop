# Packaging

LazyDesktop produces several distribution artifacts. This page documents each
one and how it is built.

## Artifact overview

| Artifact | Platform | Builder |
|----------|----------|---------|
| `.AppImage` | Linux | linuxdeploy + Qt plugin |
| `.deb` | Debian / Ubuntu | `debian/` + `dpkg-deb` |
| `.pkg.tar.zst` | Arch Linux | `PKGBUILD` + makepkg |
| `.msi` | Windows | WiX Toolset |
| Docker image | Any | `Dockerfile` + `docker-compose.yml` |

## From the justfile

```bash
just build-release            # compile a release binary first
just release-linux-appimage   # .AppImage
just release-linux-deb        # .deb
just release-linux-arch       # .pkg.tar.zst
just release-windows-msi      # .msi
just release                  # all of the above
```

## Linux .AppImage

```bash
just release-linux-appimage
```

This strips the release binary, packages it with linuxdeploy and the Qt
plugin into an AppDir, then produces `lazydesktop-x86_64.AppImage` with
appimagetool.

## Debian / Ubuntu .deb

```bash
./scripts/build-deb.sh
# or: just release-linux-deb
```

The script runs `dpkg-buildpackage -us -uc -b` using the `debian/` directory.
Install with `sudo dpkg -i lazydesktop_*.deb`.

> **Caveat:** `debian/rules` still uses the Meson build system
> (`--buildsystem=meson`), and no `meson.build` is currently in the tree.
> The XMake migration (see the [roadmap](../../ROADMAP.md)) should update the
> packaging scripts before they are used to build source packages.

## Arch Linux .pkg.tar.zst

```bash
./scripts/build-arch.sh
# or: just release-linux-arch
```

Runs `makepkg -s --cleanbuild` using the `PKGBUILD`, producing
`lazydesktop-<version>-1-x86_64.pkg.tar.zst`.

## Windows .msi

```bash
just release-windows-msi
```

Uses the WiX Toolset with `install/windows/lazydesktop.wxs` and
`install/windows/build-msi.bat`.

## Docker

```bash
docker compose build
```

See the [Docker guide](../getting-started/docker.md) for running instructions.

## CI / release automation

Tagging the repository with `v*` triggers `.github/workflows/release.yml`,
which builds and uploads:

- **Linux**: `.deb` + `.AppImage` (ubuntu-24.04)
- **Arch**: `.pkg.tar.zst` (archlinux container)
- **Windows**: `.msi` (windows-2022, WiX)
- A `checksums.txt` of all artifacts

It then creates a GitHub Release with generated notes, marked as a
**prerelease**. See [release process](../release-process.md).

## Requirements for local release builds

- linuxdeploy + linuxdeploy-plugin-qt + appimagetool (or `mise install`)
- `debhelper`, `meson`, `ninja` for the `.deb`
- `base-devel` (makepkg) for Arch
- WiX Toolset (`.NET tool`) for the MSI
