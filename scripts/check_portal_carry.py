#!/usr/bin/env python3
"""Measure that a portal pair carries every kind of drawable that goes into it, through the real render stack.

Jon, 2026-10-25: "anything passing through the portal is properly handled
seemlessly from the perspective the player". What goes into a pair's entry,
inside its opening, must not be drawn behind the entry where it is: it is
drawn out of the exit. A body was cut so; a slash, a spark, a helmet, a
trail were not (each was drawn whole on its side).

For each case this films a scenario twice with `capture_scene`, one sim tick
a frame, windows off: once with the drawable kind (a slash, a burst of
particles, a worn helmet, ...) and once without it. The two films differ by
that kind's pixels alone. Then for each frame:

- `behind`: its pixels behind the entry's plane, inside the opening (to
  `DEPTH` world units past it). Drawn there, it went in and was not taken
  out of the world.
- `out`: its pixels in front of the exit, inside the opening (to `DEPTH`).

A case fails when any frame has more than `MAX_BEHIND` pixels behind the
entry, or when no frame has any out of the exit (nothing was carried: the
film missed, or the pair carried nothing).

    python3 scripts/check_portal_carry.py [--build] [--tiers low] [--cases slash,sparks,helmet] [--carry off] [--out DIR]

`--build` builds the tool first. It runs under `xvfb-run` when there is no
display. Writes `DIR/report.json`, each case's counts per frame, and a
picture of the worst frame with the two regions outlined.
"""
from __future__ import annotations

import argparse
import json
import math
import os
import re
import shutil
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CAPTURE = REPO / "target" / "debug" / "capture_scene"

# A channel differing by more than this is a pixel of the drawable.
DIFFERS = 24
# The most pixels of the drawable a frame may draw behind the entry (an
# anti-aliased fringe at the plane).
MAX_BEHIND = 40
# How deep past each plane the regions reach, in world units.
DEPTH = 60.0
# The image.
SIZE = (1280, 720)


@dataclass(frozen=True)
class Aperture:
    centre: tuple[float, float]
    # Outward normal (world, y down): the side in front of the face.
    normal: tuple[float, float]
    half: float


@dataclass(frozen=True)
class Shot:
    """The focus given to `capture_scene` (`--camera-zoom debug`), and the
    centre and size of the world the image shows: the camera's framing puts
    the focus off the centre (and a boss's encounter widens the view).
    Measured from the portal lines of a film."""

    focus: tuple[float, float]
    centre: tuple[float, float]
    view: tuple[float, float]


@dataclass(frozen=True)
class Case:
    name: str
    what: str
    room: str
    shot: Shot
    entry: Aperture
    exit: Aperture
    # Sim ticks: the first shot, and how many.
    first: int
    frames: int
    # The options both films take, and those only the film with the kind takes.
    both: tuple[str, ...] = ()
    kind: tuple[str, ...] = ()


# portal_lab's B pair: two faces on one wall, facing right (`sandbox.ldtk`,
# `b_red` under `b_teal`); its D pair: a floor face into a wall face
# (`d_cyan`, `d_rose`). Each plane is where a film draws its line of light: a
# portal settles on the face of its wall, not at the centre of its LDtk box.
B_RED = Aperture((1200.0, 850.0), (1.0, 0.0), 46.0)
B_TEAL = Aperture((1200.0, 550.0), (1.0, 0.0), 46.0)
D_CYAN = Aperture((1450.0, 896.0), (0.0, -1.0), 46.0)
D_ROSE = Aperture((1776.0, 700.0), (-1.0, 0.0), 46.0)
# Measured 2026-10-25.
LAB = Shot(focus=(1190.0, 606.0), centre=(1190.0, 562.0), view=(1600.0, 900.0))

CASES = [
    Case(
        "slash", "a slash into a wall portal comes out of its partner", "portal_lab", LAB, B_RED, B_TEAL,
        first=70, frames=40,
        both=("--player-at", "1222,876@60", "--player-holds", "left@64", "--player-holds", "none@66"),
        kind=("--player-holds", "attack@68"),
    ),
    Case(
        "sparks", "a fireball and its sparks into a wall portal", "portal_lab", LAB, B_RED, B_TEAL,
        first=60, frames=50,
        both=("--player-at", "1500,876@30",),
        kind=("--shot", "62:1320,850:-1,0:fireball"),
    ),
    Case(
        "helmet", "a helmet on a body that falls into a floor portal", "portal_lab", LAB, D_CYAN, D_ROSE,
        first=88, frames=60,
        both=("--player-at", "2200,876@30", "--player-at", "1450.0,720.0@90:0.0,0.0"),
        kind=("--gear", "space_helmet@40"),
    ),
]
TIERS = ("low",)


def to_pixel(case: Case, p: tuple[float, float]) -> tuple[float, float]:
    shot = case.shot
    sx, sy = SIZE[0] / shot.view[0], SIZE[1] / shot.view[1]
    return ((p[0] - shot.centre[0]) * sx + SIZE[0] / 2, (p[1] - shot.centre[1]) * sy + SIZE[1] / 2)


def region(case: Case, ap: Aperture, behind: bool):
    """The pixels of the opening's slab, behind its face or in front of it."""
    import numpy as np

    ys, xs = np.mgrid[0 : SIZE[1], 0 : SIZE[0]]
    shot = case.shot
    sx, sy = SIZE[0] / shot.view[0], SIZE[1] / shot.view[1]
    wx = (xs + 0.5 - SIZE[0] / 2) / sx + shot.centre[0]
    wy = (ys + 0.5 - SIZE[1] / 2) / sy + shot.centre[1]
    nx, ny = ap.normal
    tx, ty = -ny, nx
    d = (wx - ap.centre[0]) * nx + (wy - ap.centre[1]) * ny
    t = (wx - ap.centre[0]) * tx + (wy - ap.centre[1]) * ty
    across = np.abs(t) < ap.half
    if behind:
        return across & (d < -1.5) & (d > -DEPTH)
    return across & (d > 1.5) & (d < DEPTH)


def film(case: Case, tier: str, folder: Path, with_kind: bool, carry: str) -> tuple[int, Path]:
    name = "with" if with_kind else "without"
    out = folder / name
    out.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(CAPTURE), case.room, f"{case.shot.focus[0]},{case.shot.focus[1]}", str(out / "f.png"), f"{SIZE[0]}x{SIZE[1]}",
        "--warmup", str(case.first), "--frames", str(case.frames), "--stride", "1", "--tick-per-frame",
        "--camera-zoom", "debug", "--portal-effect", "off", "--carry", carry, *case.both,
    ]
    if with_kind:
        cmd += list(case.kind)
    if not os.environ.get("DISPLAY") and shutil.which("xvfb-run"):
        cmd = ["xvfb-run", "-a", *cmd]
    env = dict(os.environ, AMBITION_QUALITY_PROFILE=tier)
    with open(folder / f"{name}.log", "w") as log:
        code = subprocess.run(cmd, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=3600).returncode
    return code, out


def shots(log: Path) -> dict[int, int]:
    return {int(t): int(n) for n, t in re.findall(r"capture_scene: shot (\d+) at sim tick (\d+)", log.read_text(errors="replace"))}


def measure(case: Case, folder: Path) -> dict:
    import numpy as np
    from PIL import Image, ImageDraw

    a, b = shots(folder / "with.log"), shots(folder / "without.log")
    ticks = sorted(set(a) & set(b))
    behind_mask = region(case, case.entry, True)
    out_mask = region(case, case.exit, False)
    rows = []
    worst = None
    for tick in ticks:
        pa, pb = folder / "with" / f"f.{a[tick]:04d}.png", folder / "without" / f"f.{b[tick]:04d}.png"
        if not (pa.exists() and pb.exists()):
            continue
        x = np.asarray(Image.open(pa).convert("RGB")).astype(np.int16)
        y = np.asarray(Image.open(pb).convert("RGB")).astype(np.int16)
        kind = np.abs(x - y).max(axis=2) > DIFFERS
        row = {"tick": tick, "kind": int(kind.sum()), "behind": int((kind & behind_mask).sum()), "out": int((kind & out_mask).sum())}
        rows.append(row)
        if worst is None or row["behind"] > worst[0]["behind"]:
            worst = (row, pa, kind)
    if worst is not None:
        row, path, kind = worst
        picture = Image.open(path).convert("RGB")
        overlay = np.asarray(picture).copy()
        overlay[kind] = (255, 0, 255)
        picture = Image.fromarray(overlay)
        draw = ImageDraw.Draw(picture)
        for ap, colour in ((case.entry, (255, 255, 0)), (case.exit, (0, 255, 255))):
            nx, ny = ap.normal
            tx, ty = -ny, nx
            for side in (-1, 1):
                p0 = to_pixel(case, (ap.centre[0] + tx * ap.half * side, ap.centre[1] + ty * ap.half * side))
                p1 = to_pixel(case, (ap.centre[0] + tx * ap.half * side - nx * DEPTH, ap.centre[1] + ty * ap.half * side - ny * DEPTH))
                p2 = to_pixel(case, (ap.centre[0] + tx * ap.half * side + nx * DEPTH, ap.centre[1] + ty * ap.half * side + ny * DEPTH))
                draw.line([p1, p2], fill=colour)
            draw.line([to_pixel(case, (ap.centre[0] - tx * ap.half, ap.centre[1] - ty * ap.half)), to_pixel(case, (ap.centre[0] + tx * ap.half, ap.centre[1] + ty * ap.half))], fill=colour)
        picture.save(folder / "worst.png")
    (folder / "counts.json").write_text(json.dumps(rows))
    if not rows:
        return {"ok": False, "why": "no shot of the two films is of one sim tick"}
    most_behind = max(r["behind"] for r in rows)
    total_out = sum(r["out"] for r in rows)
    total_kind = sum(r["kind"] for r in rows)
    why = None
    if total_kind == 0:
        why = "the two films do not differ: the kind was not drawn"
    elif most_behind > MAX_BEHIND:
        bad = [r["tick"] for r in rows if r["behind"] > MAX_BEHIND]
        why = f"drawn behind the entry, where it went in: up to {most_behind} px, {len(bad)} frames (ticks {bad[0]}-{bad[-1]})"
    elif total_out == 0:
        why = "nothing came out of the exit"
    return {"ok": why is None, "why": why, "most_behind": most_behind, "out_px": total_out, "frames": len(rows)}


def run_case(case: Case, tier: str, out: Path, carry: str) -> dict:
    folder = out / f"{case.name}_{tier}"
    if folder.exists():
        shutil.rmtree(folder)
    folder.mkdir(parents=True)
    result = {"case": case.name, "what": case.what, "tier": tier, "folder": str(folder)}
    with ThreadPoolExecutor(max_workers=2) as pool:
        films = list(pool.map(lambda with_kind: film(case, tier, folder, with_kind, carry), (True, False)))
    for code, path in films:
        if code != 0:
            return {**result, "ok": False, "why": f"capture_scene exited {code} ({path}.log)"}
    return {**result, **measure(case, folder)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--tiers", default=",".join(TIERS))
    parser.add_argument("--cases", default=",".join(c.name for c in CASES))
    parser.add_argument("--out", default=str(REPO / "target" / "portal-carry"))
    parser.add_argument("--jobs", type=int, default=3, help="cases run at once; each runs two renders")
    parser.add_argument("--build", action="store_true", help="build capture_scene first")
    parser.add_argument(
        "--carry", choices=("on", "off"), default="on",
        help="off: the pairs carry nothing, each drawable is drawn whole on its side (each case must fail)",
    )
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
        results = list(pool.map(lambda job: run_case(job[0], job[1], out, args.carry), jobs))
    (out / "report.json").write_text(json.dumps(results, indent=2))
    failed = 0
    for r in results:
        mark = "ok  " if r["ok"] else "FAIL"
        detail = r.get("why") or f"behind the entry at most {r['most_behind']} px; {r['out_px']} px out of the exit over {r['frames']} frames"
        print(f"{mark} {r['case']:>8} {r['tier']:<6} {r['what']}: {detail}")
        failed += not r["ok"]
    print(f"{len(results) - failed}/{len(results)} kinds carried through their pairs. Report: {out / 'report.json'}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
