#!/usr/bin/env python3
"""Propose (never apply) the LDtk player-tileset retarget as a unified diff.

Five `.ldtk` worlds declare `sprite_player_robot_v3` with
`relPath=../sprites/player_robot_v3_spritesheet.png` — 3072x2468, 7.6 MP —
purely so the LDtk editor can draw a `PlayerStart` entity preview. No level
layer in any of the five uses the tileset
(`scripts/measure_ldtk_tileset_usage.py` establishes that), so the runtime
decodes 7.6 MP at every boot and never draws it.

⛔⛔ THE RETARGET IS NOT A relPath SWAP. `tileRect` / `uiTileRect` /
`tileGridSize` / `pxWid` / `pxHei` are all in TILESET PIXEL coordinates. Point
the same definition at a quarter-size PNG and the 256x256 crop that framed one
animation frame now spans a third of the image, so the editor preview breaks
while the file still looks plausible. This script recomputes every one of them
from the REAL PNG header of the file it points at.

⭐ IT PRESERVES THE CROP AS A FRACTION, not as pixels: a rect is re-expressed as
the same proportion of the new image. That keeps each world's preview framing
what it frames today — including sanic's, whose definition is already
inconsistent (it declares 1681x1728 for the same 3072x2468 file). ⚠ Preserving
a fraction preserves sanic's EXISTING framing, which may itself be wrong; this
script does not decide that.

ⓘ The four content worlds also declare `pxHei: 2484` against a 2468-pixel file.
The proposal fixes that as a side effect of reading the real header.

⛔ THE WORLDS LIVE IN `game/ambition_map_assets`, WHICH IS JON'S SUBMODULE.
This script writes a diff and nothing else. `--apply` exists but is deliberately
not what the committed patch was produced with.

Usage:
    scripts/propose_ldtk_tileset_retarget.py                    # diff to stdout
    scripts/propose_ldtk_tileset_retarget.py --out dev/patches/x.patch
    scripts/propose_ldtk_tileset_retarget.py --tier sprites_potato
"""

from __future__ import annotations

import argparse
import difflib
import json
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
MAP_ASSETS = REPO / "game/ambition_map_assets"
ASSETS = REPO / "crates/ambition_platformer2d_actor_monolith/assets"
SHEET = "player_robot_v3_spritesheet.png"
CURRENT_REL = f"../sprites/{SHEET}"


def png_size(path: Path) -> tuple[int, int] | None:
    try:
        header = path.open("rb").read(24)
    except OSError:
        return None
    if len(header) < 24 or header[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    return struct.unpack(">II", header[16:24])


def rescale(value: int, declared: int, actual: int, *, floor_at_one: bool) -> int:
    """Re-express `value` as the same fraction of a differently-sized image.

    ⚠ Never a single constant: the x and y factors differ (0_25x of a
    3072x2468 sheet is 832x653), so width-ish and height-ish fields must be
    scaled against their own axis.

    ⛔ `floor_at_one` IS ONLY FOR EXTENTS. A zero-width rect makes the editor
    preview vanish, so a width that rounds to 0 is clamped to 1 — but an ORIGIN
    of 0 is the top-left corner and must stay 0. My first version clamped both
    and turned every `"x": 0, "y": 0` into `"x": 1, "y": 1`, silently nudging
    the crop of all five worlds.
    """
    if not declared:
        return value
    scaled = round(value * actual / declared)
    return max(1, scaled) if floor_at_one else scaled


def retarget_world(world: Path, tier: str) -> tuple[list[str], list[str]] | None:
    """Return (before, after) line lists, or None when nothing changes."""
    original = world.read_text().splitlines(keepends=True)
    data = json.loads("".join(original))
    tilesets = [
        t for t in data.get("defs", {}).get("tilesets", []) if t.get("relPath") == CURRENT_REL
    ]
    if not tilesets:
        return None
    new_size = png_size(ASSETS / tier / SHEET)
    if new_size is None:
        return None
    new_w, new_h = new_size

    lines = list(original)
    for tileset in tilesets:
        uid = tileset["uid"]
        dec_w, dec_h = tileset.get("pxWid") or 0, tileset.get("pxHei") or 0
        grid = tileset.get("tileGridSize") or 0
        new_grid = rescale(grid, dec_w, new_w, floor_at_one=True)

        for index, line in enumerate(lines):
            if f'"relPath": "{CURRENT_REL}"' in line:
                lines[index] = line.replace(CURRENT_REL, f"../{tier}/{SHEET}")
            elif re.match(r'^\s*"pxWid": %d,\s*$' % dec_w, line) and dec_w:
                lines[index] = re.sub(r"(\"pxWid\": )\d+", rf"\g<1>{new_w}", line)
            elif re.match(r'^\s*"pxHei": %d,\s*$' % dec_h, line) and dec_h:
                lines[index] = re.sub(r"(\"pxHei\": )\d+", rf"\g<1>{new_h}", line)
            elif re.match(r'^\s*"tileGridSize": %d,\s*$' % grid, line) and grid:
                lines[index] = re.sub(
                    r"(\"tileGridSize\": )\d+", rf"\g<1>{new_grid}", line
                )
            elif f'"tilesetUid": {uid},' in line and '"tileRect"' in line or (
                f'"tilesetUid": {uid},' in line and '"uiTileRect"' in line
            ):
                def fix(match: re.Match) -> str:
                    key, value = match.group(1), int(match.group(2))
                    axis = (dec_w, new_w) if key in ("x", "w") else (dec_h, new_h)
                    return (
                        f'"{key}": '
                        f"{rescale(value, axis[0], axis[1], floor_at_one=key in ('w', 'h'))}"
                    )

                lines[index] = re.sub(
                    r'"([xywh])": (\d+)', fix, line
                )
    return (original, lines) if lines != original else None


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--tier", default="sprites_0_25x")
    ap.add_argument("--out", type=str, default=None)
    ap.add_argument(
        "--apply",
        action="store_true",
        help="write the files in place (the committed patch was NOT made this way)",
    )
    args = ap.parse_args(argv)

    if not MAP_ASSETS.is_dir():
        print(
            f"NO {MAP_ASSETS.relative_to(REPO)} — run "
            "`git submodule update --init --recursive`.\n⛔ Absent is not zero."
        )
        return 2
    if png_size(ASSETS / args.tier / SHEET) is None:
        print(
            f"NO {args.tier}/{SHEET} ON DISK. The sprite tiers are gitignored "
            "generated output;\nthis proposal needs the real header of the file "
            "it retargets to.\n⛔ Absent is not zero."
        )
        return 2

    chunks = []
    touched = 0
    for world in sorted(MAP_ASSETS.rglob("*.ldtk")):
        result = retarget_world(world, args.tier)
        if result is None:
            continue
        before, after = result
        touched += 1
        rel = world.relative_to(REPO).as_posix()
        chunks.extend(
            difflib.unified_diff(before, after, fromfile=f"a/{rel}", tofile=f"b/{rel}")
        )
        if args.apply:
            world.write_text("".join(after))

    if not touched:
        print(
            "NO WORLD NAMES THE FULL-RESOLUTION SHEET. Either the retarget has "
            "already landed\nor the declarations moved. Refusing to emit an "
            "empty patch as if it were a no-op result."
        )
        return 2

    text = "".join(chunks)
    if args.out:
        Path(args.out).write_text(text)
        print(f"wrote {args.out} — {touched} world(s), {text.count(chr(10))} diff lines")
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
