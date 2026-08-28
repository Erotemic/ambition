#!/usr/bin/env python3
"""Rank Tracy zones (Bevy systems and render passes) without the Tracy GUI.

Two inputs, both optional:

* `tracy_zones.csv` — `tracy-csvexport` aggregate: one row per zone with
  whole-session totals. Always produced when the export succeeds.
* `tracy_zone_instances.csv` — `tracy-csvexport --unwrap`: one row per zone
  INSTANCE with its start time. Present only where the installed exporter
  supports it.

Whole-session totals are necessary and not sufficient: a system that costs 20ms
for six seconds of a four-minute session is invisible in a total and is exactly
what somebody profiling a stutter is looking for. When instances are available
this also writes per-window tables, and names the worst window.

Column names differ between Tracy releases, so every field is looked up through
a candidate list rather than by a fixed index.
"""

from __future__ import annotations

import csv
import os
import sys

# Candidate column names, most specific first.
NAME_COLUMNS = ("name", "zone", "zone_name")
TOTAL_COLUMNS = ("total_ns", "total", "sum_ns", "total_time")
COUNT_COLUMNS = ("counts", "count", "calls")
MEAN_COLUMNS = ("mean_ns", "mean", "average_ns")
MAX_COLUMNS = ("max_ns", "max", "maximum_ns")
MIN_COLUMNS = ("min_ns", "min")
START_COLUMNS = ("ns_since_start", "start_ns", "start", "time_ns", "timestamp")
DURATION_COLUMNS = ("exec_time_ns", "duration_ns", "exec_time", "duration", "self_time_ns")


def pick(row: dict, names: tuple[str, ...]) -> str | None:
    for name in names:
        value = row.get(name)
        if value not in (None, ""):
            return value
    return None


def number(row: dict, names: tuple[str, ...]) -> float:
    value = pick(row, names)
    if value is None:
        return 0.0
    try:
        return float(value)
    except ValueError:
        return 0.0


def read_csv(path: str) -> list[dict]:
    """Read a `tracy-csvexport` table, repairing its unquoted `name` column.

    ⛔ `tracy-csvexport` writes the zone name RAW, and a Bevy system name is a
    Rust path that routinely contains commas:

        check_conditions{name="Assets<ExtendedMaterial<StandardMaterial, ForwardDecalMaterialExt>>::track_assets"}

    A plain `csv.DictReader` shifts every later column on such a row, so
    `exec_time_ns` picks up part of a file path and the windowed totals come
    out thousands of times too large -- wrong, and wrong in a way that looks
    like a finding. The trailing columns are fixed in number, so split from the
    RIGHT and give the overflow back to the name.
    """
    if not os.path.exists(path):
        return []
    with open(path, newline="", encoding="utf-8", errors="replace") as handle:
        header_line = handle.readline().rstrip("\n")
        if not header_line:
            return []
        columns = header_line.split(",")
        width = len(columns)
        rows = []
        for line in handle:
            line = line.rstrip("\n")
            if not line:
                continue
            parts = line.split(",")
            if len(parts) > width:
                overflow = len(parts) - width
                parts = [",".join(parts[: overflow + 1])] + parts[overflow + 1 :]
            elif len(parts) < width:
                parts += [""] * (width - len(parts))
            rows.append(dict(zip(columns, parts)))
        return rows


def aggregate_table(rows: list[dict], limit: int = 40) -> list[str]:
    ranked = sorted(rows, key=lambda row: number(row, TOTAL_COLUMNS), reverse=True)
    lines = [
        "```text",
        f'{"total_ms":>10} {"count":>9} {"mean_us":>9} {"max_us":>9}  zone',
    ]
    for row in ranked[:limit]:
        total_ns = number(row, TOTAL_COLUMNS)
        count = number(row, COUNT_COLUMNS)
        mean_ns = number(row, MEAN_COLUMNS) or (total_ns / count if count else 0.0)
        max_ns = number(row, MAX_COLUMNS)
        name = pick(row, NAME_COLUMNS) or "<unnamed>"
        lines.append(
            f"{total_ns / 1e6:10.1f} {count:9.0f} {mean_ns / 1e3:9.1f} {max_ns / 1e3:9.1f}  {name[:90]}"
        )
    lines.append("```")
    return lines


def window_rows(instances: list[dict], window_s: float) -> list[dict]:
    """Per-(window, zone) totals from unwrapped zone instances."""
    buckets: dict[tuple[int, str], dict] = {}
    for row in instances:
        start_ns = number(row, START_COLUMNS)
        duration_ns = number(row, DURATION_COLUMNS)
        name = pick(row, NAME_COLUMNS) or "<unnamed>"
        index = int(start_ns / 1e9 / window_s)
        key = (index, name)
        bucket = buckets.setdefault(
            key,
            {
                "window_start_s": f"{index * window_s:.3f}",
                "window_end_s": f"{(index + 1) * window_s:.3f}",
                "zone": name,
                "_total_ns": 0.0,
                "_max_ns": 0.0,
                "_count": 0,
            },
        )
        bucket["_total_ns"] += duration_ns
        bucket["_max_ns"] = max(bucket["_max_ns"], duration_ns)
        bucket["_count"] += 1
    out = []
    for bucket in buckets.values():
        count = bucket["_count"]
        out.append(
            {
                "window_start_s": bucket["window_start_s"],
                "window_end_s": bucket["window_end_s"],
                "zone": bucket["zone"],
                "total_ms": f"{bucket['_total_ns'] / 1e6:.3f}",
                "count": count,
                "mean_us": f"{(bucket['_total_ns'] / count) / 1e3:.2f}" if count else "0",
                "max_us": f"{bucket['_max_ns'] / 1e3:.2f}",
            }
        )
    out.sort(key=lambda row: (float(row["window_start_s"]), -float(row["total_ms"])))
    return out


def main(argv: list[str]) -> int:
    if len(argv) not in (2, 3):
        print("usage: tracy_zone_report.py <bundle-dir> [census-hz]", file=sys.stderr)
        return 2
    out_dir = argv[1]
    try:
        census_hz = float(argv[2]) if len(argv) == 3 else 1.0
    except ValueError:
        census_hz = 1.0
    # Match the census cadence so a Tracy window and a camera-count row cover
    # the same interval; anything faster than a second is noise for this table.
    window_s = max(1.0, 1.0 / census_hz if census_hz > 0 else 1.0)

    aggregate = read_csv(os.path.join(out_dir, "tracy_zones.csv"))
    instances = read_csv(os.path.join(out_dir, "tracy_zone_instances.csv"))

    lines = [
        "# Tracy zones (per-Bevy-system and per-render-pass timings)",
        "",
        "`perf` cannot produce this: a Bevy system is not a native symbol, and a",
        "render pass is a graph node rather than a function. Counts matter as much",
        "as totals -- a cheap zone entered ten thousand times is a scheduling",
        "problem, not a slow function.",
        "",
    ]

    if aggregate:
        lines += ["## Whole session, ranked by total time", ""]
        lines += aggregate_table(aggregate)
        lines.append("")
    else:
        lines += [
            "## Whole session",
            "",
            "No aggregate zone export. `tracy_zones.csv` is absent or empty.",
            "",
        ]

    if instances:
        windows = window_rows(instances, window_s)
        with open(os.path.join(out_dir, "tracy_zone_windows.csv"), "w", newline="", encoding="utf-8") as handle:
            writer = csv.DictWriter(
                handle,
                fieldnames=["window_start_s", "window_end_s", "zone", "total_ms", "count", "mean_us", "max_us"],
            )
            writer.writeheader()
            writer.writerows(windows)

        by_window: dict[str, float] = {}
        for row in windows:
            by_window[row["window_start_s"]] = by_window.get(row["window_start_s"], 0.0) + float(row["total_ms"])
        worst = sorted(by_window.items(), key=lambda item: -item[1])[:5]
        lines += [
            f"## Worst {window_s:.0f}s windows",
            "",
            "Ranked by total zone time inside the window. Full per-window table:",
            "`tracy_zone_windows.csv`.",
            "",
            "```text",
        ]
        for start, total_ms in worst:
            lines.append(f"window {float(start):8.1f}s  {total_ms:10.1f} ms of zone time")
            # `windows` is already sorted by (start, -total_ms), so the first
            # eight rows for a window are its eight most expensive zones.
            in_window = [row for row in windows if row["window_start_s"] == start][:8]
            for row in in_window:
                lines.append(
                    f"    {float(row['total_ms']):9.2f} ms  x{int(row['count']):<6d} {row['zone'][:70]}"
                )
        lines += ["```", ""]
    else:
        lines += [
            "## Time-resolved zones",
            "",
            "NOT COLLECTED. This `tracy-csvexport` does not support `--unwrap`, so",
            "only whole-session zone totals exist. A slow interval inside a long",
            "session is diluted in the table above; use the per-window `perf`",
            "reports in `perf_windows/` and the census CSVs to locate it, then",
            "re-run a short bounded capture over that phase.",
            "",
        ]

    with open(os.path.join(out_dir, "tracy_summary.md"), "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
