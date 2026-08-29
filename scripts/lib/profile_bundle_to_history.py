#!/usr/bin/env python3
"""Turn one profiling bundle into one appendable row of runtime history.

`scripts/profile_desktop.sh` writes a bundle that answers "why was THIS run
slow". It cannot answer "is the frame slower than it was last week", because the
bundle is hundreds of megabytes of trace that gets deleted. This module reads
the small artifacts the bundle already derived — `metadata.json`, the census
CSVs, the Tracy zone export, the perf thread report — and normalizes them into
one JSON object that stays true after the trace is gone.

Two contracts hold the whole design up:

⛔ **A run that died before the game started is not a runtime measurement.**
   A failed warm build still produces a bundle directory with metadata and a
   host environment in it. Appending a row for one would put a commit in the
   series with no frame time and every structural count at zero, and the next
   `latest --against` would read it as a spectacular improvement. `assess_run`
   refuses and nothing is written.

⭐ **Two rows may be compared only if their comparability fields are equal.**
   Frame time is not a property of a commit; it is a property of a commit *on a
   scenario, on a machine, at a renderer, under a set of instruments*. Those
   dimensions are hashed into `comparable_key`, stored expanded next to it in
   `comparable_fields`, and `scripts/perf_history.py` refuses across a
   mismatch and names the field. A lavapipe run and a hardware-GPU run get
   different keys; so do a Tracy run and an unprofiled one, which on this
   project differ by ~9x.

Missing optional instruments degrade to nulls. A bundle with no Tracy, no perf
and no GPU is still a legitimate frame measurement of the thing it measured.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import time
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import measurement_paths  # noqa: E402
from profile_bundle_summary import ADAPTER, SOFTWARE_MARKERS, Bundle  # noqa: E402

SCHEMA = 1
KIND = "runtime_frame"

LEDGER = measurement_paths.RUNTIME_LEDGER

# ⭐ The scenario's CONTENT version, and the reason it is a hand-kept constant
# rather than something derived. Anything derived from the scene would move with
# the very numbers being measured, so a room gaining fifty entities would look
# like a regression AND silently re-key the row that could have proved it.
# BUMP THIS when a scenario's authored content changes enough that its old rows
# are no longer the same workload. A bump is a code edit a reviewer sees; a
# forgotten bump is a silent invalid comparison.
SCENARIO_VERSIONS = {
    "sandbox": 1,
}
DEFAULT_SCENARIO_VERSION = 1

# The dimensions a frame time is only comparable WITHIN. A difference in any one
# of these makes two rows describe different experiments, and the query tool
# refuses rather than subtracting them.
COMPARABILITY_FIELDS = (
    "scenario.id",
    "scenario.version",
    "scenario.headless",
    "build.cargo_profile",
    "build.features",
    "build.package",
    "build.binary",
    "host.machine_id",
    "host.cpu_model",
    "host.logical_cpus",
    "gpu.rendering",
    "gpu.adapter",
    "display.resolution",
    "instruments.tracy",
    "instruments.perf",
    "instruments.census",
    "instruments.census_hz",
)

# ⚠ Real threats to comparability that are nonetheless too coarse to split the
# series on: a kernel upgrade or a rustc bump would orphan every earlier row.
# The query tool prints these as warnings beside a comparison instead.
ADVISORY_FIELDS = (
    "host.kernel",
    "host.mem_total_kb",
    "build.rustc",
    "build.rust_target",
    "scenario.ticks",
    "run.frames",
    "instruments.profiler_cycle_share_pct",
)

# perf.data on a bundle whose game never ran is still tens of kilobytes of
# headers, so file size proves nothing. These are the artifacts that only exist
# because the process emitted frames.
FRAME_SOURCES = ("frame_times.csv", "frame_windows.csv")

CPU_MODEL = re.compile(r"^model name\s*:\s*(.+)$", re.M)
LOGICAL_CPUS = re.compile(r"^logical_cpus=(\d+)$", re.M)
MEM_TOTAL = re.compile(r"^MemTotal:\s+(\d+) kB$", re.M)
NVIDIA_ROW = re.compile(r"^(NVIDIA [^,]+),\s*([0-9.]+),", re.M)
ADAPTER_FIELD = re.compile(r'(\w+):\s*(?:"([^"]*)"|(\w+))')

# Bevy's Tracy zones are named by kind with the system path in braces, so the
# structural counts are a prefix tally and need no trace parsing.
ZONE_PREFIXES = {
    "system{": "system_executions",
    "check_conditions{": "run_condition_evaluations",
    "system_commands{": "command_flushes",
}


class BundleDiedBeforeStart(Exception):
    """The bundle records a build or a launch that never reached a frame."""


# ── reading the bundle ────────────────────────────────────────────────────


def number(value, default=None):
    try:
        return float(value)
    except (TypeError, ValueError):
        return default


def status_code(bundle: Bundle, name: str) -> int | None:
    """A `*.status` file's exit code, or None when it is absent or prose.

    The front door writes `skipped` and a whole sentence into `warm-build.status`
    for the two cases where no build was attempted, so a non-integer here means
    "no build ran", NOT "the build failed".
    """
    raw = bundle.read(f"{name}.status").strip()
    try:
        return int(raw)
    except ValueError:
        return None


def stamped_lines(bundle: Bundle) -> int:
    log = bundle.read("game-stderr-stamped.txt") or bundle.read("game-stdout-stamped.txt")
    return sum(1 for line in log.splitlines() if line.startswith("["))


def frame_rows(bundle: Bundle) -> tuple[list[dict], str | None]:
    """The best frame series in the bundle, and which file it came from.

    `frame_times.csv` is the census series and shares its clock with every other
    census row. `frame_windows.csv` is the always-on 5s summary, which survives
    `--no-census` and carries no mean or min.
    """
    rows = bundle.rows("frame_times.csv")
    if rows:
        return rows, "frame_times.csv"
    rows = bundle.rows("frame_windows.csv")
    if rows:
        return rows, "frame_windows.csv"
    return [], None


def assess_run(bundle: Bundle) -> None:
    """Raise unless this bundle holds a game that actually ran and drew frames.

    ⛔ The three shapes of a dead run, all of which leave a plausible-looking
    bundle directory behind:

    * the warm build failed — `warm-build.status` holds a nonzero exit code, and
      the launch that follows it fails the same way (seen live: status 101, a
      cargo feature error, 449 bytes of stamped log and no census at all);
    * the build was interrupted — no status file was ever written and the
      bundle holds only `metadata.txt`, `host-environment.txt` and the build's
      own streams;
    * the process launched but produced no frame series — an attach-mode
      capture that never sees the game's stdio, or a launch that aborted during
      App construction.
    """
    warm = status_code(bundle, "warm-build")
    if warm is not None and warm != 0:
        tail = "\n".join(bundle.read("warm-build.stderr").strip().splitlines()[-6:])
        raise BundleDiedBeforeStart(
            f"the warm build exited {warm}; this bundle measures a BUILD FAILURE, "
            f"not a runtime.\n{tail}"
        )

    launched = any(
        bundle.exists(f"{name}.status")
        for name in ("perf-record", "perf-stat", "strace")
    )
    if not launched:
        raise BundleDiedBeforeStart(
            "no capture status file in the bundle: the run never got past the build "
            "step, so no process was ever launched"
        )

    rows, source = frame_rows(bundle)
    if not rows:
        if stamped_lines(bundle) == 0:
            raise BundleDiedBeforeStart(
                "no stamped game log and no frame series: nothing in this bundle "
                "observed a running game"
            )
        raise BundleDiedBeforeStart(
            "the game logged, but no frame series reached this bundle "
            f"({' / '.join(FRAME_SOURCES)} are both empty). There is no frame cost "
            "here to record."
        )
    if source and sum(number(row.get("frames"), 0.0) for row in rows) <= 0:
        raise BundleDiedBeforeStart(f"{source} exists but reports zero frames")


# ── the derived metric blocks ─────────────────────────────────────────────


def weighted(rows: list[dict], column: str, weight: str = "frames") -> float | None:
    """Frame-count-weighted mean of a per-window column.

    Weighting matters: the first census window of a headless run holds three
    frames and one 430ms startup hitch, and an unweighted mean lets it outvote
    six hundred steady frames.
    """
    total = 0.0
    mass = 0.0
    for row in rows:
        value = number(row.get(column))
        count = number(row.get(weight), 0.0) or 0.0
        if value is None or count <= 0:
            continue
        total += value * count
        mass += count
    return round(total / mass, 4) if mass > 0 else None


def frame_metrics(bundle: Bundle) -> dict:
    rows, source = frame_rows(bundle)
    if source == "frame_windows.csv":
        # The always-on census names its columns with a `_ms` suffix and reports
        # no mean or min at all.
        keys = {"p50": "p50_ms", "p90": None, "p95": "p95_ms", "p99": "p99_ms",
                "max": "max_ms", "min": None, "mean": None}
    else:
        keys = {"p50": "p50", "p90": None, "p95": "p95", "p99": "p99",
                "max": "max", "min": "min", "mean": "mean"}
    out: dict[str, object] = {}
    for name, column in keys.items():
        out[name] = weighted(rows, column) if column else None
    # ⭐ max and min are the only two that survive aggregation exactly; take them
    # whole rather than as a weighted mean of per-window extremes.
    if keys["max"]:
        extremes = [number(row.get(keys["max"])) for row in rows]
        out["max"] = max((v for v in extremes if v is not None), default=None)
    if keys["min"]:
        floors = [number(row.get(keys["min"])) for row in rows]
        out["min"] = min((v for v in floors if v is not None), default=None)
    out["source"] = source
    out["windows"] = len(rows)
    # ⚠ NAME THE ESTIMATOR. A frame-weighted mean of per-window p95s is not the
    # session's p95, and a reader a year from now must not assume it is. Both
    # sides of a comparison use the same estimator, which is what makes the
    # DELTA meaningful even though the absolute is approximate.
    out["percentile_method"] = (
        "frame-weighted mean of per-window percentiles; max/min are exact. "
        "Not a whole-session percentile."
    )
    # ⚠ `[census] frame` emits mean/p50/p95/p99/min/max and no p90. The column
    # is here because a series cannot be back-filled with a dimension it never
    # recorded; it stays null until the census emits one.
    out["p90_note"] = "null: [census] frame does not emit p90"
    return out


def spike_metrics(bundle: Bundle, frames: float | None) -> dict:
    spikes = bundle.rows("frame_spikes.csv")
    worst = max((number(row.get("frame_ms"), 0.0) or 0.0 for row in spikes), default=None)
    per_1000 = round(len(spikes) / frames * 1000.0, 3) if frames else None
    return {
        "count": len(spikes),
        "threshold_ms": 33.4,
        "per_1000_frames": per_1000,
        "worst_ms": worst,
    }


def phase_metrics(bundle: Bundle) -> dict | None:
    """Mean milliseconds per frame in each main-schedule phase.

    The phase list is read off the row, not hardcoded: the census takes it from
    the app's own `MainScheduleOrder`, so a new phase must appear here without
    an edit.
    """
    rows = bundle.rows("schedule_phases.csv")
    if not rows:
        return None
    labels = [key for key in rows[0] if key not in ("wall_s", "t", "frames")]
    phases = {label: weighted(rows, label) for label in labels}
    known = [value for value in phases.values() if value is not None]
    phases["_budget_ms"] = round(sum(known), 4) if known else None
    return phases


def read_tracy_zones(bundle: Bundle) -> list[dict]:
    """`tracy_zones.csv`, with its unquoted comma-bearing `name` column repaired.

    ⛔ `tracy-csvexport` writes the zone name RAW and a Bevy system name is a
    Rust path that routinely contains commas, which shifts every later column on
    such a row. The trailing columns are fixed in number, so split from the
    RIGHT. `scripts/lib/tracy_zone_report.py` carries the same repair.
    """
    path = os.path.join(bundle.path, "tracy_zones.csv")
    if not os.path.exists(path):
        return []
    with open(path, newline="", encoding="utf-8", errors="replace") as handle:
        header = handle.readline().rstrip("\n")
        if not header:
            return []
        columns = header.split(",")
        rows = []
        for line in handle:
            line = line.rstrip("\n")
            if not line:
                continue
            parts = line.split(",")
            if len(parts) > len(columns):
                overflow = len(parts) - len(columns)
                parts = [",".join(parts[: overflow + 1])] + parts[overflow + 1 :]
            rows.append(dict(zip(columns, parts)))
        return rows


def scheduler_metrics(bundle: Bundle, frames: float | None) -> dict:
    """Structural per-frame work: how many systems, conditions and flushes.

    ⭐ These are the counts that stay true when timings do not. A Tracy capture
    inflates every microsecond in the bundle by ~9x on this project, but it
    counts zone entries exactly, so the structural half of a Tracy row remains
    comparable against another Tracy row AND informative on its own.
    """
    schedules = bundle.rows("schedule_census.csv")
    out: dict[str, object] = {
        "registered_systems": None,
        "registered_systems_peak": None,
        "schedules": None,
        "per_schedule": None,
    }
    if schedules:
        last = schedules[-1]
        out["registered_systems"] = number(last.get("systems"))
        out["registered_systems_peak"] = max(
            (number(row.get("systems"), 0.0) or 0.0 for row in schedules), default=None
        )
        out["schedules"] = number(last.get("schedules"))
        # Any column past the four fixed ones is a schedule label the census
        # discovered; carrying them by name is what turns "Update is 65% of the
        # frame" into "Update is 822 of 876 systems".
        extra = {
            key: number(value)
            for key, value in last.items()
            if key not in ("wall_s", "t", "schedules", "systems")
        }
        out["per_schedule"] = extra or None

    zones = read_tracy_zones(bundle)
    totals = {name: 0.0 for name in ZONE_PREFIXES.values()}
    for row in zones:
        name = row.get("name", "")
        for prefix, field in ZONE_PREFIXES.items():
            if name.startswith(prefix):
                totals[field] += number(row.get("counts"), 0.0) or 0.0
                break
    if zones:
        out.update({f"{field}_total": int(value) for field, value in totals.items()})
        out.update(
            {
                f"{field}_per_frame": (round(value / frames, 1) if frames else None)
                for field, value in totals.items()
            }
        )
        out["per_frame_divisor"] = frames
    else:
        # ⚠ Not zero. A `--no-tracy` run has no zones to count, and zero here
        # would read as "the scheduler did nothing".
        for field in ZONE_PREFIXES.values():
            out[f"{field}_total"] = None
            out[f"{field}_per_frame"] = None
        out["per_frame_divisor"] = None
    return out


def scene_metrics(bundle: Bundle) -> dict:
    rows = bundle.rows("runtime_census.csv")
    if not rows:
        return {key: None for key in
                ("entities", "entities_peak", "archetypes", "archetypes_peak",
                 "components", "bodies", "bodies_peak", "players")}
    last = rows[-1]

    def peak(column: str) -> float | None:
        values = [number(row.get(column)) for row in rows]
        return max((v for v in values if v is not None), default=None)

    return {
        "entities": number(last.get("entities")),
        "entities_peak": peak("entities"),
        "archetypes": number(last.get("archetypes")),
        "archetypes_peak": peak("archetypes"),
        "components": number(last.get("components")),
        "bodies": number(last.get("bodies")),
        "bodies_peak": peak("bodies"),
        "players": number(last.get("players")),
    }


def host_facts(bundle: Bundle, meta: dict) -> dict:
    text = bundle.read("host-environment.txt")
    cpu = CPU_MODEL.search(text)
    cpus = LOGICAL_CPUS.search(text)
    mem = MEM_TOTAL.search(text)
    uname = meta.get("uname", "")
    # `uname -a` is one string; the kernel release is the third word and is the
    # part that a reader actually compares between two rows.
    parts = uname.split()
    return {
        # ⛔⛔ NOT THE HOSTNAME. It is reused across hosts -- two different
        # boxes have both been called `aivm-2404`, one an i7-7700HQ with 6
        # logical CPUs and one an i9-11900K with 12, and a timing baseline from
        # the first was read as belonging to the second. `/etc/machine-id` is
        # per-installation. A bundle taken before the front door recorded it
        # falls back to the hostname and is MARKED weak, so the key still
        # separates what it can while saying the id cannot be trusted to.
        "machine_id": meta.get("machine_id")
        or (f"hostname:{meta['hostname']}" if meta.get("hostname") else None)
        or (parts[1] if len(parts) > 1 else None),
        "machine_id_source": "machine-id" if meta.get("machine_id") else "hostname",
        "cpu_model": cpu.group(1).strip() if cpu else None,
        "logical_cpus": int(cpus.group(1)) if cpus else None,
        "mem_total_kb": int(mem.group(1)) if mem else None,
        "kernel": parts[2] if len(parts) > 2 else None,
        "os": parts[0] if parts else None,
        "uname": uname or None,
    }


def gpu_facts(bundle: Bundle, meta: dict) -> dict:
    """Which adapter drew, and whether a GPU was involved at all.

    ⛔ This is not decoration. A software-rasterized run spends most of its
    cycles in JIT'd shader code that no symbol will ever explain, and its frame
    time has nothing to do with a hardware run's. `rendering` is in the
    comparability key precisely so the two can never be subtracted.
    """
    headless = meta.get("headless") == "yes"
    log = bundle.read("game-stderr-stamped.txt") or bundle.read("game-stdout-stamped.txt")
    match = ADAPTER.search(log) or ADAPTER.search(bundle.read("perf-record.stderr"))
    fields: dict[str, str] = {}
    if match:
        for key, quoted, bare in ADAPTER_FIELD.findall(match.group(0)):
            fields[key] = quoted or bare

    if headless:
        # `--headless` selects `backends: None`: no window, no adapter, no
        # render app. "unknown" would be wrong — this is a positive fact.
        rendering = "headless"
    elif not match:
        rendering = "unknown"
    elif any(marker in match.group(0) for marker in SOFTWARE_MARKERS):
        rendering = "software"
    else:
        rendering = "hardware"

    driver = None
    nvidia = NVIDIA_ROW.search(bundle.read("host-environment.txt"))
    if fields.get("driver_info"):
        driver = f"{fields.get('driver', '')} {fields['driver_info']}".strip()
    elif nvidia:
        driver = f"{nvidia.group(1)} {nvidia.group(2)}"

    return {
        "rendering": rendering,
        "adapter": fields.get("name"),
        "device_type": fields.get("device_type"),
        "backend": fields.get("backend"),
        "driver": driver,
        "adapter_info": match.group(0) if match else None,
    }


def display_facts(bundle: Bundle, meta: dict) -> dict:
    if meta.get("headless") == "yes":
        return {"resolution": "headless", "cameras": None, "world_rendering_peak": None}
    sizes: dict[str, int] = {}
    for row in bundle.rows("camera_views.csv"):
        if row.get("target") == "window" and row.get("size"):
            sizes[row["size"]] = sizes.get(row["size"], 0) + 1
    views = bundle.rows("view_totals.csv")
    return {
        "resolution": max(sizes, key=sizes.get) if sizes else None,
        "cameras": number(views[-1].get("cameras")) if views else None,
        "world_rendering_peak": max(
            (number(row.get("world_rendering"), 0.0) or 0.0 for row in views), default=None
        )
        if views
        else None,
    }


PROFILER_SHARE = re.compile(r"^\s*([0-9.]+)%\s+(\S.*?)\s*$", re.M)


def profiler_share(bundle: Bundle) -> float | None:
    """Percent of sampled cycles spent in the profiler's own threads.

    ⚠ The single number that decides whether the absolute times in a row are
    usable. It is not in the comparability key — two Tracy runs on the same
    machine may land at 45% and 55% and still be worth comparing — but the query
    tool prints it beside every Tracy row, and 55% is why this project's frame
    baseline is quoted from a `--no-tracy` run.
    """
    text = bundle.read("perf-report-by-thread.txt")
    if not text:
        return None
    share = 0.0
    seen = False
    for percent, name in PROFILER_SHARE.findall(text):
        seen = True
        if "tracy" in name.lower():
            share += float(percent)
    return round(share, 1) if seen else None


def instrument_facts(bundle: Bundle, meta: dict) -> dict:
    """What was actually observing the run — not what was requested.

    ⭐ REQUESTED IS NOT EFFECTIVE. `--features profile` can be asked for and
    refused (a package that does not carry the feature), and `tracy-capture` can
    be absent. The key must carry what happened; the request is kept beside it
    so a surprising row can be explained.
    """
    tracy_effective = bool(
        bundle.exists("tracy.trace")
        or bundle.exists("tracy_zones.csv")
        or bundle.exists("tracy_summary.md")
    )
    perf_status = status_code(bundle, "perf-record")
    return {
        "tracy": tracy_effective,
        "tracy_requested": meta.get("tracy_requested") == "yes",
        "tracy_skipped_reason": (bundle.read("tracy.skipped").strip() or None),
        "perf": bool(bundle.exists("perf.data") or bundle.exists("perf_report.txt")),
        "perf_status": perf_status,
        "perf_events": meta.get("perf_events") or None,
        "sampling_frequency_hz": number(meta.get("sampling_frequency_hz")),
        "census": meta.get("census_enabled") == "yes",
        "census_hz": number(meta.get("census_hz")),
        "profiler_cycle_share_pct": profiler_share(bundle),
    }


# ── the record ────────────────────────────────────────────────────────────


def dig(record: dict, path: str):
    """`dig(row, "gpu.rendering")`. Missing intermediate levels read as None."""
    node = record
    for part in path.split("."):
        if not isinstance(node, dict):
            return None
        node = node.get(part)
    return node


def comparability(record: dict) -> tuple[str, dict]:
    fields = {path: dig(record, path) for path in COMPARABILITY_FIELDS}
    blob = json.dumps(fields, sort_keys=True, default=str)
    return hashlib.sha256(blob.encode("utf-8")).hexdigest()[:16], fields


def comparable_label(fields: dict) -> str:
    """A key a human can read in a table without expanding the dict."""
    scenario = fields.get("scenario.id") or "?"
    version = fields.get("scenario.version")
    where = fields.get("gpu.rendering") or "?"
    tracy = "tracy" if fields.get("instruments.tracy") else "no-tracy"
    return (
        f"{scenario}@v{version}/{fields.get('build.cargo_profile') or '?'}"
        f"/{where}/{tracy}/{fields.get('host.machine_id') or '?'}"
    )


def scenario_facts(meta: dict, version_override: int | None) -> dict:
    headless = meta.get("headless") == "yes"
    # A windowed run has no `headless_scenario`, so its workload is whatever the
    # operator passed through `--`: a launch target, a start room, or nothing.
    # ⛔ NOT `run_command` — that carries the cargo profile and features, which
    # are build dimensions and already have their own columns. Folding them into
    # the scenario id would split one workload into a group per build.
    if headless:
        scenario_id = meta.get("headless_scenario") or "unknown"
    else:
        typed = (meta.get("script_command") or "").split()
        passthrough = typed[typed.index("--") + 1 :] if "--" in typed else []
        scenario_id = "windowed:" + ("+".join(passthrough) if passthrough else "default")
    version = version_override
    if version is None:
        version = SCENARIO_VERSIONS.get(scenario_id, DEFAULT_SCENARIO_VERSION)
    return {
        "id": scenario_id,
        "version": version,
        "headless": headless,
        "ticks": number(meta.get("headless_ticks")) if headless else None,
        "mode": meta.get("mode"),
        "duration_seconds": meta.get("duration_seconds"),
        "run_command": meta.get("run_command"),
        "script_command": meta.get("script_command"),
    }


def build_facts(meta: dict) -> dict:
    raw = (meta.get("cargo_features") or "").replace(",", " ").split()
    return {
        "cargo_profile": meta.get("cargo_profile"),
        "profile_dir": meta.get("profile_dir"),
        # Sorted so two runs that named the same features in a different order
        # land in the same comparability group.
        "features": sorted(raw),
        "package": meta.get("package"),
        "binary": meta.get("binary"),
        "binary_path": meta.get("binary_path"),
        "rustc": meta.get("rustc_version"),
        "rust_target": meta.get("rust_target"),
        "cargo_jobs": meta.get("cargo_jobs"),
    }


def measured_at(meta: dict) -> str | None:
    """The run's own UTC stamp as ISO-8601.

    ⚠ Distinct from `recorded_at`, which is when the ROW was written. On a
    backfilled row the two are weeks apart, and only the first one orders the
    series.
    """
    stamp = meta.get("utc_stamp") or ""
    match = re.fullmatch(r"(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})Z", stamp)
    if not match:
        return None
    y, mo, d, h, mi, s = match.groups()
    return f"{y}-{mo}-{d}T{h}:{mi}:{s}Z"


def build_record(
    bundle_dir: str,
    *,
    label: str = "",
    scenario_version: int | None = None,
    run_id: str | None = None,
) -> dict:
    """One normalized row. Raises `BundleDiedBeforeStart` if there is no run."""
    bundle = Bundle(bundle_dir)
    assess_run(bundle)
    meta = bundle.metadata()

    rows, _ = frame_rows(bundle)
    frames = sum(number(row.get("frames"), 0.0) or 0.0 for row in rows) or None

    log = bundle.read("game-stderr-stamped.txt") or bundle.read("game-stdout-stamped.txt")
    stamps = [float(m.group(1)) for m in re.finditer(r"^\[\s*([0-9.]+)s\]", log, re.M)]

    record: dict[str, object] = {
        "schema": SCHEMA,
        "kind": KIND,
        "recorded_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "commit": meta.get("git_head_short") or "unknown",
        "commit_full": meta.get("git_head"),
        "branch": meta.get("git_branch"),
        "dirty": not meta.get("git_clean", True),
        "dirty_files": meta.get("git_dirty_files") or [],
        "run_id": run_id or uuid.uuid4().hex[:12],
        "label": label,
        "record_id": os.path.basename(os.path.normpath(bundle_dir)),
        "measured_at": measured_at(meta),
        # ⭐ Where the raw bundle lived, kept because the row outlives it. The
        # tarball is hundreds of megabytes and gets deleted; this is how a
        # reader knows what to look for on a backup and what to stop looking for.
        "bundle": {
            "name": os.path.basename(os.path.normpath(bundle_dir)),
            "path": meta.get("output_dir") or os.path.abspath(bundle_dir),
            "tarball": (bundle.read("package-path.txt").strip() or None),
            "present_at_ingest": True,
        },
        "scenario": scenario_facts(meta, scenario_version),
        "build": build_facts(meta),
        "host": host_facts(bundle, meta),
        "gpu": gpu_facts(bundle, meta),
        "display": display_facts(bundle, meta),
        "instruments": instrument_facts(bundle, meta),
        "run": {
            "frames": frames,
            "observed_log_seconds": round(max(stamps), 3) if stamps else None,
            "census_windows": len(rows),
            "capture_status": status_code(bundle, "perf-record"),
            "warm_build_status": status_code(bundle, "warm-build"),
        },
        "frame_ms": frame_metrics(bundle),
        "spikes": spike_metrics(bundle, frames),
        "phases_ms": phase_metrics(bundle),
        "scheduler": scheduler_metrics(bundle, frames),
        "scene": scene_metrics(bundle),
        "provenance": {
            "backfilled": False,
            "recorded_from": os.path.abspath(bundle_dir),
            "transcribed_from_prose": [],
        },
    }
    key, fields = comparability(record)
    record["comparable_key"] = key
    record["comparable_fields"] = fields
    record["comparable_label"] = comparable_label(fields)
    return record


def append(record: dict, ledger: Path = LEDGER) -> Path:
    measurement_paths.require_writable(ledger)
    ledger.parent.mkdir(parents=True, exist_ok=True)
    with ledger.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(record, sort_keys=True) + "\n")
    return ledger


def load(ledger: Path = LEDGER) -> list[dict]:
    """Every row, oldest first. An absent or uninitialized ledger has no rows."""
    if not Path(ledger).exists():
        return []
    rows = []
    with Path(ledger).open(encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Append one profiling bundle to the runtime frame-cost series.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument("bundle", help="a directory written by scripts/profile_desktop.sh")
    parser.add_argument("--label", default="", help="free text tag, e.g. 'after set-gating'")
    parser.add_argument(
        "--scenario-version",
        type=int,
        default=None,
        help="override the scenario content version (default: the SCENARIO_VERSIONS table)",
    )
    parser.add_argument("--ledger", default=str(LEDGER), help="target JSONL")
    parser.add_argument("--no-record", action="store_true", help="print the row; append nothing")
    args = parser.parse_args(argv)

    try:
        record = build_record(
            args.bundle, label=args.label, scenario_version=args.scenario_version
        )
    except BundleDiedBeforeStart as dead:
        print(
            f"⛔ refusing to record {args.bundle}\n"
            f"   {dead}\n"
            "   A run that never reached a frame is not a runtime measurement, and a\n"
            "   row of zeroes would read as an improvement. Nothing was written.",
            file=sys.stderr,
        )
        return 3

    if args.no_record:
        print(json.dumps(record, indent=2, sort_keys=True))
        return 0

    ledger = append(record, Path(args.ledger))
    print(f"appended {record['record_id']} to {ledger}")
    print(f"  group  {record['comparable_label']}  ({record['comparable_key']})")
    print(f"  frame  mean {record['frame_ms']['mean']} ms  p99 {record['frame_ms']['p99']} ms")
    if record["instruments"]["tracy"]:
        share = record["instruments"]["profiler_cycle_share_pct"]
        print(f"  ⚠ Tracy was attached (profiler share: {share}%); times are an upper bound")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
