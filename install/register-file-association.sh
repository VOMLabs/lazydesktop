#!/bin/bash
# Register LazyAddons file associations for LazyDesktop
# Run this script after installing LazyDesktop

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MIME_DIR="${HOME}/.local/share/mime/packages"
APP_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor/scalable/mimetypes"

mkdir -p "${MIME_DIR}"
mkdir -p "${APP_DIR}"
mkdir -p "${ICON_DIR}"

# Install MIME type definition
cp "${SCRIPT_DIR}/lazydesktop-addons.xml" "${MIME_DIR}/"

# Install desktop entry
cp "${SCRIPT_DIR}/lazydesktop-addon.desktop" "${APP_DIR}/"

# Update MIME database
update-mime-database "${HOME}/.local/share/mime" 2>/dev/null || true

# Update desktop database
update-desktop-database "${HOME}/.local/share/applications" 2>/dev/null || true

echo "File associations registered for .lza and .zip addon packages."
echo ""
echo "You may need to log out and back in for changes to take effect."
echo ""
echo "To test:"
echo "  xdg-mime query default application/vnd.lazydesktop.addon"
