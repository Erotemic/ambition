#!/usr/bin/env python3
"""Does each quality tier's sheet variant actually hold fewer pixels?

`sprites_0_5x`, `sprites_0_25x` and `sprites_potato` exist so a room or a
setting can ask for a cheaper character and get one. Nothing checks that the
variant IS cheaper. A variant that was published without being downscaled costs
the FULL page at every stage — decode, upload, residency — while the tier system
believes it saved 4x or 16x, and nothing looks wrong on screen because the art
is correct, merely larger than asked for.

⛔⛔ THIS IS THE OPPOSITE OF A QUALITY REGRESSION AND MUST NOT BE CONFUSED WITH
ONE. Jon's standing rule is that nothing may draw FEWER pixels than the setting
asks for. This measures sheets that draw MORE — full-resolution art delivered
where a smaller tier was requested. Fixing it costs no visual quality at the
tier that asked; it makes the tier mean what it says.

⭐ THE QUANTITY IS TOTAL PAGE MEGAPIXELS ACROSS ALL PAGES, not one page's
dimensions, and the difference decides the answer. A packed multi-page atlas
repacks per tier: `noether`'s 0_5x page is TALLER than its Full page
(4096x4041 vs 4096x3875) and looks like a failure by that measure, while its
total across pages is genuinely smaller. Measured by page dimensions alone this
script reported six offenders; measured by what residency actually pays, four.

⭐ `--ages` ANSWERS THE QUESTION THE OFFENDER LIST CANNOT: is a bad variant a
LEFTOVER from an earlier render, or did the generator produce it in the same run
as its own full-resolution sheet? Those have different fixes, and a clean regen
elsewhere only settles the first. Measured 2026-09-02: three of the four were
written WITHIN MINUTES of their own full sheet — a live defect, not staleness —
while `officer`'s variants predate its full sheet by ten hours, which is the
other mechanism.

⚠ mtimes are per-machine and a copy rewrites them; read the CONTRAST inside one
tree, never the absolute dates.

Usage:
    scripts/measure_tier_variant_scaling.py
    scripts/measure_tier_variant_scaling.py --all          # every sheet, not just offenders
    scripts/measure_tier_variant_scaling.py --ages         # stale leftover, or made now?
"""

from __future__ import annotations

import argparse
import collections
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

# Tier dir -> the fraction of Full's LINEAR size it claims. `potato` has no
# declared ratio (the generator floors every frame at 8px), so it is reported
# but never judged against a ratio it never promised.
TIERS = [
    ("sprites", None),
    ("sprites_0_5x", 0.5),
    ("sprites_0_25x", 0.25),
    ("sprites_potato", None),
]

IMAGES_LIST_RE = re.compile(r"images:\s*\[([^\]]*)\]")
IMAGE_RE = re.compile(r'(?<!s)image:\s*"([^"]+)"')
QUOTED_RE = re.compile(r'"([^"]+)"')


def png_size(path: Path) -> tuple[int, int] | None:
    """IHDR only — this walks every sheet at every tier and must not decode."""
    try:
        with path.open("rb") as handle:
            header = handle.read(24)
    except OSError:
        return None
    if len(header) < 24 or header[:8] != b"\x89PNG\r\n\x1a\n" or header[12:16] != b"IHDR":
        return None
    return struct.unpack(">II", header[16:24])


def sheet_megapixels(manifest: Path) -> tuple[float, int] | None:
    """Total page MP across every page this manifest publishes."""
    try:
        text = manifest.read_text(errors="ignore")
    except OSError:
        return None
    listed = IMAGES_LIST_RE.search(text)
    if listed:
        names = QUOTED_RE.findall(listed.group(1))
    else:
        single = IMAGE_RE.search(text)
        names = [single.group(1)] if single else []
    if not names:
        return None
    total = 0
    for name in names:
        size = png_size(manifest.parent / name)
        if size is None:
            # ⛔ A missing page makes the total an UNDERCOUNT, which would read
            # as "this tier is smaller". Refuse the sheet rather than shrink it.
            return None
        total += size[0] * size[1]
    return total / 1e6, len(names)


def collect() -> dict[Path, dict[str, tuple[float, int]]]:
    """Keyed by path RELATIVE TO THE TIER DIR, so a sheet in a subdirectory
    matches its own variant rather than a same-named sheet at the top level."""
    rows: dict[Path, dict[str, tuple[float, int]]] = collections.defaultdict(dict)
    for tier, _ in TIERS:
        tier_dir = ASSETS / tier
        if not tier_dir.is_dir():
            continue
        for manifest in sorted(tier_dir.rglob("*_spritesheet.ron")):
            measured = sheet_megapixels(manifest)
            if measured is not None:
                rows[manifest.relative_to(tier_dir)][tier] = measured
    return rows


def report_ages(names: list[str]) -> None:
    """For each named sheet, when its variants were written against its own
    full-resolution sheet and against the tier's median.

    ⛔ THE MEDIAN IS THE CONTROL. A variant that is merely old tells you
    nothing; a variant that is old *while the rest of its tier is new* was
    skipped by the run that rewrote the others.
    """
    import datetime
    import statistics

    for tier, _ in TIERS[1:3]:
        tier_dir = ASSETS / tier
        sheets = list(tier_dir.glob("*_spritesheet.png"))
        if not sheets:
            continue
        times = sorted(sheet.stat().st_mtime for sheet in sheets)
        median = statistics.median(times)
        print(
            f"\n{tier}: {len(sheets)} sheets, median "
            f"{datetime.datetime.fromtimestamp(median):%Y-%m-%d %H:%M}"
        )
        for name in names:
            variant = tier_dir / f"{name}_spritesheet.png"
            full = ASSETS / "sprites" / f"{name}_spritesheet.png"
            if not variant.exists() or not full.exists():
                print(f"   {name:<12} absent at this tier")
                continue
            at = variant.stat().st_mtime
            gap = (at - full.stat().st_mtime) / 3600
            verdict = (
                "SAME RUN as its own full sheet — a live generator defect"
                if abs(gap) < 1
                else f"written {gap:+.1f} h from its own full sheet — regenerated apart"
            )
            print(
                f"   {name:<12} {datetime.datetime.fromtimestamp(at):%Y-%m-%d %H:%M} "
                f"({(at - median) / 86400:+.1f} d vs median, "
                f"rank {sum(1 for x in times if x < at)}/{len(times)})  {verdict}"
            )


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--all", action="store_true", help="every sheet, not just offenders")
    ap.add_argument(
        "--ages",
        action="store_true",
        help="was an offending variant left over, or made in the same run?",
    )
    args = ap.parse_args(argv)

    rows = collect()
    offenders: list[tuple[Path, float, dict[str, float]]] = []
    print(f"{'sheet':<44} {'Full MP':>8} {'0_5x':>8} {'0_25x':>8} {'potato':>8}")
    for rel, per_tier in sorted(rows.items()):
        full = per_tier.get("sprites")
        if not full:
            continue
        cells = {t: per_tier.get(t, (0.0, 0))[0] for t, _ in TIERS}
        # ⚠ 95%, not 100%: a repack can differ by a pixel row without being a
        # failure to downscale, and `officer`'s 0_5x is one pixel TALLER than
        # its Full page. The question is whether the tier is meaningfully
        # cheaper, not whether it is byte-identical.
        failed = {
            t: cells[t]
            for t, frac in TIERS[1:3]
            if per_tier.get(t) and cells[t] >= full[0] * 0.95
        }
        if failed:
            offenders.append((rel, full[0], failed))
        if args.all or failed:
            mark = "   ⛔ NOT SMALLER: " + ",".join(sorted(failed)) if failed else ""
            print(
                f"{str(rel)[:44]:<44} {cells['sprites']:8.2f} {cells['sprites_0_5x']:8.2f} "
                f"{cells['sprites_0_25x']:8.2f} {cells['sprites_potato']:8.2f}{mark}"
            )

    # ⛔ THE SECOND MECHANISM, AND IT LOOKS NOTHING LIKE THE FIRST. A sheet with
    # NO variant at a tier cannot be spotted by comparing megapixels — there is
    # nothing to compare — but the room still asks for the cheap tier and still
    # gets the expensive art. Same cost, different cause, and a census that
    # reported only the first would call the tree 4/213 clean when it is 5/213.
    missing: dict[str, list[str]] = {}
    for rel, per_tier in sorted(rows.items()):
        if "sprites" not in per_tier:
            continue
        absent = [t for t, _ in TIERS[1:3] if t not in per_tier]
        if absent:
            missing[rel.name] = absent

    print(f"\n{len(offenders)} sheet(s) whose smaller tiers are not smaller.")
    if missing:
        print(f"{len(missing)} sheet(s) publish NO variant at a reduced tier:")
        for name, absent in missing.items():
            full_mp = next(
                (per["sprites"][0] for rel, per in rows.items() if rel.name == name), 0.0
            )
            print(f"  {full_mp:6.2f} MP  {name:<44} missing {', '.join(absent)}")
    if args.ages:
        named = [rel.name.replace("_spritesheet.ron", "") for rel, _, _ in offenders]
        if not named:
            print("\nno offenders to age-check.")
        else:
            report_ages(named)

    if offenders:
        wasted = sum(mp for _, full, failed in offenders for mp in failed.values())
        asked = sum(
            full * (0.25 if t == "sprites_0_5x" else 0.0625)
            for _, full, failed in offenders
            for t in failed
        )
        print(
            f"  A room asking for those tiers decodes {wasted:.1f} MP where the "
            f"tier promises ~{asked:.1f} MP."
        )
        print(
            "  ⛔ THAT IS A COST DEFECT, NOT A QUALITY ONE: the art shown is "
            "correct and larger than asked. Fixing it removes no pixels from any\n"
            "     tier that requested them."
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
