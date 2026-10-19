#!/usr/bin/env bash
set -euo pipefail

PREFIX="${1:-$HOME/.local}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

install -Dm755 "$HERE/mdlite" "$PREFIX/bin/mdlite"
install -Dm644 "$HERE/marklite.desktop" "$PREFIX/share/applications/marklite.desktop"
install -Dm644 "$HERE/marklite.png" "$PREFIX/share/icons/hicolor/256x256/apps/marklite.png"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$PREFIX/share/applications" >/dev/null 2>&1 || true
fi

echo "MarkLite installed to $PREFIX"
echo "Run: $PREFIX/bin/mdlite"
