#!/usr/bin/env python3
"""Draw AppIcon.png: a sheet of the same warm paper the window is made of,
with the ink tick that marks a task done and the red dot that marks it urgent.

Pure stdlib on purpose — the repo has no image toolchain, and this runs once
when the icon changes, not on every build.
"""

import math
import struct
import sys
import zlib

SIZE = 1024

GROUND = (0xFB, 0xFA, 0xF7)
HAIRLINE = (0xEB, 0xE7, 0xE0)
INK = (0x1F, 0x1D, 0x1A)
PRIORITY = (0xD9, 0x2D, 0x20)

# macOS leaves a margin around the squircle so icons line up in the Dock.
INSET = 100.0
RADIUS = 185.0

TICK = [(352.0, 528.0), (466.0, 642.0), (686.0, 396.0)]
TICK_WIDTH = 62.0

DOT_CENTER = (686.0, 686.0)
DOT_RADIUS = 46.0


def rounded_rect_sdf(x, y):
    half = (SIZE - 2 * INSET) / 2
    dx = abs(x - SIZE / 2) - (half - RADIUS)
    dy = abs(y - SIZE / 2) - (half - RADIUS)
    outside = math.hypot(max(dx, 0.0), max(dy, 0.0))
    return outside + min(max(dx, dy), 0.0) - RADIUS


def segment_sdf(x, y, a, b):
    ax, ay = a
    bx, by = b
    px, py = x - ax, y - ay
    vx, vy = bx - ax, by - ay
    t = max(0.0, min(1.0, (px * vx + py * vy) / (vx * vx + vy * vy)))
    return math.hypot(px - vx * t, py - vy * t)


def tick_sdf(x, y):
    return min(segment_sdf(x, y, *pair) for pair in zip(TICK, TICK[1:])) - TICK_WIDTH / 2


def coverage(distance):
    """Antialias from the signed distance: one pixel of feather across the edge."""
    return max(0.0, min(1.0, 0.5 - distance))


def over(dst, src, alpha):
    return tuple(round(s * alpha + d * (1 - alpha)) for s, d in zip(src, dst))


def render():
    rows = []
    for py in range(SIZE):
        y = py + 0.5
        row = bytearray()
        for px in range(SIZE):
            x = px + 0.5

            paper = coverage(rounded_rect_sdf(x, y))
            if paper == 0.0:
                row += b"\x00\x00\x00\x00"
                continue

            rgb = GROUND
            # A near-white icon needs an edge of its own on a white background.
            rgb = over(rgb, HAIRLINE, coverage(abs(rounded_rect_sdf(x, y) + 1.5) - 1.5))
            rgb = over(rgb, INK, coverage(tick_sdf(x, y)))
            rgb = over(rgb, PRIORITY, coverage(math.hypot(x - DOT_CENTER[0], y - DOT_CENTER[1]) - DOT_RADIUS))

            row += bytes(rgb) + bytes([round(paper * 255)])
        rows.append(bytes(row))
    return rows


def write_png(path, rows):
    raw = b"".join(b"\x00" + row for row in rows)

    def chunk(kind, payload):
        body = kind + payload
        return struct.pack(">I", len(payload)) + body + struct.pack(">I", zlib.crc32(body))

    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(raw, 9)))
        f.write(chunk(b"IEND", b""))


if __name__ == "__main__":
    write_png(sys.argv[1] if len(sys.argv) > 1 else "AppIcon.png", render())
