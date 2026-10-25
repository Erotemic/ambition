#!/usr/bin/env python3
"""Measure that what goes through a portal meets up at its far line, every frame.

Jon, 2026-10-25, of GNU-ton's forearm at the floor line: "I see a
misalignment in the arms ... That is a serious problem." A body half through
a pair is drawn in two parts at the far line: the part still in front of the
near line, seen in that line's window, and the part past it, carried out of
the far line. The two must meet. On Low the window was captured only every
100 ms (250 at Potato) while the carry is drawn each frame, so a moving body
met an older picture of itself: the arm stepped by up to 17 px at the line.

For each case this films the same ticks twice with `capture_scene`, one sim
tick a frame:

- `view`: as the player sees it, the window of PORTAL open
  (`--seam PORTAL:window`). The tool writes the window's polygon and the
  affine map from each of its pixels to the `truth` shot.
- `bare`: the same, with the carry off: what is in front of the far line
  when nothing is carried out of it.
- `truth`: the camera where the view through PORTAL is, no windows, and the
  carry off (`--seam PORTAL:far --carry off`): the world drawn whole, as it
  is past the near line. Both halves of the crossing are in it, joined.

Each frame, two bands of the view next to the far line are compared with
the truth through that map, each at its best shift of up to `SEARCH` px:
the band of the window (behind the line), and the carried part in the band
in front of it (the pixels where the view differs from `bare`: the room in
front of the far line is not the room past the near line). A case fails
when, in any frame, either's best shift is not (0, 0) within `SLACK` px:
the two halves do not meet. The error at no shift is reported, not judged
(sparks and dust drawn over the band are in neither picture).

    python3 scripts/check_portal_seam_over_time.py [--build] [--tiers low,potato] [--out DIR]

Runs under `xvfb-run` when there is no display; needs numpy and PIL.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CAPTURE = REPO / "target" / "debug" / "capture_scene"

SIZE = (1280, 720)
# Pixels next to the far line in each band (the line of light and its glow
# are drawn over the first few, and are left out).
SKIP_PX = 8
BAND_PX = 14
# The search for the best shift, and how far from no shift it may be.
SEARCH = 6
SLACK = 1
# A channel differing by more than this from `bare` is a carried pixel; a
# frame with fewer carried pixels than `MIN_CARRIED` carries nothing there.
CARRIED_DIFFERS = 24
MIN_CARRIED = 40


@dataclass(frozen=True)
class Case:
    name: str
    what: str
    room: str
    focus: tuple[float, float]
    player: tuple[float, float]
    # The far line: its channel.
    portal: str
    first: int
    frames: int


CASES = [
    Case("gnu_hammer", "GNU-ton's forearm at the floor line while the fist is stuck",
         "gnu_ton_arena", (900, 900), (400, 1200), "c129", 704, 24),
]
TIERS = ("low", "potato")


def film(case: Case, tier: str, folder: Path, side: str) -> tuple[int, Path]:
    out = folder / side
    out.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(CAPTURE), case.room, f"{case.focus[0]},{case.focus[1]}", str(out / "f.png"), f"{SIZE[0]}x{SIZE[1]}",
        "--player-at", f"{case.player[0]},{case.player[1]}@1",
        "--warmup", str(case.first), "--frames", str(case.frames), "--stride", "1", "--tick-per-frame",
        "--camera-zoom", "debug",
    ]
    if side == "view":
        cmd += ["--seam", f"{case.portal}:window", "--carry", "on"]
    elif side == "bare":
        cmd += ["--seam", f"{case.portal}:window", "--carry", "off"]
    else:
        cmd += ["--seam", f"{case.portal}:far", "--carry", "off"]
    if not os.environ.get("DISPLAY") and shutil.which("xvfb-run"):
        cmd = ["xvfb-run", "-a", *cmd]
    env = dict(os.environ, AMBITION_QUALITY_PROFILE=tier)
    with open(folder / f"{side}.log", "w") as log:
        code = subprocess.run(cmd, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=3600).returncode
    return code, out


def shots(log: Path) -> dict[int, int]:
    return {int(t): int(n) for n, t in re.findall(r"capture_scene: shot (\d+) at sim tick (\d+)", log.read_text(errors="replace"))}


def bands(seam: dict):
    """The two bands beside the far line, as pixel boxes (x0, y0, x1, y1):
    the window's (behind the line) and the carried part's (in front)."""
    (ax, ay), (bx, by) = seam["aperture"]
    poly = seam["window"]
    # The window is on the side of the line its polygon is.
    wy = sum(p[1] for p in poly) / len(poly)
    wx = sum(p[0] for p in poly) / len(poly)
    x0, x1 = sorted((ax, bx))
    y0, y1 = sorted((ay, by))
    if abs(by - ay) <= abs(bx - ax):
        # A level line.
        y = (ay + by) * 0.5
        behind = -1 if wy < y else 1
        win = (int(x0) + 4, int(y + behind * (SKIP_PX + BAND_PX)), int(x1) - 4, int(y + behind * SKIP_PX))
        car = (int(x0) + 4, int(y - behind * SKIP_PX), int(x1) - 4, int(y - behind * (SKIP_PX + BAND_PX)))
    else:
        x = (ax + bx) * 0.5
        behind = -1 if wx < x else 1
        win = (int(x + behind * (SKIP_PX + BAND_PX)), int(y0) + 4, int(x + behind * SKIP_PX), int(y1) - 4)
        car = (int(x - behind * SKIP_PX), int(y0) + 4, int(x - behind * (SKIP_PX + BAND_PX)), int(y1) - 4)
    norm = lambda b: (min(b[0], b[2]), min(b[1], b[3]), max(b[0], b[2]), max(b[1], b[3]))
    return norm(win), norm(car)


def measure(case: Case, folder: Path) -> dict:
    import numpy as np
    from PIL import Image

    seam_files = sorted((folder / "view").glob("*.seam.json"))
    if not seam_files:
        return {"ok": False, "why": "the view wrote no seam (did the window open?)"}
    view_shots, bare_shots, truth_shots = shots(folder / "view.log"), shots(folder / "bare.log"), shots(folder / "truth.log")
    ticks = sorted(set(view_shots) & set(bare_shots) & set(truth_shots))
    frames = []
    for tick in ticks:
        vi, ti = view_shots[tick], truth_shots[tick]
        seam_path = folder / "view" / f"f.{vi:04d}.seam.json"
        seam = json.loads((seam_path if seam_path.exists() else seam_files[0]).read_text())
        a, b, c, d, e, f = seam["to_far"]
        if max(abs(a - 1), abs(b), abs(d), abs(e - 1)) > 1e-2:
            return {"ok": False, "why": f"the map to the truth shot is not a translation ({seam['to_far']}): not measured"}
        load = lambda side, i: np.asarray(Image.open(folder / side / f"f.{i:04d}.png").convert("RGB")).astype(float)
        view, bare, truth = load("view", vi), load("bare", bare_shots[tick]), load("truth", ti)
        row = {"tick": tick}
        for name, (x0, y0, x1, y1) in zip(("window", "carried"), bands(seam)):
            ox, oy = int(round(c)), int(round(f))
            part = view[y0:y1, x0:x1]
            if name == "carried":
                mask = np.abs(part - bare[y0:y1, x0:x1]).max(axis=2) > CARRIED_DIFFERS
                if mask.sum() < MIN_CARRIED:
                    continue
            else:
                mask = np.ones(part.shape[:2], dtype=bool)
            best = None
            for dy in range(-SEARCH, SEARCH + 1):
                for dx in range(-SEARCH, SEARCH + 1):
                    t = truth[y0 + oy + dy:y1 + oy + dy, x0 + ox + dx:x1 + ox + dx]
                    if t.shape != part.shape:
                        continue
                    err = float(np.abs(part - t)[mask].mean())
                    if best is None or err < best[0]:
                        best = (err, dx, dy)
            at_zero = float(np.abs(part - truth[y0 + oy:y1 + oy, x0 + ox:x1 + ox])[mask].mean())
            row[name] = {"shift": best[1:], "error": round(at_zero, 2), "pixels": int(mask.sum())}
        frames.append(row)
    bad = [
        r for r in frames
        if any(
            max(abs(r[n]["shift"][0]), abs(r[n]["shift"][1])) > SLACK
            for n in ("window", "carried")
            if n in r
        )
    ]
    return {"ok": bool(frames) and not bad, "frames": len(frames), "bad": len(bad), "worst": bad[:5], "why": "" if frames else "no frame filmed"}


def run_case(case: Case, tier: str, out: Path) -> dict:
    folder = out / f"{case.name}_{tier}"
    folder.mkdir(parents=True, exist_ok=True)
    result = {"case": case.name, "what": case.what, "tier": tier, "folder": str(folder)}
    for side in ("view", "bare", "truth"):
        code, _ = film(case, tier, folder, side)
        if code != 0:
            return {**result, "ok": False, "why": f"capture_scene {side} exited {code} ({folder / side}.log)"}
    return {**result, **measure(case, folder)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--tiers", default=",".join(TIERS))
    parser.add_argument("--cases", default=",".join(c.name for c in CASES))
    parser.add_argument("--out", default=str(REPO / "target" / "portal-seam-over-time"))
    parser.add_argument("--jobs", type=int, default=2)
    parser.add_argument("--build", action="store_true", help="build capture_scene first")
    args = parser.parse_args()
    if args.build:
        built = subprocess.run(["cargo", "build", "-j", "6", "-p", "ambition_app_tools", "--bin", "capture_scene"], cwd=REPO)
        if built.returncode != 0:
            return built.returncode
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    names = set(args.cases.split(","))
    jobs = [(case, tier) for case in CASES if case.name in names for tier in args.tiers.split(",")]
    with ThreadPoolExecutor(max_workers=max(1, args.jobs)) as pool:
        results = list(pool.map(lambda job: run_case(job[0], job[1], out), jobs))
    (out / "report.json").write_text(json.dumps(results, indent=2))
    for r in results:
        verdict = "ok" if r.get("ok") else "FAIL"
        print(f"{verdict:4} {r['case']:12} {r['tier']:7} frames={r.get('frames', 0)} bad={r.get('bad', '-')} {r.get('why', '')}")
        for row in r.get("worst", []):
            print(f"       tick {row['tick']}: window {row.get('window')} carried {row.get('carried')}")
    return 0 if all(r.get("ok") for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
