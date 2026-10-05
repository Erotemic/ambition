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
import math
import csv
import os
import subprocess
import sys
from collections import defaultdict
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "tools" / "ambition_sprite2d_renderer"))

import numpy as np  # noqa: E402
import yaml  # noqa: E402
from PIL import Image  # noqa: E402

sys.path.insert(0, str(REPO / "scripts"))
from measure_published_flipbooks import _sheet_rows  # noqa: E402

from ambition_sprite2d_renderer.authoring.part_flipbook import (  # noqa: E402
    PartFlipbook,
    largest_wrong_blob,
    parity,
    tween_draws,
)

SPRITES = REPO / "crates" / "ambition_platformer2d_actor_monolith" / "assets" / "sprites"

PARITY_BOUND = 0.01
#: D6's bound. It was 10 while the impostor blended in linear light (a turned
#: part's outline came out a shade apart along a line: 9 pixels on Mary-O).
#: Blended in gamma space as the baked frame was (`ART_COMPOSITING`), measured
#: on llvmpipe 2026-10-03: Mary-O's three forms at most 6 in either facing and
#: either anchor; robot v3's 1,888 frames at most 1. Dropping any one VISIBLE
#: draw from any Mary-O frame makes a blob of 12 or more.
BLOB_BOUND = 6
#: ⭐ THE ORACLE OBEYS THE RUNTIME'S LAW. The game draws parts in linear light:
#: sRGB pages decoded before bilinear filtering, blended in linear light
#: (`WORLD_COMPOSITING`, and the atlas with it). The published art was
#: composited in gamma space, and PIL replays it that way, so against it every
#: thin line and anti-aliased outline drifts a shade (robot: blobs to 21 px)
#: and the gate measured the law, not the draw. So the published DRAWS are
#: replayed by `_runtime_oracle`, a small model of the GPU's sprite road, and
#: the gate is exact again. `AMBITION_PARITY_COMPOSITING=srgb` captures in the
#: art's gamma law and keeps the PIL replay (the legacy drift).
RUNTIME_LINEAR = os.environ.get("AMBITION_PARITY_COMPOSITING") != "srgb"
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
    parser.add_argument(
        "--composed",
        action="store_true",
        help="composite every body through the impostor (AMBITION_PART_PRESENTATION=impostor); "
        "by default the parts draw directly, as the game draws a body nothing reads as one image",
    )
    parser.add_argument("--out", type=Path, default=REPO / "target" / "rig_parity")
    parser.add_argument("--no-build", action="store_true", help="reuse the last captures in --out")
    parser.add_argument("--jobs", type=int, default=min(4, os.cpu_count() or 1), help="processes scoring the captures")
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
        env = {**os.environ, "AMBITION_PART_PRESENTATION": "impostor" if args.composed else "direct"}
        run = subprocess.run(command, cwd=REPO, capture_output=True, text=True, env=env)
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

    # ⛔ Every target's inputs are read HERE before a capture is scored: read
    # lazily in the workers, one sheet without a block the reader expected
    # threw away 90 minutes of scoring (2026-10-04).
    unreadable = []
    for target in sorted({row["target"] for row in rows}):
        try:
            _flipbook(target)
            _sheet(target)
        except Exception as error:  # noqa: BLE001 - reported, then refused
            unreadable.append(f"{target}: {type(error).__name__}: {error}")
    if unreadable:
        print("inputs the scorer cannot read:\n  " + "\n  ".join(unreadable), file=sys.stderr)
        return 2

    worst = defaultdict(lambda: [0.0, 0, 0.0, 0])
    failures = []
    distinct = defaultdict(set)
    clipped = defaultdict(list)
    soft = defaultdict(list)
    # Each capture is scored on its own, so the captures are spread over
    # processes (`--jobs`); a large character's 5,000 captures took over an
    # hour in one.
    with ProcessPoolExecutor(max_workers=args.jobs) as pool:
        scored = list(pool.map(_score, rows, [args.phase] * len(rows), chunksize=8))
    for row, result in zip(rows, scored):
        target, name, frame = row["target"], row["row"], int(row["frame"])
        flip = row["flip"] == "true"
        # ⛔ Premise: the harness drew something, and it drew the PINNED frame.
        # An empty capture agrees with an empty capture.
        if result["empty"]:
            print(f"{target} {name}[{frame}]: an empty capture", file=sys.stderr)
            return 2
        distinct[target].add(result["digest"])
        wrong, blob = result["wrong"], result["blob"]
        in_between = result["in_between"]
        if result["lost"] > 0 and not in_between and f"{name}#{frame}" not in clipped[target]:
            clipped[target].append(f"{name}#{frame}")
        w = worst[(target, name)]
        worst[(target, name)] = [
            max(w[0], wrong), max(w[1], blob), max(w[2], result["baked_wrong"]), max(w[3], result["baked_blob"])
        ]
        inside = wrong <= PARITY_BOUND and blob <= BLOB_BOUND
        # ⚠ A frame the game draws soft on BOTH roads passes by the second
        # measure, and is counted: director's `punch`[1] draws its parts and
        # its baked frame as the same picture (0 wrong pixels), both softer
        # than the crisp oracle (a blob of 8, 2026-10-03).
        like_baked = result["vs_baked_wrong"] <= PARITY_BOUND and result["vs_baked_blob"] <= BLOB_BOUND
        if not in_between and not inside:
            if like_baked:
                soft[target].append(f"{name}#{frame}{'~flip' if flip else ''}")
            else:
                failures.append(f"{target} {name}[{frame}] flip={flip}: {wrong:.4f}, blob {blob}")
    flipbooks = {target: _flipbook(target) for target in targets}
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
    for target, frames in soft.items():
        print(
            f"  {target}: {len(frames)} frame(s) outside the bounds against the published draws but inside "
            f"them against the game's own baked draw: {', '.join(frames)}"
        )
    for target, frames in clipped.items():
        print(f"  {target}: the baked frame cuts off art the parts draw, on {len(frames)} frame(s): {', '.join(frames)}")
    if failures:
        print(f"{len(failures)} frame(s) outside A <= {PARITY_BOUND:.0%} and B <= {BLOB_BOUND} px:")
        for line in failures:
            print("  " + line)
        return 1
    print(f"ok: every part-drawn frame inside A <= {PARITY_BOUND:.0%} and B <= {BLOB_BOUND} px of the published draws, or of the game's own baked draw (listed above)")
    return 0


_FLIPBOOKS: dict = {}
_SHEETS: dict = {}


def _flipbook(target):
    """The published flipbook (the placement is the file's: snapped for a rig
    painted at frame resolution, continuous for a supersampled one)."""
    if target not in _FLIPBOOKS:
        _FLIPBOOKS[target] = PartFlipbook.from_published(SPRITES / f"{target}_parts.ron")
    return _FLIPBOOKS[target]


def _score(row, phase):
    """One capture, measured: the game's PART draw against the published draws
    (the gate), and the game's BAKED draw against the published frame (for the
    record)."""
    target, name, frame = row["target"], row["row"], int(row["frame"])
    flip = row["flip"] == "true"
    feet = (float(row["feet_x"]), float(row["feet_y"]))
    root_x = float(row["root_x"])
    flipbook = _flipbook(target)
    if target not in _SHEETS:
        _SHEETS[target] = _sheet(target)
    baked = _unpremultiplied(Image.open(row["baked"]).convert("RGBA"))
    parts = _unpremultiplied(Image.open(row["parts"]).convert("RGBA"))
    if baked.getchannel("A").getbbox() is None or parts.getchannel("A").getbbox() is None:
        return {"empty": True}
    # The oracle for the GAME's part draw is the PUBLISHED draws drawn the
    # baked road's way (PIL), unclipped. The offline gate already proves those
    # draws are the baked frame inside the frame. At a phase, the oracle is the
    # published tween rule (`tween_draws`); a clip that steps is its frame.
    oracle = Image.new("RGBA", parts.size, (0, 0, 0, 0))
    draws = tween_draws(flipbook, name, frame, phase)
    # The frame's top left is where the game put it (the anchor's feet at
    # `feet`); the flipbook draws from its own feet within the frame.
    feet_frame = _feet_in_frame(row, parts.size, flipbook.frame_size)
    at = (feet[0] - feet_frame[0] + flipbook.feet[0], feet[1] - feet_frame[1] + flipbook.feet[1])
    if RUNTIME_LINEAR:
        oracle = _runtime_oracle(flipbook, draws, parts.size, at, flip, root_x, flipbook.opacity_of(name, frame))
    else:
        flipbook.draw_frame(oracle, name, frame, at, flip, draws=draws, mirror_x=root_x)
    # ⛔ Clipped to the body's impostor cell (mirrored with it): the part road
    # cannot draw past it, and the published frame never reached there either.
    # Unclipped, a banner past the cell of a body facing left read as 1153
    # wrong pixels of a correct draw (oni leader, 2026-10-03).
    if "cell_x0" in row:
        cell = [round(float(row[key])) for key in ("cell_x0", "cell_y0", "cell_x1", "cell_y1")]
        inside = Image.new("L", oracle.size, 0)
        inside.paste(255, (max(0, cell[0]), max(0, cell[1]), min(oracle.width, cell[2]), min(oracle.height, cell[3])))
        oracle = Image.composite(oracle, Image.new("RGBA", oracle.size, (0, 0, 0, 0)), inside)
    published = _published_frame(_SHEETS[target], feet_frame, flipbook, name, frame, parts.size, feet, flip, root_x)
    # ⚠ AN IN-BETWEEN IS REPORTED, NOT GATED. PIL rounds a tweened part to
    # whole pixels; the GPU draws it between pixels and does not. The raster
    # difference is that rounding (1-2%, blobs to 39 on Mary-O, 2026-10-02).
    # The tween's PLACES are gated where they are exact: the renderer's
    # `test_a_tweened_clip_places_each_part_where_the_in_between_pose_does` and
    # the runtime's `a_tweened_clip_draws_between_its_frames`.
    return {
        "empty": False,
        "digest": hash(parts.tobytes()),
        "wrong": parity(oracle, parts),
        "blob": largest_wrong_blob(oracle, parts),
        "baked_wrong": parity(published, baked),
        "baked_blob": largest_wrong_blob(published, baked),
        # The game's part draw against the game's BAKED draw: the two roads a
        # player could see side by side.
        "vs_baked_wrong": parity(baked, parts),
        "vs_baked_blob": largest_wrong_blob(baked, parts),
        # Art the frame cut off: pixels the draws plainly cover (not the faint
        # fringe a continuous part's resampling adds) where the frame has none.
        "lost": int(((np.asarray(oracle)[..., 3] > 64) & (np.asarray(published)[..., 3] == 0)).sum()),
        "in_between": phase > 0.0 and bool(flipbook.tweens.get(name)),
    }


def _unpremultiplied(image):
    """A capture as straight alpha.

    ⛔ A CAMERA THAT CLEARS TO TRANSPARENT WRITES PREMULTIPLIED COLOUR. Sprites
    blend `src * a + dst * (1 - a)`, so over a transparent clear a half-covered
    pixel stores half its colour. Compared as if it were straight alpha, every
    edge and every translucent effect reads too dark, and the measure goes
    blind to exactly the pixels a premultiplication bug changes."""
    array = np.asarray(image, dtype=np.float64)
    alpha = array[..., 3:4]
    if not RUNTIME_LINEAR:
        # Blended in gamma: the target stores the sRGB colour times alpha.
        rgb = np.where(alpha > 0, np.clip(array[..., :3] * 255.0 / np.maximum(alpha, 1.0), 0, 255), 0)
    else:
        # ⛔ Blended in linear light: the sRGB target stores the LINEAR colour
        # times alpha, encoded. Divided as gamma, every half-covered pixel of
        # the outer silhouette read a shade off (robot outlines, blobs of 11).
        linear = _decode(array[..., :3] / 255.0)
        rgb = np.where(alpha > 0, np.clip(_encode(linear / np.maximum(alpha / 255.0, 1.0 / 255.0)), 0, 1) * 255.0, 0)
    return Image.fromarray(np.concatenate([rgb, alpha], axis=-1).round().astype(np.uint8), "RGBA")


def _runtime_oracle(flipbook, draws, size, feet_at, flip, mirror_x, opacity):
    """The draws as the game's sprite road draws them, straight alpha.

    A model of the GPU, independent of the runtime's code: each part is a quad
    over its rect, covering the pixels whose centres fall inside it; its sRGB
    texels are decoded, then sampled bilinearly at those centres (straight
    colour and alpha, clamped to the rect), tinted by the draw's colour (an sRGB
    colour, decoded) and faded by its opacity; each is blended over the last in
    linear light, premultiplied, over a transparent clear. A frame that fades
    as one picture fades the composite. Mirrored about `mirror_x` as
    `PartFlipbook.draw_frame` mirrors."""
    from scipy.ndimage import map_coordinates

    width, height = size
    axis = feet_at[0] if mirror_x is None else mirror_x
    fx = feet_at[0] if not flip else width - (2.0 * axis - feet_at[0])
    fy = feet_at[1]
    ys, xs = np.mgrid[0:height, 0:width].astype(np.float64) + 0.5
    colour = np.zeros((height, width, 3))
    alpha = np.zeros((height, width))
    for draw in draws:
        texels = np.asarray(flipbook.part_image(draw.part).convert("RGBA"), dtype=np.float64) / 255.0
        texel_rgb = _decode(texels[..., :3]) * _decode(np.asarray(draw.tint, dtype=np.float64))
        texel_a = texels[..., 3]
        ph, pw = texel_a.shape
        pivot = flipbook.parts[draw.part].pivot
        ax, ay = draw.at[0] + fx, draw.at[1] + fy
        c, s = math.cos(draw.rotation), math.sin(draw.rotation)
        qx, qy = xs - ax, ys - ay
        u = pivot[0] + (c * qx + s * qy) / draw.scale[0]
        v = pivot[1] + (-s * qx + c * qy) / draw.scale[1]
        inside = (u >= 0.0) & (u < pw) & (v >= 0.0) & (v < ph)
        if not inside.any():
            continue
        coords = [np.clip(v - 0.5, 0, ph - 1), np.clip(u - 0.5, 0, pw - 1)]
        a = map_coordinates(texel_a, coords, order=1, mode="nearest") * draw.opacity * inside
        rgb = np.stack([map_coordinates(texel_rgb[..., k], coords, order=1, mode="nearest") for k in range(3)], axis=-1)
        colour = rgb * a[..., None] + colour * (1.0 - a[..., None])
        alpha = a + alpha * (1.0 - a)
    colour *= opacity
    alpha *= opacity
    if flip:
        colour, alpha = colour[:, ::-1], alpha[:, ::-1]
    straight = np.where(alpha[..., None] > 0, _encode(colour / np.maximum(alpha[..., None], 1e-9)), 0.0)
    rgba = np.concatenate([np.clip(straight, 0, 1), alpha[..., None]], axis=-1)
    return Image.fromarray((rgba * 255.0).round().astype(np.uint8), "RGBA")


def _decode(srgb):
    return np.where(srgb <= 0.04045, srgb / 12.92, np.power((srgb + 0.055) / 1.055, 2.4))


def _encode(linear):
    linear = np.clip(linear, 0.0, None)
    return np.where(linear <= 0.0031308, linear * 12.92, 1.055 * np.power(linear, 1 / 2.4) - 0.055)


def _sheet(target):
    sheet = yaml.safe_load((SPRITES / f"{target}_spritesheet.yaml").read_text())
    pages = [Image.open(SPRITES / name).convert("RGBA") for name in (sheet.get("images") or [sheet.get("image", f"{target}_spritesheet.png")])]
    # ⛔ Where the GAME puts the frame: by the sheet's feet ANCHOR, which is
    # not always its `feet_pixel` (director, officer and medic disagree by 8 to
    # 26 px, 2026-10-03). The harness root is built at the anchor, so the
    # oracle is placed by it too.
    return {row["animation"]: row for row in _sheet_rows(sheet)}, pages


def _feet_in_frame(row, image_size, frame_size):
    """Where the game put the feet within the frame: the capture's feet pixel
    less the frame's top left in the image.

    ⛔ ASKED OF THE CAPTURE, NOT THE SHEET. The game places the frame by its
    feet ANCHOR, which is not always the sheet's `feet_pixel` (director,
    officer and medic disagree by 8 to 26 px), falls back to the bottom centre
    when a sheet has no `body_metrics` (weird_hermit), and can be overridden
    in code (`feet_anchor_y_override`). Read off the YAML, the oracle restated
    the first rule, missed the other two, and a sheet without the block
    crashed a 90-minute scoring run (2026-10-04). The capture writes the
    frame's top left (`frame_x0`, `frame_y0`); an older index is read by the
    capture's own rule, the frame centred and floored in the image.
    """
    if "frame_x0" in row:
        x0, y0 = float(row["frame_x0"]), float(row["frame_y0"])
    else:
        x0, y0 = ((image_size[0] - frame_size[0]) // 2, (image_size[1] - frame_size[1]) // 2)
    return float(row["feet_x"]) - x0, float(row["feet_y"]) - y0


def _published_frame(sheet, feet_frame, flipbook, name, index, size, feet, flip, root_x):
    rows, pages = sheet
    rect = rows[name]["rects"][index]
    # A paged sheet names each frame's page `fpage`.
    atlas = pages[int(rect.get("fpage", rows[name].get("page", 0)))]
    frame = Image.new("RGBA", flipbook.frame_size, (0, 0, 0, 0))
    # A sparse sheet trims each frame and states its `off`; a grid sheet's
    # rect is the whole frame.
    frame.paste(atlas.crop((rect["x"], rect["y"], rect["x"] + rect["w"], rect["y"] + rect["h"])), tuple(rect.get("off", (0, 0))))
    canvas = Image.new("RGBA", size, (0, 0, 0, 0))
    # Mirrored about the root's column, as `PartFlipbook.draw_frame` mirrors.
    fx = feet[0] if not flip else size[0] - (2.0 * root_x - feet[0])
    canvas.alpha_composite(frame, (round(fx - feet_frame[0]), round(feet[1] - feet_frame[1])))
    return canvas.transpose(Image.FLIP_LEFT_RIGHT) if flip else canvas


if __name__ == "__main__":
    raise SystemExit(main())
