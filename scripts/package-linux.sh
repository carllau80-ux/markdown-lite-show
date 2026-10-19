#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-$(grep -m1 '^version = ' "$ROOT/Cargo.toml" | cut -d'"' -f2)}"
BIN="${2:-$ROOT/target/release/mdlite}"
ARCH_RAW="$(uname -m)"
case "$ARCH_RAW" in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) ARCH="$ARCH_RAW" ;;
esac
OUT="$ROOT/dist/linux"
STAGE="$OUT/MarkLite"

if [[ ! -x "$BIN" ]]; then
    echo "Missing executable: $BIN" >&2
    exit 1
fi

rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$BIN" "$STAGE/mdlite"
cp "$ROOT/README.md" "$ROOT/LICENSE" "$STAGE/"
cp "$ROOT/packaging/linux/marklite.desktop" "$STAGE/"
cp "$ROOT/packaging/linux/install.sh" "$STAGE/"
cp "$ROOT/assets/icon.png" "$STAGE/marklite.png"
chmod +x "$STAGE/mdlite" "$STAGE/install.sh"

ARCHIVE="$ROOT/dist/MarkLite-${VERSION}-linux-${ARCH}.tar.gz"
tar -C "$OUT" -czf "$ARCHIVE" MarkLite
echo "$ARCHIVE"
