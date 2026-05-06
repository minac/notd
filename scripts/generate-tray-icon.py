#!/usr/bin/env python3
"""Generate the macOS menu bar (tray) icons for notd.

Produces two 44x44 PNGs:

  - tray-light.png: black rounded square, white inner circle, six
    colored Trivial-Pursuit-style pie slices. Used on light menu bars.
  - tray-dark.png:  white rounded square, black inner circle, same
    pie. Used on dark menu bars.

The icons are full-color (not template images) so the pie colors stay
visible. Tauri swaps between them at runtime when the system
appearance changes.

Requires `rsvg-convert` (Homebrew package `librsvg`).
"""

import math
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "src-tauri" / "icons"

SIZE = 44
INSET = 2
SQUARE = SIZE - 2 * INSET
CORNER = 7
CENTER = SIZE / 2
PIE_R = 13.5
RING_STROKE = 1.6

# Canonical Trivial Pursuit category colors (top, then clockwise).
COLORS = [
    "#3a87e8",  # geography (blue)
    "#e8568b",  # entertainment (pink)
    "#f0c92c",  # history (yellow)
    "#7d4f3a",  # arts & literature (brown)
    "#3eb049",  # science & nature (green)
    "#e9762e",  # sports & leisure (orange)
]


def slice_path(start_deg: float, end_deg: float, r: float) -> str:
    a1 = math.radians(start_deg)
    a2 = math.radians(end_deg)
    x1, y1 = r * math.cos(a1), r * math.sin(a1)
    x2, y2 = r * math.cos(a2), r * math.sin(a2)
    large = 1 if (end_deg - start_deg) > 180 else 0
    return (
        f"M 0,0 L {x1:.3f},{y1:.3f} A {r:.3f},{r:.3f} 0 {large},1 {x2:.3f},{y2:.3f} Z"
    )


def make_svg(box_fill: str, ring_color: str) -> str:
    slices = []
    for i in range(6):
        start = i * 60 - 90  # first slice points up
        end = start + 60
        slices.append(
            f'<path d="{slice_path(start, end, PIE_R)}" fill="{COLORS[i]}" '
            f'stroke="{box_fill}" stroke-width="0.6" stroke-linejoin="miter"/>'
        )
    slice_block = "\n    ".join(slices)

    return f"""<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg"
     width="{SIZE}" height="{SIZE}" viewBox="0 0 {SIZE} {SIZE}">
  <rect x="{INSET}" y="{INSET}" width="{SQUARE}" height="{SQUARE}"
        rx="{CORNER}" ry="{CORNER}" fill="{box_fill}"/>
  <g transform="translate({CENTER} {CENTER})">
    {slice_block}
  </g>
  <circle cx="{CENTER}" cy="{CENTER}" r="{PIE_R}" fill="none"
          stroke="{ring_color}" stroke-width="{RING_STROKE}"/>
</svg>
"""


def render(svg: str, out: Path) -> None:
    with tempfile.NamedTemporaryFile(suffix=".svg", mode="w", delete=False) as f:
        f.write(svg)
        path = Path(f.name)
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
                str(out),
                str(path),
            ],
            check=True,
        )
    finally:
        path.unlink(missing_ok=True)


def main() -> None:
    ICONS.mkdir(parents=True, exist_ok=True)
    render(make_svg("black", "white"), ICONS / "tray-light.png")
    render(make_svg("white", "black"), ICONS / "tray-dark.png")
    # Remove the previous template-style icon if it's still around.
    legacy = ICONS / "tray.png"
    if legacy.exists():
        legacy.unlink()
    print(f"wrote {ICONS / 'tray-light.png'}")
    print(f"wrote {ICONS / 'tray-dark.png'}")


if __name__ == "__main__":
    main()
