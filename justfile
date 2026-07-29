# ─── Variables ───────────────────────────────────────────
builddir := "build"
project := "lazydesktop"
binary := builddir + "/linux/x86_64/debug/" + project
binary_release := builddir + "/linux/x86_64/release/" + project
appdir := "AppDir"

# ─── Default ─────────────────────────────────────────────
default: build

# ─── Setup ───────────────────────────────────────────────
setup:
    xmake f -m debug

setup-release:
    xmake f -m release

# ─── Build ───────────────────────────────────────────────
build:
    xmake

build-release:
    xmake -m release

# ─── Generate compile_commands.json for LSP (clangd) ────
compile-commands:
    xmake project -k compile_commands --lsp=clangd

# ─── Run ─────────────────────────────────────────────────
run: build
    {{ binary }}

# ─── Clean ───────────────────────────────────────────────
clean:
    xmake clean --all
    rm -rf {{ builddir }} {{ appdir }} .xmake

distclean: clean
    rm -rf crates/*/target .xmake

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
release: release-linux-appimage release-linux-deb release-linux-arch release-windows-msi

# ─── Release: Linux .AppImage ────────────────────────────
release-linux-appimage: build-release
    # Strip debug symbols
    strip {{ binary_release }}
    # Bundle into AppDir using linuxdeploy
    linuxdeploy-x86_64.AppImage --appdir {{ appdir }} --executable {{ binary_release }} --plugin qt
    # Generate AppImage
    appimagetool-x86_64.AppImage {{ appdir }}
    mv {{ project }}*-x86_64.AppImage {{ project }}-x86_64.AppImage

# ─── Release: Linux .deb ─────────────────────────────────
release-linux-deb: build-release
    ./scripts/build-deb.sh

# ─── Release: Arch Linux .pkg.tar.zst ───────────────────
release-linux-arch: build-release
    ./scripts/build-arch.sh

# ─── Release: Windows .msi ───────────────────────────────
release-windows-msi: build-release
    ./install/windows/build-msi.bat
