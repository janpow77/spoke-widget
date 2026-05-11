#!/usr/bin/env python3
"""Regenerate the bundled tray icons.

This is a build-time helper: it produces small, themable PNG icons for
each spoke-status (green/yellow/red/grey/unknown) plus the application
icons in the sizes Tauri expects.

Usage:
    python3 scripts/gen_icons.py
"""
from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw


ROOT = Path(__file__).resolve().parent.parent
ICONS_DIR = ROOT / "src-tauri" / "icons"
ICONS_DIR.mkdir(parents=True, exist_ok=True)


def draw_state_icon(color: tuple[int, int, int, int], size: int = 64) -> Image.Image:
    """A filled circle with a thin dark border. Works on both light & dark trays."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    pad = size // 8
    d.ellipse(
        (pad, pad, size - pad, size - pad),
        fill=color,
        outline=(30, 30, 30, 230),
        width=max(1, size // 32),
    )
    return img


def draw_unknown(size: int = 64) -> Image.Image:
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    pad = size // 8
    d.ellipse(
        (pad, pad, size - pad, size - pad),
        fill=(160, 160, 160, 255),
        outline=(30, 30, 30, 230),
        width=max(1, size // 32),
    )
    # crude "?" using two arcs + a dot would be overkill; instead draw a small
    # rectangle to differentiate from "grey".
    cx, cy = size // 2, size // 2
    r = size // 6
    d.rectangle((cx - r // 2, cy - r, cx + r // 2, cy + r), fill=(30, 30, 30, 230))
    return img


def main() -> None:
    states = {
        "tray-green.png": (60, 180, 90, 255),
        "tray-yellow.png": (235, 190, 40, 255),
        "tray-red.png": (215, 60, 60, 255),
        "tray-grey.png": (140, 140, 140, 255),
    }
    for name, color in states.items():
        draw_state_icon(color).save(ICONS_DIR / name)
        print(f"wrote {name}")

    draw_unknown().save(ICONS_DIR / "tray-unknown.png")
    print("wrote tray-unknown.png")

    # Bundle icons (Tauri auto-picks the right format per platform).
    app_icon = draw_state_icon((60, 180, 90, 255), size=256)
    for size in (32, 128, 256):
        app_icon.resize((size, size), Image.LANCZOS).save(ICONS_DIR / f"{size}x{size}.png")
    app_icon.resize((256, 256), Image.LANCZOS).save(ICONS_DIR / "128x128@2x.png")
    # icon.ico (multi-size)
    base = app_icon
    base.save(
        ICONS_DIR / "icon.ico",
        sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    # icon.icns: PIL writes a valid icns for macOS bundles.
    base.save(ICONS_DIR / "icon.icns", format="ICNS")
    # PNG fallback used by Linux bundles.
    base.save(ICONS_DIR / "icon.png")
    print("wrote app icons (32, 128, 128@2x, 256, .ico, .icns, icon.png)")


if __name__ == "__main__":
    main()
