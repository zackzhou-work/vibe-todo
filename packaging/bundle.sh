#!/usr/bin/env bash
# Build the release binary and wrap it in Vibe Todo.app.
#
# The bare binary runs, but only as a nameless process: no Dock icon, no menu
# bar title, and macOS refuses it a few window privileges a bundled app gets.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/target/release/bundle"
APP="$OUT/Vibe Todo.app"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"

cargo build --release --manifest-path "$ROOT/Cargo.toml"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$ROOT/target/release/vibe-todo" "$APP/Contents/MacOS/vibe-todo"
sed "s/__VERSION__/$VERSION/g" "$ROOT/packaging/Info.plist" > "$APP/Contents/Info.plist"
printf 'APPL????' > "$APP/Contents/PkgInfo"

# Regenerate the .icns only when the icon source is newer than it.
ICNS="$ROOT/packaging/AppIcon.icns"
if [ ! -f "$ICNS" ] || [ "$ROOT/packaging/make-icon.py" -nt "$ICNS" ]; then
	"$ROOT/packaging/make-icon.sh"
fi
cp "$ICNS" "$APP/Contents/Resources/AppIcon.icns"

# arm64 refuses to launch anything unsigned; ad-hoc is enough for local use.
codesign --force --sign - --timestamp=none "$APP"

# Finder caches icons per path, and the copy above leaves the old one behind.
touch "$APP"

echo "$APP"
