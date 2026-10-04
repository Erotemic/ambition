#!/usr/bin/env python3
"""Where do published part flipbooks waste texels?
(`docs/planning/engine/mary-o-part-realization.md`; the cost census is
`scripts/measure_part_flipbook_cost.py`, this asks WHY a page is big.)

Reads each shipped `<target>_parts.ron` with its pages and splits its part
texels by cause:

* `overlay` — tracks recorded from an effect layer (`overlay:<layer>`): a whole
  raster a frame, where a rig would place a few pieces (Carl Stargan's orbit
  rings, 2026-10-04).
* `per_frame` — rig tracks with a new raster in most frames they draw: shapes
  painted already posed, not a piece turned into place.
* `duplicate` — parts whose pixels repeat another part's, up to its pivot
  (`same`), a mirror (`mirror`) or a quarter turn (`turn`): art stored twice
  that one part and a draw transform could carry.

and counts the draws a frame. A part is counted in the first cause that
claims it, in that order (an overlay raster is not also a duplicate).

    scripts/measure_part_waste.py                 # every flipbook, worst first
    scripts/measure_part_waste.py --target sybil --tracks
    scripts/measure_part_waste.py --record --label "before dedupe"

Rows go to `dev/ambition_dev_measurements/part_flipbook_cost.jsonl` (kind
`part_flipbook_waste`) with `--record`.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(REPO / "tools" / "ambition_sprite2d_renderer"))

from PIL import Image  # noqa: E402

from ambition_sprite2d_renderer.authoring.part_flipbook import PartFlipbook  # noqa: E402
from measure_part_flipbook_cost import LEDGER, _envelope  # noqa: E402

SPRITES = REPO / "crates" / "ambition_platformer2d_actor_monolith" / "assets" / "sprites"

#: A rig track is `per_frame` when more than this share of the frames that
#: draw it draw a raster no earlier frame of the track drew.
PER_FRAME_SHARE = 0.5


def _key(image: Image.Image) -> str:
    """The part's pixels, trimmed to their drawn extent: equal keys are the
    same art whatever the pivot or padding."""
    box = image.getchannel("A").getbbox()
    trimmed = image.crop(box) if box else image.crop((0, 0, 1, 1))
    return hashlib.sha1(f"{trimmed.size}".encode() + trimmed.tobytes()).hexdigest()


def _variants(image: Image.Image):
    """(kind, key) of each transform a draw can carry: a mirror (scale -1) or
    a quarter turn (rotation)."""
    yield "mirror", _key(image.transpose(Image.FLIP_LEFT_RIGHT))
    yield "mirror", _key(image.transpose(Image.FLIP_TOP_BOTTOM))
    for turn in (Image.ROTATE_90, Image.ROTATE_180, Image.ROTATE_270):
        yield "turn", _key(image.transpose(turn))


def waste(flipbook: PartFlipbook) -> dict:
    parts = flipbook.parts
    area = [p.image.width * p.image.height for p in parts]
    # Which tracks draw each part, and each track's rasters in frame order.
    tracks_of = defaultdict(set)
    frames_of = defaultdict(int)
    new_of = defaultdict(int)
    seen = defaultdict(set)
    draws_per_frame = []
    for _duration, frames in flipbook.clips.values():
        for draws in frames:
            draws_per_frame.append(len(draws))
            for d in draws:
                track = d.track or f"part{d.part}"
                tracks_of[d.part].add(track)
                frames_of[track] += 1
                if d.part not in seen[track]:
                    seen[track].add(d.part)
                    new_of[track] += 1
    per_frame_tracks = {
        t for t in frames_of if not t.startswith("overlay:") and frames_of[t] > 2 and new_of[t] > PER_FRAME_SHARE * frames_of[t]
    }
    cause = {}
    for i in range(len(parts)):
        tracks = tracks_of.get(i, set())
        if any(t.startswith("overlay:") for t in tracks):
            cause[i] = "overlay"
        elif tracks and tracks <= per_frame_tracks:
            cause[i] = "per_frame"
    # Duplicates among the rest: the first part of each art is kept.
    first = {}
    dup_kind = {}
    for i, p in enumerate(parts):
        if i in cause:
            continue
        key = _key(p.image)
        if key in first:
            dup_kind[i] = "same"
            continue
        match = next((kind for kind, k in _variants(p.image) if k in first), None)
        if match:
            dup_kind[i] = match
            continue
        first[key] = i
    total = sum(area)
    by = defaultdict(int)
    for i, c in cause.items():
        by[c] += area[i]
    dups = defaultdict(int)
    for i, kind in dup_kind.items():
        dups[kind] += area[i]
    page_texels = sum(page.width * page.height for page in flipbook.pages)
    return {
        "parts": len(parts),
        "part_texels": total,
        "page_texels": page_texels,
        "overlay_texels": by["overlay"],
        "per_frame_texels": by["per_frame"],
        "duplicate_texels": dict(dups),
        "duplicate_parts": len(dup_kind),
        "per_frame_tracks": sorted(per_frame_tracks),
        "draws_per_frame_max": max(draws_per_frame, default=0),
        "track_rasters": {t: (len(seen[t]), frames_of[t]) for t in sorted(frames_of)},
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--target", action="append", help="repeatable; default every published flipbook")
    parser.add_argument("--tracks", action="store_true", help="also print each track's rasters / frames")
    parser.add_argument("--sprites", type=Path, default=SPRITES, help="a directory of published flipbooks (default: the game's)")
    parser.add_argument("--record", action="store_true")
    parser.add_argument("--label", default="")
    args = parser.parse_args()
    targets = args.target or sorted(path.name[: -len("_parts.ron")] for path in args.sprites.glob("*_parts.ron"))
    if not targets:
        print(f"no part flipbooks under {args.sprites}", file=sys.stderr)
        return 2
    results = {}
    for target in targets:
        results[target] = waste(PartFlipbook.from_published(args.sprites / f"{target}_parts.ron"))
    # ⛔ A census that read nothing is not a clean bill.
    assert len(results) == len(targets), "a flipbook was skipped"

    def share(r, n):
        return f"{n / max(1, r['part_texels']):6.1%}"

    print(f"{'target':32} {'parts':>6} {'part MTx':>9} {'overlay':>7} {'perframe':>8} {'dup':>7}  {'draws':>5}  per-frame tracks")
    for target, r in sorted(results.items(), key=lambda kv: -kv[1]["part_texels"]):
        dup = sum(r["duplicate_texels"].values())
        print(f"{target:32} {r['parts']:6d} {r['part_texels'] / 1e6:9.3f} {share(r, r['overlay_texels']):>7} "
              f"{share(r, r['per_frame_texels']):>8} {share(r, dup):>7}  {r['draws_per_frame_max']:5d}  "
              f"{', '.join(r['per_frame_tracks'][:6])}{' …' if len(r['per_frame_tracks']) > 6 else ''}")
        if args.tracks:
            for track, (rasters, frames) in r["track_rasters"].items():
                print(f"    {track:28} {rasters:4d} rasters / {frames:4d} frames")
    total = sum(r["part_texels"] for r in results.values())
    print(f"{len(results)} flipbooks, {total / 1e6:.1f} MTexel of parts: overlay "
          f"{sum(r['overlay_texels'] for r in results.values()) / total:.1%}, per-frame "
          f"{sum(r['per_frame_texels'] for r in results.values()) / total:.1%}, duplicate "
          f"{sum(sum(r['duplicate_texels'].values()) for r in results.values()) / total:.1%}")
    if args.record:
        import secrets

        run_id = secrets.token_hex(6)
        with LEDGER.open("a") as ledger:
            for target, r in results.items():
                row = _envelope("part_flipbook_waste", run_id, args.label)
                row.update({"target": target, **{k: v for k, v in r.items() if k != "track_rasters"}})
                ledger.write(json.dumps(row, sort_keys=True) + "\n")
        print(f"appended {len(results)} row(s) to {LEDGER.relative_to(REPO)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
