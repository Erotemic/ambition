#!/usr/bin/env python3
"""What does drawing a character from parts cost, against its baked sheet?
(`docs/planning/engine/mary-o-part-realization.md`.)

Two halves, written to the measurements repo so a finding is read back rather
than re-derived (`dev/ambition_dev_measurements/part_flipbook_cost.jsonl`):

* ``static`` — the published files, per character and per quality tier: the
  baked sheet's frames, page texels and PNG bytes; the part flipbook's parts,
  page texels and bytes, its draw table, and draws per frame. Machine
  independent: a property of the published assets at a commit.
* ``runtime`` — ``crates/ambition_render/examples/rigged_sprite_bench.rs`` per
  character: the update time of N bodies drawn from parts against the same N
  from the baked sheet (ECS side; ``--render`` adds Bevy's renderer). Machine
  DEPENDENT: each row carries the host and a ``comparable_key``, and only rows
  sharing a key may be compared.

    scripts/measure_part_flipbook_cost.py static                    # every character
    scripts/measure_part_flipbook_cost.py runtime --target noether --actors 1,10,50
    scripts/measure_part_flipbook_cost.py runtime --largest 8 --render
    scripts/measure_part_flipbook_cost.py report                    # latest rows, summarised

``static`` and ``runtime`` append rows and rewrite
``summaries/part-flipbook-cost.md``; ``--no-record`` prints only.
Run with the sprite renderer's interpreter (it reads flipbooks with the
renderer's own reader).
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import platform
import re
import secrets
import statistics
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "tools" / "ambition_sprite2d_renderer"))

import yaml  # noqa: E402
from PIL import Image  # noqa: E402

from ambition_sprite2d_renderer.authoring.part_flipbook import PartFlipbook  # noqa: E402

ASSETS = REPO / "crates" / "ambition_platformer2d_actor_monolith" / "assets"
TIERS = {"full": "sprites", "0_5x": "sprites_0_5x", "0_25x": "sprites_0_25x", "potato": "sprites_potato"}
LEDGER = REPO / "dev" / "ambition_dev_measurements" / "part_flipbook_cost.jsonl"
SUMMARY = REPO / "dev" / "ambition_dev_measurements" / "summaries" / "part-flipbook-cost.md"
SCHEMA = 1


def _git(*args: str, cwd: Path = REPO) -> str:
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True).stdout.strip()


def _envelope(kind: str, run_id: str, label: str) -> dict:
    renderer = REPO / "tools" / "ambition_sprite2d_renderer"
    return {
        "schema": SCHEMA,
        "kind": kind,
        "recorded_at": dt.datetime.now().astimezone().isoformat(timespec="seconds"),
        "commit": _git("rev-parse", "--short=12", "HEAD"),
        "dirty": bool(_git("status", "--porcelain", "--untracked-files=no")),
        "renderer_commit": _git("rev-parse", "--short=12", "HEAD", cwd=renderer),
        "run_id": run_id,
        "label": label,
    }


def _image_cost(directory: Path, names: list[str]) -> dict:
    """Texels and PNG bytes of the named images; a missing one is reported, not zero."""
    texels = size = 0
    missing = []
    for name in names:
        path = directory / name
        if not path.exists():
            missing.append(name)
            continue
        with Image.open(path) as image:
            texels += image.width * image.height
        size += path.stat().st_size
    return {"pages": len(names), "texels": texels, "png_bytes": size, "missing": missing}


def _sheet_pages(sheet: dict, target: str) -> list[str]:
    return sheet.get("images") or [sheet.get("image", f"{target}_spritesheet.png")]


def _sheet_frames(sheet: dict) -> tuple[int, int]:
    """(frames, texels inside the frames' trimmed rects)."""
    if "rows" in sheet:
        rects = [rect for row in sheet["rows"] for rect in row["rects"]]
    else:
        rects = [frame for name in sheet.get("animation_order", []) for frame in sheet["animations"][name]["frames"]]
    return len(rects), sum(int(rect["w"]) * int(rect["h"]) for rect in rects)


def _flipbook_pages(ron: Path) -> list[str]:
    text = ron.read_text()
    block = re.search(r"pages: \[([^\]]*)\]", text)
    return re.findall(r'"([^"]+\.png)"', block.group(1)) if block else []


def static_rows(run_id: str, label: str) -> list[dict]:
    rows = []
    full = ASSETS / TIERS["full"]
    for sheet_yaml in sorted(full.glob("*_spritesheet.yaml")):
        target = sheet_yaml.name[: -len("_spritesheet.yaml")]
        sheet = yaml.safe_load(sheet_yaml.read_text())
        frames, rect_texels = _sheet_frames(sheet)
        ron = full / f"{target}_parts.ron"
        row = _envelope("part_flipbook_static", run_id, label)
        row.update(
            {
                "target": target,
                "frame_size": [int(sheet["frame_width"]), int(sheet["frame_height"])],
                "frames": frames,
                "sheet": {"rect_texels": rect_texels, "full": _image_cost(full, _sheet_pages(sheet, target))},
                "parts": None,
            }
        )
        if ron.exists():
            flipbook = PartFlipbook.from_published(ron, None if "placement:" in ron.read_text() else "continuous")
            per_frame = [len(draws) for _ms, frames_ in flipbook.clips.values() for draws in frames_]
            row["parts"] = {
                "placement": flipbook.placement,
                "parts": len(flipbook.parts),
                "tight_texels": flipbook.tight_texels(),
                "clips": len(flipbook.clips),
                "baked_clips": len(flipbook.baked_clips),
                "draws": sum(per_frame),
                "draws_per_frame": {
                    "mean": round(statistics.fmean(per_frame), 2) if per_frame else None,
                    "p95": sorted(per_frame)[int(0.95 * (len(per_frame) - 1))] if per_frame else None,
                    "max": max(per_frame, default=None),
                },
                "ron_bytes": ron.stat().st_size,
                "full": _image_cost(full, _flipbook_pages(ron)),
            }
        # Every quality tier the sheet and the flipbook publish.
        for tier, folder in TIERS.items():
            if tier == "full":
                continue
            directory = ASSETS / folder
            tier_yaml = directory / sheet_yaml.name
            if tier_yaml.exists():
                tier_sheet = yaml.safe_load(tier_yaml.read_text())
                row["sheet"][tier] = _image_cost(directory, _sheet_pages(tier_sheet, target))
            tier_ron = directory / f"{target}_parts.ron"
            if row["parts"] is not None and tier_ron.exists():
                row["parts"][tier] = _image_cost(directory, _flipbook_pages(tier_ron))
        rows.append(row)
    return rows


def _host() -> dict:
    cpu = ""
    try:
        cpu = next(line.split(":", 1)[1].strip() for line in open("/proc/cpuinfo") if line.startswith("model name"))
    except (OSError, StopIteration):
        pass
    return {"machine": platform.node(), "cpu": cpu, "logical_cpus": os.cpu_count(), "kernel": platform.release()}


def runtime_rows(targets: list[str], actors: str, frames: int, render: bool, run_id: str, label: str) -> list[dict]:
    host = _host()
    args = ["--frames", str(frames), "--actors", actors] + (["--render"] if render else [])
    build = ["cargo", "build", "-q", "-p", "ambition_render", "--example", "rigged_sprite_bench", "--profile", "profiling"]
    subprocess.run(build, cwd=REPO, check=True)
    binary = REPO / "target" / "profiling" / "examples" / "rigged_sprite_bench"
    rows = []
    for target in targets:
        out = subprocess.run([str(binary), "--target", target, *args], cwd=REPO, capture_output=True, text=True)
        text = out.stdout + out.stderr
        if out.returncode != 0:
            print(f"{target}: bench failed ({out.returncode})\n{text[-800:]}", file=sys.stderr)
            continue
        adapter = re.search(r"adapter name=([^\n,]+)", text)
        renderer = ("software" if adapter and "llvmpipe" in adapter.group(1).lower() else "hardware") if render else "none"
        comparable = {
            "bench": "rigged_sprite_bench",
            "frames": frames,
            "render": render,
            "renderer": renderer,
            "adapter": adapter.group(1).strip() if adapter else None,
            "cargo_profile": "profiling",
            "machine": host["machine"],
            "cpu": host["cpu"],
            "logical_cpus": host["logical_cpus"],
        }
        head = re.search(r"max_draws=(\d+) parts=(\d+)", text)
        for count in sorted({int(a) for a in re.findall(r"actors=(\d+) path=", text)}):
            paths = {}
            for path in ("baked", "rigged"):
                match = re.search(
                    rf"actors={count} path={path} entities=(\d+) visible_sprites=(\d+)"
                    rf"(?: extracted_sprites=(\d+) sprite_batches=(\d+))? update_us median=([\d.]+) p95=([\d.]+)",
                    text,
                )
                if match:
                    paths[path] = {
                        "entities": int(match[1]),
                        "visible_sprites": int(match[2]),
                        "extracted_sprites": int(match[3]) if match[3] else None,
                        "sprite_batches": int(match[4]) if match[4] else None,
                        "update_us_median": float(match[5]),
                        "update_us_p95": float(match[6]),
                    }
            row = _envelope("part_flipbook_runtime", run_id, label)
            row.update(
                {
                    "target": target,
                    "actors": count,
                    "max_draws": int(head[1]) if head else None,
                    "parts": int(head[2]) if head else None,
                    "baked": paths.get("baked"),
                    "rigged": paths.get("rigged"),
                    "host": host,
                    "comparable_fields": comparable,
                    "comparable_key": hashlib.sha1(json.dumps(comparable, sort_keys=True).encode()).hexdigest()[:12],
                }
            )
            rows.append(row)
    return rows


def _ratio(a, b):
    return None if not a or not b else round(a / b, 3)


def summarise(rows: list[dict]) -> str:
    """The latest run of each kind, as markdown."""
    lines = ["# Part flipbooks against baked sheets: what parts cost", ""]
    lines.append(
        "Written by `scripts/measure_part_flipbook_cost.py` from the latest rows of "
        "`part_flipbook_cost.jsonl`. Static numbers are a property of the published assets at a "
        "commit; runtime numbers belong to a machine (compare only rows that share a `comparable_key`)."
    )
    lines.append("")
    static = [row for row in rows if row["kind"] == "part_flipbook_static"]
    if static:
        latest = max(row["run_id"] for row in static if row["recorded_at"] == max(r["recorded_at"] for r in static))
        run = [row for row in static if row["run_id"] == latest]
        drawn = [row for row in run if row["parts"]]
        head = run[0]
        lines.append(f"## Static census — {head['recorded_at']}, commit `{head['commit']}`{' (dirty)' if head['dirty'] else ''}, renderer `{head['renderer_commit']}`")
        lines.append("")
        sheet_bytes = sum(row["sheet"]["full"]["png_bytes"] for row in drawn)
        part_bytes = sum(row["parts"]["full"]["png_bytes"] for row in drawn)
        sheet_texels = sum(row["sheet"]["full"]["texels"] for row in drawn)
        part_texels = sum(row["parts"]["full"]["texels"] for row in drawn)
        lines.append(f"- **{len(drawn)} of {len(run)}** published sheets have a part flipbook; the rest: "
                     + ", ".join(sorted(row["target"] for row in run if not row["parts"])) + ".")
        lines.append(f"- Full resolution, those {len(drawn)}: sheet pages **{sheet_bytes / 2**20:.1f} MiB / {sheet_texels / 1e6:.1f} MTexel**, "
                     f"part pages **{part_bytes / 2**20:.1f} MiB / {part_texels / 1e6:.1f} MTexel** "
                     f"(**{_ratio(part_bytes, sheet_bytes)}x** the bytes, **{_ratio(part_texels, sheet_texels)}x** the texels), "
                     f"plus **{sum(row['parts']['ron_bytes'] for row in drawn) / 2**20:.1f} MiB** of draw tables.")
        for tier in ("0_5x", "0_25x", "potato"):
            pairs = [(row["sheet"].get(tier), row["parts"].get(tier)) for row in drawn]
            pairs = [(s, p) for s, p in pairs if s and p]
            if pairs:
                sb = sum(s["png_bytes"] for s, _ in pairs)
                pb = sum(p["png_bytes"] for _, p in pairs)
                lines.append(f"- Tier `{tier}` ({len(pairs)} characters with both): sheet {sb / 2**20:.1f} MiB, parts {pb / 2**20:.1f} MiB (**{_ratio(pb, sb)}x**).")
        per_frame = [row["parts"]["draws_per_frame"]["max"] or 0 for row in drawn]
        lines.append(f"- Draws per frame: median of characters' max **{statistics.median(per_frame)}**, worst **{max(per_frame)}** "
                     f"({max(drawn, key=lambda r: r['parts']['draws_per_frame']['max'] or 0)['target']}).")
        lines.append("")
        lines.append("| character | frames | sheet MiB | parts MiB | bytes x | texels x | parts | draws/frame mean / max |")
        lines.append("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
        for row in sorted(drawn, key=lambda r: -r["sheet"]["full"]["png_bytes"]):
            s, p = row["sheet"]["full"], row["parts"]["full"]
            lines.append(
                f"| {row['target']} | {row['frames']} | {s['png_bytes'] / 2**20:.2f} | {p['png_bytes'] / 2**20:.2f} | "
                f"{_ratio(p['png_bytes'], s['png_bytes'])} | {_ratio(p['texels'], s['texels'])} | {row['parts']['parts']} | "
                f"{row['parts']['draws_per_frame']['mean']} / {row['parts']['draws_per_frame']['max']} |"
            )
        lines.append("")
    runtime = [row for row in rows if row["kind"] == "part_flipbook_runtime"]
    if runtime:
        lines.append("## Runtime (machine-dependent)")
        lines.append("")
        lines.append("| recorded | key | target | actors | render | baked µs (median) | parts µs (median) | parts − baked | sprites baked / parts |")
        lines.append("| --- | --- | --- | ---: | --- | ---: | ---: | ---: | --- |")
        for row in runtime[-60:]:
            b, r = row["baked"] or {}, row["rigged"] or {}
            delta = round(r["update_us_median"] - b["update_us_median"], 1) if b and r else None
            lines.append(
                f"| {row['recorded_at'][:16]} | `{row['comparable_key']}` | {row['target']} | {row['actors']} | "
                f"{row['comparable_fields']['renderer']} | {b.get('update_us_median')} | {r.get('update_us_median')} | {delta} | "
                f"{b.get('visible_sprites')} / {r.get('visible_sprites')} |"
            )
        lines.append("")
    hall = [row for row in rows if row["kind"] == "part_flipbook_hall_load"]
    if hall:
        lines.append("## Room load, parts against baked (`scripts/measure_hall_load_parts_ab.py`; machine-dependent)")
        lines.append("")
        lines.append("Medians of reps 2..N (rep 1 warms the page cache). `last insert` is when the last character "
                     "image landed, on the app's clock.")
        lines.append("")
        lines.append("| recorded | key | room | arm | char. images | MP | decode ms | last insert s | live decodes | spikes (worst ms) | wall s | RSS MB |")
        lines.append("| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |")
        for row in hall[-20:]:
            m = row["median"]
            lines.append(
                f"| {row['recorded_at'][:16]} | `{row['comparable_key']}` | {row['room']} | {row['arm']} | "
                f"{m['character_images']} | {m['character_megapixels']} | {m['character_decode_ms']} | "
                f"{m['last_character_insert_s']} | {m['decoded_during_gameplay']} | {m['frame_spikes']} ({m['worst_spike_ms']}) | "
                f"{m['wall_s']} | {m['max_rss_mb']} |"
            )
        lines.append("")
    return "\n".join(lines) + "\n"


def _read_ledger() -> list[dict]:
    if not LEDGER.exists():
        return []
    return [json.loads(line) for line in LEDGER.read_text().splitlines() if line.strip()]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("static", "runtime"):
        p = sub.add_parser(name)
        p.add_argument("--label", default="")
        p.add_argument("--no-record", action="store_true", help="print, append nothing")
        if name == "runtime":
            p.add_argument("--target", action="append", default=[])
            p.add_argument("--largest", type=int, default=0, help="the N characters with the largest sheets")
            p.add_argument("--actors", default="1,10,50")
            p.add_argument("--frames", type=int, default=600)
            p.add_argument("--render", action="store_true")
    sub.add_parser("report")
    args = parser.parse_args()
    if args.command == "report":
        print(summarise(_read_ledger()), end="")
        return 0
    run_id = secrets.token_hex(6)
    if args.command == "static":
        rows = static_rows(run_id, args.label)
    else:
        targets = list(args.target)
        if args.largest:
            drawn = [row for row in static_rows(run_id, "") if row["parts"]]
            drawn.sort(key=lambda r: -r["sheet"]["full"]["png_bytes"])
            targets += [row["target"] for row in drawn[: args.largest] if row["target"] not in targets]
        if not targets:
            parser.error("runtime needs --target or --largest")
        rows = runtime_rows(targets, args.actors, args.frames, args.render, run_id, args.label)
    if not rows:
        print("measured nothing", file=sys.stderr)
        return 2
    if args.no_record:
        print(summarise(rows), end="")
        return 0
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    with LEDGER.open("a") as ledger:
        for row in rows:
            ledger.write(json.dumps(row, sort_keys=True) + "\n")
    SUMMARY.parent.mkdir(parents=True, exist_ok=True)
    SUMMARY.write_text(summarise(_read_ledger()))
    print(f"appended {len(rows)} row(s) to {LEDGER.relative_to(REPO)}; wrote {SUMMARY.relative_to(REPO)}")
    print(summarise(rows).split("\n| character")[0])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
