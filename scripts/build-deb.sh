#!/bin/bash
# Build .deb package for Debian/Ubuntu
# Requires: debhelper, cargo, rustc
#
# Usage:
#   ./scripts/build-deb.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

cd "${SCRIPT_DIR}"

# Ensure we have the debian directory
if [ ! -d "debian" ]; then
    echo "Error: debian/ directory not found. Run from project root."
    exit 1
fi

# Install build dependencies (if run as root or with sudo)
if command -v dpkg-checkbuilddeps &>/dev/null; then
    echo "Checking build dependencies..."
    dpkg-checkbuilddeps 2>/dev/null || {
        echo "Missing build dependencies. Install them with:"
        echo "  sudo apt-get install $(dpkg-checkbuilddeps 2>&1 | sed "s/.*: //")"
    }
fi

# Clean any previous build artifacts
dh_clean 2>/dev/null || true

# Build the package
echo "Building .deb package..."
dpkg-buildpackage -us -uc -b

echo ""
echo "=== Package built successfully ==="
echo "Look for .deb files in $(dirname "${SCRIPT_DIR}")/"
