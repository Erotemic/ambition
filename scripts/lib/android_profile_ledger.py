#!/usr/bin/env python3
"""Turn one Android profile bundle into one row of play-cost history.

`scripts/profile_android.sh` writes a bundle that answers "why was THIS run on
the phone slow". It cannot answer "does the game play better on the phone than
it did last week", because the bundle is untracked and local to one machine.
This module reads the small text files the bundle already has — `metadata.txt`,
`frame-census.txt`, `game-log.stdout`, `getprop-summary.stdout`, the clock and
heat dumps, the simpleperf text reports — and makes one JSON row that stays
true after the bundle is gone. The ledger is
`dev/ambition_dev_measurements/android_play_cost.jsonl`; the page that reads it
is `dev/ambition_dev_measurements/android-play-ledger.md`.

Contracts:

⛔ **A run that never reached gameplay is not a measurement.** The frame numbers
   come only from `[frame-census]` windows that START at t >= 30 s (the app's
   own clock; the first windows are boot and load). A bundle with no such
   window is refused (exit 3) and nothing is written. A row of nulls would read
   as a run, and the next delta would be nonsense.

⭐ **Two rows are compared only if their `comparable_key` is equal.** The key is
   a hash of `comparable_fields`: device model, Android release, game,
   boot route, cargo profile, autoplay, hub games, and the profiler setup
   (script mode and simpleperf rate). `table` groups by the key and computes a
   delta only against the previous row of the SAME key. The run length is NOT
   in the key: a 60 s capture and a 30 s capture describe the same workload,
   but they cover different parts of the route. `windows` says how many windows
   each row has, and `frames_30_90s` restates the numbers over the windows that
   start in [30 s, 90 s), which every capture since 10-16 covers.

⚠ **The game says what it ran; the script only says what it asked for.** The
   game prints `[census] config boot_route=` and `[census] config autoplay=`.
   Those win over `metadata.txt`. Autoplay is `"off"` when the game printed its
   boot route and no autoplay line (they are printed by the same composition
   step), and `null` when the log has neither (an older build: unknown).

⚠ Nulls mean *nothing measured this*, never zero. `gfxinfo` is not read: it
   reports 0 frames for this app (the game draws with Vulkan, not with the
   Android view system). The GPU clock is not readable on the Pixel 5 without
   root, so `gpufreq` is null there. simpleperf is never run here: the thread
   and code-owner split is counted from `simpleperf-device-samples.txt` (one
   block per sample, exact), else from `simpleperf-device-basic.txt` (rows
   round to 0.01 %, so its shares are of the mass it lists). Both were written
   by the script during the run. `our_code` is a sample whose leaf frame is in
   `libambition_app.so` (the game, Bevy and every Rust crate).

Usage:

    python3 scripts/lib/android_profile_ledger.py append BUNDLE [BUNDLE...]
    python3 scripts/lib/android_profile_ledger.py row BUNDLE      # print, write nothing
    python3 scripts/lib/android_profile_ledger.py table [--markdown-only]

`append` exits 3 if it refused any bundle (it still appends the others). It
skips a `record_id` that the ledger already has, and copies the
bundle's `android-profile-summary.md` to `summaries/<record_id>.md` (tracked),
so a reader on another machine has the readable half of the bundle.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import statistics
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import measurement_paths  # noqa: E402

SCHEMA = 1
KIND = "android_play"
LEDGER = measurement_paths.MEASUREMENTS / "android_play_cost.jsonl"
SUMMARIES = measurement_paths.MEASUREMENTS / "summaries"

#: The first window that counts starts at or after this app time (seconds).
STEADY_START_S = 30.0
#: The common span: windows that START in [30 s, 90 s).
SPAN_START_S, SPAN_END_S = 30.0, 90.0

#: The fields a play cost is only comparable within.
COMPARABILITY_FIELDS = (
    "device_model",
    "android_release",
    "game",
    "boot_route",
    "cargo_profile",
    "autoplay",
    "hub_games",
    "profile_mode",
    "simpleperf_hz",
)

WINDOW = re.compile(
    r"\[frame-census\]\s+(?P<start>[\d.]+)s-(?P<end>[\d.]+)s\s+frames=(?P<frames>\d+)"
    r"\s+p50=(?P<p50>[\d.]+)ms\s+p95=(?P<p95>[\d.]+)ms\s+p99=(?P<p99>[\d.]+)ms"
    r"\s+max=(?P<max>[\d.]+)ms"
    r"(?:\s+archetypes=(?P<archetypes>\d+)(?:\s+\([+-]?\d+\))?)?"
    r"(?:\s+entities=(?P<entities>\d+))?"
)
CONFIG = re.compile(r"\[census\] config (?P<key>boot_route|autoplay)=(?P<value>\S+)")
SYSTEMS_RAN = re.compile(
    r"\[census\] systems_ran t=(?P<t>[\d.]+) registered=(?P<registered>\d+) ran=(?P<ran>\d+)"
    r"(?: schedules:(?P<schedules>.*?))?(?: crates:.*)?$"
)
GETPROP = re.compile(r"^\[(?P<key>[^\]]+)\]: \[(?P<value>.*)\]$")
CPUFREQ = re.compile(r"^(?P<cpu>cpu\d+) cur=(?P<cur>\d*) max=(?P<max>\d*) hw_max=(?P<hw>\d*)")
PERF_ROW = re.compile(
    r"^\s*(?P<pct>[\d.]+)%\s+(?P<comm>.+?)\s+(?P<pid>\d+)\s+(?P<tid>\d+)\s+(?P<dso>\S+)\s"
)


class NotGameplay(Exception):
    """The bundle has no frame-census window in steady gameplay."""


def _read(bundle: Path, name: str) -> str | None:
    path = bundle / name
    if not path.is_file():
        return None
    return path.read_text(errors="replace")


def parse_metadata(text: str) -> dict:
    """`key=value` lines; the git porcelain block becomes a list."""
    meta: dict = {}
    porcelain: list[str] = []
    inside = False
    for line in text.splitlines():
        if line == "git_status_porcelain_begin":
            inside = True
        elif line == "git_status_porcelain_end":
            inside = False
        elif inside:
            porcelain.append(line.strip())
        elif "=" in line:
            key, _, value = line.partition("=")
            meta.setdefault(key, value)
    meta["git_status_porcelain"] = porcelain
    return meta


def parse_windows(text: str) -> list[dict]:
    """Every `[frame-census]` window, in log order."""
    windows = []
    for line in text.splitlines():
        match = WINDOW.search(line)
        if not match:
            continue
        row = {k: (float(v) if v is not None else None) for k, v in match.groupdict().items()}
        row["frames"] = int(row["frames"])
        for key in ("archetypes", "entities"):
            row[key] = int(row[key]) if row[key] is not None else None
        windows.append(row)
    return windows


def frame_stats(windows: list[dict]) -> dict | None:
    """FPS and frame-time medians over the given windows; None if empty.

    `fps_median` is the median of the per-window FPS (frames / window seconds).
    The ms fields are the median over windows of each window's percentile, and
    `max_ms` is the worst frame in any window.
    """
    if not windows:
        return None
    fps = [w["frames"] / (w["end"] - w["start"]) for w in windows if w["end"] > w["start"]]
    seconds = sum(w["end"] - w["start"] for w in windows)
    return {
        "windows": len(windows),
        "span_s": [round(windows[0]["start"], 3), round(windows[-1]["end"], 3)],
        "frames": sum(w["frames"] for w in windows),
        "fps_median": round(statistics.median(fps), 2),
        "fps_min_window": round(min(fps), 2),
        "fps_mean": round(sum(w["frames"] for w in windows) / seconds, 2),
        "p50_ms_median": round(statistics.median(w["p50"] for w in windows), 2),
        "p95_ms_median": round(statistics.median(w["p95"] for w in windows), 2),
        "p99_ms_median": round(statistics.median(w["p99"] for w in windows), 2),
        "max_ms": round(max(w["max"] for w in windows), 2),
    }


def parse_log(text: str | None) -> dict:
    """The game's own `[census] config` rows and `[census] systems_ran` rows."""
    config: dict[str, str] = {}
    systems = []
    for line in (text or "").splitlines():
        match = CONFIG.search(line)
        if match:
            config.setdefault(match["key"], match["value"])
            continue
        match = SYSTEMS_RAN.search(line)
        if match:
            schedules = {}
            for item in (match["schedules"] or "").split():
                name, _, counts = item.partition("=")
                ran, _, registered = counts.partition("/")
                if ran.isdigit() and registered.isdigit():
                    schedules[name] = [int(ran), int(registered)]
            systems.append(
                {
                    "t": float(match["t"]),
                    "registered": int(match["registered"]),
                    "ran": int(match["ran"]),
                    "schedules": schedules,
                }
            )
    return {"config": config, "systems_ran": systems}


def systems_ran_summary(rows: list[dict]) -> dict | None:
    """Median systems that ran per frame over steady rows; the last row's schedules."""
    steady = [r for r in rows if r["t"] >= STEADY_START_S] or rows
    if not steady:
        return None
    return {
        "samples": len(steady),
        "ran_median": statistics.median(r["ran"] for r in steady),
        "registered_last": steady[-1]["registered"],
        "schedules_last": steady[-1]["schedules"],
    }


def parse_getprop(text: str | None) -> dict:
    props = {}
    for line in (text or "").splitlines():
        match = GETPROP.match(line.strip())
        if match:
            props[match["key"]] = match["value"]
    return {
        "model": props.get("ro.product.model"),
        "device": props.get("ro.product.device"),
        "android_release": props.get("ro.build.version.release"),
        "sdk": props.get("ro.build.version.sdk"),
        "soc": props.get("ro.soc.model"),
        "fingerprint": props.get("ro.build.fingerprint"),
    }


def parse_cpufreq(text: str | None) -> dict | None:
    """Per cluster (CPUs that share `hw_max`): current and allowed clocks in MHz.

    `capped` is true when a cluster's allowed max is under its hardware max: the
    phone was holding the CPU down (heat, or a power mode).
    """
    clusters: dict[int, dict] = {}
    for line in (text or "").splitlines():
        match = CPUFREQ.match(line.strip())
        if not match or not match["hw"]:
            continue
        hw = int(match["hw"])
        entry = clusters.setdefault(hw, {"cpus": [], "cur_mhz": [], "max_mhz": None})
        entry["cpus"].append(match["cpu"])
        if match["cur"]:
            entry["cur_mhz"].append(int(match["cur"]) // 1000)
        if match["max"]:
            entry["max_mhz"] = int(match["max"]) // 1000
    if not clusters:
        return None
    out = []
    for hw, entry in sorted(clusters.items()):
        out.append(
            {
                "cpus": f"{entry['cpus'][0]}-{entry['cpus'][-1]}",
                "hw_max_mhz": hw // 1000,
                "max_mhz": entry["max_mhz"],
                "cur_mhz": entry["cur_mhz"],
                "capped": entry["max_mhz"] is not None and entry["max_mhz"] < hw // 1000,
            }
        )
    return {"clusters": out, "any_capped": any(c["capped"] for c in out)}


def parse_gpufreq(text: str | None) -> dict | None:
    """The kgsl clock values, or None when every value is empty (no root)."""
    values = {}
    for line in (text or "").splitlines():
        key, sep, value = line.partition("=")
        if sep and value.strip():
            values[key.strip()] = value.strip()
    return values or None


def parse_thermal(text: str | None) -> dict | None:
    if not text:
        return None
    status = re.search(r"Thermal Status: (\d+)", text)
    temps = [
        float(m[1])
        for m in re.finditer(r"Temperature\{mValue=([-\d.]+), mType=0, mName=cpu", text)
    ]
    return {
        "status": int(status[1]) if status else None,
        "cpu_max_c": round(max(temps), 1) if temps else None,
    }


def dso_class(dso: str) -> str:
    name = dso.rsplit("/", 1)[-1]
    if name == "libambition_app.so":
        return "our_code"
    if dso.startswith("[kernel"):
        return "kernel"
    if "vulkan" in name or "adreno" in name or "libgsl" in name or "kgsl" in name:
        return "gpu_driver"
    if "/bionic/" in dso or name in ("libc.so", "libm.so"):
        return "libc"
    return "other"


def parse_simpleperf_basic(text: str | None) -> dict | None:
    """CPU-sample shares by thread name and by code owner, from the full report.

    Thread names are the kernel's (cut to 15 characters); the main thread and
    the render thread are both `android_main`, so `tids` says how many threads
    share a name.
    """
    if not text:
        return None
    samples = re.search(r"^Samples: (\d+)", text, re.M)
    threads: dict[str, dict] = {}
    owners: dict[str, float] = {}
    total = 0.0
    for line in text.splitlines():
        match = PERF_ROW.match(line)
        if not match:
            continue
        pct = float(match["pct"])
        total += pct
        thread = threads.setdefault(match["comm"].strip(), {"pct": 0.0, "tids": set()})
        thread["pct"] += pct
        thread["tids"].add(match["tid"])
        owner = dso_class(match["dso"])
        owners[owner] = owners.get(owner, 0.0) + pct
    if total == 0:
        return None
    ranked = sorted(threads.items(), key=lambda kv: -kv[1]["pct"])
    # Shares of the PARSED mass: rows round to 0.01 %, so `covered_pct` < 100.
    return {
        "source": "report-basic",
        "samples": int(samples[1]) if samples else None,
        "covered_pct": round(total, 1),
        "threads": {
            k: {"pct": round(v["pct"] * 100 / total, 1), "tids": len(v["tids"])}
            for k, v in ranked[:8]
        },
        "owners_pct": {
            k: round(v * 100 / total, 1) for k, v in sorted(owners.items(), key=lambda kv: -kv[1])
        },
    }


def parse_simpleperf_samples(path: Path) -> dict | None:
    """Exact CPU-sample shares from `simpleperf-device-samples.txt` (`report-sample`).

    Each `sample:` block names its thread and the file of its leaf frame (the
    first `  file:` line, before the indented callchain). Weighted by
    `event_count`. This is the exact split; the `basic` report rounds every row
    to 0.01 %, and its long tail of one-sample rows loses ~15 % of the mass.
    """
    if not path.is_file():
        return None
    threads: dict[str, dict] = {}
    owners: dict[str, int] = {}
    total = samples = 0
    thread = tid = None
    count = 0

    def close() -> None:
        nonlocal total, samples
        if thread is None or leaf is None:
            return
        samples += 1
        total += count
        entry = threads.setdefault(thread, {"count": 0, "tids": set()})
        entry["count"] += count
        entry["tids"].add(tid)
        owner = dso_class(leaf)
        owners[owner] = owners.get(owner, 0) + count

    leaf = None
    with path.open(errors="replace") as handle:
        for line in handle:
            if line.startswith("sample:"):
                close()
                thread = tid = leaf = None
                count = 1
            elif line.startswith("  event_count: "):
                count = int(line.split(":", 1)[1])
            elif line.startswith("  thread_name: "):
                thread = line.split(":", 1)[1].strip()
            elif line.startswith("  thread_id: "):
                tid = line.split(":", 1)[1].strip()
            elif line.startswith("  file: ") and leaf is None:
                leaf = line.split(":", 1)[1].strip()
        close()
    if not total:
        return None
    ranked = sorted(threads.items(), key=lambda kv: -kv[1]["count"])
    return {
        "source": "report-sample",
        "samples": samples,
        "covered_pct": 100.0,
        "threads": {
            k: {"pct": round(v["count"] * 100 / total, 1), "tids": len(v["tids"])}
            for k, v in ranked[:8]
        },
        "owners_pct": {
            k: round(v * 100 / total, 1) for k, v in sorted(owners.items(), key=lambda kv: -kv[1])
        },
    }


def parse_simpleperf_stat(text: str | None) -> dict | None:
    if not text:
        return None
    clock = re.search(r"([\d.]+)\(ms\)\s+cpu-clock", text)
    seconds = re.search(r"Total test time: ([\d.]+) seconds", text)
    switches = re.search(r"([\d,]+)\s+context-switches", text)
    faults = re.search(r"([\d,]+)\s+page-faults", text)
    if not (clock and seconds):
        return None
    secs = float(seconds[1])
    return {
        "seconds": round(secs, 2),
        "cpu_cores_busy": round(float(clock[1]) / 1000.0 / secs, 2),
        "context_switches_per_s": round(int(switches[1].replace(",", "")) / secs) if switches else None,
        "page_faults_per_s": round(int(faults[1].replace(",", "")) / secs) if faults else None,
    }


def cargo_profile(bundle: Path, meta: dict) -> str | None:
    """The cargo profile the APK was built with, or `preinstalled` when the run built nothing."""
    command = _read(bundle, "profile-build.command.txt")
    if command:
        match = re.search(r"--cargo-profile\s+(\S+)", command)
        if match:
            return match[1]
    if meta.get("profile_build") == "yes":
        return meta.get("build_rust_mode")
    return "preinstalled" if meta.get("profile_build") == "no" else None


def game_of(boot_route: str | None) -> str | None:
    if not boot_route:
        return None
    return boot_route.removesuffix("_gameplay")


def comparable(fields: dict) -> tuple[str, str]:
    blob = json.dumps({k: fields.get(k) for k in COMPARABILITY_FIELDS}, sort_keys=True)
    key = hashlib.sha256(blob.encode("utf-8")).hexdigest()[:12]
    shown = {k: ("unknown" if v is None else v) for k, v in fields.items()}
    label = " · ".join(
        [
            f"{shown['device_model']} (Android {shown['android_release']})",
            f"game={shown['game']}",
            f"route={shown['boot_route']}",
            f"autoplay={shown['autoplay']}",
            f"{shown['cargo_profile']}",
            f"hub={shown['hub_games']}",
            f"{shown['profile_mode']}@{shown['simpleperf_hz']}Hz",
        ]
    )
    return key, label


def build_row(bundle: Path) -> dict:
    bundle = bundle.resolve()
    meta_text = _read(bundle, "metadata.txt")
    if meta_text is None:
        raise NotGameplay(f"{bundle} has no metadata.txt; it is not a profile_android.sh bundle")
    meta = parse_metadata(meta_text)
    log_text = _read(bundle, "game-log.stdout")
    census_text = _read(bundle, "frame-census.txt") or log_text or ""
    windows = parse_windows(census_text)
    steady = [w for w in windows if w["start"] >= STEADY_START_S]
    if not steady:
        raise NotGameplay(
            f"{len(windows)} [frame-census] window(s), none starting at t >= "
            f"{STEADY_START_S:g} s: the run never reached steady gameplay"
        )
    span = [w for w in steady if SPAN_START_S <= w["start"] < SPAN_END_S]
    log = parse_log(log_text)
    launch_env = meta.get("launch_env") or ""
    env = dict(item.partition("=")[::2] for item in launch_env.split() if "=" in item)

    boot_route = log["config"].get("boot_route") or meta.get("boot_route") or None
    if "autoplay" in log["config"]:
        autoplay = log["config"]["autoplay"]
    elif env.get("AMBITION_AUTOPLAY"):
        autoplay = env["AMBITION_AUTOPLAY"]
    elif "boot_route" in log["config"]:
        autoplay = "off"
    else:
        autoplay = None
    device = parse_getprop(_read(bundle, "getprop-summary.stdout"))
    build_status = (_read(bundle, "profile-build.status") or "").strip() or None
    stamp = meta.get("utc_stamp")
    recorded = (
        time.strftime("%Y-%m-%dT%H:%M:%SZ", time.strptime(stamp, "%Y%m%dT%H%M%SZ"))
        if stamp
        else None
    )
    dirty = [p for p in meta["git_status_porcelain"] if p and not p.startswith("?")]
    fields = {
        "device_model": device["model"],
        "android_release": device["android_release"],
        "game": game_of(boot_route),
        "boot_route": boot_route,
        "cargo_profile": cargo_profile(bundle, meta),
        "autoplay": autoplay,
        "hub_games": meta.get("hub_games"),
        "profile_mode": meta.get("mode"),
        "simpleperf_hz": meta.get("sampling_frequency_hz"),
    }
    key, label = comparable(fields)
    last = windows[-1]
    tarball = bundle.parent / f"{bundle.name}.tar.gz"
    return {
        "schema": SCHEMA,
        "kind": KIND,
        "record_id": bundle.name,
        "recorded_utc": recorded,
        "git_head": meta.get("git_head"),
        # ⚠ The phone runs what was BUILT. A failed build leaves the old APK
        # installed, and then `git_head` does not name the code that ran.
        "build_status": int(build_status) if build_status and build_status.isdigit() else None,
        "git_dirty_paths": dirty,
        "game": fields["game"],
        "boot_route": boot_route,
        "autoplay": autoplay,
        "launch_env": launch_env or None,
        "hub_games": meta.get("hub_games"),
        "cargo_profile": fields["cargo_profile"],
        "capture": {
            "mode": meta.get("mode"),
            "warmup_s": _int(meta.get("warmup_seconds")),
            "duration_s": _int(meta.get("duration_seconds")),
            "simpleperf_hz": _int(meta.get("sampling_frequency_hz")),
            "host": meta.get("hostname"),
        },
        "device": device,
        "comparable_fields": fields,
        "comparable_key": key,
        "comparable_label": label,
        "frames": frame_stats(steady),
        "frames_30_90s": frame_stats(span),
        "census_windows_total": len(windows),
        "end_of_run": {
            "t_s": round(last["end"], 3),
            "entities": last["entities"],
            "archetypes": last["archetypes"],
        },
        "systems_ran": systems_ran_summary(log["systems_ran"]),
        "cpufreq_before": parse_cpufreq(_read(bundle, "cpufreq-before-record.stdout")),
        "cpufreq_after": parse_cpufreq(_read(bundle, "cpufreq-after-record.stdout")),
        "gpufreq_after": parse_gpufreq(_read(bundle, "gpufreq-after-record.stdout")),
        "thermal_before": parse_thermal(_read(bundle, "thermal-before-record.stdout")),
        "thermal_after": parse_thermal(_read(bundle, "thermal-after-record.stdout")),
        "simpleperf": parse_simpleperf_samples(bundle / "simpleperf-device-samples.txt")
        or parse_simpleperf_basic(_read(bundle, "simpleperf-device-basic.txt")),
        "simpleperf_stat": parse_simpleperf_stat(_read(bundle, "simpleperf-stat.txt")),
        "bundle": {
            "path": _repo_relative(bundle),
            "tarball": _repo_relative(tarball) if tarball.is_file() else None,
            "summary": f"summaries/{bundle.name}.md"
            if (bundle / "android-profile-summary.md").is_file()
            else None,
        },
    }


def _int(value: str | None) -> int | None:
    return int(value) if value and value.isdigit() else None


def _repo_relative(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(measurement_paths.MEASUREMENTS))
    except ValueError:
        return str(path)


def read_ledger(ledger: Path) -> list[dict]:
    if not ledger.is_file():
        return []
    return [json.loads(line) for line in ledger.read_text().splitlines() if line.strip()]


def cmd_append(bundles: list[str], ledger: Path) -> int:
    measurement_paths.require_writable(ledger)
    known = {row["record_id"] for row in read_ledger(ledger)}
    worst = 0
    for name in bundles:
        bundle = Path(name)
        try:
            row = build_row(bundle)
        except NotGameplay as why:
            print(f"⛔ refusing {bundle.name}: {why}. Nothing was written.", file=sys.stderr)
            worst = max(worst, 3)
            continue
        if row["record_id"] in known:
            print(f"skip {row['record_id']}: already in {ledger.name}")
            continue
        summary = bundle / "android-profile-summary.md"
        if summary.is_file():
            SUMMARIES.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(summary, SUMMARIES / f"{row['record_id']}.md")
        with ledger.open("a") as handle:
            handle.write(json.dumps(row, sort_keys=True) + "\n")
        known.add(row["record_id"])
        f = row["frames"]
        print(
            f"appended {row['record_id']}: {f['fps_median']} FPS median over "
            f"{f['windows']} windows · group {row['comparable_key']} ({row['comparable_label']})"
        )
    return worst


def _fmt(value, spec: str = "") -> str:
    return "—" if value is None else format(value, spec)


def _delta(new, old, spec: str = "+.1f", pct: bool = False) -> str:
    if new is None or old is None:
        return "—"
    text = format(new - old, spec)
    if pct and old:
        text += f" ({(new - old) / old * 100:+.0f}%)"
    return text


def render_table(rows: list[dict]) -> str:
    """One markdown table per comparable group, oldest first, deltas to the previous row."""
    groups: dict[str, list[dict]] = {}
    for row in rows:
        groups.setdefault(row["comparable_key"], []).append(row)
    order = sorted(groups.values(), key=lambda g: max(r["recorded_utc"] or "" for r in g), reverse=True)
    out = []
    header = (
        "| run (UTC) | git | windows | FPS median | Δ FPS | FPS min window | FPS 30–90 s "
        "| p50 ms | p95 ms | p99 ms | worst ms | entities at end | our code % | CPU cores busy |"
    )
    rule = "|" + "---|" * (header.count("|") - 1)
    for group in order:
        group.sort(key=lambda r: r["recorded_utc"] or "")
        out.append(f"### {group[0]['comparable_label']}")
        out.append("")
        out.append(f"`comparable_key` `{group[0]['comparable_key']}` · {len(group)} run(s)")
        out.append("")
        out.append(header)
        out.append(rule)
        previous = None
        for row in group:
            f, span = row["frames"], row["frames_30_90s"] or {}
            perf = row.get("simpleperf") or {}
            stat = row.get("simpleperf_stat") or {}
            delta = (
                _delta(f["fps_median"], previous["frames"]["fps_median"], pct=True)
                if previous
                else "—"
            )
            when = (row["recorded_utc"] or "?").replace("T", " ").removesuffix("Z")[:16]
            out.append(
                "| "
                + " | ".join(
                    [
                        f"{when}<br>`{row['record_id']}`",
                        f"`{(row['git_head'] or '?')[:9]}`",
                        str(f["windows"]),
                        _fmt(f["fps_median"], ".1f"),
                        delta,
                        _fmt(f["fps_min_window"], ".1f"),
                        _fmt(span.get("fps_median"), ".1f"),
                        _fmt(f["p50_ms_median"], ".1f"),
                        _fmt(f["p95_ms_median"], ".1f"),
                        _fmt(f["p99_ms_median"], ".1f"),
                        _fmt(f["max_ms"], ".1f"),
                        _fmt(row["end_of_run"]["entities"]),
                        _fmt((perf.get("owners_pct") or {}).get("our_code"), ".0f"),
                        _fmt(stat.get("cpu_cores_busy"), ".2f"),
                    ]
                )
                + " |"
            )
            previous = row
        out.append("")
    return "\n".join(out)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="The Android play-cost ledger: one row per profile_android.sh bundle.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument("--ledger", default=str(LEDGER), help="target JSONL")
    sub = parser.add_subparsers(dest="command", required=True)
    append = sub.add_parser("append", help="append bundles; skip record_ids already present")
    append.add_argument("bundles", nargs="+")
    row = sub.add_parser("row", help="print one bundle's row; write nothing")
    row.add_argument("bundle")
    sub.add_parser("table", help="print the markdown table, grouped by comparable_key")
    args = parser.parse_args(argv)
    ledger = Path(args.ledger)

    if args.command == "append":
        return cmd_append(args.bundles, ledger)
    if args.command == "row":
        try:
            print(json.dumps(build_row(Path(args.bundle)), indent=2, sort_keys=True))
        except NotGameplay as why:
            print(f"⛔ refusing {args.bundle}: {why}", file=sys.stderr)
            return 3
        return 0
    rows = read_ledger(ledger)
    if not rows:
        print(f"no rows in {ledger}", file=sys.stderr)
        return 1
    print(render_table(rows))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
