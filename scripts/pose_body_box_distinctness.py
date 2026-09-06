#!/usr/bin/env python3
"""Count DISTINCT per-pose body boxes in a sprite sheet's authored `.ron`.

⭐ WHY THIS EXISTS AS A SECOND INSTRUMENT, which is a thing to justify rather than
assume. The fighter lane has a probe that counts distinct `(collision,
sprite_offset)` pairs through the RUNTIME resolution path. This one reads the
AUTHORED `.ron` text and counts distinct `hurtbox.bbox` values directly. Same
question, deliberately disjoint roads: the runtime probe can only report what
resolution produces, so a derivation that collapses two genuinely different boxes
and a sheet that authored one box twice look identical to it. This one cannot tell
you what the game DRAWS. Neither supersedes the other; they disagree only if a
derivation is lossy, which is itself the finding.

⛔⛔ WHAT IT WAS BUILT TO SETTLE, 2026-09-06. `mary_o_v2` was reported as publishing
"no per-pose body metrics at all", with `SheetBodyMetrics::pose_body_bbox`'s
`.or(self.body_pixel_bbox)` fallback named as the mechanism. The sheet text says
otherwise: all NINE of her animations carry a populated
`hurtbox: Some((..., bbox: Some((x: 12, y: 26, w: 14, h: 21))))` -- and every one is
the SAME rectangle, equal to the sheet-level `body_pixel_bbox`. So the `filter_map`
succeeds, `.or()` is never reached, and the defect is upstream in the GENERATOR
writing a constant into every pose.

⇒ The distinction is not pedantic: on the fallback theory the fix is to delete or
fail-loud the `.or()`, and that produces NO signal for her. A guard on
`bbox.is_some()` cannot see this defect at all. Only DISTINCTNESS can.

Reference readings at the time of writing:
    mary_o_v2         9 poses,   1 distinct   <- one rectangle for every pose
    player_robot_v3 133 poses,  83 distinct   <- what a measured sheet looks like

Usage:
    python3 scripts/pose_body_box_distinctness.py <sheet.ron> [<sheet.ron> ...]
    python3 scripts/pose_body_box_distinctness.py --all
"""

from __future__ import annotations

import argparse
import collections
import pathlib
import re
import sys

# One authored animation entry: the pose name, then its hurtbox bbox if it has one.
_POSE = re.compile(
    r'"(?P<pose>[a-z_0-9]+)": \(hurtbox: Some\(\(parts: \[.*?\], '
    r"bbox: Some\(\((?P<bbox>x: -?\d+, y: -?\d+, w: \d+, h: \d+)\)\)"
)
_SHEET_BOX = re.compile(r"body_pixel_bbox: Some\(\((x: -?\d+, y: -?\d+, w: \d+, h: \d+)\)\)")
_SEARCH_ROOTS = ("crates", "game")


def read_sheet(path: pathlib.Path) -> dict:
    """Parse one sheet. Raises on an unreadable file rather than reporting zero.

    ⛔ A scanner that swallows a read error reports its own finding as the
    repository's -- an empty parse here would read as "this sheet has no per-pose
    boxes", which is the exact claim this script exists to adjudicate.
    """
    text = path.read_text(encoding="utf-8")
    poses = {m.group("pose"): m.group("bbox") for m in _POSE.finditer(text)}
    sheet_box = _SHEET_BOX.search(text)
    return {
        "path": path,
        "poses": poses,
        "distinct": len(set(poses.values())),
        "sheet_box": sheet_box.group(1) if sheet_box else None,
    }


def report(row: dict) -> str:
    poses, distinct = row["poses"], row["distinct"]
    if not poses:
        # ⚠ Genuinely absent per-pose metrics -- the case the original report
        # described. Distinct from "one box repeated", and the two want different
        # fixes, so they must never print the same line.
        return f"{row['path']}: NO per-pose hurtbox bbox at all (sheet box: {row['sheet_box']})"
    shared = distinct == 1 and len(poses) > 1
    flag = "  ⛔ ONE BOX FOR EVERY POSE" if shared else ""
    same_as_sheet = ""
    if shared and next(iter(poses.values())) == row["sheet_box"]:
        same_as_sheet = " (identical to the sheet-level body_pixel_bbox)"
    return (
        f"{row['path']}: {len(poses)} poses, {distinct} distinct{flag}{same_as_sheet}"
    )


def find_all() -> list[pathlib.Path]:
    found: list[pathlib.Path] = []
    for root in _SEARCH_ROOTS:
        base = pathlib.Path(root)
        if not base.is_dir():
            continue
        for p in base.rglob("*_spritesheet.ron"):
            if "target" in p.parts or p.is_symlink():
                continue
            found.append(p)
    return sorted(set(found))


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("sheets", nargs="*", type=pathlib.Path)
    ap.add_argument("--all", action="store_true", help="every tracked *_spritesheet.ron")
    args = ap.parse_args(argv)

    sheets = list(args.sheets)
    if args.all:
        sheets.extend(find_all())
    if not sheets:
        ap.error("name at least one sheet, or pass --all")

    rows = [read_sheet(p) for p in sheets]
    for row in sorted(rows, key=lambda r: (r["distinct"], str(r["path"]))):
        print(report(row))

    flat = [r for r in rows if r["poses"] and r["distinct"] == 1 and len(r["poses"]) > 1]
    print(
        f"\n{len(rows)} sheet(s); {len(flat)} publish ONE box for every pose."
        "\n⇒ A flat sheet resolves fine and passes any `bbox.is_some()` guard:"
        "\n  the box is plausible, it is just not THIS pose's."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
