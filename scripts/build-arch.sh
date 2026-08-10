#!/bin/bash
# Build .pkg.tar.zst package for Arch Linux
# Requires: base-devel, meson, ninja
#
# Usage:
#   ./scripts/build-arch.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

cd "${SCRIPT_DIR}"

if [ ! -f "PKGBUILD" ]; then
    echo "Error: PKGBUILD not found. Run from project root."
    exit 1
fi

echo "Building Arch Linux package..."
makepkg -s --cleanbuild

echo ""
echo "=== Package built successfully ==="
echo "Package: lazydesktop-0.2.0-1-x86_64.pkg.tar.zst"
echo ""
echo "Install with: sudo pacman -U lazydesktop-0.2.0-1-x86_64.pkg.tar.zst"
