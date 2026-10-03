#!/usr/bin/env python3
"""In-engine parity: does the GAME draw a character's part flipbook as its baked
sheet? (`docs/planning/engine/mary-o-part-realization.md`, Validation 2.)

Runs `crates/ambition_render/examples/rigged_sprite_parity.rs`, which draws
every frame of each target through Bevy's renderer twice — from the baked sheet
and from the part flipbook — and diffs each pair with the renderer's own
measures (`part_flipbook.parity` and `largest_wrong_blob`, standard tolerance:
64 levels, one pixel of slack).

    scripts/measure_rigged_parity.py                       # Mary-O's three forms
    scripts/measure_rigged_parity.py --target pirate_admiral --scale 2

Exit 0 when every frame is inside the bounds, 1 when one is not, 2 when the
harness measured nothing it could trust.

Run it with an interpreter that has the sprite renderer's dependencies (it
imports the renderer's measures and flipbook reader).

The oracle for the game's PART draw is the published draws drawn the baked
road's way, unclipped; the offline gate proves those draws are the baked frame
inside the frame. The game's BAKED draw and the art the baked frame cuts off
are reported beside it, not gated.

⚠ The adapter matters. On a software rasterizer (llvmpipe) the sampling is
llvmpipe's; a hardware GPU may filter a turned part differently. The report
states the adapter Bevy picked.
"""

from __future__ import annotations

import argparse
import csv
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "tools" / "ambition_sprite2d_renderer"))

import numpy as np  # noqa: E402
import yaml  # noqa: E402
from PIL import Image  # noqa: E402

from ambition_sprite2d_renderer.authoring.part_flipbook import (  # noqa: E402
    PartFlipbook,
    largest_wrong_blob,
    parity,
    tween_draws,
)

SPRITES = REPO / "crates" / "ambition_platformer2d_actor_monolith" / "assets" / "sprites"

PARITY_BOUND = 0.01
#: Wider than the offline gate's 6, and measured, not guessed. The GPU turns a
#: part by bilinear sampling and PIL by premultiplied bicubic, so a turned
#: part's outline can come out one shade apart along a one-pixel line: 9 pixels
#: on Mary-O's fire `transform`[3] (a part turned 88 degrees, over the aura),
#: on llvmpipe, 2026-10-02. Dropping any one VISIBLE draw from any Mary-O frame
#: makes a blob of 12 or more (618 draws measured; the three that make 0 are
#: wholly covered), so 10 still sees every missing part and effect.
BLOB_BOUND = 10
DEFAULT_TARGETS = ("mary_o_v2", "mary_o_v2_tall", "mary_o_v2_fire")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--target", action="append", help="repeatable; default Mary-O's three forms")
    parser.add_argument("--scale", type=float, default=1.0, help="drawn size over the frame size (1 = texel per pixel)")
    parser.add_argument("--both-facings", action="store_true")
    parser.add_argument(
        "--centre-anchored", action="store_true",
        help="build the root as a player with a sheet-authored quad (Anchor::CENTER), not at its feet",
    )
    parser.add_argument(
        "--phase", type=float, default=0.0,
        help="how far into each frame (0..1); above 0 a tweened clip is checked against its in-between",
    )
    parser.add_argument("--out", type=Path, default=REPO / "target" / "rig_parity")
    parser.add_argument("--no-build", action="store_true", help="reuse the last captures in --out")
    args = parser.parse_args()
    targets = args.target or list(DEFAULT_TARGETS)

    if not args.no_build:
        command = [
            "cargo", "run", "-q", "-p", "ambition_render", "--features", "capture",
            "--example", "rigged_sprite_parity", "--",
            "--out", str(args.out), "--scale", str(args.scale),
        ]
        for target in targets:
            command += ["--target", target]
        if args.both_facings:
            command.append("--both-facings")
        if args.centre_anchored:
            command.append("--centre-anchored")
        if args.phase:
            command += ["--phase", str(args.phase)]
        run = subprocess.run(command, cwd=REPO, capture_output=True, text=True)
        if run.returncode != 0:
            print(run.stdout + run.stderr, file=sys.stderr)
            return 2
        adapter = next((line for line in run.stderr.splitlines() if "software rendering" in line), None)
        print("adapter: software (llvmpipe)" if adapter else "adapter: hardware (no software-rendering warning)")

    index = args.out / "index.tsv"
    rows = [row for row in csv.DictReader(index.open(), delimiter="\t") if row["target"] in targets]
    if not rows:
        print(f"no captures for {targets} in {index}", file=sys.stderr)
        return 2
    if args.scale != 1.0:
        print("the published-draw oracle is drawn texel per pixel: run with --scale 1", file=sys.stderr)
        return 2

    flipbooks = {
        target: PartFlipbook.from_published(SPRITES / f"{target}_parts.ron", placement="snapped") for target in targets
    }
    sheets = {target: _sheet(target) for target in targets}
    worst = defaultdict(lambda: [0.0, 0, 0.0, 0])
    failures = []
    distinct = defaultdict(set)
    clipped = defaultdict(list)
    for row in rows:
        target, name, frame = row["target"], row["row"], int(row["frame"])
        flip = row["flip"] == "true"
        feet = (float(row["feet_x"]), float(row["feet_y"]))
        root_x = float(row["root_x"])
        baked = _unpremultiplied(Image.open(row["baked"]).convert("RGBA"))
        parts = _unpremultiplied(Image.open(row["parts"]).convert("RGBA"))
        # ⛔ Premise: the harness drew something, and it drew the PINNED frame.
        # An empty capture agrees with an empty capture.
        if baked.getchannel("A").getbbox() is None or parts.getchannel("A").getbbox() is None:
            print(f"{target} {name}[{frame}]: an empty capture", file=sys.stderr)
            return 2
        distinct[target].add(parts.tobytes())
        # The oracle for the GAME's part draw is the PUBLISHED draws drawn the
        # baked road's way (PIL), unclipped. The offline gate already proves
        # those draws are the baked frame inside the frame.
        oracle = Image.new("RGBA", parts.size, (0, 0, 0, 0))
        # At a phase, the oracle is the published tween rule (`tween_draws`); a
        # clip that steps is its frame.
        draws = tween_draws(flipbooks[target], name, frame, args.phase)
        flipbooks[target].draw_frame(oracle, name, frame, feet, flip, draws=draws, mirror_x=root_x)
        wrong, blob = parity(oracle, parts), largest_wrong_blob(oracle, parts)
        # For the record: the published FRAME there, and how the game's baked
        # road draws it.
        published = _published_frame(sheets[target], flipbooks[target], name, frame, parts.size, feet, flip, root_x)
        # ⚠ AN IN-BETWEEN IS REPORTED, NOT GATED. PIL rounds a tweened part to
        # whole pixels; the GPU draws it between pixels and does not. The
        # raster difference is that rounding (1-2%, blobs to 39 on Mary-O,
        # 2026-10-02). The tween's PLACES are gated where they are exact: the
        # renderer's `test_a_tweened_clip_places_each_part_where_the_in_between_pose_does`
        # and the runtime's `a_tweened_clip_draws_between_its_frames`.
        in_between = args.phase > 0.0 and bool(flipbooks[target].tweens.get(name))
        baked_wrong, baked_blob = parity(published, baked), largest_wrong_blob(published, baked)
        lost = int((np.asarray(oracle)[..., 3] > 0).sum() - (np.asarray(published)[..., 3] > 0).sum())
        if lost > 0 and not in_between and f"{name}#{frame}" not in clipped[target]:
            clipped[target].append(f"{name}#{frame}")
        w = worst[(target, name)]
        worst[(target, name)] = [max(w[0], wrong), max(w[1], blob), max(w[2], baked_wrong), max(w[3], baked_blob)]
        if not in_between and (wrong > PARITY_BOUND or blob > BLOB_BOUND):
            failures.append(f"{target} {name}[{frame}] flip={flip}: {wrong:.4f}, blob {blob}")
    for target in targets:
        frames = sum(1 for row in rows if row["target"] == target)
        # Frames that draw the same picture are legitimate (a held pose), but a
        # harness that drew ONE picture for every pin pinned nothing.
        if frames > 1 and len(distinct[target]) < 2:
            print(f"{target}: every capture is the same picture — the pin did not reach the drawing", file=sys.stderr)
            return 2
    print(f"{len(rows)} frames. Columns: the game's PART draw against the published draws (the gate),")
    print("then the game's BAKED draw against the published frame (for the record).")
    for (target, name), (wrong, blob, baked_wrong, baked_blob) in sorted(worst.items()):
        note = "  (in-between: reported, not gated)" if args.phase > 0.0 and flipbooks[target].tweens.get(name) else ""
        print(
            f"  {target:16} {name:12} parts {wrong * 100:5.2f}% blob {blob:3d}"
            f"   | baked {baked_wrong * 100:5.2f}% blob {baked_blob:3d}{note}"
        )
    for target, frames in clipped.items():
        print(f"  {target}: the baked frame cuts off art the parts draw, on {len(frames)} frame(s): {', '.join(frames)}")
    if failures:
        print(f"{len(failures)} frame(s) outside A <= {PARITY_BOUND:.0%} and B <= {BLOB_BOUND} px:")
        for line in failures:
            print("  " + line)
        return 1
    print(f"ok: every part-drawn frame inside A <= {PARITY_BOUND:.0%} and B <= {BLOB_BOUND} px")
    return 0


def _unpremultiplied(image):
    """A capture as straight alpha.

    ⛔ A CAMERA THAT CLEARS TO TRANSPARENT WRITES PREMULTIPLIED COLOUR. Sprites
    blend `src * a + dst * (1 - a)`, so over a transparent clear a half-covered
    pixel stores half its colour. Compared as if it were straight alpha, every
    edge and every translucent effect reads too dark, and the measure goes
    blind to exactly the pixels a premultiplication bug changes."""
    array = np.asarray(image, dtype=np.float64)
    alpha = array[..., 3:4]
    rgb = np.where(alpha > 0, np.clip(array[..., :3] * 255.0 / np.maximum(alpha, 1.0), 0, 255), 0)
    return Image.fromarray(np.concatenate([rgb, alpha], axis=-1).round().astype(np.uint8), "RGBA")


def _sheet(target):
    sheet = yaml.safe_load((SPRITES / f"{target}_spritesheet.yaml").read_text())
    atlas = Image.open(SPRITES / sheet["image"]).convert("RGBA")
    return {row["animation"]: row for row in sheet["rows"]}, atlas


def _published_frame(sheet, flipbook, name, index, size, feet, flip, root_x):
    rows, atlas = sheet
    rect = rows[name]["rects"][index]
    frame = Image.new("RGBA", flipbook.frame_size, (0, 0, 0, 0))
    frame.paste(atlas.crop((rect["x"], rect["y"], rect["x"] + rect["w"], rect["y"] + rect["h"])), tuple(rect["off"]))
    canvas = Image.new("RGBA", size, (0, 0, 0, 0))
    # Mirrored about the root's column, as `PartFlipbook.draw_frame` mirrors.
    fx = feet[0] if not flip else size[0] - (2.0 * root_x - feet[0])
    canvas.alpha_composite(frame, (round(fx - flipbook.feet[0]), round(feet[1] - flipbook.feet[1])))
    return canvas.transpose(Image.FLIP_LEFT_RIGHT) if flip else canvas


if __name__ == "__main__":
    raise SystemExit(main())
