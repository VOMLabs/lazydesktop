# Maintainer: LazyDesktop Team <team@lazydesktop.dev>
# Contributor: Your Name <you@example.com>

pkgname=lazydesktop
pkgver=0.3.0
pkgrel=1
pkgdesc='A native Git GUI for the KDE Plasma desktop'
arch=('x86_64' 'aarch64')
url='https://lazydesktop.dev'
license=('MIT')
depends=(
  'git'
  'shared-mime-info'
)
makedepends=(
  'rust'
  'cargo'
  'cmake'
  'ninja'
)

source=("${pkgname}-${pkgver}.tar.gz::https://github.com/itzzmateo/lazydesktop/archive/v${pkgver}.tar.gz")
sha256sums=('SKIP')
validpgpkeys=()

build() {
  cargo build --release --workspace
}

check() {
  cargo test --workspace
}

package() {
  install -Dm755 "${srcdir}/${pkgname}-${pkgver}/target/release/lazydesktop" "${pkgdir}/usr/bin/lazydesktop"
  install -Dm644 "${srcdir}/${pkgname}-${pkgver}/data/lazydesktop.desktop" "${pkgdir}/usr/share/applications/lazydesktop.desktop"
  install -Dm644 "${srcdir}/${pkgname}-${pkgver}/data/icons/hicolor/scalable/apps/lazydesktop.svg" "${pkgdir}/usr/share/icons/hicolor/scalable/apps/lazydesktop.svg"

  # Install SVG icon in additional sizes via symlinks
  install -dm755 "${pkgdir}/usr/share/icons/hicolor/48x48/apps"
  install -dm755 "${pkgdir}/usr/share/icons/hicolor/256x256/apps"
  ln -s "../../../scalable/apps/lazydesktop.svg" \
    "${pkgdir}/usr/share/icons/hicolor/48x48/apps/lazydesktop.svg"
  ln -s "../../../scalable/apps/lazydesktop.svg" \
    "${pkgdir}/usr/share/icons/hicolor/256x256/apps/lazydesktop.svg"
}