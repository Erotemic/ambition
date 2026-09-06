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


# ⭐⭐ A COUNT OF 1 IS NOT THE DEFECT; A RATIO NEAR ZERO IS. The first version of
# this script flagged `distinct == 1`, which fires only on the DEGENERATE TAIL of
# the distribution and let a sheet with 7 boxes across 136 poses read as healthy.
# Measured across the roster:
#
#     boss                          6 poses    1 distinct   0.17
#     mary_o_v2                     9 poses    1 distinct   0.11
#     mary_o_v2_tall               12 poses    2 distinct   0.17
#     mary_o_v2_fire               12 poses    3 distinct   0.25
#     perfect_cellular_automaton  136 poses    7 distinct   0.05
#     noether                     123 poses  111 distinct   0.90
#     player_robot_v3             133 poses   95 distinct   0.71
#
# ⚠ THE THRESHOLD IS NOT A JUDGEMENT CALL BECAUSE THE DISTRIBUTION HAS A HOLE IN
# IT: nothing sits between 0.25 and 0.71, so every cut in that range returns the
# same set and the answer does not depend on where in the gap it lands. That is the
# only honest way to pick a magnitude -- show the verdict is insensitive to it --
# and `--ratio` exists so a reader can move it and see that for themselves.
#
# ⚠ IT IS ALSO THE TIER-INDEPENDENT READING. Distinctness moves with resolution
# (`player_robot_v3` reads 27/83/92/95 across potato/0_25x/0_5x/1x) because low
# tiers quantise boxes together, but the RATIO's ordering does not move. A bare
# count is a statement about which tier you happened to read.
FLAT_RATIO = 0.5


def report(row: dict, ratio_cut: float = FLAT_RATIO) -> str:
    poses, distinct = row["poses"], row["distinct"]
    if not poses:
        # ⚠ Genuinely absent per-pose metrics -- a DIFFERENT defect from "one box
        # repeated", wanting a different fix, so it never prints the same line.
        return f"{row['path']}: NO per-pose hurtbox bbox at all (sheet box: {row['sheet_box']})"
    ratio = distinct / len(poses)
    flag = ""
    if len(poses) > 1 and ratio <= ratio_cut:
        flag = "  ⛔ BARELY MOVES" if distinct > 1 else "  ⛔ ONE BOX FOR EVERY POSE"
    same_as_sheet = ""
    if distinct == 1 and len(poses) > 1 and next(iter(poses.values())) == row["sheet_box"]:
        same_as_sheet = " (identical to the sheet-level body_pixel_bbox)"
    return (
        f"{row['path']}: {len(poses)} poses, {distinct} distinct, "
        f"ratio {ratio:.2f}{flag}{same_as_sheet}"
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
    ap.add_argument(
        "--ratio",
        type=float,
        default=FLAT_RATIO,
        help="flag sheets at or below this distinct/pose ratio (default %(default)s); "
        "the observed distribution has a hole between 0.25 and 0.71, so any cut in "
        "that range gives the same answer -- move it and see",
    )
    args = ap.parse_args(argv)

    sheets = list(args.sheets)
    if args.all:
        sheets.extend(find_all())
    if not sheets:
        ap.error("name at least one sheet, or pass --all")

    rows = [read_sheet(p) for p in sheets]

    def rank(r: dict) -> tuple:
        # Ratio first: the reading that survives a tier change.
        return (r["distinct"] / len(r["poses"]) if r["poses"] else -1.0, str(r["path"]))

    for row in sorted(rows, key=rank):
        print(report(row, args.ratio))

    flat = [
        r
        for r in rows
        if r["poses"] and len(r["poses"]) > 1 and r["distinct"] / len(r["poses"]) <= args.ratio
    ]
    print(
        f"\n{len(rows)} sheet(s); {len(flat)} at or below ratio {args.ratio}."
        "\n⇒ These resolve fine and pass any `bbox.is_some()` guard: the box is"
        "\n  plausible, it is just not THIS pose's. Only DISTINCTNESS can see it,"
        "\n  and only as a RATIO -- a count of 1 is the tail, not the defect."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
