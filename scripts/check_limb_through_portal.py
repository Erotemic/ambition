#!/usr/bin/env python3
"""Measure that a limb that goes through a portal pair is drawn whole, through the real render stack.

GNU-ton's giant throws its blows through a pair: the arm goes into the near
line and its fist comes out of the far line. Jon, 2026-10-25: "his arm just
disappears when he punches through the portal". The arm must be drawn at
every moment of the blow: the part in front of the near line there, the part
through it at the far line, with no frame where a part is missing, doubled or
drawn where it is not.

For each case this films the fight twice with `capture_scene`, one sim tick a
frame, with the same input: once as the game draws it, and once with the
limb drawn nowhere (`--limbs-unseen SLOT`). The two films differ by the
limb's pixels alone, so each frame gives the limb's visible area (pixels
where a channel differs by more than `DIFFERS`). A case films with the
portal windows off (the limb as its pieces are drawn) or on (also the limb
that a window sees).

A case fails when, at any frame:

- the limb's area is less than `MIN_OF_MEDIAN` of its median before the
  first blow (the limb vanished, or the part of it in the pair was not
  drawn), or
- it changed from the frame before by more than `MAX_STEP` times (the limb
  jumped, or a piece was drawn stretched or in the wrong place; windows
  off only), or
- no frame of the film had the limb in a pair (the film missed the blow).

    tools/ambition_ldtk_tools/.venv/bin/python scripts/check_limb_through_portal.py [--build] [--tiers low,high] [--out DIR]

`--build` builds the tool first. It runs under `xvfb-run` when there is no
display. Writes `DIR/report.json`, each case's area per frame, and the frames
that failed side by side.

Measured 2026-10-25 (the first blows of the fight, 1280x720, `low`): before
the fix the limb's twin grew each frame (its size was read as a squash), the
arm vanished for the frames its twin took to be drawn, and it was drawn as a
bar from the near line to the floor; see the commit for the numbers.
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

# A channel differing by more than this is a pixel of the limb.
DIFFERS = 24
# The least of its median area the limb may show at any frame.
MIN_OF_MEDIAN = 0.35
# The most the limb's area may change from one frame to the next (a ratio).
MAX_STEP = 1.8


@dataclass(frozen=True)
class Case:
    name: str
    what: str
    room: str
    focus: tuple[float, float]
    player: tuple[float, float]
    # The limb measured: a `limb:<slot>` of the part-drawn body.
    slot: str
    # The sim ticks filmed: from, count.
    first: int
    frames: int
    # `--portal-effect`: `off` (pieces only) or `cones` (with windows).
    effect: str = "off"
    # The most the limb's area may change in a tick. With windows a limb
    # that goes in is seen again in the window of the other end, so its
    # area doubles in a tick by right: that case checks only that the limb
    # is drawn.
    max_step: float | None = MAX_STEP


CASES = [
    Case("hammer", "GNU-ton's first blows, the player on the floor", "gnu_ton_arena", (900, 900), (400, 1200),
         "hand_left", 280, 360),
    Case("hammer_windows", "the same, with the portal windows", "gnu_ton_arena", (900, 900), (400, 1200),
         "hand_left", 280, 180, "cones", None),
]
TIERS = ("low", "high")


def film(case: Case, tier: str, folder: Path, unseen: bool) -> tuple[int, Path]:
    name = "unseen" if unseen else "seen"
    out = folder / name
    out.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(CAPTURE), case.room, f"{case.focus[0]},{case.focus[1]}", str(out / "f.png"), "1280x720",
        "--player-at", f"{case.player[0]},{case.player[1]}@1",
        "--warmup", str(case.first), "--frames", str(case.frames), "--stride", "1", "--tick-per-frame",
        "--camera-zoom", "debug", "--portal-effect", case.effect,
    ]
    if unseen:
        cmd += ["--limbs-unseen", case.slot]
    if not os.environ.get("DISPLAY") and shutil.which("xvfb-run"):
        cmd = ["xvfb-run", "-a", *cmd]
    env = dict(os.environ, AMBITION_QUALITY_PROFILE=tier, RUST_LOG="warn,ambition_portal::piece_trace=debug")
    with open(folder / f"{name}.log", "w") as log:
        code = subprocess.run(cmd, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=3600).returncode
    return code, out


def read_log(log: Path) -> tuple[dict[int, int], set[int]]:
    """Each shot's sim tick, and the ticks of the shots whose frames drew
    some body cut by a pair. A trace line is of the frame of the next shot."""
    ticks: dict[int, int] = {}
    cut_shots: set[int] = set()
    shot = 0
    for line in log.read_text(errors="replace").splitlines():
        m = re.search(r"capture_scene: shot (\d+) at sim tick (\d+)", line)
        if m:
            ticks[int(m.group(1))] = int(m.group(2))
            shot = int(m.group(1)) + 1
            continue
        if "piece_trace" in line and line.rstrip().endswith(" cut"):
            cut_shots.add(shot)
    return ticks, {ticks[s] for s in cut_shots if s in ticks}


def measure(case: Case, folder: Path) -> dict:
    import numpy as np
    from PIL import Image

    # Two runs do not start on one tick: shots are paired by their sim tick.
    seen_ticks, cut_ticks = read_log(folder / "seen.log")
    unseen_ticks, _ = read_log(folder / "unseen.log")
    by_tick = {tick: shot for shot, tick in unseen_ticks.items()}
    pairs = [(tick, shot, by_tick[tick]) for shot, tick in sorted(seen_ticks.items()) if tick in by_tick]
    seen = [folder / "seen" / f"f.{a:04d}.png" for _, a, _ in pairs]
    unseen = [folder / "unseen" / f"f.{b:04d}.png" for _, _, b in pairs]
    ticks = [tick for tick, _, _ in pairs]
    if len(pairs) < case.frames // 2:
        return {"ok": False, "why": f"only {len(pairs)} shots of the two films are of one sim tick"}
    areas: list[int] = []
    masks = []
    for a, b in zip(seen, unseen):
        x = np.asarray(Image.open(a).convert("RGB")).astype(np.int16)
        y = np.asarray(Image.open(b).convert("RGB")).astype(np.int16)
        mask = np.abs(x - y).max(axis=2) > DIFFERS
        areas.append(int(mask.sum()))
        masks.append(mask)
    cut = sorted(cut_ticks)
    # The limb's area as its body draws it before the first blow's cut: what
    # it is measured against. (A limb that vanished for most of the film
    # would make the film's own median nothing.)
    before = [area for tick, area in zip(ticks, areas) if not cut or tick < cut[0]]
    median = float(np.median(before or areas))
    bad: list[dict] = []
    for i, area in enumerate(areas):
        if area < MIN_OF_MEDIAN * median:
            bad.append({"frame": i, "tick": ticks[i], "area": area, "why": "the limb is not drawn"})
        elif case.max_step is None:
            continue
        elif i > 0 and max(area, 1) / max(areas[i - 1], 1) > case.max_step:
            bad.append({"frame": i, "tick": ticks[i], "area": area, "why": f"grew from {areas[i - 1]}"})
        elif i > 0 and max(areas[i - 1], 1) / max(area, 1) > case.max_step:
            bad.append({"frame": i, "tick": ticks[i], "area": area, "why": f"shrank from {areas[i - 1]}"})
    for b in bad[:12]:
        i = b["frame"]
        x = np.asarray(Image.open(seen[i]).convert("RGB")).copy()
        x[masks[i]] = (255, 0, 255)
        prev = np.asarray(Image.open(seen[max(i - 1, 0)]).convert("RGB")).copy()
        prev[masks[max(i - 1, 0)]] = (255, 0, 255)
        Image.fromarray(np.concatenate([prev, x], axis=1)).save(folder / f"failed_{i:04d}.png")
    (folder / "areas.json").write_text(json.dumps({"ticks": ticks, "areas": areas, "cut_ticks": cut}))
    ok = not bad and bool(cut) and median > 0
    why = None
    if median <= 0:
        why = "the limb is not drawn before the blow: the film does not show it"
    elif not cut:
        why = "no body was drawn cut by a pair: the film missed the blow"
    elif bad:
        why = f"{len(bad)} frames: " + "; ".join(f"tick {b['tick']} {b['why']} ({b['area']} px)" for b in bad[:6])
    return {"ok": ok, "why": why, "median_area": median, "frames_cut": len(cut), "bad": bad}


def run_case(case: Case, tier: str, out: Path) -> dict:
    folder = out / f"{case.name}_{tier}"
    if folder.exists():
        shutil.rmtree(folder)
    folder.mkdir(parents=True)
    result = {"case": case.name, "what": case.what, "tier": tier, "folder": str(folder)}
    with ThreadPoolExecutor(max_workers=2) as pool:
        films = list(pool.map(lambda unseen: film(case, tier, folder, unseen), (False, True)))
    for code, path in films:
        if code != 0:
            return {**result, "ok": False, "why": f"capture_scene exited {code} ({path}.log)"}
    return {**result, **measure(case, folder)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--tiers", default=",".join(TIERS))
    parser.add_argument("--cases", default=",".join(c.name for c in CASES))
    parser.add_argument("--out", default=str(REPO / "target" / "limb-through-portal"))
    parser.add_argument("--jobs", type=int, default=2, help="cases run at once; each runs two renders")
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
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    jobs = [(case, tier) for case in cases for tier in args.tiers.split(",")]
    with ThreadPoolExecutor(max_workers=max(1, min(args.jobs, 3))) as pool:
        results = list(pool.map(lambda job: run_case(job[0], job[1], out), jobs))
    (out / "report.json").write_text(json.dumps(results, indent=2))
    failed = 0
    for r in results:
        mark = "ok  " if r["ok"] else "FAIL"
        detail = r.get("why") or f"median {r['median_area']:.0f} px, {r['frames_cut']} frames cut by a pair"
        print(f"{mark} {r['case']:>8} {r['tier']:<6} {r['what']}: {detail}")
        failed += not r["ok"]
    print(f"{len(results) - failed}/{len(results)} limbs drawn whole through their pairs. Report: {out / 'report.json'}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
