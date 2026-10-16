#!/usr/bin/env python3
"""The portal seam lab: send the seam gauge through portal pairs and measure
how seamless each crossing looks.

The gauge is a plate of pure magenta and pure blue cells, colours nothing
else in the lab uses. It is sent as each kind of mover: thrown as a ground
item (`gauge`, the held item `seam_gauge`) and fired as a shot (`shot`, the
projectile look `seam_gauge`, at the plain shot's 300 px/s). The robot
(`robot`, the player launched at the entry) is measured by its difference
from a clean plate, the first frame, filmed before it arrives. Each scenario
sends it into one end of a `portal_lab` pair and films every frame with both
ends of the pair in the shot. For each frame the lab counts the gauge's pixels and
finds their centre. A crossing that looks seamless keeps the count steady (the
gauge is drawn whole, split across the pair while it straddles) and moves the
centre only as far as the gauge's speed explains. Reported per scenario:

- `crossed`: the sim's transit trace (`ambition_portal::transit_trace`) shows
  the gauge straddled and crossed (no crossing is a scenario that missed).
- `vanished`: frames of the first crossing where none of the gauge shows.
- `dip` / `double`: the least / most of the gauge drawn while it crossed,
  against the whole gauge on its way in (a part not drawn / a part drawn
  twice). A teleport shows as a dip: the gauge is drawn whole on one side,
  then whole on the other, with nothing split between.
- `fallback`: frames the cutter (`ambition_portal::piece_trace`) drew the
  gauge whole and copied, because it could not cut its sprite.

It films with the view windows off (`--portal-effect off`) unless asked
(`--windows`): a window that sees the gauge shows it a second time, and this
measures how the gauge itself is drawn. Not part of the default test suite:
it films with the real render stack.

    python3 scripts/portal_seam_lab.py [--cases A,B] [--movers gauge,shot] [--speeds slow,fast] [--out DIR] [--jobs 6]

Writes `DIR/report.md`, `DIR/report.json` and a contact sheet per scenario.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CAPTURE = REPO / "target" / "debug" / "capture_scene"


@dataclass(frozen=True)
class Case:
    name: str
    what: str
    focus: tuple[float, float]
    size: tuple[int, int]
    # The entry's centre and its outward normal (world, y down).
    entry: tuple[float, float]
    normal: tuple[float, float]


# The pairs of `portal_lab` (sandbox.ldtk), each framed with both ends in the shot.
CASES = [
    Case("A", "floor <-> floor", (450, 820), (560, 240), (300, 900), (0, -1)),
    Case("B", "wall <-> wall", (1240, 700), (360, 440), (1190, 850), (1, 0)),
    Case("C", "floor <-> ceiling", (900, 560), (360, 760), (900, 900), (0, -1)),
    Case("D", "floor <-> wall", (1600, 800), (520, 300), (1450, 900), (0, -1)),
    Case("door", "thin-wall door", (2208, 856), (360, 240), (2200, 856), (-1, 0)),
    Case("E", "slope <-> wall", (3200, 860), (640, 300), (2950, 892), (-0.5, -0.866)),
    Case("F", "moving plank <-> floor", (3290, 800), (520, 440), (3320, 960), (0, -1)),
    Case("G", "spinning plank <-> shelf", (3010, 260), (480, 320), (2900, 336), (0, -1)),
]

SPEEDS = {"slow": 150.0, "medium": 400.0, "fast": 900.0}
MOVERS = ("gauge", "shot", "robot")
# The movement kernel's gravity (`movement::tuning::GRAVITY`), for the robot's aim.
ROBOT_GRAVITY = 2250.0
# Frames filmed before the robot arrives: the first is the clean plate.
PLATE_FRAMES = 2
# The plain shot's speed (`capture_scene --shot`); a shot has one speed.
SHOT_SPEED = 300.0
START_OFF = 110.0  # px in front of the entry
# `ambition_held_items::GROUND_ITEM_GRAVITY` (px/s^2, down).
ITEM_GRAVITY = 1400.0
THROW_TICK = 70
# Shots after the throw reaches its entry (one a tick).
FRAMES_AFTER = 36
# Where the player waits, out of every shot.
PLAYER_AWAY = (1700.0, 300.0)

# A gauge pixel: magenta, blue, or a blend of the two (a cell edge, a
# turned gauge's filtered texels), with next to no green. Measured
# 2026-10-16: a whole gauge counts within 3% from frame to frame (counting
# only pure cells, 8%); a purple portal glow carries 32+ green, and two
# pixels of D's glow pass.
def is_gauge(r: int, g: int, b: int) -> bool:
    return g < 24 and b > 110 and r <= b + 12


def folder_name(case: Case, mover: str, speed_name: str) -> str:
    if mover == "gauge":
        return f"{case.name}_{speed_name}"
    if mover == "robot":
        return f"{case.name}_robot_{speed_name}"
    return f"{case.name}_{mover}"


def run_case(case: Case, mover: str, speed_name: str, out: Path, windows: bool = False) -> dict:
    folder = out / folder_name(case, mover, speed_name)
    folder.mkdir(parents=True, exist_ok=True)
    nx, ny = case.normal
    start = (case.entry[0] + nx * START_OFF, case.entry[1] + ny * START_OFF)
    if mover == "gauge":
        # Aimed to arrive at the entry's centre after `t`, under the item's
        # gravity: v = d / t - g t / 2, so a throw at a wall portal arcs into
        # the opening instead of falling short of it.
        t = START_OFF / SPEEDS[speed_name]
        vel = (-nx * START_OFF / t, -ny * START_OFF / t - ITEM_GRAVITY * t / 2.0)
        send = ["--item", f"{THROW_TICK}:{start[0]:.1f},{start[1]:.1f}:{vel[0]:.1f},{vel[1]:.1f}:seam_gauge"]
    elif mover == "shot":
        # A shot flies straight, with no gravity: aimed along the normal.
        t = START_OFF / SHOT_SPEED
        send = ["--shot", f"{THROW_TICK}:{start[0]:.1f},{start[1]:.1f}:{-nx:.3f},{-ny:.3f}:seam_gauge"]
    else:
        # The robot is launched as the gauge is thrown, under its own gravity.
        t = START_OFF / SPEEDS[speed_name]
        vel = (-nx * START_OFF / t, -ny * START_OFF / t - ROBOT_GRAVITY * t / 2.0)
        send = ["--player-at", f"{start[0]:.1f},{start[1]:.1f}@{THROW_TICK}:{vel[0]:.1f},{vel[1]:.1f}"]
    warmup = THROW_TICK - PLATE_FRAMES if mover == "robot" else THROW_TICK
    cmd = [
        str(CAPTURE),
        "portal_lab",
        f"{case.focus[0]},{case.focus[1]}",
        str(folder / "f.png"),
        f"{case.size[0]}x{case.size[1]}",
        "--player-at", f"{PLAYER_AWAY[0]},{PLAYER_AWAY[1]}@30",
        *send,
        "--warmup", str(warmup),
        # One sim tick a shot, the same each run.
        "--frames", str(int(t * 60) + FRAMES_AFTER),
        "--tick-per-frame",
        "--portal-effect", "cones" if windows else "off",
        "--stride", "1",
    ]
    env = dict(
        os.environ,
        # `low`: no bloom. At `high`, bloom spreads a lamp's or a portal
        # line's light over the gauge's colours, and it measured as a dip (F
        # and G: 0.26-0.83 at `high`, 0.97-0.98 at `low`, 2026-10-16).
        AMBITION_QUALITY_PROFILE=os.environ.get("AMBITION_QUALITY_PROFILE", "low"),
        # The sim's transit steps and what the cutter drew, beside each film.
        RUST_LOG="ambition_portal::transit_trace=debug,ambition_portal::piece_trace=debug,"
        "ambition_platformer2d::portal=info,warn",
    )
    with open(folder / "run.log", "w") as log:
        code = subprocess.run(cmd, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=900).returncode
    return measure(case, mover, speed_name, folder, code)


def gauge_pixels(path: Path):
    from PIL import Image

    image = Image.open(path).convert("RGB")
    w, h = image.size
    data = image.load()
    count, sx, sy = 0, 0.0, 0.0
    for y in range(h):
        for x in range(w):
            if is_gauge(*data[x, y]):
                count += 1
                sx += x
                sy += y
    return count, ((sx / count, sy / count) if count else None)


def plate_pixels(path: Path, plate: Path) -> tuple[int, None]:
    """Pixels that differ from the clean plate: the robot, and whatever else
    changed (a portal's line of light shimmers, a little)."""
    from PIL import Image, ImageChops

    diff = ImageChops.difference(Image.open(path).convert("RGB"), Image.open(plate).convert("RGB"))
    return sum(1 for r, g, b in diff.getdata() if max(r, g, b) > 48), None


def first_crossing(log: str, mover: str) -> tuple[int, int] | None:
    """The sim ticks of the first crossing: the mover's first `Begin` and the
    `Clear` after it, from the sim's transit trace (one line per body per
    tick, each naming its tick). The mover is the first body of its kind
    (`free` for a ground item, `kernel` for a shot and the robot) that begins
    a crossing."""
    kind = "free body=" if mover == "gauge" else "kernel body="
    by_body: dict[str, list[tuple[int, str]]] = {}
    for line in log.splitlines():
        found = re.search(r"tick=(\d+) " + kind + r"(\S+) .*step=(.*)", line)
        if found:
            by_body.setdefault(found.group(2), []).append((int(found.group(1)), found.group(3)))
    for steps in by_body.values():
        begin = next((tick for tick, step in steps if step.startswith("Begin")), None)
        if begin is not None:
            clear = next((tick for tick, step in steps if tick > begin and step.startswith("Clear")), steps[-1][0])
            return begin, clear
    return None


def frames_by_tick(log: str) -> dict[int, int]:
    """The film's frame of each sim tick (`capture_scene: shot N at sim tick T`)."""
    return {int(t): int(n) for n, t in re.findall(r"capture_scene: shot (\d+) at sim tick (\d+)", log)}


def measure(case: Case, mover: str, speed_name: str, folder: Path, code: int) -> dict:
    frames = sorted(folder.glob("f.*.png"))
    if mover == "robot":
        seen = [plate_pixels(f, frames[0]) for f in frames] if frames else []
    else:
        seen = [gauge_pixels(f) for f in frames]
    counts = [c for c, _ in seen]
    log = (folder / "run.log").read_text(errors="replace")
    # The film starts before the throw: frame `f0` is the mover's first tick,
    # the first frame with a good part of a gauge (a few pixels of a glow
    # can pass the colour test).
    f0 = next((i for i, c in enumerate(counts) if c > 0.2 * max(counts)), None)
    crossing = first_crossing(log, mover)
    result = {
        "case": case.name,
        "what": case.what,
        "mover": mover,
        "speed": f"{SHOT_SPEED:.0f} px/s" if mover == "shot" else speed_name,
        "exit_code": code,
        "frames": len(frames),
        "crossed": crossing is not None,
        "transfers_logged": log.count("transferred through the portal pair"),
        "fallback_frames": log.count("fallback: whole, copied"),
        "counts": counts,
    }
    frame_of = frames_by_tick(log)
    if crossing is not None:
        # The frame of the first tick at or after each.
        at = lambda tick: next((frame_of[t] for t in sorted(frame_of) if t >= tick), None)
        crossing = (at(crossing[0]), at(crossing[1]))
    if f0 is not None and crossing is not None and None not in crossing:
        begin, clear = crossing[0], min(crossing[1], len(counts) - 1)
        approach = [c for c in counts[f0:begin] if c > 0] or [counts[begin]]
        whole = sorted(approach)[len(approach) // 2]
        window = counts[begin:clear + 1]
        result.update(
            {
                "whole_px": whole,
                "crossing_frames": [begin, clear],
                # The least / most of the gauge drawn while it crossed, against
                # the whole gauge on its way in.
                "dip": round(min(window) / whole, 3) if whole else None,
                "double": round(max(window) / whole, 3) if whole else None,
                "vanished": sum(1 for c in window if c == 0),
            }
        )
    contact_sheet(frames, folder / "sheet.png")
    return result


def contact_sheet(frames: list[Path], out: Path, cols: int = 8) -> None:
    from PIL import Image

    if not frames:
        return
    images = [Image.open(f).convert("RGB") for f in frames]
    w, h = images[0].size
    scale = max(1, w // 240)
    tw, th = w // scale, h // scale
    rows = (len(images) + cols - 1) // cols
    sheet = Image.new("RGB", (tw * cols, th * rows))
    for i, image in enumerate(images):
        sheet.paste(image.resize((tw, th)), ((i % cols) * tw, (i // cols) * th))
    sheet.save(out)


def report(results: list[dict], out: Path) -> None:
    lines = [
        "# Portal seam lab",
        "",
        "The first crossing of each throw. `dip` / `double`: the least / most of the gauge drawn",
        "while it crossed, against the whole gauge on its way in (1.0 is seamless). `fallback`:",
        "frames the cutter could not cut the sprite and drew it whole and copied.",
        "",
        "| case | pair | mover | speed | crossed | vanished | dip | double | fallback |",
        "|---|---|---|---|---|---|---|---|---|",
    ]
    for r in results:
        lines.append(
            f"| {r['case']} | {r['what']} | {r['mover']} | {r['speed']} | {'yes' if r['crossed'] else 'NO'} "
            f"| {r.get('vanished', '-')} | {r.get('dip', '-')} | {r.get('double', '-')} | {r['fallback_frames']} |"
        )
    (out / "report.md").write_text("\n".join(lines) + "\n")
    (out / "report.json").write_text(json.dumps(results, indent=1))
    print("\n".join(lines))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--cases", default=",".join(c.name for c in CASES))
    ap.add_argument("--speeds", default=",".join(SPEEDS), help="the gauge's throw speeds (a shot has one)")
    ap.add_argument("--movers", default=",".join(MOVERS))
    ap.add_argument("--out", default=str(REPO / "untracked" / "portal_seam_lab"))
    ap.add_argument("--jobs", type=int, default=6)
    ap.add_argument("--windows", action="store_true", help="film with the view windows (a window that sees the gauge shows it again)")
    ap.add_argument("--measure-only", action="store_true", help="measure the films already in --out")
    args = ap.parse_args()
    if not CAPTURE.exists():
        print(f"build capture_scene first: {CAPTURE}", file=sys.stderr)
        return 2
    wanted = set(args.cases.split(","))
    speeds = args.speeds.split(",")
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    movers = args.movers.split(",")
    jobs = [
        (c, m, s)
        for c in CASES
        if c.name in wanted
        for m in movers
        for s in (speeds[:1] if m == "shot" else speeds)
    ]
    if args.measure_only:
        results = [measure(c, m, s, out / folder_name(c, m, s), 0) for c, m, s in jobs]
    else:
        with ThreadPoolExecutor(max_workers=min(args.jobs, 6)) as pool:
            results = list(pool.map(lambda job: run_case(*job, out, args.windows), jobs))
    report(results, out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
