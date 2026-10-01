#!/usr/bin/env python3
"""Rig Packet 8: measure the rigged-sprite realization on a hardware GPU.

Run this on a machine with a hardware GPU, from the repository root:

    python3 scripts/rig_packet8_gpu_bench.py

It builds `rigged_sprite_bench` (`crates/ambition_render/examples/`) with the
`profiling` profile and runs the matrix that the Packet 8 decision needs:

* the ECS mode (no renderer): 1, 10, 50 and 100 actors;
* the renderer with one view, with two views, and with `--tiny` (a small
  target, so almost no pixels are filled).

Each render run waits for the GPU at the end of every frame, so the frame time
includes the GPU work, not only the CPU that submits it.

The output is a directory `target/rig_packet8/<host>_<UTC time>/` with the raw
log of each run, `report.json`, and `report.md`. Send `report.md` back (or commit it
under `docs/planning/engine/measurements/`).

⛔ The script stops if the render adapter is a software rasterizer (llvmpipe,
lavapipe, SwiftShader, WARP), because that is the measurement this machine
already has. Use `--allow-software` only to test the script itself.

Only the Python standard library is used. Requirements: `cargo` on the PATH and
the published sprite assets in the checkout (run `scripts/regen/sprites.sh` if
the bench says a sheet or a flipbook is missing).
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import re
import shutil
import socket
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
EXAMPLE = "rigged_sprite_bench"
PACKAGE = "ambition_render"
PROFILE = "profiling"

#: The adapter names of software rasterizers. A run on one of them repeats the
#: measurement that Packet 8 already has.
SOFTWARE_ADAPTERS = re.compile(r"llvmpipe|lavapipe|softpipe|swiftshader|microsoft basic render|warp", re.I)

ROW = re.compile(
    r"\[rigged_sprite_bench\] actors=(?P<actors>\d+) path=(?P<path>baked|rigged) "
    r"entities=(?P<entities>\d+) visible_sprites=(?P<visible>\d+)"
    r"(?: extracted_sprites=(?P<extracted>\d+) sprite_batches=(?P<batches>\d+))?"
    r" update_us median=(?P<median>[\d.]+) p95=(?P<p95>[\d.]+)"
)
ADAPTER = re.compile(r"\[rigged_sprite_bench\] adapter name=(?P<name>.*?) backend=(?P<backend>\S+) device_type=(?P<device_type>\S+)")


def run(cmd: list[str], log: Path) -> str:
    """Run `cmd` in the repository; write its output to `log`; stop on failure."""
    print("$", " ".join(cmd), flush=True)
    proc = subprocess.run(cmd, cwd=REPO, capture_output=True, text=True)
    log.write_text(proc.stdout + "\n--- stderr ---\n" + proc.stderr)
    if proc.returncode != 0:
        tail = "\n".join((proc.stdout + proc.stderr).splitlines()[-30:])
        sys.exit(f"command failed ({proc.returncode}); the log is {log}\n{tail}")
    return proc.stdout


def parse(stdout: str) -> tuple[list[dict], dict | None]:
    rows = []
    for match in ROW.finditer(stdout):
        row = {key: match.group(key) for key in ("actors", "path", "entities", "visible", "extracted", "batches", "median", "p95")}
        for key in ("actors", "entities", "visible", "extracted", "batches"):
            row[key] = int(row[key]) if row[key] is not None else None
        for key in ("median", "p95"):
            row[key] = float(row[key])
        rows.append(row)
    adapter = ADAPTER.search(stdout)
    return rows, (adapter.groupdict() if adapter else None)


def machine() -> dict:
    info = {
        "host": socket.gethostname(),
        "platform": platform.platform(),
        "python": platform.python_version(),
        "cpu": platform.processor() or platform.machine(),
    }
    try:
        info["git_rev"] = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=REPO, capture_output=True, text=True, check=True
        ).stdout.strip()
        info["git_dirty"] = bool(
            subprocess.run(["git", "status", "--porcelain", "--untracked-files=no"], cwd=REPO, capture_output=True, text=True).stdout.strip()
        )
    except (OSError, subprocess.CalledProcessError):
        info["git_rev"] = "unknown"
    if shutil.which("nvidia-smi"):
        smi = subprocess.run(["nvidia-smi", "--query-gpu=name,driver_version", "--format=csv,noheader"], capture_output=True, text=True)
        info["nvidia_smi"] = smi.stdout.strip()
    return info


def table(rows: list[dict], render: bool) -> list[str]:
    by_actors: dict[int, dict[str, dict]] = {}
    for row in rows:
        by_actors.setdefault(row["actors"], {})[row["path"]] = row
    if render:
        lines = [
            "| Actors | Frame ms median (baked / rigged) | p95 ms (baked / rigged) | Extracted sprites | Sprite batches |",
            "|---:|---:|---:|---:|---:|",
        ]
    else:
        lines = [
            "| Actors | Update µs median (baked / rigged) | Added µs | Entities (baked / rigged) | Visible sprites |",
            "|---:|---:|---:|---:|---:|",
        ]
    for actors in sorted(by_actors):
        pair = by_actors[actors]
        baked, rigged = pair.get("baked"), pair.get("rigged")
        if not (baked and rigged):
            continue
        if render:
            lines.append(
                f"| {actors} | {baked['median'] / 1000:.2f} / {rigged['median'] / 1000:.2f} "
                f"| {baked['p95'] / 1000:.2f} / {rigged['p95'] / 1000:.2f} "
                f"| {baked['extracted']} / {rigged['extracted']} | {baked['batches']} / {rigged['batches']} |"
            )
        else:
            lines.append(
                f"| {actors} | {baked['median']:.1f} / {rigged['median']:.1f} | {rigged['median'] - baked['median']:.1f} "
                f"| {baked['entities']} / {rigged['entities']} | {baked['visible']} / {rigged['visible']} |"
            )
    return lines


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--frames", type=int, default=600, help="measured frames per render run (default 600)")
    parser.add_argument("--ecs-frames", type=int, default=3000, help="measured frames per ECS run (default 3000)")
    parser.add_argument("--actors", default="1,10,50,100", help="actor counts (default 1,10,50,100)")
    parser.add_argument("--target", default="pirate_admiral", help="the character with a part flipbook")
    parser.add_argument("--out", type=Path, default=None, help="output directory")
    parser.add_argument("--allow-software", action="store_true", help="run on a software rasterizer (tests the script only)")
    args = parser.parse_args()

    if not shutil.which("cargo"):
        sys.exit("cargo is not on the PATH")
    stamp = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out = args.out or REPO / "target" / "rig_packet8" / f"{socket.gethostname()}_{stamp}"
    out.mkdir(parents=True, exist_ok=True)

    base = ["cargo", "run", "--quiet", "-p", PACKAGE, "--example", EXAMPLE, "--profile", PROFILE, "--"]
    common = ["--target", args.target, "--actors", args.actors]
    print("building the bench (the first build takes several minutes)...", flush=True)
    run(["cargo", "build", "-p", PACKAGE, "--example", EXAMPLE, "--profile", PROFILE], out / "build.log")

    runs = [
        ("ecs", [*common, "--frames", str(args.ecs_frames)], False),
        ("render_1_view", [*common, "--frames", str(args.frames), "--render", "--views", "1"], True),
        ("render_2_views", [*common, "--frames", str(args.frames), "--render", "--views", "2"], True),
        ("render_1_view_tiny", [*common, "--frames", str(args.frames), "--render", "--views", "1", "--tiny"], True),
    ]
    results: dict[str, dict] = {}
    adapter = None
    for name, extra, render in runs:
        stdout = run([*base, *extra], out / f"{name}.log")
        rows, run_adapter = parse(stdout)
        if render:
            if run_adapter is None:
                sys.exit(f"{name}: the bench printed no adapter line; the log is {out / (name + '.log')}")
            adapter = adapter or run_adapter
            software = SOFTWARE_ADAPTERS.search(run_adapter["name"]) or run_adapter["device_type"] == "Cpu"
            if software and not args.allow_software:
                sys.exit(
                    f"the render adapter is a software rasterizer ({run_adapter['name']}); this run would repeat the "
                    "measurement Packet 8 already has. Run on a hardware GPU, or pass --allow-software to test the script."
                )
        if not rows:
            sys.exit(f"{name}: no result rows were parsed; the log is {out / (name + '.log')}")
        results[name] = {"args": extra, "render": render, "rows": rows}

    report = {"machine": machine(), "adapter": adapter, "target": args.target, "runs": results}
    (out / "report.json").write_text(json.dumps(report, indent=2))

    lines = [
        f"# Rig Packet 8 on a hardware GPU — {report['machine']['host']}",
        "",
        f"- Adapter: {adapter['name']} ({adapter['backend']}, {adapter['device_type']})" if adapter else "- Adapter: unknown",
        f"- Platform: {report['machine']['platform']}; CPU: {report['machine']['cpu']}",
        f"- Revision: {report['machine'].get('git_rev')}{' (dirty)' if report['machine'].get('git_dirty') else ''}",
        f"- Target: `{args.target}`; render frames: {args.frames}; ECS frames: {args.ecs_frames}",
        "- Render frame times include a wait for the GPU at the end of each frame.",
        "",
    ]
    titles = {
        "ecs": "ECS only (no renderer)",
        "render_1_view": "Renderer, 1 view, 1280 × 720",
        "render_2_views": "Renderer, 2 views",
        "render_1_view_tiny": "Renderer, 1 view, tiny target (almost no fill)",
    }
    for name, result in results.items():
        lines += [f"## {titles[name]}", "", *table(result["rows"], result["render"]), ""]
    (out / "report.md").write_text("\n".join(lines))
    print("\n".join(lines))
    print(f"\nwrote {out / 'report.md'} and {out / 'report.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
