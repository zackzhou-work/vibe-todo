#!/usr/bin/env bash
# Rasterize the icon and pack it into packaging/AppIcon.icns.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

ICONSET="$WORK/AppIcon.iconset"
mkdir -p "$ICONSET"
python3 "$HERE/make-icon.py" "$WORK/icon-1024.png"

for size in 16 32 128 256 512; do
	sips -z $size $size "$WORK/icon-1024.png" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
	sips -z $((size * 2)) $((size * 2)) "$WORK/icon-1024.png" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done

iconutil --convert icns "$ICONSET" --output "$HERE/AppIcon.icns"
echo "$HERE/AppIcon.icns"
