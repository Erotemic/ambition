#!/usr/bin/env python3
"""Measure how seamless portal view windows are, through the real render stack.

A window must show what a camera on the far side shows: the same world, the
same backdrop, the same scale and light, with no black, no seam and no
stretch. For each case this films two shots with `capture_scene`:

- `window`: the room as the player sees it, with the window of one portal
  open (`--seam PORTAL:window`). The tool writes the window's polygon in
  pixels and the affine map from each pixel to the far shot.
- `far`: no windows, and the camera where the view through the portal is
  (`--seam PORTAL:far`). This is what the window must show.

Each pixel of the window (less a band at its edge and at the aperture, where
the portal's own rim and line of light are drawn over it) is compared with
the pixel of the far shot that shows the same place. A pixel differs when a
channel differs by more than `DIFFERS`. A case fails when more than
`MAX_DIFFERING` of its pixels differ, or when the window did not open (a
check that compares nothing proves nothing). Both shots hide the parallax
layer in front of the play: it is between the eye and the portal, and a
window does not show it.

    python3 scripts/check_portal_window_seams.py [--build] [--tiers low,high] [--cases A,D] [--out DIR] [--jobs 3]

`--build` builds the tool first (`cargo build -p ambition_app_tools --bin
capture_scene`). It runs under `xvfb-run` when there is no display. Writes
`DIR/report.json` and, for each case, the two shots and a picture of the
differing pixels (red). The default test plan (`scripts/run_tests.py`) runs
two cases at two tiers.

Measured 2026-10-25, 640x360: a seamless window differs in at most 0.02% of
its pixels. The defects this guards measured: a window with no backdrop
(80-99%), a window captured at a quarter of the view's density (0.75-3.0%,
blur), windows of other portals drawn into a capture for the wrong eye
(2.9%), a capture sized for another view than the camera's (3.4%).
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CAPTURE = REPO / "target" / "debug" / "capture_scene"

# A channel differing by more than this is a differing pixel.
DIFFERS = 12
# The most of a window's pixels that may differ.
MAX_DIFFERING = 0.005
# Pixels left out at the edge of the window (filtering of its polygon), and
# around the aperture (the portal's rim and line of light are drawn on it).
EDGE_PX = 3
RIM_PX = 18


@dataclass(frozen=True)
class Case:
    name: str
    what: str
    room: str
    focus: tuple[float, float]
    player: tuple[float, float]
    # The portal whose window is checked: the world point nearest to it.
    portal: str


# Each kind of pair: the player stands in front of one end, so its window is
# open over the far side. The player is not in the window.
CASES = [
    Case("A", "floor <-> floor (a turn round)", "portal_lab", (330, 860), (215, 876), "300,900"),
    Case("B", "wall <-> wall, one wall", "portal_lab", (1150, 800), (1250, 876), "1190,850"),
    Case("C", "floor <-> ceiling (straight through)", "portal_lab", (900, 860), (790, 876), "900,900"),
    Case("D", "floor <-> wall (a quarter turn)", "portal_lab", (1450, 860), (1340, 876), "1450,900"),
    Case("gnu_floor", "floor <-> wall, GNU-ton's hall", "gnu_ton_conservation_hall", (420, 800), (300, 872), "400,896"),
]
TIERS = ("potato", "low", "medium", "high")
WARMUP = 90


def run_side(case: Case, tier: str, side: str, folder: Path) -> tuple[int, Path]:
    shot = folder / f"{side}.png"
    cmd = [
        str(CAPTURE), case.room, f"{case.focus[0]},{case.focus[1]}", str(shot), "640x360",
        "--player-at", f"{case.player[0]},{case.player[1]}@30",
        "--warmup", str(WARMUP), "--tick-per-frame",
        "--seam", f"{case.portal}:{side}",
    ]
    if not os.environ.get("DISPLAY") and shutil.which("xvfb-run"):
        cmd = ["xvfb-run", "-a", *cmd]
    env = dict(os.environ, AMBITION_QUALITY_PROFILE=tier)
    with open(folder / f"{side}.log", "w") as log:
        code = subprocess.run(cmd, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=900).returncode
    return code, shot


def compare(folder: Path) -> dict:
    import numpy as np
    from PIL import Image, ImageDraw, ImageFilter

    seam = json.loads((folder / "window.seam.json").read_text())
    near = np.asarray(Image.open(folder / "window.png").convert("RGB")).astype(np.int16)
    far = np.asarray(Image.open(folder / "far.png").convert("RGB")).astype(np.int16)
    h, w = near.shape[:2]
    poly = seam["window"]
    area = 0.5 * abs(sum(poly[i][0] * poly[i - 1][1] - poly[i - 1][0] * poly[i][1] for i in range(len(poly))))
    if area < 1.0:
        return {"pixels": 0, "differing": 1.0, "why": "the window did not open"}
    mask = Image.new("L", (w, h), 0)
    draw = ImageDraw.Draw(mask)
    draw.polygon([tuple(p) for p in seam["window"]], fill=255)
    mask = mask.filter(ImageFilter.MinFilter(2 * EDGE_PX + 1))
    draw = ImageDraw.Draw(mask)
    draw.line([tuple(p) for p in seam["aperture"]], fill=0, width=2 * RIM_PX)
    inside = np.asarray(mask) > 0
    ys, xs = np.nonzero(inside)
    a, b, c, d, e, f = seam["to_far"]
    u = xs + 0.5
    v = ys + 0.5
    fx = np.floor(a * u + b * v + c).astype(int)
    fy = np.floor(d * u + e * v + f).astype(int)
    seen = (fx >= 0) & (fx < w) & (fy >= 0) & (fy < h)
    xs, ys, fx, fy = xs[seen], ys[seen], fx[seen], fy[seen]
    if xs.size == 0:
        return {"pixels": 0, "differing": 1.0, "why": "no pixel of the window is in the far shot"}
    diff = np.abs(near[ys, xs] - far[fy, fx]).max(axis=1)
    bad = diff > DIFFERS
    picture = (near // 2).astype(np.uint8)
    picture[ys[bad], xs[bad]] = (255, 0, 0)
    Image.fromarray(picture).save(folder / "differs.png")
    return {
        "pixels": int(xs.size),
        "differing": float(bad.mean()),
        "mean_difference": float(diff.mean()),
    }


def run_case(case: Case, tier: str, out: Path) -> dict:
    folder = out / f"{case.name}_{tier}"
    folder.mkdir(parents=True, exist_ok=True)
    result = {"case": case.name, "what": case.what, "tier": tier, "folder": str(folder)}
    for side in ("window", "far"):
        code, _ = run_side(case, tier, side, folder)
        if code != 0:
            return {**result, "ok": False, "why": f"capture_scene {side} exited {code} ({folder / side}.log)"}
    measured = compare(folder)
    ok = measured["pixels"] > 0 and measured["differing"] <= MAX_DIFFERING
    return {**result, **measured, "ok": ok}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--tiers", default=",".join(TIERS))
    parser.add_argument("--cases", default=",".join(c.name for c in CASES))
    parser.add_argument("--out", default=str(REPO / "target" / "portal-window-seams"))
    parser.add_argument("--jobs", type=int, default=3, help="cases run at once; each runs two renders")
    parser.add_argument("--build", action="store_true", help="build capture_scene first")
    args = parser.parse_args()
    if args.build:
        built = subprocess.run(["cargo", "build", "-j", "6", "-p", "ambition_app_tools", "--bin", "capture_scene"], cwd=REPO)
        if built.returncode != 0:
            return built.returncode
    if not CAPTURE.exists():
        print(f"{CAPTURE} is missing: cargo build -p ambition_app_tools --bin capture_scene", file=sys.stderr)
        return 2
    wanted = set(args.cases.split(","))
    cases = [c for c in CASES if c.name in wanted]
    tiers = args.tiers.split(",")
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    jobs = [(case, tier) for case in cases for tier in tiers]
    with ThreadPoolExecutor(max_workers=max(1, min(args.jobs, 6))) as pool:
        results = list(pool.map(lambda job: run_case(job[0], job[1], out), jobs))
    (out / "report.json").write_text(json.dumps(results, indent=2))
    failed = 0
    for r in results:
        mark = "ok  " if r["ok"] else "FAIL"
        detail = r.get("why") or f"{r['differing'] * 100:5.2f}% of {r['pixels']} px differ (mean {r['mean_difference']:.1f})"
        print(f"{mark} {r['case']:>8} {r['tier']:<6} {r['what']}: {detail}")
        failed += not r["ok"]
    print(f"{len(results) - failed}/{len(results)} windows seamless; limit {MAX_DIFFERING * 100:.1f}% of pixels "
          f"differing by more than {DIFFERS}. Report: {out / 'report.json'}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
