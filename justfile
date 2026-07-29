# ─── Variables ───────────────────────────────────────────
builddir := "build"
project := "lazydesktop"
appdir := "AppDir"

# ─── Default ─────────────────────────────────────────────
default: build

# ─── Setup ───────────────────────────────────────────────
setup:
    meson setup {{builddir}} --buildtype=debugoptimized -Dwarning_level=3

setup-release:
    meson setup {{builddir}} --buildtype=release -Dwarning_level=0 -Db_strip=true -Db_lto=true

# ─── Build ───────────────────────────────────────────────
build: setup
    ninja -C {{builddir}}

build-release: setup-release
    ninja -C {{builddir}}

# ─── Run ─────────────────────────────────────────────────
run: build
    ./{{builddir}}/src/{{project}}

# ─── Clean ───────────────────────────────────────────────
clean:
    rm -rf {{builddir}} {{appdir}}

distclean: clean
    rm -rf subprojects/*

# ─── Format ───────────────────────────────────────────────
format:
    clang-format -i -style=file src/*.cpp src/*.h

# ─── Tidy ─────────────────────────────────────────────────
tidy:
    clang-tidy src/*.cpp src/*.h -- -std=c++23

# ─── Lint ────────────────────────────────────────────────
lint:
    pre-commit run --all-files

# ─── Release (all distribution artifacts) ────────────────
release: (release-linux-appimage release-linux-deb release-linux-arch release-windows-msi)

# ─── Release: Linux .AppImage ────────────────────────────
release-linux-appimage: build-release
    # Strip debug symbols
    strip {{builddir}}/src/{{project}}
    # Bundle into AppDir using linuxdeploy
    linuxdeploy-x86_64.AppImage --appdir {{appdir}} --executable {{builddir}}/src/{{project}} --plugin qt
    # Generate AppImage
    appimagetool-x86_64.AppImage {{appdir}}
    mv {{project}}*-x86_64.AppImage {{project}}-x86_64.AppImage

# ─── Release: Linux .deb ─────────────────────────────────
release-linux-deb: build-release
    ./scripts/build-deb.sh

# ─── Release: Arch Linux .pkg.tar.zst ───────────────────
release-linux-arch: build-release
    ./scripts/build-arch.sh

# ─── Release: Windows .msi ───────────────────────────────
release-windows-msi: build-release
    ./install/windows/build-msi.bat
