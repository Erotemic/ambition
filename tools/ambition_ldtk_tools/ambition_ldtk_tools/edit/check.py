#!/usr/bin/env python3
"""Check whether a hypothetical entity placement is safe.

Read-only — reports overlaps and nearest-neighbor distance so you
can verify a `px` / `size` *before* running `entity add` or
`entity move`. Pairs with `door snap` (which finds the right y for
a door) and `door free-spots` (which finds gaps in an existing
door row).

Usage:

    # Is the slot at (908, 624) sized 48x96 in central_hub_main free?
    python -m ambition_ldtk_tools entity check \\
      --ldtk <ldtk> \\
      --level central_hub_main \\
      --px 908,624 --size 48,96

    # Only check against LoadingZones (e.g. avoid overlapping doors
    # but ignore decorative DebugLabels overlapping is fine):
    python -m ambition_ldtk_tools entity check \\
      --ldtk <ldtk> --level central_hub_main \\
      --px 908,624 --size 48,96 \\
      --against LoadingZone

    # Warn if anything's closer than 64 px center-to-center:
    python -m ambition_ldtk_tools entity check \\
      --ldtk <ldtk> --level central_hub_main \\
      --px 908,624 --size 48,96 --min-spacing 64

Exit code is 0 when the placement is safe (no overlaps, and the
nearest neighbor is at least --min-spacing away if specified), 1
otherwise. Makes the tool drop straight into a shell-script
pipeline before an authoring run.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/check.py -> repo root.
REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.area_authoring import load_project  # noqa: E402
from ambition_ldtk_tools.edit.set_field import (  # noqa: E402
    find_ambition_layer,
    find_level,
)


def _rects_overlap(a: tuple[int, int, int, int], b: tuple[int, int, int, int]) -> bool:
    ax, ay, aw, ah = a
    bx, by, bw, bh = b
    return ax < bx + bw and ax + aw > bx and ay < by + bh and ay + ah > by


def _center_distance(a: tuple[int, int, int, int], b: tuple[int, int, int, int]) -> float:
    ax = a[0] + a[2] / 2.0
    ay = a[1] + a[3] / 2.0
    bx = b[0] + b[2] / 2.0
    by = b[1] + b[3] / 2.0
    return ((ax - bx) ** 2 + (ay - by) ** 2) ** 0.5


def _parse_pair(raw: str, label: str) -> tuple[int, int]:
    parts = raw.split(",")
    if len(parts) != 2:
        raise SystemExit(f"--{label} expects X,Y (or W,H); got {raw!r}")
    try:
        return int(parts[0]), int(parts[1])
    except ValueError as ex:
        raise SystemExit(f"--{label} expects integers; got {raw!r}: {ex}")


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=REPO_ROOT
        / "crates"
        / "ambition_sandbox"
        / "assets"
        / "ambition"
        / "worlds"
        / "sandbox.ldtk",
    )
    parser.add_argument("--level", required=True, help="level identifier")
    parser.add_argument("--px", required=True, help="top-left X,Y of the rect to check")
    parser.add_argument("--size", required=True, help="W,H of the rect to check")
    parser.add_argument(
        "--against",
        action="append",
        default=[],
        help=(
            "restrict the overlap check to specific entity identifiers "
            "(repeatable). Default: check against ALL Ambition-layer entities."
        ),
    )
    parser.add_argument(
        "--ignore",
        action="append",
        default=[],
        help=(
            "skip overlap reporting against these entity identifiers "
            "(repeatable). Useful to ignore CameraZones (which span the "
            "whole room by design)."
        ),
    )
    parser.add_argument(
        "--ignore-iid",
        action="append",
        default=[],
        help=(
            "skip overlap reporting against a specific iid (repeatable). "
            "Use when re-checking a slot you intend to overwrite."
        ),
    )
    parser.add_argument(
        "--min-spacing",
        type=int,
        default=0,
        help=(
            "warn (and exit 1) if any neighbor's center is closer than "
            "this many pixels. Default 0 disables the spacing check."
        ),
    )
    args = parser.parse_args(argv)

    cx, cy = _parse_pair(args.px, "px")
    cw, ch = _parse_pair(args.size, "size")
    check_rect = (cx, cy, cw, ch)

    project = load_project(args.ldtk)
    level = find_level(project, args.level)
    layer = find_ambition_layer(level)

    overlaps: list[dict] = []
    nearest: tuple[float, dict] | None = None
    for ent in layer.get("entityInstances", []):
        ident = ent.get("__identifier", "<unknown>")
        iid = ent.get("iid", "<no-iid>")
        if args.against and ident not in args.against:
            continue
        if ident in args.ignore:
            continue
        if iid in args.ignore_iid:
            continue
        rect = (
            int(ent["px"][0]),
            int(ent["px"][1]),
            int(ent.get("width", 0)),
            int(ent.get("height", 0)),
        )
        if _rects_overlap(check_rect, rect):
            overlaps.append(
                {
                    "identifier": ident,
                    "iid": iid,
                    "px": [rect[0], rect[1]],
                    "size": [rect[2], rect[3]],
                }
            )
            continue
        dist = _center_distance(check_rect, rect)
        if nearest is None or dist < nearest[0]:
            nearest = (
                dist,
                {
                    "identifier": ident,
                    "iid": iid,
                    "px": [rect[0], rect[1]],
                    "size": [rect[2], rect[3]],
                },
            )

    print(
        f"check: level={args.level} rect=px={list(check_rect[:2])} "
        f"size={list(check_rect[2:])}"
    )
    if overlaps:
        print(f"  OVERLAP: {len(overlaps)} entit{'y' if len(overlaps) == 1 else 'ies'}:")
        for o in overlaps:
            print(
                f"    - {o['identifier']} ({o['iid']}) at px={o['px']} size={o['size']}"
            )
    else:
        print("  no overlap")
    if nearest is not None:
        dist, ent = nearest
        warn = ""
        if args.min_spacing and dist < args.min_spacing:
            warn = f" (UNDER min-spacing={args.min_spacing})"
        print(
            f"  nearest neighbor: {ent['identifier']} ({ent['iid']}) "
            f"at px={ent['px']} — center distance {dist:.1f}px{warn}"
        )

    rc = 0
    if overlaps:
        rc = 1
    if (
        args.min_spacing
        and nearest is not None
        and nearest[0] < args.min_spacing
    ):
        rc = 1
    return rc


if __name__ == "__main__":
    raise SystemExit(main())
