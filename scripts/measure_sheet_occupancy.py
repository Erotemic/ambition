#!/usr/bin/env python3
"""How much of each spritesheet PAGE is pixels the runtime ever samples?

A character sheet is decoded, uploaded and held resident as a whole PNG page.
Only the frame RECTS its baked manifest names are ever drawn from. Everything
else — the label strip, padding between frames, the tail of a partly-filled
pack row, dead space under a short animation — is megapixels paid for at every
stage of `image_stages` and sampled by nothing.

⛔⛔ THIS MEASURES WASTE, NOT A PROPOSAL. No repack is suggested here and none
should be until the number exists: if the top of the list is a handful of
sheets, that is a content fix and NOT a pipeline change, and those are different
items with different costs. Jon's standing rule also applies — nothing may draw
fewer pixels than the quality setting asks for — and repacking does not draw
fewer pixels, it stops decoding pixels nothing draws. Those are different
claims and this script only supports the second.

⭐ THE SHEET LIST MIRRORS `crates/ambition_sprite_sheet/build.rs`, which is what
actually bakes the registry: four sprite dirs under the actor-monolith crate,
each scanned at its top level plus ONE level of subdirectories (boss multi-sheet
packages live there), for `*_spritesheet.ron`. Keys carry the tier marker the
same way (`root`, `root.0_5x`, `root.0_25x`, `root.potato`). It is not a glob:
a manifest outside those dirs is not baked and is not the runtime's population.

⚠ COVERED AREA IS A UNION, NOT A SUM. Rows share rects — a sheet whose `idle`
and `talk` name the same frame would be double-counted by a sum, and a sheet
could report over 100% occupancy, which is the tell that the arithmetic is
wrong rather than the art. Computed exactly by coordinate compression.

Usage:
    scripts/measure_sheet_occupancy.py                 # Full tier, ranked by waste
    scripts/measure_sheet_occupancy.py --all-tiers
    scripts/measure_sheet_occupancy.py --top 40
    scripts/measure_sheet_occupancy.py --json out.json
"""

from __future__ import annotations

import argparse
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

ASSET_OWNER = REPO / "crates/ambition_platformer2d_actor_monolith"
# The four dirs build.rs scans, in its order. `sprites` is the Full tier.
SPRITE_DIRS = [
    ("full", ASSET_OWNER / "assets/sprites"),
    ("0_5x", ASSET_OWNER / "assets/sprites_0_5x"),
    ("0_25x", ASSET_OWNER / "assets/sprites_0_25x"),
    ("potato", ASSET_OWNER / "assets/sprites_potato"),
]

MANIFEST_SUFFIX = "_spritesheet.ron"
# ⛔⛔ TWO RECT SHAPES, AND ASSUMING ONE PRODUCES A SPECTACULAR FALSE FINDING.
# 384 manifests carry plain grid rects `(x:, y:, w:, h:)`; 174 carry PACKED ones
# `(x:, y:, w:, h:, page: N, off: (dx, dy))` — a trimmed multi-page atlas. A
# regex anchored on `h: NNN)` matches the first and NOTHING in the second, which
# on the first run reported the largest sheets at 0% occupancy and a repo-wide
# "447.9 MP of waste". Those sheets are among the most tightly packed in the
# tree. Caught because 0% is not a plausible measurement, not because the regex
# looked wrong.
RECT_RE = re.compile(
    r"\(x:\s*(-?\d+),\s*y:\s*(-?\d+),\s*w:\s*(\d+),\s*h:\s*(\d+)"
    r"(?:,\s*page:\s*(\d+))?"
)
IMAGE_RE = re.compile(r'(?<!s)image:\s*"([^"]+)"')
IMAGES_LIST_RE = re.compile(r'images:\s*\[([^\]]*)\]')
QUOTED_RE = re.compile(r'"([^"]+)"')
FRAME_COUNT_RE = re.compile(r"frame_count:\s*(\d+)")
RECTS_BLOCK_RE = re.compile(r"rects:\s*\[", re.M)


def frame_rects(text: str) -> list[tuple[int, int, int, int, int]]:
    """Only the rects inside a `rects: [ ... ]` block.

    ⛔⛔ A FLAT REGEX OVER THE FILE IS WRONG AND LOOKS RIGHT. `body_metrics`
    carries `body_pixel_bbox`, per-animation `hurtbox` and `hitbox` rects in the
    IDENTICAL `(x: .., y: .., w: .., h: ..)` shape — dozens per sheet, in image
    space, overlapping the frames. Counting them as sampled area would inflate
    occupancy toward (and past) 100% and turn this census into an argument that
    there is no waste. Found before the first run, by reading a manifest rather
    than trusting the shape of the regex.
    """
    out: list[tuple[int, int, int, int, int]] = []
    for opening in RECTS_BLOCK_RE.finditer(text):
        depth = 0
        i = opening.end() - 1
        while i < len(text):
            if text[i] == "[":
                depth += 1
            elif text[i] == "]":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        block = text[opening.end() : i]
        for x, y, w, h, page in RECT_RE.findall(block):
            out.append((int(x), int(y), int(w), int(h), int(page or 0)))
    return out


def baked_manifests(tier_dir: Path) -> list[Path]:
    """`build.rs`'s scan: this dir, plus one level of subdirs. Not recursive."""
    if not tier_dir.is_dir():
        return []
    found = [p for p in sorted(tier_dir.iterdir()) if p.name.endswith(MANIFEST_SUFFIX)]
    for sub in sorted(tier_dir.iterdir()):
        if sub.is_dir():
            found += [p for p in sorted(sub.iterdir()) if p.name.endswith(MANIFEST_SUFFIX)]
    return found


def png_size(path: Path) -> tuple[int, int] | None:
    """Width/height from the IHDR chunk — stdlib only, no decode.

    ⭐ Reading 24 bytes instead of decoding: this script walks every published
    sheet at every tier, and decoding them to ask their size would be the same
    work the census exists to complain about.
    """
    try:
        with path.open("rb") as handle:
            header = handle.read(24)
    except OSError:
        return None
    if len(header) < 24 or header[:8] != b"\x89PNG\r\n\x1a\n" or header[12:16] != b"IHDR":
        return None
    width, height = struct.unpack(">II", header[16:24])
    return width, height


def union_area(rects: list[tuple[int, int, int, int]]) -> tuple[int, bool]:
    """Exact area of the union of axis-aligned rects, and whether any overlapped.

    ⛔ NOT A SUM, AND THE DIFFERENCE IS NOT PEDANTRY. Rows share frames — an
    `idle` and a `talk` naming the same rect would be counted twice, and a sheet
    could report over 100% occupancy, which reads as a broken sheet rather than
    as broken arithmetic.

    ⭐ FAST PATH FIRST, because these are packed grids: dedupe, and if no two
    rects overlap the sum IS the union. The exact sweep runs only when that is
    false. The bool is returned rather than hidden so the caller can report how
    often the slow path was needed instead of assuming.
    """
    boxes = sorted({(x, y, x + w, y + h) for x, y, w, h in rects if w > 0 and h > 0})
    if not boxes:
        return 0, False

    # Sweep by x0 with an active set; only rects whose x-ranges overlap can
    # intersect at all.
    overlapped = False
    active: list[tuple[int, int, int, int]] = []
    for box in boxes:
        active = [a for a in active if a[2] > box[0]]
        for a in active:
            if a[1] < box[3] and box[1] < a[3]:
                overlapped = True
                break
        if overlapped:
            break
        active.append(box)

    if not overlapped:
        return sum((x1 - x0) * (y1 - y0) for x0, y0, x1, y1 in boxes), False

    # Exact union: coordinate compression over the (few) overlapping sets.
    xs = sorted({v for b in boxes for v in (b[0], b[2])})
    ys = sorted({v for b in boxes for v in (b[1], b[3])})
    area = 0
    for xi in range(len(xs) - 1):
        x0, x1 = xs[xi], xs[xi + 1]
        spanning = [b for b in boxes if b[0] <= x0 and b[2] >= x1]
        if not spanning:
            continue
        covered = 0
        for yi in range(len(ys) - 1):
            y0, y1 = ys[yi], ys[yi + 1]
            if any(b[1] <= y0 and b[3] >= y1 for b in spanning):
                covered += y1 - y0
        area += covered * (x1 - x0)
    return area, True


def page_images(text: str) -> list[str]:
    """Page index -> PNG filename.

    A packed sheet lists every page in `images: [...]` with page 0 first and the
    same name as the singular `image:` field; a plain sheet has only `image:`.
    Both are read rather than one being inferred from the other.
    """
    listed = IMAGES_LIST_RE.search(text)
    if listed:
        return QUOTED_RE.findall(listed.group(1))
    single = IMAGE_RE.search(text)
    return [single.group(1)] if single else []


def sheets_in(tier_dir: Path) -> list[dict]:
    """One row per PAGE — a packed sheet's pages are separate PNGs and separate
    residency, so averaging them would hide a mostly-empty tail page."""
    rows: list[dict] = []
    for manifest in baked_manifests(tier_dir):
        try:
            text = manifest.read_text(errors="ignore")
        except OSError:
            continue
        images = page_images(text)
        rects = frame_rects(text)
        declared_frames = sum(int(n) for n in FRAME_COUNT_RE.findall(text))
        sheet = manifest.name[: -len(MANIFEST_SUFFIX)]
        if not images:
            rows.append({"sheet": sheet, "skipped": "no image named in the manifest"})
            continue

        by_page: dict[int, list[tuple[int, int, int, int]]] = {}
        for x, y, w, h, page in rects:
            by_page.setdefault(page, []).append((x, y, w, h))

        # ⛔ A RECT NAMING A PAGE THE MANIFEST DOES NOT PUBLISH is a parse
        # failure, not an empty page. Reported rather than silently dropped.
        stray = sorted(pg for pg in by_page if pg >= len(images))
        if stray:
            rows.append(
                {
                    "sheet": sheet,
                    "skipped": f"rects name page(s) {stray} but only {len(images)} image(s) published",
                }
            )
            continue

        for page, name in enumerate(images):
            png = manifest.parent / name
            size = png_size(png)
            if size is None:
                rows.append(
                    {"sheet": f"{sheet}#{page}", "skipped": f"no readable PNG at {name}"}
                )
                continue
            width, height = size
            page_px = width * height
            covered, overlapped = union_area(by_page.get(page, []))
            rows.append(
                {
                    "manifest": str(manifest.relative_to(REPO)),
                    "sheet": sheet if len(images) == 1 else f"{sheet}#{page}",
                    "image": name,
                    "width": width,
                    "height": height,
                    "page_mp": page_px / 1e6,
                    "covered_mp": covered / 1e6,
                    "waste_mp": (page_px - covered) / 1e6,
                    "occupancy": (covered / page_px) if page_px else 0.0,
                    "frames": len(by_page.get(page, [])),
                    "declared_frames": declared_frames,
                    "overlapping_rects": overlapped,
                }
            )
    return rows



def report_orphans(wanted: list[tuple[str, Path]]) -> None:
    """⛔ SUPERSEDED — this now DELEGATES to `measure_orphan_shipped_pages.py`.

    This function used to carry its own definition of "orphan": PNGs whose name
    contains `_spritesheet` and that no manifest in the same tier names. That
    definition disagreed with the fuller census in three ways — it ignored art
    that is not sheet-named (the reduced-tier portraits, the single largest
    class), it never checked whether a filename is declared in committed source,
    and it reported one undifferentiated number where the confident and the
    speculative populations deserve to be told apart.

    ⚠ TWO SCRIPTS WITH TWO DEFINITIONS EVENTUALLY PRINT TWO ANSWERS, and the
    older docstring's "15 files per tier, 775 MP at Full" was already being read
    beside the newer figures. One definition, in one place.
    """
    import importlib.util

    script = REPO / "scripts/measure_orphan_shipped_pages.py"
    if not script.exists():
        print(f"\n⚠ {script} is absent; orphan reporting lives there now.")
        return
    spec = importlib.util.spec_from_file_location("orphan_shipped_pages", script)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    print("\n(orphans: delegating to scripts/measure_orphan_shipped_pages.py)")
    module.main([])


def report(rows: list[dict], label: str, top: int) -> None:
    measured = [r for r in rows if "skipped" not in r]
    skipped = [r for r in rows if "skipped" in r]
    if not measured:
        print(f"\n{label}: no measurable sheets")
        return

    page = sum(r["page_mp"] for r in measured)
    covered = sum(r["covered_mp"] for r in measured)
    waste = page - covered

    print(f"\n=== {label} — {len(measured)} pages ===")
    print(f"{'sheet':<44} {'page MP':>8} {'used MP':>8} {'occ':>6} {'waste MP':>9}")
    for r in sorted(measured, key=lambda r: -r["waste_mp"])[:top]:
        print(
            f"{r['sheet']:<44} {r['page_mp']:8.2f} {r['covered_mp']:8.2f} "
            f"{r['occupancy']:5.0%} {r['waste_mp']:9.2f}"
        )

    print(
        f"\n  TOTAL  page {page:.1f} MP · sampled {covered:.1f} MP · "
        f"occupancy {covered / page:.0%} · WASTE {waste:.1f} MP"
    )
    # ⭐ CONCENTRATION IS THE DECIDING NUMBER, not the total: a handful of bad
    # sheets is a content fix, a flat distribution is a pipeline change, and
    # those are different items. Say which this is.
    ranked = sorted(measured, key=lambda r: -r["waste_mp"])
    for n in (5, 10, 25):
        if len(ranked) > n:
            share = sum(r["waste_mp"] for r in ranked[:n]) / waste if waste else 0
            print(f"  top {n:>2} sheets hold {share:.0%} of the waste")
    # ⛔⛔ THE PARSER'S OWN PREMISE, CHECKED EVERY RUN. A single-page sheet's
    # parsed rects must equal the `frame_count` its rows declare. This is what
    # separates "the art is empty" from "the regex missed" — the first version of
    # this script reported the biggest sheets at 0% occupancy and 447.9 MP of
    # repo-wide waste because it could not read the packed rect shape at all.
    # A census that cannot fail this way will publish that number again.
    single = [r for r in measured if "#" not in r["sheet"]]
    mismatched = [r for r in single if r["frames"] != r["declared_frames"]]
    if mismatched:
        print(
            f"\n  ⛔ PARSE SUSPECT on {len(mismatched)} single-page sheet(s): parsed "
            "rects != declared frame_count. Do NOT quote the totals above."
        )
        for r in mismatched[:5]:
            print(f"      {r['sheet']}: {r['frames']} parsed vs {r['declared_frames']} declared")
    else:
        print(
            f"  ✔ parse premise: all {len(single)} single-page sheets' rects match "
            "their declared frame_count"
        )
    if any(r["occupancy"] > 1.0 for r in measured):
        print("  ⛔ a page reports >100% occupancy — the union arithmetic is wrong")

    if skipped:
        print(f"\n  ⚠ {len(skipped)} manifest(s) NOT measured — absent is not zero:")
        for r in skipped[:6]:
            print(f"      {r['sheet']}: {r['skipped']}")
        if len(skipped) > 6:
            print(f"      … and {len(skipped) - 6} more")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--all-tiers", action="store_true", help="every quality tier, not just Full")
    ap.add_argument("--top", type=int, default=25)
    ap.add_argument("--json", type=str, default=None)
    ap.add_argument(
        "--orphans",
        action="store_true",
        help="also report sheet PNGs that no manifest names",
    )
    args = ap.parse_args(argv)

    wanted = SPRITE_DIRS if args.all_tiers else SPRITE_DIRS[:1]
    everything: dict[str, list[dict]] = {}
    for tier, path in wanted:
        rows = sheets_in(path)
        everything[tier] = rows
        report(rows, f"{tier} ({path.relative_to(REPO)})", args.top)

    if args.orphans:
        report_orphans(wanted)
    if args.json:
        Path(args.json).write_text(json.dumps(everything, indent=2, sort_keys=True))
        print(f"\nwrote {args.json}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
