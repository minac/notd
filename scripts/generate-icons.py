#!/usr/bin/env python3
"""Generate placeholder Tauri icons for notd.

Builds a 1024x1024 source PNG (a centered colored circle on a soft background),
then writes the sizes Tauri expects. macOS .icns is left to iconutil afterward.
"""

import os
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ICONS_DIR = ROOT / "src-tauri" / "icons"
ICONS_DIR.mkdir(parents=True, exist_ok=True)


def make_png(width: int, height: int, pixel_fn) -> bytes:
    raw = bytearray()
    for y in range(height):
        raw.append(0)  # filter byte: None
        for x in range(width):
            r, g, b, a = pixel_fn(x, y)
            raw.extend((r, g, b, a))

    def chunk(typ: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + typ
            + data
            + struct.pack(">I", zlib.crc32(typ + data) & 0xFFFFFFFF)
        )

    signature = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    idat = zlib.compress(bytes(raw), 9)
    return signature + chunk(b"IHDR", ihdr) + chunk(b"IDAT", idat) + chunk(b"IEND", b"")


def color_for_size(size: int):
    # Background: near-white. Foreground: a soft red dot (palette index 0).
    bg = (250, 250, 250, 255)
    fg = (217, 83, 79, 255)
    cx = (size - 1) / 2.0
    cy = (size - 1) / 2.0
    radius = size * 0.32
    radius_sq = radius * radius
    edge = max(1.0, size * 0.01)
    edge_sq = (radius + edge) * (radius + edge)

    def pixel(x: int, y: int):
        dx = x - cx
        dy = y - cy
        d_sq = dx * dx + dy * dy
        if d_sq <= radius_sq:
            return fg
        if d_sq <= edge_sq:
            # simple antialias band
            t = (d_sq - radius_sq) / (edge_sq - radius_sq)
            a = int(255 * (1 - t))
            return (fg[0], fg[1], fg[2], a) if a > 0 else bg
        return bg

    return pixel


def write(size: int, name: str) -> Path:
    path = ICONS_DIR / name
    data = make_png(size, size, color_for_size(size))
    path.write_bytes(data)
    return path


if __name__ == "__main__":
    write(32, "32x32.png")
    write(128, "128x128.png")
    write(256, "128x128@2x.png")
    write(512, "icon-512.png")
    write(1024, "icon.png")
    print("wrote icons to", ICONS_DIR)
