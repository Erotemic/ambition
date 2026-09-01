#!/usr/bin/env python3
"""Turn the game's `[census]` stderr rows into one CSV per census kind.

The game emits every workload census as a single line:

    [    12.345s] [census] camera t=12.000 entity=64v1 role=main_gameplay ...

The leading `[   12.345s]` stamp is wall-clock seconds since the profiler
launched the process; the `t=` field is seconds since the census clock started
inside it. BOTH are written to every row: the stamp is what lines a census row
up against a perf window, and `t=` is what lines it up against the other
censuses sampled in the same frame.

Also lifts the always-on `[frame-spike]` / `[frame-census]` / `[image]` /
`[image-census]` lines, so a bundle taken without the census gate still has
frame timings.
"""

from __future__ import annotations

import csv
import os
import re
import sys

STAMP = re.compile(r"^\[\s*([0-9.]+)s\]\s?(.*)$")
CENSUS = re.compile(r"^\[census\]\s+(\w+)\s+(.*)$")
# key=value, where value is either a double-quoted string (Rust's {:?} on a
# name with spaces in it) or a run of non-space characters.
FIELD = re.compile(r'(\w+)=("(?:[^"\\]|\\.)*"|\S*)')

SPIKE = re.compile(r"^\[frame-spike\]\s+([0-9.]+)s\s+([0-9.]+)ms")
WINDOW = re.compile(
    r"^\[frame-census\]\s+([0-9.]+)s-([0-9.]+)s\s+frames=(\d+)\s+"
    r"p50=([0-9.]+)ms\s+p95=([0-9.]+)ms\s+p99=([0-9.]+)ms\s+max=([0-9.]+)ms"
)
# `live=` says whether gameplay was running when the decode landed, and it is
# emitted on BOTH branches so its ABSENCE means "this log predates the marker",
# not "nothing was late". Optional here for exactly that reason.
# ⭐ WORLD EVENTS — room loads, session starts/ends — with a game clock and a
# frame number. These say WHAT THE PLAYER WAS DOING when a number moved, and the
# summary's decode classifier already depends on them: a big decode seconds after
# a `room-loaded` is a room still arriving, the same decode in settled play is a
# contract violation. That classifier re-regexed the raw log because this was not
# parsed; a signal something already depends on belongs in a CSV.
WORLD_EVENT = re.compile(
    r"^\[world-event\]\s+(?P<t>[0-9.]+)s\s+f\s*(?P<frame>\d+)\s+(?P<kind>\S+)"
    r"(?:\s+(?P<detail>.*))?$"
)

# ⭐ THE ARRIVAL RATE, WHICH IS THE EXTRACT-SPIKE PREDICTOR. Each image that
# reaches `Assets<Image>` is extracted into the render world exactly once, and
# that extract is what lands on a frame — measured at 454.9ms max against a 0.1ms
# mean on hardware. So "+N images this window" forecasts the spike better than any
# cumulative total does. The module docstring claimed these lines were lifted;
# they were not, and the one signal that predicts the hitch never reached a CSV.
IMAGE_CENSUS = re.compile(
    r"^\[image-census\]\s+(?P<t>[0-9.]+)s\s+\+(?P<images>\d+) images"
    r"\s+\(\+(?P<mp>[0-9.]+)MP\)\s+\|\s+total (?P<total_images>\d+) images,"
    r"\s+(?P<total_mp>[0-9.]+)MP,\s+(?P<resident_mb>[0-9.]+)MB"
)

# ⛔ THE CONTRACT VIOLATION, NAMED. A fighter in the roster is known at select
# time, so its sheets have the whole preparation window to arrive. One still
# resolving after the opening bell decodes on gameplay frames — the shape that
# measured 516ms. The engine emits this at most once per character, so a row here
# is a defect rather than a rate.
LATE_ART = re.compile(
    r"^\[late-art\]\s+ticks_after_live=(?P<ticks>\d+)\s+character=(?P<character>\S+)"
)

IMAGE = re.compile(
    r"^\[image\]\s+(?P<t>[0-9.]+)s\s+(?P<w>\d+)x(?P<h>\d+)\s+(?P<mp>[0-9.]+)MP"
    r"\s+(?:live=(?P<live>[01])\s+)?(?P<path>.*)$"
)

# One output file per census kind. Kinds not listed here still get a CSV named
# after themselves; this table only fixes the names the summary and the docs
# refer to.
CSV_NAMES = {
    "frame": "frame_times.csv",
    "camera": "camera_views.csv",
    "views": "view_totals.csv",
    "ecs": "runtime_census.csv",
    "schedules": "schedule_census.csv",
    "draws": "draw_census.csv",
    "render_targets": "render_target_census.csv",
    "render_pass": "render_diagnostics.csv",
    "render_pass_summary": "render_diagnostics_status.csv",
    "portal": "portal_activity.csv",
    "assets": "asset_activity.csv",
    "phases": "schedule_phases.csv",
    # The same split on a CPU clock. `wall - cpu` per phase is the stall; a
    # phase whose wall far exceeds its CPU was WAITING, not working.
    "phases_cpu": "schedule_phases_cpu.csv",
    "config": "census_config.csv",
}


def unquote(value: str) -> str:
    if len(value) >= 2 and value[0] == '"' and value[-1] == '"':
        return value[1:-1].replace('\\"', '"').replace("\\\\", "\\")
    return value


def read_stamped_log(out_dir: str) -> list[tuple[float, str]]:
    """Every stamped line as `(wall_seconds, text)`, oldest first."""
    lines: list[tuple[float, str]] = []
    for name in ("game-stderr-stamped.txt", "game-stdout-stamped.txt"):
        path = os.path.join(out_dir, name)
        if not os.path.exists(path):
            continue
        with open(path, encoding="utf-8", errors="replace") as handle:
            for line in handle:
                match = STAMP.match(line)
                if match:
                    lines.append((float(match.group(1)), match.group(2).rstrip()))
    lines.sort(key=lambda row: row[0])
    return lines


def write_csv(path: str, columns: list[str], rows: list[dict]) -> None:
    with open(path, "w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=columns, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)


def collect_census(lines: list[tuple[float, str]]) -> dict[str, list[dict]]:
    by_kind: dict[str, list[dict]] = {}
    for wall, text in lines:
        match = CENSUS.match(text)
        if not match:
            continue
        kind, rest = match.group(1), match.group(2)
        row: dict[str, object] = {"wall_s": f"{wall:.3f}"}
        for key, value in FIELD.findall(rest):
            row[key] = unquote(value)
        by_kind.setdefault(kind, []).append(row)
    return by_kind


def collect_always_on(lines: list[tuple[float, str]], out_dir: str) -> dict[str, int]:
    """The censuses that run without the profiling gate."""
    spikes, windows, images, arrivals, world_events = [], [], [], [], []
    late_art = []
    for wall, text in lines:
        match = SPIKE.match(text)
        if match:
            spikes.append(
                {"wall_s": f"{wall:.3f}", "game_s": match.group(1), "frame_ms": match.group(2)}
            )
            continue
        match = WINDOW.match(text)
        if match:
            windows.append(
                {
                    "wall_s": f"{wall:.3f}",
                    "window_start_s": match.group(1),
                    "window_end_s": match.group(2),
                    "frames": match.group(3),
                    "p50_ms": match.group(4),
                    "p95_ms": match.group(5),
                    "p99_ms": match.group(6),
                    "max_ms": match.group(7),
                }
            )
            continue
        match = WORLD_EVENT.match(text)
        if match:
            world_events.append(
                {
                    "wall_s": f"{wall:.3f}",
                    "game_s": match.group("t"),
                    "frame": match.group("frame"),
                    "kind": match.group("kind"),
                    "detail": (match.group("detail") or "").strip(),
                }
            )
            continue
        match = IMAGE_CENSUS.match(text)
        if match:
            arrivals.append(
                {
                    "wall_s": f"{wall:.3f}",
                    "game_s": match.group("t"),
                    "images_this_window": match.group("images"),
                    "megapixels_this_window": match.group("mp"),
                    "total_images": match.group("total_images"),
                    "total_megapixels": match.group("total_mp"),
                    "resident_mb": match.group("resident_mb"),
                }
            )
            continue
        match = LATE_ART.match(text)
        if match:
            late_art.append(
                {
                    "wall_s": f"{wall:.3f}",
                    "ticks_after_live": match.group("ticks"),
                    "character": match.group("character"),
                }
            )
            continue
        match = IMAGE.match(text)
        if match:
            images.append(
                {
                    "wall_s": f"{wall:.3f}",
                    "game_s": match.group("t"),
                    "width": match.group("w"),
                    "height": match.group("h"),
                    "megapixels": match.group("mp"),
                    # The engine appends its own explanation after the path; keep
                    # the path alone so the column stays joinable.
                    "path": match.group("path").split(" — DECODED DURING")[0].strip(),
                    # "" (not "0") when the log predates the marker: unknown is
                    # not the same answer as no.
                    "during_gameplay": match.group("live") or "",
                }
            )
    write_csv(
        os.path.join(out_dir, "frame_spikes.csv"),
        ["wall_s", "game_s", "frame_ms"],
        spikes,
    )
    write_csv(
        os.path.join(out_dir, "frame_windows.csv"),
        ["wall_s", "window_start_s", "window_end_s", "frames", "p50_ms", "p95_ms", "p99_ms", "max_ms"],
        windows,
    )
    write_csv(
        os.path.join(out_dir, "world_events.csv"),
        ["wall_s", "game_s", "frame", "kind", "detail"],
        world_events,
    )
    write_csv(
        os.path.join(out_dir, "image_arrivals.csv"),
        ["wall_s", "game_s", "images_this_window", "megapixels_this_window",
         "total_images", "total_megapixels", "resident_mb"],
        arrivals,
    )
    write_csv(
        os.path.join(out_dir, "late_match_critical_art.csv"),
        ["wall_s", "ticks_after_live", "character"],
        late_art,
    )
    write_csv(
        os.path.join(out_dir, "image_decodes.csv"),
        ["wall_s", "game_s", "width", "height", "megapixels", "path", "during_gameplay"],
        images,
    )
    return {
        "frame_spikes": len(spikes),
        "frame_windows": len(windows),
        "image_decodes": len(images),
        "image_arrivals": len(arrivals),
        "world_events": len(world_events),
    }


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: profile_census_csv.py <bundle-dir>", file=sys.stderr)
        return 2
    out_dir = argv[1]
    lines = read_stamped_log(out_dir)
    if not lines:
        # Attach modes never see the game's stdio. Say so in the bundle rather
        # than leaving a reader to wonder whether the game printed nothing.
        with open(os.path.join(out_dir, "census.missing"), "w", encoding="utf-8") as handle:
            handle.write(
                "no stamped game log in this bundle: the census rows travel on the "
                "game's stderr, which an attach-mode capture never sees. Use a "
                "*-run mode for camera/entity/portal rows.\n"
            )
        return 0

    written = []
    for kind, rows in sorted(collect_census(lines).items()):
        columns: list[str] = []
        for row in rows:
            for key in row:
                if key not in columns:
                    columns.append(key)
        name = CSV_NAMES.get(kind, f"census_{kind}.csv")
        write_csv(os.path.join(out_dir, name), columns, rows)
        written.append(f"{name}: {len(rows)} rows")

    counts = collect_always_on(lines, out_dir)
    written.extend(f"{name}.csv: {count} rows" for name, count in sorted(counts.items()))
    with open(os.path.join(out_dir, "census_files.txt"), "w", encoding="utf-8") as handle:
        handle.write("\n".join(written) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
