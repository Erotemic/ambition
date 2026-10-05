#!/usr/bin/env python3
"""How much a part-drawn body changes when it switches between drawn directly
and composited, and how far each road drifts from its baked sheet.

A body is drawn directly in the world until something reads it as one image
(a hit flash, a portal, a fade); then it is composited into an impostor cell
for as long as that lasts (`docs/planning/engine/semantic-part-rendering-and-ragdolls.md`).
The same frame on the two roads should look the same, or every hit makes the
outline pop. The world cameras blend in linear light; the impostor's compositing
law is a knob (`AMBITION_IMPOSTOR_COMPOSITING=srgb|linear`). For each law and
each drawn scale this captures every frame of each target on both roads
(`measure_rigged_parity.py`), then compares them:

* SWITCH: direct vs composited, the same frame. The road switch the player sees.
* DRIFT: each road vs the game's own baked sheet drawn at that scale.

Each comparison reports the parity gate's measures (`parity`, `largest_wrong_blob`:
a pixel is wrong past 64 levels, forgiving a one-pixel shift) and a STRICT one: the
pixels, composited over mid grey, that differ by more than 8 levels, and the
largest difference. The gate forgives what a strict eye may still see pop.

    python3 scripts/measure_composition_switch.py --target robot --scale 1 --scale 0.5
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "tools" / "ambition_sprite2d_renderer"))

import numpy as np  # noqa: E402
from PIL import Image  # noqa: E402

from ambition_sprite2d_renderer.authoring.part_flipbook import largest_wrong_blob, parity  # noqa: E402

STRICT_LEVELS = 8


def over_grey(image: Image.Image) -> np.ndarray:
    rgba = np.asarray(image.convert("RGBA")).astype(np.float64)
    alpha = rgba[..., 3:4] / 255.0
    return rgba[..., :3] * alpha + 128.0 * (1.0 - alpha)


def strict(a: Image.Image, b: Image.Image) -> tuple[int, float]:
    diff = np.abs(over_grey(a) - over_grey(b)).max(axis=-1)
    return int((diff > STRICT_LEVELS).sum()), float(diff.max())


def capture(law: str, composed: bool, scale: float, targets: list[str], out: Path, jobs: int) -> None:
    command = [sys.executable, str(REPO / "scripts" / "measure_rigged_parity.py"), "--scale", str(scale), "--out", str(out), "--jobs", str(jobs)]
    for target in targets:
        command += ["--target", target]
    if composed:
        command.append("--composed")
    env = {**os.environ, "AMBITION_IMPOSTOR_COMPOSITING": law}
    run = subprocess.run(command, cwd=REPO, capture_output=True, text=True, env=env)
    # 1 is the parity gate's verdict, which this measures rather than obeys.
    # Off one texel per pixel the gate has no oracle and refuses to score, after
    # writing the captures this compares.
    off_scale = scale != 1.0 and "drawn texel per pixel" in run.stderr
    if run.returncode not in (0, 1) and not off_scale:
        print(run.stdout + run.stderr, file=sys.stderr)
        raise SystemExit(f"capture failed: {law} composed={composed} scale={scale}")


def frames(out: Path) -> dict[tuple[str, str], Path]:
    return {
        (target.name, path.name[: -len("_parts.png")]): path
        for target in sorted(p for p in out.iterdir() if p.is_dir())
        for path in sorted(target.glob("*_parts.png"))
    }


def summarize(label: str, rows: list[tuple[float, int, int, float]]) -> str:
    wrong = sorted(r[0] for r in rows)
    blobs = sorted(r[1] for r in rows)
    counts = sorted(r[2] for r in rows)
    peaks = sorted(r[3] for r in rows)
    n = len(rows)
    return (
        f"{label:<34} frames {n:4}  parity med {100 * wrong[n // 2]:.2f}% max {100 * wrong[-1]:.2f}%"
        f"  blob med {blobs[n // 2]} max {blobs[-1]}"
        f"  strict>{STRICT_LEVELS} med {counts[n // 2]} max {counts[-1]}  peak {peaks[-1]:.0f}"
    )


def compare(a: dict, b: dict) -> list[tuple[float, int, int, float]]:
    keys = sorted(set(a) & set(b))
    if not keys:
        raise SystemExit("no frame captured on both roads; nothing compared")
    rows = []
    for key in keys:
        x, y = Image.open(a[key]).convert("RGBA"), Image.open(b[key]).convert("RGBA")
        rows.append((parity(x, y), largest_wrong_blob(x, y), *strict(x, y)))
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--target", action="append", required=True)
    parser.add_argument("--scale", action="append", type=float, required=True)
    parser.add_argument("--law", action="append", choices=["srgb", "linear"])
    parser.add_argument("--out", type=Path, default=REPO / "target" / "composition_switch")
    parser.add_argument("--jobs", type=int, default=3)
    parser.add_argument("--no-capture", action="store_true", help="reuse the captures in --out")
    args = parser.parse_args()
    laws = args.law or ["srgb", "linear"]
    for scale in args.scale:
        for law in laws:
            outs = {}
            for composed in (False, True):
                out = args.out / f"{law}_{'composed' if composed else 'direct'}_{scale}"
                if not args.no_capture:
                    capture(law, composed, scale, args.target, out, args.jobs)
                outs[composed] = out
            direct, composed = frames(outs[False]), frames(outs[True])
            baked = {
                key: path.with_name(path.name.replace("_parts.png", "_baked.png"))
                for key, path in direct.items()
                if path.with_name(path.name.replace("_parts.png", "_baked.png")).exists()
            }
            print(f"== impostor law {law}, scale {scale}")
            print(summarize("SWITCH direct vs composited", compare(direct, composed)))
            print(summarize("DRIFT  direct vs baked", compare(baked, direct)))
            print(summarize("DRIFT  composited vs baked", compare(baked, composed)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
