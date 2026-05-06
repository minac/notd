#!/usr/bin/env python3
"""Generate the macOS menu bar (tray) icon for notd.

Produces a 44x44 PNG: a rounded square with a calligraphic "N" punched
out as transparency. Tauri renders it as a template image, so macOS
recolors it for light/dark menu bars — yielding a white-N-on-black
square in dark mode and a black-N-on-white square in light mode.

Requires `rsvg-convert` (Homebrew package `librsvg`) and the macOS
system font Snell Roundhand.
"""

import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "src-tauri" / "icons" / "tray.png"
SIZE = 44
INSET = 2  # leave 2px of breathing room on each side
SQUARE = SIZE - 2 * INSET
RADIUS = 7

SVG = f"""<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg"
     width="{SIZE}" height="{SIZE}" viewBox="0 0 {SIZE} {SIZE}">
  <defs>
    <mask id="cutout" maskUnits="userSpaceOnUse" x="0" y="0" width="{SIZE}" height="{SIZE}">
      <rect x="{INSET}" y="{INSET}" width="{SQUARE}" height="{SQUARE}"
            rx="{RADIUS}" ry="{RADIUS}" fill="white"/>
      <text x="{SIZE / 2}" y="{SIZE * 0.78}"
            font-family="Snell Roundhand, Apple Chancery, cursive"
            font-weight="700" font-size="{SIZE * 0.78:.2f}"
            text-anchor="middle" fill="black">N</text>
    </mask>
  </defs>
  <rect x="{INSET}" y="{INSET}" width="{SQUARE}" height="{SQUARE}"
        rx="{RADIUS}" ry="{RADIUS}" fill="black" mask="url(#cutout)"/>
</svg>
"""


def main() -> None:
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(suffix=".svg", mode="w", delete=False) as f:
        f.write(SVG)
        svg_path = Path(f.name)
    try:
        subprocess.run(
            [
                "rsvg-convert",
                "--width",
                str(SIZE),
                "--height",
                str(SIZE),
                "--format",
                "png",
                "--output",
                str(OUT),
                str(svg_path),
            ],
            check=True,
        )
    finally:
        svg_path.unlink(missing_ok=True)
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
