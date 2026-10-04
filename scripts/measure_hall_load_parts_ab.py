#!/usr/bin/env python3
"""Does drawing characters from parts load the hall of characters faster?
(`docs/planning/engine/mary-o-part-realization.md`; one metric among several,
the rest in `scripts/measure_part_flipbook_cost.py`.)

Interleaved A/B of one `capture_scene` run per arm per rep in a room (default
`hall_of_characters`): arm `parts` is the shipped default, arm `baked` sets
`AMBITION_RIGGED_SPRITES=0` so every character realizes its baked sheet. Each
run's `[image]` lines (the asset census: one per decoded image, with its
decode time and the road that demanded it) are reduced to:

* the character images decoded, their megapixels and their summed
  demand→insert milliseconds, split by road (`character-sheet`,
  `character-parts`);
* the time the LAST character image was inserted (the cast fully resident,
  on the app's own clock) and how many were decoded during gameplay (`live=1`);
* `[frame-spike]` count and worst, wall seconds and max RSS (`/usr/bin/time -v`).

⛔ Arms are interleaved (rep1: parts, baked; rep2: ...) so a noisy neighbour
lands on both. ⛔ The first rep warms the page cache; read it, do not average
it in (`--reps 3` reports the median of reps 2..N per arm).

    scripts/measure_hall_load_parts_ab.py                  # 3 reps, record
    scripts/measure_hall_load_parts_ab.py --reps 5 --room central_hub_complex
    scripts/measure_hall_load_parts_ab.py --no-build --no-record

Rows go to `dev/ambition_dev_measurements/part_flipbook_cost.jsonl` (kind
`part_flipbook_hall_load`), each with the host and a `comparable_key`.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import secrets
import statistics
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "scripts"))

from measure_part_flipbook_cost import LEDGER, SUMMARY, _envelope, _host, _read_ledger, summarise  # noqa: E402

ARMS = {"parts": {}, "baked": {"AMBITION_RIGGED_SPRITES": "0"}}
IMAGE = re.compile(
    r"^\[image\]\s+(?P<at>[\d.]+)s f\s*\d+ (?P<w>\d+)x(?P<h>\d+)\s+(?P<mp>[\d.]+)MP live=(?P<live>\d) (?P<path>\S+)"
    r"(?:.*?demand→insert (?P<ms>\d+)ms)?(?:.*?via (?P<road>[\w-]+))?"
)
SPIKE = re.compile(r"^\[frame-spike\]\s+[\d.]+s\s+([\d.]+)ms")


def reduce_log(text: str) -> dict:
    roads: dict = {}
    last_character = None
    live = 0
    for line in text.splitlines():
        match = IMAGE.match(line)
        if not match:
            continue
        road = match["road"] or "other"
        if not road.startswith("character"):
            continue
        entry = roads.setdefault(road, {"images": 0, "megapixels": 0.0, "decode_ms": 0})
        entry["images"] += 1
        entry["megapixels"] = round(entry["megapixels"] + float(match["mp"]), 1)
        entry["decode_ms"] += int(match["ms"] or 0)
        at = float(match["at"])
        last_character = at if last_character is None else max(last_character, at)
        live += int(match["live"])
    # ⛔ `[image]` prints only the big decodes: the complete record is the last
    # `[image-census]` line's "resident by road" (count x megapixels per road).
    census = [line for line in text.splitlines() if line.startswith("[image-census]")]
    resident = {}
    if census:
        tail = census[-1].split("resident by road:", 1)[-1]
        for road, count, mp in re.findall(r"([\w-]+)(?:\([^)]*\))? (\d+)×([\d.]+)MP", tail):
            resident[road] = {"images": int(count), "megapixels": float(mp)}
        total = re.search(r"total (\d+) images, ([\d.]+)MP, ([\d.]+)MB resident", census[-1])
    spikes = [float(m[1]) for line in text.splitlines() if (m := SPIKE.match(line))]
    frames = [
        (float(m[1]), float(m[2]))
        for line in text.splitlines()
        if (m := re.match(r"^\[frame-census\].*? p50=([\d.]+)ms p95=([\d.]+)ms", line))
    ]
    # The adapter, when the log names it (the first-run quality seed does).
    adapter = re.search(r"for an? (\w+) adapter \((.*)\); this is", text)
    wall = re.search(r"Elapsed \(wall clock\) time.*: (?:(\d+):)?(\d+):([\d.]+)", text)
    rss = re.search(r"Maximum resident set size \(kbytes\): (\d+)", text)
    return {
        "roads": roads,
        "character_images": sum(r["images"] for r in roads.values()),
        "character_megapixels": round(sum(r["megapixels"] for r in roads.values()), 1),
        "character_decode_ms": sum(r["decode_ms"] for r in roads.values()),
        "last_character_insert_s": last_character,
        "decoded_during_gameplay": live,
        "resident_by_road": resident,
        "character_resident_megapixels": round(sum(v["megapixels"] for k, v in resident.items() if k.startswith("character")), 1),
        "resident_mb": float(total[3]) if census and total else None,
        # Steady state: the median of the frame-census windows' p50 after the first.
        "frame_windows": len(frames),
        "adapter": f"{adapter[1]}: {adapter[2]}" if adapter else None,
        "frame_p50_ms": statistics.median([p50 for p50, _ in frames[1:]]) if len(frames) > 1 else None,
        "frame_p95_ms": statistics.median([p95 for _, p95 in frames[1:]]) if len(frames) > 1 else None,
        "frame_spikes": len(spikes),
        "worst_spike_ms": max(spikes, default=None),
        "wall_s": (int(wall[1] or 0) * 3600 + int(wall[2]) * 60 + float(wall[3])) if wall else None,
        "max_rss_mb": round(int(rss[1]) / 1024, 1) if rss else None,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--room", default="hall_of_characters")
    parser.add_argument("--reps", type=int, default=3)
    # ⛔ The steady state is the `[frame-census]` windows AFTER the first (5 s
    # each; the first holds the load). 400 frames lasted 16 s on llvmpipe and
    # 7 s on a GPU, which printed one window and no frame times at all
    # (toothbrush, 2026-10-04). 1800 frames is 30 s at 60 fps.
    parser.add_argument("--warmup", type=int, default=1800, help="frames before the shot")
    parser.add_argument("--profile", default="profiling")
    parser.add_argument("--no-build", action="store_true")
    parser.add_argument("--no-record", action="store_true")
    parser.add_argument("--label", default="")
    parser.add_argument("--out", type=Path, default=REPO / "target" / "tmp" / "hall-load-parts-ab")
    args = parser.parse_args()
    if not args.no_build:
        subprocess.run(["scripts/setup/target_bindmount.sh", "--check"], cwd=REPO, check=True)
        subprocess.run(
            ["cargo", "build", "-q", "-p", "ambition_app_tools", "--bin", "capture_scene", "--profile", args.profile],
            cwd=REPO,
            check=True,
        )
    binary = REPO / "target" / args.profile / "capture_scene"
    args.out.mkdir(parents=True, exist_ok=True)
    host = _host()
    run_id = secrets.token_hex(6)
    runs: dict = {arm: [] for arm in ARMS}
    for rep in range(1, args.reps + 1):
        for arm, env in ARMS.items():
            log = args.out / f"{arm}-rep{rep}.log"
            command = ["/usr/bin/time", "-v", str(binary), args.room, "player", str(args.out / f"{arm}-rep{rep}.png"),
                       "640x360", "--warmup", str(args.warmup)]
            result = subprocess.run(command, cwd=REPO / "game" / "ambition_app_tools", capture_output=True, text=True,
                                    env={**__import__("os").environ, **env})
            log.write_text(result.stdout + result.stderr)
            reduced = reduce_log(result.stdout + result.stderr)
            reduced.update({"rep": rep, "exit": result.returncode, "log": str(log.relative_to(REPO))})
            runs[arm].append(reduced)
            if reduced["frame_windows"] < 2:
                print(f"⚠ rep {rep} {arm}: {reduced['frame_windows']} [frame-census] window(s), so no steady-state "
                      f"frame time: raise --warmup (the first 5 s window is the load)", flush=True)
            print(f"rep {rep} {arm:5}: exit {result.returncode}, adapter {reduced['adapter']}, character resident {reduced['character_resident_megapixels']} MP "
                  f"({reduced['resident_mb']} MB all), frame p50 {reduced['frame_p50_ms']} ms p95 {reduced['frame_p95_ms']} ms, "
                  f"big decodes {reduced['character_images']}, decode {reduced['character_decode_ms']} ms, last insert "
                  f"{reduced['last_character_insert_s']}s, live {reduced['decoded_during_gameplay']}, spikes "
                  f"{reduced['frame_spikes']} (worst {reduced['worst_spike_ms']}), wall {reduced['wall_s']}s, rss {reduced['max_rss_mb']} MB",
                  flush=True)
    comparable = {"tool": "capture_scene", "room": args.room, "warmup": args.warmup, "cargo_profile": args.profile,
                  "machine": host["machine"], "cpu": host["cpu"], "logical_cpus": host["logical_cpus"]}
    rows = []
    for arm, reps in runs.items():
        steady = [r for r in reps if r["rep"] > 1 and r["exit"] == 0] or [r for r in reps if r["exit"] == 0]

        def median(key):
            values = [r[key] for r in steady if r[key] is not None]
            return statistics.median(values) if values else None

        row = _envelope("part_flipbook_hall_load", run_id, args.label)
        row.update({
            "arm": arm,
            "env": ARMS[arm],
            "room": args.room,
            "reps": reps,
            "median_of_reps": "2..N" if len(reps) > 1 else "1",
            "median": {key: median(key) for key in ("character_resident_megapixels", "resident_mb", "frame_p50_ms",
                                                    "frame_p95_ms", "character_images", "character_megapixels", "character_decode_ms",
                                                    "last_character_insert_s", "decoded_during_gameplay",
                                                    "frame_spikes", "worst_spike_ms", "wall_s", "max_rss_mb")},
            "host": host,
            "comparable_fields": comparable,
            "comparable_key": hashlib.sha1(json.dumps(comparable, sort_keys=True).encode()).hexdigest()[:12],
        })
        rows.append(row)
        print(f"{arm:5} median: {row['median']}")
    if args.no_record:
        return 0
    with LEDGER.open("a") as ledger:
        for row in rows:
            ledger.write(json.dumps(row, sort_keys=True) + "\n")
    SUMMARY.write_text(summarise(_read_ledger()))
    print(f"appended {len(rows)} row(s) to {LEDGER.relative_to(REPO)}; wrote {SUMMARY.relative_to(REPO)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
