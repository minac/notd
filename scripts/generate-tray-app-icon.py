#!/usr/bin/env python3
"""Generate the macOS menu bar (tray) icons for notd from the app icon.

Produces two 44x44 PNGs derived from src-tauri/icons/icon.icns:

  - tray-app-light.png: black rounded square with the calligraphic
    glyph cut out as a transparent hole. Used on light menu bars.
  - tray-app-dark.png:  white rounded square, same transparent glyph
    hole. Used on dark menu bars.

These fill their 22pt slot more aggressively than the previous tray
icons (minimal padding around the rounded square). Tauri swaps between
them at runtime when the system appearance changes.

Requires `iconutil` (macOS) and `magick` (Homebrew package `imagemagick`).
"""

import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "src-tauri" / "icons"
ICNS = ICONS / "icon.icns"

SIZE = 44
INTERMEDIATE = SIZE * 4  # downscale from 4x for crisp anti-aliasing


def extract_largest(work: Path) -> Path:
    """Unpack icon.icns and return the largest PNG inside it."""
    iconset = work / "notd.iconset"
    subprocess.run(
        ["iconutil", "-c", "iconset", str(ICNS), "-o", str(iconset)],
        check=True,
    )
    pngs = sorted(iconset.glob("*.png"), key=lambda p: p.stat().st_size)
    return pngs[-1]


def render(source: Path, fill: str, out: Path) -> None:
    """Cut the white glyph out of `source`, recolor opaque pixels `fill`,
    and write a 44x44 RGBA PNG to `out`.

    Works in two stages: render at 4x first so the alpha mask edges
    stay smooth, then resize down with Lanczos.
    """
    subprocess.run(
        [
            "magick",
            str(source),
            # turn the white glyph into transparent holes; rounded-corner
            # transparency from the source is preserved.
            "-fuzz",
            "25%",
            "-transparent",
            "white",
            # recolor every remaining opaque pixel to the target fill.
            "-fill",
            fill,
            "-colorize",
            "100",
            "-filter",
            "Lanczos",
            "-resize",
            f"{INTERMEDIATE}x{INTERMEDIATE}",
            "-resize",
            f"{SIZE}x{SIZE}",
            "-define",
            "png:color-type=6",
            str(out),
        ],
        check=True,
    )


def main() -> None:
    if not shutil.which("magick"):
        raise SystemExit("magick (ImageMagick) not found on PATH")
    ICONS.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        source = extract_largest(Path(tmp))
        render(source, "black", ICONS / "tray-app-light.png")
        render(source, "white", ICONS / "tray-app-dark.png")
    print(f"wrote {ICONS / 'tray-app-light.png'}")
    print(f"wrote {ICONS / 'tray-app-dark.png'}")


if __name__ == "__main__":
    main()
