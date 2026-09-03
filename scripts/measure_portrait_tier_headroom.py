#!/usr/bin/env python3
"""Would a reduced-tier portrait ever be big enough to draw?

The generator emits `*_portraits.png` at all four sprite tiers; `build.rs`
bakes portrait manifests from `assets/sprites` only, so 487 files / 14.2 MB
have no road (`scripts/measure_orphan_shipped_pages.py`). Before anyone decides
whether to stop generating them or to start baking them, the question worth
answering is whether a cheaper portrait is USEFUL at all.

⭐ THE ANSWER TURNS ON A FACT ABOUT THE UI, NOT ABOUT THE ART. Portrait draw
size is chosen by VIEWPORT, not by quality tier:
`DialogLayoutProfile::for_viewport` picks 56x62 (phone landscape), 82x94 (phone
portrait / small tablet) or 104x120 (everything else). No quality setting is
consulted. So a tier can never be the thing that decides which portrait
resolution is appropriate — the window size is.

This script compares each tier's portrait FRAME against those draw sizes, and
reports the display scale factor at which the tier starts upscaling. A tier
whose frame is smaller than the box it is drawn into is not a cheaper portrait;
it is a blurrier one.

⚠ LOGICAL PX, NOT PHYSICAL. A 104x120 box is 208x240 on a 2x display, so the
`needs` column is the frame a tier must have to stay sharp at that scale.

Usage:
    scripts/measure_portrait_tier_headroom.py
    scripts/measure_portrait_tier_headroom.py --scales 1 2 3
"""

from __future__ import annotations

import argparse
import re
import struct
import subprocess
import sys
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
ASSETS = REPO / "crates/ambition_platformer2d_actor_monolith/assets"
DIALOG = REPO / "game/ambition_content/src/presentation/dialog.rs"
TIERS = ["sprites", "sprites_0_5x", "sprites_0_25x", "sprites_potato"]

SIZE_RE = re.compile(
    r"portrait_width:\s*([\d.]+),\s*\n\s*portrait_height:\s*([\d.]+),", re.MULTILINE
)
FRAME_RE = re.compile(r"frame_width:\s*(\d+),\s*\n\s*frame_height:\s*(\d+),")


def png_size(path: Path) -> tuple[int, int] | None:
    try:
        header = path.open("rb").read(24)
    except OSError:
        return None
    if len(header) < 24 or header[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    return struct.unpack(">II", header[16:24])


def draw_sizes() -> list[tuple[float, float]]:
    """The portrait boxes the dialog UI actually draws into.

    ⛔ AN EMPTY RESULT IS A PARSER FAILURE, NOT "NOTHING DRAWS PORTRAITS".
    `main` refuses it rather than reporting infinite headroom.
    """
    if not DIALOG.exists():
        return []
    return [
        (float(w), float(h)) for w, h in SIZE_RE.findall(DIALOG.read_text())
    ]


def portrait_frames(name: str) -> dict[str, tuple[int, int]]:
    """Each tier's frame size for one portrait sheet.

    Only the full-resolution manifest exists (that is the finding), so the
    reduced tiers' frames are derived from the PNG dimension ratio — the
    generator downsamples the whole sheet, so the frame scales with it.
    """
    manifest = ASSETS / "sprites" / f"{name}.ron"
    full_png = png_size(ASSETS / "sprites" / f"{name}.png")
    if not manifest.exists() or full_png is None:
        return {}
    found = FRAME_RE.search(manifest.read_text())
    if not found:
        return {}
    frame_w, frame_h = int(found.group(1)), int(found.group(2))
    out = {"sprites": (frame_w, frame_h)}
    for tier in TIERS[1:]:
        size = png_size(ASSETS / tier / f"{name}.png")
        if size:
            out[tier] = (
                max(1, round(frame_w * size[0] / full_png[0])),
                max(1, round(frame_h * size[1] / full_png[1])),
            )
    return out


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--name", default="alice_portraits")
    ap.add_argument("--scales", type=float, nargs="+", default=[1.0, 2.0])
    args = ap.parse_args(argv)

    boxes = draw_sizes()
    if not boxes:
        print(
            "NO PORTRAIT DRAW SIZES PARSED from DialogLayoutProfile. Either the\n"
            "layout moved or the pattern is stale. Refusing to report headroom\n"
            "against an empty set of draw sizes — that would say every tier is\n"
            "big enough, which is the most reassuring possible wrong answer."
        )
        return 2

    frames = portrait_frames(args.name)
    if not frames:
        print(
            f"NO PORTRAIT FRAME FOR {args.name}. The sprite tree is gitignored\n"
            "generated output. ⛔ Absent is not zero."
        )
        return 2

    print(
        "portrait draw boxes, chosen by VIEWPORT and never by quality tier\n"
        "(DialogLayoutProfile::for_viewport): "
        + ", ".join(f"{int(w)}x{int(h)}" for w, h in boxes)
        + f"\n\nframe sizes for {args.name}:\n"
    )
    largest = max(boxes, key=lambda b: b[0] * b[1])
    smallest = min(boxes, key=lambda b: b[0] * b[1])
    header = f"{'tier':<16} {'frame':>12}"
    for scale in args.scales:
        header += f"  {'needs ' + str(scale) + 'x':>13}  {'verdict @' + str(scale) + 'x':>16}"
    print(header)
    for tier in TIERS:
        frame = frames.get(tier)
        if frame is None:
            print(f"{tier:<16} {'absent':>12}")
            continue
        row = f"{tier:<16} {f'{frame[0]}x{frame[1]}':>12}"
        for scale in args.scales:
            need_w, need_h = largest[0] * scale, largest[1] * scale
            small_w, small_h = smallest[0] * scale, smallest[1] * scale
            if frame[0] >= need_w and frame[1] >= need_h:
                verdict = "covers every box"
            elif frame[0] >= small_w and frame[1] >= small_h:
                verdict = "smallest box only"
            else:
                verdict = "UPSCALES ALWAYS"
            row += f"  {f'{int(need_w)}x{int(need_h)}':>13}  {verdict:>16}"
        print(row)
    print(
        "\n⇒ A tier whose frame is under the box it is drawn into is not a cheaper\n"
        "  portrait, it is a blurrier one. And because the box is chosen by\n"
        "  VIEWPORT, no quality setting can select a tier that fits."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
