# Installation

LazyDesktop is distributed as a native binary for Linux, with experimental
Windows (MSI) packaging. Install it the way that suits your setup.

## Arch Linux

From the repository root (or a copy of the source), build and install the
package with `makepkg`:

```bash
makepkg -si
```

This produces `lazydesktop-<version>-1-x86_64.pkg.tar.zst` and installs it
with `pacman`. Alternatively build only the package:

```bash
makepkg -s --cleanbuild
sudo pacman -U lazydesktop-<version>-1-x86_64.pkg.tar.zst
```

The PKGBUILD depends on `qt6-base`, `yaml-cpp`, `git`, and
`shared-mime-info`.

## Debian / Ubuntu

Build a `.deb` with the helper script (requires `debhelper`, `meson`,
`ninja`, and the Qt 6 / yaml-cpp dev packages):

```bash
./scripts/build-deb.sh
```

Then install the produced file:

```bash
sudo dpkg -i lazydesktop_*.deb
```

## AppImage

1. Download the latest `lazydesktop-*-x86_64.AppImage` from the
   [Releases](https://github.com/itzzmateo/lazydesktop/releases) page.
2. Make it executable:

   ```bash
   chmod +x lazydesktop-*-x86_64.AppImage
   ```

3. Run it:

   ```bash
   ./lazydesktop-*-x86_64.AppImage
   ```

## Windows (MSI)

The Windows installer is built with the WiX toolset. Prebuilt `.msi` files
are attached to each GitHub release. Download and run the MSI, then launch
**LazyDesktop** from the Start menu.

> **Note:** Windows and macOS builds are best-effort. Linux (KDE Plasma) is
> the primary target.

## Docker

Prefer to run LazyDesktop in a container? See the
[Docker guide](docker.md).

## From source

See [build from source](build-from-source.md) for a full walkthrough.
