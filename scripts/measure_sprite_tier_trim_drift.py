#!/usr/bin/env python3
"""Report how far each resolution tier's TRIM FRACTIONS drift from the base sheet's.

⭐ WHY THIS EXISTS. A character's drawn quad is
`authored_render * (trim_w / frame_w, trim_h / frame_h)` -- the authored render size
is in world units and is tier-independent, so the TRIM FRACTION is what decides the
quad's shape. If a tier's fraction differs from the base sheet's, that character is
drawn at a different ASPECT RATIO at that quality setting.

Measured 2026-09-06, `mary_o_v2` idle:

    base    trim 63x86 in a 160x192 frame -> 0.394 x 0.448 -> quad 24.00 x 32.76
    potato  trim  7x5  in a  10x12  frame -> 0.700 x 0.417 -> quad 42.67 x 30.48

Both reproduce the rendered quads to the last digit, and the aspect flips from
portrait to landscape. This is the measurable half of Jon's report that *"the size of
the snake has seemed to vary depending on the global game state"*.

⭐⭐ THE GRADIENT IS THE EVIDENCE FOR THE MECHANISM. Alpha-trim is measured on the
DOWNSCALED image, so an anti-aliasing fringe of roughly fixed PIXEL width becomes a
larger FRACTION of a smaller frame. That predicts drift growing as the frame shrinks,
and it does, monotonically -- run it and read the three counts.

⛔⛔ THIS IS A REPORT, NOT A GATE, AND MUST STAY ONE. The variant directories are
GITIGNORED generated art, so what this measures is THIS MACHINE's copy. A gate reading
them would answer a question about the machine rather than about the tree -- and it
would be silently green on any box that has never run the variant generator. Absent
tiers are skipped and said so, never counted as passing.

⚠ A base row whose trim fills the whole frame is UNTRIMMED (grid mode). Comparing it
against a packed tier is a packing-mode difference, not a measurement error; those
rows dominated the first run's output and are excluded.
"""
import argparse
import pathlib
import re
import sys

ASSETS = pathlib.Path(__file__).resolve().parents[1] / (
    "crates/ambition_platformer2d_actor_monolith/assets"
)
TIERS = ["sprites_0_5x", "sprites_0_25x", "sprites_potato"]
#: A row must be trimmed on both sides to be comparable at all.
UNTRIMMED = 0.999


def parse_sheet(path: pathlib.Path):
    """`(frame_w, frame_h, {anim: (trim_w, trim_h)})` from a sheet RON, or None.

    Reads the FIRST rect of each row: frames within one animation share a packing
    pass, and the question here is the row's scale, not per-frame variation.
    """
    text = path.read_text()
    fw = re.search(r"frame_width:\s*(\d+)", text)
    fh = re.search(r"frame_height:\s*(\d+)", text)
    if not (fw and fh):
        return None
    rows = {}
    for match in re.finditer(r'animation:\s*"([^"]+)".*?rects:\s*\[(.*?)\]', text, re.S):
        rect = re.search(r"w:\s*(\d+),\s*h:\s*(\d+)", match.group(2))
        if rect:
            rows[match.group(1)] = (int(rect.group(1)), int(rect.group(2)))
    return int(fw.group(1)), int(fh.group(1)), rows


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--threshold", type=float, default=0.05,
                    help="fraction drift counted as a divergence (default 0.05)")
    ap.add_argument("--show", type=int, default=6, help="worst rows to print per tier")
    args = ap.parse_args()

    base_dir = ASSETS / "sprites"
    if not base_dir.is_dir():
        print(f"no base sheets at {base_dir} — nothing to compare")
        return 0

    present = [t for t in TIERS if (ASSETS / t).is_dir()]
    absent = [t for t in TIERS if t not in present]
    print(f"tiers present: {present or 'none'}")
    if absent:
        # Said out loud: an absent tier is UNMEASURED, which is not the same fact as
        # a tier that was measured and agreed.
        print(f"tiers ABSENT (gitignored, unmeasured — NOT a pass): {absent}")

    findings = {tier: [] for tier in present}
    for base_file in sorted(base_dir.glob("*_spritesheet.ron")):
        base = parse_sheet(base_file)
        if not base:
            continue
        base_w, base_h, base_rows = base
        for tier in present:
            tier_file = ASSETS / tier / base_file.name
            if not tier_file.exists():
                continue
            other = parse_sheet(tier_file)
            if not other:
                continue
            tier_w, tier_h, tier_rows = other
            for anim, (bw, bh) in base_rows.items():
                if anim not in tier_rows:
                    continue
                bxf, byf = bw / base_w, bh / base_h
                if bxf >= UNTRIMMED and byf >= UNTRIMMED:
                    continue
                tw, th = tier_rows[anim]
                txf, tyf = tw / tier_w, th / tier_h
                findings[tier].append(
                    (max(abs(txf - bxf), abs(tyf - byf)),
                     base_file.stem, anim, bxf, txf, byf, tyf)
                )

    worst_share = 0.0
    for tier in present:
        rows = sorted(findings[tier], reverse=True)
        if not rows:
            print(f"\n=== {tier}: no comparable rows ===")
            continue
        bad = [r for r in rows if r[0] > args.threshold]
        share = len(bad) / len(rows)
        worst_share = max(worst_share, share)
        print(f"\n=== {tier}: {len(bad)}/{len(rows)} rows drift > {args.threshold} "
              f"({share:.1%}) ===")
        for drift, sheet, anim, bxf, txf, byf, tyf in rows[:args.show]:
            print(f"  {drift:.3f}  {sheet:<34} {anim:<14} "
                  f"wfrac {bxf:.3f}->{txf:.3f}  hfrac {byf:.3f}->{tyf:.3f}")

    print("\n⇒ Drift should GROW as the frame shrinks; that gradient is the signature "
          "of alpha-trim measured on the downscaled image.")
    print("⛔ Report only — the tiers are gitignored generated art, so this describes "
          "THIS MACHINE's copy, not the tree.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
