# Maintainer: LazyDesktop Team <team@lazydesktop.dev>
# Contributor: Your Name <you@example.com>

pkgname=lazydesktop
pkgver=0.1.0
pkgrel=1
pkgdesc='A KDE-native Git GUI alternative with LazyAddons plugin system'
arch=('x86_64' 'aarch64')
url='https://lazydesktop.dev'
license=('MIT')
depends=(
  'qt6-base'
  'yaml-cpp'
  'git'
  'shared-mime-info'
)
makedepends=(
  'meson'
  'ninja'
  'gcc'   # or clang
  'lua'
)
optdepends=(
  'lua: Lua plugin support'
  'python-uv: Python plugin support'
)
source=("${pkgname}-${pkgver}.tar.gz::https://github.com/itzzmateo/lazydesktop/archive/v${pkgver}.tar.gz")
sha256sums=('SKIP')
validpgpkeys=()

build() {
  arch-meson "${pkgname}-${pkgver}" build \
    -Dbuildtype=release \
    -Dwarning_level=0
  ninja -C build
}

check() {
  echo "No test suite configured."
}

package() {
  DESTDIR="${pkgdir}" ninja -C build install

  # Update MIME database on install
  install -Dm644 "${srcdir}/${pkgname}-${pkgver}/data/lazydesktop-addons.xml" \
    "${pkgdir}/usr/share/mime/packages/lazydesktop-addons.xml"

  # Install SVG icon in additional sizes via symlinks
  install -dm755 "${pkgdir}/usr/share/icons/hicolor/48x48/apps"
  install -dm755 "${pkgdir}/usr/share/icons/hicolor/256x256/apps"
  ln -s "../../../scalable/apps/lazydesktop.svg" \
    "${pkgdir}/usr/share/icons/hicolor/48x48/apps/lazydesktop.svg"
  ln -s "../../../scalable/apps/lazydesktop.svg" \
    "${pkgdir}/usr/share/icons/hicolor/256x256/apps/lazydesktop.svg"
}
