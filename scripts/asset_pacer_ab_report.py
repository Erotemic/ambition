#!/usr/bin/env python3
"""Summarise the runs written by `scripts/asset_pacer_ab.sh` into one table.

⛔ THE ADAPTER AND THE QUALITY TIER GO IN THE TABLE, NOT IN THE PROSE AROUND IT.
A capture on a Cpu adapter seeds visual quality to `potato` automatically, and a
hall number recorded without its tier has already been published once in this
repository and had to be retaken. This script refuses to print a row whose
adapter or tier it could not read.

Usage::

    python3 scripts/asset_pacer_ab_report.py [OUTDIR]
"""

from __future__ import annotations

import re
import statistics
import sys
from pathlib import Path

ARMS = ("default", "pacer64", "cpucopy")
ARM_LABEL = {
    "default": "default (no lever)",
    "pacer64": "`AMBITION_RENDER_ASSET_MB_PER_FRAME=64`",
    "cpucopy": "`AMBITION_IMAGES_RENDER_WORLD_ONLY=0`",
}

# `[image-census] ... total 235 images, 29.5MP, 118.1MB resident ... insert→gpu
# p50 117ms max 547ms | awaiting gpu 0 | ... | never drawn 215 (28.6MP: ...)`
TOTALS = re.compile(r"total (\d+) images, ([\d.]+)MP, ([\d.]+)MB resident")
GPU = re.compile(r"insert→gpu p50 (\S+) max (\S+)")
AWAITING = re.compile(r"awaiting gpu (\d+)")
NEVER = re.compile(r"never drawn (\d+) \(([\d.]+)MP")
SPIKE = re.compile(r"\[frame-spike\]\s+[\d.]+s\s+([\d.]+)ms")
RSS = re.compile(r"Maximum resident set size \(kbytes\): (\d+)")
WALL = re.compile(r"Elapsed \(wall clock\) time.*?: (?:(\d+):)?(\d+):([\d.]+)")
# ⛔ The adapter string has its OWN parentheses — `llvmpipe (LLVM 20.1.2, 256
# bits)` — so `\(([^)]+)\)` truncates it at the inner one. Anchor on the end
# of the clause instead.
ADAPTER = re.compile(r"visual quality seeded to `(\w+)` for a (\w+) adapter \((.+?)\); this is")


def ms(raw: str) -> float | None:
    """`547ms` -> 547.0; `-` (no images in that window) -> None.

    ⛔ NOT `rstrip("ms")` — `rstrip` takes a SET of characters, so it would eat
    the trailing digits of `144ms` down to `144m` and then fail to parse. The
    suffix has to come off by length.
    """
    if raw in ("-", "?"):
        return None
    if raw.endswith("ms"):
        return float(raw[:-2])
    if raw.endswith("s"):
        return float(raw[:-1]) * 1000.0
    return float(raw)


def read_run(path: Path) -> dict | None:
    text = path.read_text(errors="replace")
    census = [ln for ln in text.splitlines() if "[image-census]" in ln]
    if not census:
        return None

    # Totals come from the LAST line (the exit flush); the insert→gpu max is the
    # worst across every window, because the final window usually adds nothing
    # and prints `max -`.
    last = census[-1]
    totals = TOTALS.search(last)
    never = NEVER.search(last)
    gpu_maxes = [m for m in (ms(GPU.search(c).group(2)) for c in census if GPU.search(c)) if m is not None]
    awaiting = [int(a.group(1)) for c in census if (a := AWAITING.search(c))]
    spikes = sorted((float(m.group(1)) for m in SPIKE.finditer(text)), reverse=True)
    rss = RSS.search(text)
    wall = WALL.search(text)
    seed = ADAPTER.search(text)

    return {
        "images": int(totals.group(1)) if totals else None,
        "megapixels": float(totals.group(2)) if totals else None,
        "resident_mb": float(totals.group(3)) if totals else None,
        "never_drawn": int(never.group(1)) if never else None,
        "never_drawn_mp": float(never.group(2)) if never else None,
        "insert_to_gpu_max": max(gpu_maxes) if gpu_maxes else None,
        "awaiting_max": max(awaiting) if awaiting else None,
        "spikes": spikes[:3],
        "spike_count": len(spikes),
        "rss_mb": int(rss.group(1)) / 1024.0 if rss else None,
        "wall_s": (int(wall.group(1) or 0) * 3600 + int(wall.group(2)) * 60 + float(wall.group(3))) if wall else None,
        "tier": seed.group(1) if seed else None,
        "adapter_kind": seed.group(2) if seed else None,
        "adapter": seed.group(3) if seed else None,
    }


def fmt(value, spec="{:.1f}") -> str:
    return "—" if value is None else spec.format(value)


def main() -> int:
    outdir = Path(sys.argv[1] if len(sys.argv) > 1 else "target/tmp/asset-pacer-ab")
    runs: dict[str, list[dict]] = {}
    for arm in ARMS:
        for log in sorted(outdir.glob(f"{arm}-rep*.log")):
            if (run := read_run(log)) is not None:
                runs.setdefault(arm, []).append(run)

    if not runs:
        print(f"no parsable runs under {outdir}", file=sys.stderr)
        return 1

    every = [r for arm in runs.values() for r in arm]
    adapters = {r["adapter"] for r in every if r["adapter"]}
    tiers = {r["tier"] for r in every if r["tier"]}
    if not adapters or not tiers:
        print("refusing to report: no run named its adapter and quality tier", file=sys.stderr)
        return 2
    if len(adapters) > 1 or len(tiers) > 1:
        print(f"refusing to report: arms disagree on adapter {adapters} / tier {tiers}", file=sys.stderr)
        return 2

    kinds = {r["adapter_kind"] for r in every if r["adapter_kind"]}
    print(f"adapter    {adapters.pop()}  ({kinds.pop()} adapter)")
    print(f"tier       {tiers.pop()}  (seeded automatically for this adapter, not chosen)")
    print(f"reps       {min(len(v) for v in runs.values())} per arm, interleaved")
    print()

    head = ("| arm | resident MB | images (MP) | never drawn | insert→gpu max | awaiting gpu | "
            "3 worst spikes (ms) | spikes | max RSS MB | wall s |")
    print(head)
    print("|" + "---|" * 10)
    for arm in ARMS:
        if not (rs := runs.get(arm)):
            continue
        med = lambda key, spec="{:.1f}": fmt(  # noqa: E731
            statistics.median([r[key] for r in rs if r[key] is not None]) if any(r[key] is not None for r in rs) else None, spec)
        worst = sorted((s for r in rs for s in r["spikes"]), reverse=True)[:3]
        print(f"| {ARM_LABEL[arm]} | {med('resident_mb')} | {med('images','{:.0f}')} ({med('megapixels')}) | "
              f"{med('never_drawn','{:.0f}')} ({med('never_drawn_mp')}MP) | {med('insert_to_gpu_max')}ms | "
              f"{med('awaiting_max','{:.0f}')} | {', '.join(f'{s:.0f}' for s in worst) or '—'} | "
              f"{med('spike_count','{:.0f}')} | {med('rss_mb','{:.0f}')} | {med('wall_s','{:.2f}')} |")

    print()
    print("Per-run spread (the reason the arms are interleaved):")
    for arm in ARMS:
        for i, r in enumerate(runs.get(arm, []), 1):
            print(f"  {arm:8s} rep{i}  resident {fmt(r['resident_mb'])}MB  "
                  f"insert→gpu max {fmt(r['insert_to_gpu_max'])}ms  awaiting {r['awaiting_max']}  "
                  f"spikes {r['spike_count']}  RSS {fmt(r['rss_mb'],'{:.0f}')}MB  wall {fmt(r['wall_s'],'{:.2f}')}s")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
