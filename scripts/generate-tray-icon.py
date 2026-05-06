#!/usr/bin/env python3
"""Generate the macOS menu bar (tray) icon for notd.

Produces a single 44x44 PNG with a filled black circle on transparent
background. Tauri renders it as a template image, so macOS handles
recoloring for light/dark menu bars automatically.
"""

import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "src-tauri" / "icons" / "tray.png"
SIZE = 44


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


def main() -> None:
    cx = (SIZE - 1) / 2.0
    cy = (SIZE - 1) / 2.0
    radius = SIZE * 0.32
    inner_sq = radius * radius
    edge = 1.4
    edge_sq = (radius + edge) * (radius + edge)

    def pixel(x: int, y: int):
        dx = x - cx
        dy = y - cy
        d_sq = dx * dx + dy * dy
        if d_sq <= inner_sq:
            return (0, 0, 0, 255)
        if d_sq <= edge_sq:
            t = (d_sq - inner_sq) / (edge_sq - inner_sq)
            a = int(255 * (1.0 - t))
            return (0, 0, 0, a) if a > 0 else (0, 0, 0, 0)
        return (0, 0, 0, 0)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(make_png(SIZE, SIZE, pixel))
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
