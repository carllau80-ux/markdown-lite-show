#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-$(grep -m1 '^version = ' "$ROOT/Cargo.toml" | cut -d'"' -f2)}"
BIN="${2:-$ROOT/target/universal-apple-darwin/release/mdlite}"
OUT="$ROOT/dist/macos"
STAGE="$OUT/MarkLite"
APP="$STAGE/MarkLite.app"
ICONSET="$OUT/AppIcon.iconset"
SOURCE_ICON="$ROOT/assets/icon-1024.png"

if [[ ! -x "$BIN" ]]; then
    echo "Missing executable: $BIN" >&2
    exit 1
fi
if ! command -v sips >/dev/null || ! command -v iconutil >/dev/null; then
    echo "This script must run on macOS (sips/iconutil required)." >&2
    exit 1
fi

rm -rf "$STAGE" "$ICONSET"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$ICONSET"

cp "$BIN" "$APP/Contents/MacOS/mdlite"
chmod +x "$APP/Contents/MacOS/mdlite"

for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$SOURCE_ICON" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" "$SOURCE_ICON" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"

sed "s/__VERSION__/$VERSION/g" "$ROOT/packaging/macos/Info.plist.in" > "$APP/Contents/Info.plist"
printf 'APPL????' > "$APP/Contents/PkgInfo"
cp "$ROOT/README.md" "$ROOT/LICENSE" "$STAGE/"

# Ad-hoc signing is required for the arm64 slice to launch, but this is not
# Developer ID signing or notarization.
codesign --force --deep --sign - "$APP" >/dev/null
plutil -lint "$APP/Contents/Info.plist" >/dev/null

ARCHIVE="$ROOT/dist/MarkLite-${VERSION}-macos-universal.zip"
rm -f "$ARCHIVE"
ditto -c -k --sequesterRsrc --keepParent "$STAGE" "$ARCHIVE"
echo "$ARCHIVE"
