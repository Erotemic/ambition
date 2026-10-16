#!/usr/bin/env python3
"""The portal seam lab: throw the seam gauge through portal pairs and measure
how seamless each crossing looks.

The gauge (held item `seam_gauge`) is a plate of pure magenta and pure blue
cells, colours nothing else in the lab uses. Each scenario throws it into one
end of a `portal_lab` pair at a speed and films every frame with both ends of
the pair in the shot. For each frame the lab counts the gauge's pixels and
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

    python3 scripts/portal_seam_lab.py [--cases A,B] [--speeds slow,fast] [--out DIR] [--jobs 6]

Writes `DIR/report.md`, `DIR/report.json` and a contact sheet per scenario.
"""
from __future__ import annotations

import argparse
import json
import os
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
START_OFF = 110.0  # px in front of the entry
# `ambition_held_items::GROUND_ITEM_GRAVITY` (px/s^2, down).
ITEM_GRAVITY = 1400.0
THROW_TICK = 70
# Shots after the throw reaches its entry (one a tick).
FRAMES_AFTER = 36
# Where the player waits, out of every shot.
PLAYER_AWAY = (1700.0, 300.0)

# A gauge pixel: pure magenta or pure blue, give or take the filtering of
# the sprite (measured 2026-10-15: the gauge's cells land at 224 and up; a
# purple portal glow lands near 192-208 with some green).
HIGH, LOW = 215, 25


def run_case(case: Case, speed_name: str, out: Path, windows: bool = False) -> dict:
    speed = SPEEDS[speed_name]
    folder = out / f"{case.name}_{speed_name}"
    folder.mkdir(parents=True, exist_ok=True)
    nx, ny = case.normal
    start = (case.entry[0] + nx * START_OFF, case.entry[1] + ny * START_OFF)
    # Aimed to arrive at the entry's centre after `t`, under the item's
    # gravity: v = d / t - g t / 2, so a throw at a wall portal arcs into the
    # opening instead of falling short of it.
    t = START_OFF / speed
    vel = (-nx * START_OFF / t, -ny * START_OFF / t - ITEM_GRAVITY * t / 2.0)
    cmd = [
        str(CAPTURE),
        "portal_lab",
        f"{case.focus[0]},{case.focus[1]}",
        str(folder / "f.png"),
        f"{case.size[0]}x{case.size[1]}",
        "--player-at", f"{PLAYER_AWAY[0]},{PLAYER_AWAY[1]}@30",
        "--item", f"{THROW_TICK}:{start[0]:.1f},{start[1]:.1f}:{vel[0]:.1f},{vel[1]:.1f}:seam_gauge",
        "--warmup", str(THROW_TICK),
        # One sim tick a shot, the same each run.
        "--frames", str(int(t * 60) + FRAMES_AFTER),
        "--tick-per-frame",
        "--portal-effect", "cones" if windows else "off",
        "--stride", "1",
    ]
    env = dict(
        os.environ,
        AMBITION_QUALITY_PROFILE=os.environ.get("AMBITION_QUALITY_PROFILE", "high"),
        # The sim's transit steps and what the cutter drew, beside each film.
        RUST_LOG="ambition_portal::transit_trace=debug,ambition_portal::piece_trace=debug,"
        "ambition_platformer2d::portal=info,warn",
    )
    with open(folder / "run.log", "w") as log:
        code = subprocess.run(cmd, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=900).returncode
    return measure(case, speed_name, folder, code)


def gauge_pixels(path: Path):
    from PIL import Image

    image = Image.open(path).convert("RGB")
    w, h = image.size
    data = image.load()
    count, sx, sy = 0, 0.0, 0.0
    for y in range(h):
        for x in range(w):
            r, g, b = data[x, y]
            if g < LOW and b > HIGH and (r > HIGH or r < LOW):
                count += 1
                sx += x
                sy += y
    return count, ((sx / count, sy / count) if count else None)


def first_crossing(log: str) -> tuple[int, int] | None:
    """The ticks (counted from the item's first one) of the first crossing:
    the item's first `Begin` and the `Clear` after it, from the sim's
    transit trace (one line per tick)."""
    steps = [line.split("step=", 1)[1] for line in log.splitlines() if "free body=" in line and "step=" in line]
    begin = next((i for i, step in enumerate(steps) if step.startswith("Begin")), None)
    if begin is None:
        return None
    clear = next((i for i, step in enumerate(steps) if i > begin and step.startswith("Clear")), len(steps) - 1)
    return begin, clear


def measure(case: Case, speed_name: str, folder: Path, code: int) -> dict:
    frames = sorted(folder.glob("f.*.png"))
    seen = [gauge_pixels(f) for f in frames]
    counts = [c for c, _ in seen]
    log = (folder / "run.log").read_text(errors="replace")
    # The film starts before the throw: frame `f0` is the item's first tick.
    f0 = next((i for i, c in enumerate(counts) if c > 0), None)
    crossing = first_crossing(log)
    result = {
        "case": case.name,
        "what": case.what,
        "speed": speed_name,
        "exit_code": code,
        "frames": len(frames),
        "crossed": crossing is not None,
        "transfers_logged": log.count("transferred through the portal pair"),
        "fallback_frames": log.count("fallback: whole, copied"),
        "counts": counts,
    }
    if f0 is not None and crossing is not None:
        begin, clear = (f0 + crossing[0], min(f0 + crossing[1] + 1, len(counts) - 1))
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
        "| case | pair | speed | crossed | vanished | dip | double | fallback |",
        "|---|---|---|---|---|---|---|---|",
    ]
    for r in results:
        lines.append(
            f"| {r['case']} | {r['what']} | {r['speed']} | {'yes' if r['crossed'] else 'NO'} "
            f"| {r.get('vanished', '-')} | {r.get('dip', '-')} | {r.get('double', '-')} | {r['fallback_frames']} |"
        )
    (out / "report.md").write_text("\n".join(lines) + "\n")
    (out / "report.json").write_text(json.dumps(results, indent=1))
    print("\n".join(lines))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--cases", default=",".join(c.name for c in CASES))
    ap.add_argument("--speeds", default=",".join(SPEEDS))
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
    jobs = [(c, s) for c in CASES if c.name in wanted for s in speeds]
    if args.measure_only:
        results = [measure(c, s, out / f"{c.name}_{s}", 0) for c, s in jobs]
    else:
        with ThreadPoolExecutor(max_workers=min(args.jobs, 6)) as pool:
            results = list(pool.map(lambda job: run_case(job[0], job[1], out, args.windows), jobs))
    report(results, out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
