#!/usr/bin/env python3
"""Render compile telemetry as a self-contained HTML report.

This script is read-only: it performs no builds and writes no telemetry. Missing
measurement ledgers are treated as empty data so the report remains useful in a
clone where the measurements submodule has not been initialized.

Comparable build state is derived from recorded fresh/dirty counters rather than
configuration labels. Every section reports its sample count, and line-based
costs are presented as within-crate proxies rather than interchangeable
cross-crate timings. Charts, CSS, and scripts are embedded in the output.

Usage::

    python3 scripts/compile_report.py
    python3 scripts/compile_report.py -o /tmp/report.html
    python3 scripts/compile_report.py --print-summary"""

from __future__ import annotations

import argparse
import html
import json
import math
import statistics
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

import measurement_paths  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
DEV = ROOT / "dev"

MEASUREMENTS = measurement_paths.MEASUREMENTS
LEDGERS = measurement_paths.LEDGERS

UNITS_LEDGER = measurement_paths.UNIT_LEDGER
JOBS_LEDGER = measurement_paths.JOBS_LEDGER
SCENARIO_LEDGER = measurement_paths.SCENARIO_LEDGER
GRAPH_LEDGER = measurement_paths.GRAPH_LEDGER
CARVE_LEDGER = measurement_paths.CARVE_LEDGER

# the OUTPUT stays in `dev/` and is gitignored. It is a view, regenerated in
# under a second, and it is not a measurement — nothing about it wants the
# submodule's history.
DEFAULT_OUTPUT = DEV / "compile_report.html"

# The crate the "cheapest per line" claim is about, and the crate that turned out
# to be the real outlier. Both are direct-labelled on the scatter.
MONOLITH = "ambition_platformer2d_actor_monolith"


# --------------------------------------------------------------------------- #
# Loading
# --------------------------------------------------------------------------- #


@dataclass(frozen=True)
class LedgerLoad:
    """What a ledger file yielded, including the ways it disappointed us.

    A fresh clone has none of these files, so `missing` is an ordinary outcome
    and not an error. `malformed` counts lines that were not JSON — recorded
    rather than swallowed, because a truncated append is exactly the kind of
    thing a reader should say out loud.
    """

    path: Path
    rows: list[dict]
    missing: bool = False
    malformed: int = 0

    @property
    def n(self) -> int:
        return len(self.rows)

    @property
    def label(self) -> str:
        """Where this page LOOKED, repo-relative.

        ⛔ every empty section used to hard-code its own copy of the path, so
        moving the ledgers into `dev/ambition_dev_measurements` would have left
        four sections telling a reader to look somewhere the file has not been
        for a while. The load knows where it went; ask it.
        """
        if self.path.is_absolute() and self.path.is_relative_to(ROOT):
            return str(self.path.relative_to(ROOT))
        return str(self.path)


def load_jsonl(path: Path) -> LedgerLoad:
    if not path.exists():
        return LedgerLoad(path=path, rows=[], missing=True)
    rows: list[dict] = []
    malformed = 0
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            parsed = json.loads(line)
        except json.JSONDecodeError:
            malformed += 1
            continue
        if isinstance(parsed, dict):
            rows.append(parsed)
        else:
            malformed += 1
    return LedgerLoad(path=path, rows=rows, malformed=malformed)


# --------------------------------------------------------------------------- #
# Unit rows: builds
# --------------------------------------------------------------------------- #


@dataclass(frozen=True)
class Build:
    """One cargo invocation — one `cargo-timing-*.html`, one row of this table."""

    source: str
    name: str
    config: str | None
    phase: str | None
    profile: str | None
    commit: str | None
    started_at: str | None
    wall_seconds: float | None
    unit_seconds: float
    units: int
    first_party_units: int
    fresh_units: int | None
    dirty_units: int | None
    total_units: int | None
    cores: int | None
    load_mean: float | None
    load_max: float | None
    incremental: bool | None
    backfilled: bool
    opt_levels: dict[str, int]

    @property
    def cache_state(self) -> str:
        """Derived from the cached-unit counters, never read off `phase`.

        ⚠ three states, not two, because a build can be neither. The release
        `cold` phase had 148 of 689 units already cached — genuinely mostly
        cold, and calling it `warm` because the count was nonzero would be as
        wrong as calling the 0-of-688 build `first-party` because its label
        said so.
        """
        if self.fresh_units is None:
            return "unknown"
        if self.fresh_units == 0:
            return "cold"
        if self.total_units and self.fresh_units / self.total_units < 0.5:
            return "mostly cold"
        return "warm"

    @property
    def label_disputed(self) -> bool:
        """The `phase` column claims a rebuild the cached-unit counts deny."""
        return self.phase == "first-party" and self.cache_state == "cold"

    @property
    def dominant_opt_level(self) -> str:
        if not self.opt_levels:
            return "?"
        return max(self.opt_levels.items(), key=lambda kv: kv[1])[0]


def _build_name(source: str, started_at: str | None, index: int) -> str:
    """A short handle. The collector's filenames carry a UTC stamp; use it."""
    stem = source.rsplit("/", 1)[-1]
    if stem.startswith("cargo-timing-") and "T" in stem:
        stamp = stem[len("cargo-timing-") :].split("-", 1)[0]
        if len(stamp) >= 15:
            return f"{stamp[4:6]}-{stamp[6:8]} {stamp[9:11]}:{stamp[11:13]}"
    if started_at and len(started_at) >= 16:
        return f"{started_at[5:7]}-{started_at[8:10]} {started_at[11:16]}"
    return f"build {index + 1}"


def _header_seconds(value) -> float | None:
    """cargo's own `Total time:` from the report header, e.g. `106.4s (1m 46.4s)`.

    Used only where the collector recorded no wall clock of its own — the four
    oldest rows predate that column. It is cargo's number, not a stopwatch's.
    """
    if not isinstance(value, str):
        return None
    head = value.split("s", 1)[0].strip()
    try:
        return float(head)
    except ValueError:
        return None


def summarise_builds(unit_rows: list[dict]) -> list[Build]:
    grouped: dict[str, list[dict]] = defaultdict(list)
    for row in unit_rows:
        grouped[row.get("build_source") or "(unknown source)"].append(row)

    builds: list[Build] = []
    for index, (source, rows) in enumerate(
        sorted(grouped.items(), key=lambda kv: kv[1][0].get("build_build_started_at") or "")
    ):
        head = rows[0]
        opt_levels: dict[str, int] = defaultdict(int)
        for row in rows:
            opt_levels[str(row.get("opt_level"))] += 1
        builds.append(
            Build(
                source=source,
                name=_build_name(source, head.get("build_build_started_at"), index),
                config=head.get("config"),
                phase=head.get("phase"),
                profile=head.get("build_profile"),
                commit=head.get("commit"),
                started_at=head.get("build_build_started_at"),
                wall_seconds=(
                    head.get("build_wall_seconds")
                    if head.get("build_wall_seconds") is not None
                    else _header_seconds(head.get("build_total_seconds"))
                ),
                unit_seconds=sum(r.get("seconds") or 0.0 for r in rows),
                units=len(rows),
                first_party_units=sum(1 for r in rows if r.get("first_party")),
                fresh_units=head.get("build_fresh_units"),
                dirty_units=head.get("build_dirty_units"),
                total_units=head.get("build_total_units"),
                cores=head.get("build_cores"),
                load_mean=head.get("build_load_mean"),
                load_max=head.get("build_load_max"),
                incremental=head.get("incremental"),
                backfilled=bool(head.get("backfilled")),
                opt_levels=dict(opt_levels),
            )
        )
    return builds


# --------------------------------------------------------------------------- #
# Unit rows: cost per line
# --------------------------------------------------------------------------- #


@dataclass(frozen=True)
class CrateCost:
    crate: str
    lines: int
    seconds: float
    frontend_seconds: float
    codegen_seconds: float
    units: int

    @property
    def ms_per_line(self) -> float:
        return self.seconds / self.lines * 1000.0 if self.lines else float("inf")

    @property
    def codegen_share(self) -> float | None:
        split = self.frontend_seconds + self.codegen_seconds
        return self.codegen_seconds / split if split else None


def crate_costs_by_build(unit_rows: list[dict], *, first_party_only: bool = True) -> dict[str, list[CrateCost]]:
    """Per-crate seconds and ms/line, keyed by build.

    ⛔ **never pooled across builds.** A crate's cost is a property of the build
    it was measured in — the same crate reads 0.67 ms/line in a warm dev rebuild
    and 2.77 ms/line in a release cold build, and their average describes
    nothing that ever happened.

    Several units belong to one crate in one build (lib, test target, build
    script). Those SUM; the crate's `lines` is one number, not their total.
    """
    by_build: dict[str, dict[str, list[dict]]] = defaultdict(lambda: defaultdict(list))
    for row in unit_rows:
        if first_party_only and not row.get("first_party"):
            continue
        by_build[row.get("build_source") or "(unknown source)"][row.get("unit") or "?"].append(row)

    out: dict[str, list[CrateCost]] = {}
    for source, crates in by_build.items():
        costs = []
        for crate, rows in crates.items():
            costs.append(
                CrateCost(
                    crate=crate,
                    lines=max((r.get("lines") or 0) for r in rows),
                    seconds=sum(r.get("seconds") or 0.0 for r in rows),
                    frontend_seconds=sum(r.get("frontend_seconds") or 0.0 for r in rows),
                    codegen_seconds=sum(r.get("codegen_seconds") or 0.0 for r in rows),
                    units=len(rows),
                )
            )
        out[source] = sorted(costs, key=lambda c: c.ms_per_line)
    return out


@dataclass(frozen=True)
class SplitTotals:
    """Frontend/codegen totals with BOTH denominators kept side by side."""

    frontend_seconds: float
    codegen_seconds: float
    all_seconds: float
    units: int
    units_with_split: int

    @property
    def attributed_seconds(self) -> float:
        return self.frontend_seconds + self.codegen_seconds

    @property
    def unattributed_seconds(self) -> float:
        return self.all_seconds - self.attributed_seconds

    @property
    def codegen_share_of_split(self) -> float | None:
        return self.codegen_seconds / self.attributed_seconds if self.attributed_seconds else None

    @property
    def codegen_share_of_all(self) -> float | None:
        return self.codegen_seconds / self.all_seconds if self.all_seconds else None


def split_totals(unit_rows: list[dict]) -> SplitTotals:
    frontend = codegen = total = 0.0
    with_split = 0
    for row in unit_rows:
        total += row.get("seconds") or 0.0
        fe, cg = row.get("frontend_seconds"), row.get("codegen_seconds")
        if fe is None and cg is None:
            continue
        with_split += 1
        frontend += fe or 0.0
        codegen += cg or 0.0
    return SplitTotals(
        frontend_seconds=round(frontend, 6),
        codegen_seconds=round(codegen, 6),
        all_seconds=round(total, 6),
        units=len(unit_rows),
        units_with_split=with_split,
    )


# --------------------------------------------------------------------------- #
# Unit rows: the dimension group-by
# --------------------------------------------------------------------------- #


@dataclass(frozen=True)
class DimensionGroup:
    profile: str
    opt_level: str
    incremental: str
    units: int
    seconds: float
    crates: int


def dimension_groups(unit_rows: list[dict]) -> list[DimensionGroup]:
    """Group by the three dimensions the schema promised: profile × opt × incr.

    ⚠ this is the group-by that reveals a hole rather than a finding:
    `incremental` is `false` on every collector row, so the axis exists in the
    schema and has exactly one value in the data. The page says so instead of
    drawing a one-bar comparison.
    """
    buckets: dict[tuple[str, str, str], list[dict]] = defaultdict(list)
    for row in unit_rows:
        incremental = row.get("incremental")
        buckets[
            (
                str(row.get("build_profile") or "?"),
                str(row.get("opt_level") or "?"),
                "unrecorded" if incremental is None else ("on" if incremental else "off"),
            )
        ].append(row)

    groups = [
        DimensionGroup(
            profile=key[0],
            opt_level=key[1],
            incremental=key[2],
            units=len(rows),
            seconds=round(sum(r.get("seconds") or 0.0 for r in rows), 2),
            crates=len({r.get("unit") for r in rows}),
        )
        for key, rows in buckets.items()
    ]
    return sorted(groups, key=lambda g: (-g.seconds,))


# --------------------------------------------------------------------------- #
# Scenario rows
# --------------------------------------------------------------------------- #

# `dev/compile_telemetry_schema.md` §4, transcribed. The ledger is append-only and these four
# rows are NOT rewritten, so the mapping lives at read time.
_SCHEMA_ZERO_MAP = {
    ("env-incremental-1", ""): {"incremental": True, "profile": "test", "opt_level": "1"},
    ("env-empty", "baseline-config-default"): {"incremental": False, "profile": "test", "opt_level": "1"},
    ("env-empty", "config-incremental-on"): {"incremental": True, "profile": "dev", "opt_level": "1"},
}


def normalise_scenario(row: dict) -> dict:
    """One scenario row with its dimensions resolved, and its warm pass judged.

    A schema-1 row states `profile` / `opt_level` / `incremental` itself. A
    schema-0 row gets the documented mapping, or `None` — never a guess.

    ⚠ **`after_edit - warm_noop` is only an edit cost when the warm pass was
    actually warm.** In two of the four recorded rows it was not (the tree owed
    work the "warm" run paid for), and there the subtraction yields a negative
    or near-zero number that means nothing. Those rows are flagged, not averaged.
    """
    out = dict(row)

    if row.get("schema"):
        out["dimension_source"] = "recorded"
        out.setdefault("incremental", None)
        out.setdefault("profile", None)
        out.setdefault("opt_level", None)
    else:
        env = row.get("env") or {}
        env_key = "env-incremental-1" if env.get("CARGO_INCREMENTAL") not in (None, "", "0") else "env-empty"
        label = row.get("label") or ""
        mapped = _SCHEMA_ZERO_MAP.get((env_key, label))
        if mapped is None and env_key == "env-incremental-1":
            mapped = _SCHEMA_ZERO_MAP[("env-incremental-1", "")]
        out["dimension_source"] = "normalised (schema 0)" if mapped else "unknown (schema 0, unmapped)"
        out["incremental"] = mapped["incremental"] if mapped else None
        out["profile"] = mapped["profile"] if mapped else None
        out["opt_level"] = mapped["opt_level"] if mapped else None

    warm = row.get("warm_noop_seconds")
    edited = row.get("after_edit_seconds")
    suspect = warm is not None and edited is not None and warm >= edited * 0.5
    out["warm_pass_suspect"] = bool(suspect)
    out["edit_cost_seconds"] = (
        None if (warm is None or edited is None or suspect) else round(edited - warm, 2)
    )
    return out


# --------------------------------------------------------------------------- #
# Test job rows
# --------------------------------------------------------------------------- #


@dataclass(frozen=True)
class JobCost:
    job: str
    runs: int
    seconds: float
    executed_seconds: float
    failures: int

    @property
    def build_seconds(self) -> float:
        """`seconds - executed_seconds` — the build graph, not the running."""
        return round(self.seconds - self.executed_seconds, 6)

    @property
    def build_share(self) -> float | None:
        return self.build_seconds / self.seconds if self.seconds else None


def job_costs(run_rows: list[dict]) -> list[JobCost]:
    agg: dict[str, list[float]] = defaultdict(lambda: [0.0, 0.0, 0.0, 0.0])
    for run in run_rows:
        for job in run.get("per_job") or []:
            slot = agg[job.get("job") or "?"]
            slot[0] += 1
            slot[1] += job.get("seconds") or 0.0
            slot[2] += job.get("executed_seconds") or 0.0
            slot[3] += 0 if job.get("ok", True) else 1
    return sorted(
        (
            JobCost(
                job=name,
                runs=int(slot[0]),
                seconds=round(slot[1], 6),
                executed_seconds=round(slot[2], 6),
                failures=int(slot[3]),
            )
            for name, slot in agg.items()
        ),
        key=lambda j: -j.seconds,
    )


@dataclass(frozen=True)
class SuiteTotals:
    runs: int
    seconds: float
    executed_seconds: float
    # ⛔⛔ WALL TIME THE RUNNER COULD NOT SPLIT, and it is not build time.
    # `build_seconds` USED to be `seconds - executed_seconds` over every run,
    # which quietly handed a nextest run's entire wall clock to the build column
    # — nextest prints its `Summary [ Xs ]` on stderr, which the runner leaves
    # attached, so those jobs report no execution time at all. The build figure
    # is now derived ONLY from the runs that reported, and this is what the rest
    # is.
    unclassified_seconds: float = 0.0

    @property
    def classified_seconds(self) -> float:
        return round(self.seconds - self.unclassified_seconds, 6)

    @property
    def build_seconds(self) -> float:
        return round(self.classified_seconds - self.executed_seconds, 6)

    @property
    def build_share(self) -> float | None:
        """Build as a share of the time this report can actually account for."""
        classified = self.classified_seconds
        return self.build_seconds / classified if classified else None


def _unclassified(run: dict) -> float:
    """Wall time in one run that no runner attributed.

    ⚠ A row written before 2026-08-28 has no `unclassified_seconds` key AND an
    `executed_seconds` that may be a zero standing in for "unknown". Its zero is
    kept as-is rather than reinterpreted: guessing which historical zeros were
    real would rewrite the series this report exists to show.
    """
    return run.get("unclassified_seconds") or 0.0


def suite_totals(run_rows: list[dict]) -> SuiteTotals:
    return SuiteTotals(
        runs=len(run_rows),
        seconds=round(sum(r.get("seconds") or 0.0 for r in run_rows), 6),
        executed_seconds=round(sum(r.get("executed_seconds") or 0.0 for r in run_rows), 6),
        unclassified_seconds=round(sum(_unclassified(r) for r in run_rows), 6),
    )


def suite_series(run_rows: list[dict]) -> list[dict]:
    """One point per suite invocation, oldest first — a real 75-point series."""
    points = []
    for run in run_rows:
        finished = run.get("finished")
        seconds = run.get("seconds") or 0.0
        executed = run.get("executed_seconds") or 0.0
        unclassified = _unclassified(run)
        points.append(
            {
                "finished": finished,
                "when": _epoch_to_day(finished),
                "seconds": seconds,
                "executed": executed,
                # ⛔ the same rule as `SuiteTotals`: only the classified part of
                # a run may be called build time.
                "build": max(seconds - unclassified - executed, 0.0),
                "unclassified": unclassified,
                "jobs": run.get("jobs"),
                "passed": run.get("passed"),
                "exhaustive": bool(run.get("exhaustive")),
            }
        )
    return sorted(points, key=lambda p: p["finished"] or 0.0)


def _epoch_to_day(epoch) -> str:
    if not epoch:
        return "?"
    return datetime.fromtimestamp(float(epoch), tz=timezone.utc).strftime("%m-%d %H:%M")


# --------------------------------------------------------------------------- #
# SVG primitives
# --------------------------------------------------------------------------- #


def esc(value) -> str:
    return html.escape(str(value), quote=True)


def _fmt(value, digits: int = 1) -> str:
    if value is None:
        return "—"
    if isinstance(value, float) and (math.isnan(value) or math.isinf(value)):
        return "—"
    return f"{value:,.{digits}f}"


def _pct(value) -> str:
    return "—" if value is None else f"{value * 100:.1f}%"


def _ordinal(n: int) -> str:
    if 11 <= n % 100 <= 13:
        return f"{n}th"
    return f"{n}{ {1: 'st', 2: 'nd', 3: 'rd'}.get(n % 10, 'th') }"


def _count(value) -> str:
    """`critical_path_crates` is a LENGTH in the ledger and a LIST in the
    baseline JSON the same script writes. Accept either rather than guess."""
    if value is None:
        return "—"
    return f"{len(value):,}" if isinstance(value, (list, tuple, dict)) else f"{value:,}"


def _log_ticks(lo: float, hi: float) -> list[float]:
    ticks = []
    exp = math.floor(math.log10(lo))
    while 10**exp <= hi * 1.0000001:
        for mult in (1, 2, 5):
            value = mult * 10**exp
            if lo * 0.999 <= value <= hi * 1.001:
                ticks.append(float(value))
        exp += 1
    return ticks


def _nice_ticks(hi: float, count: int = 5) -> list[float]:
    if hi <= 0:
        return [0.0]
    raw = hi / count
    exp = math.floor(math.log10(raw))
    for mult in (1, 2, 2.5, 5, 10):
        step = mult * 10**exp
        if step >= raw:
            break
    # the last tick must be >= hi, not <= it. A top tick BELOW the maximum
    # makes every bar scaled against it overflow its own plot area, which is
    # exactly what the first draft did to the widest test-job bar.
    ticks, value = [], 0.0
    while True:
        ticks.append(round(value, 10))
        if value >= hi - 1e-9:
            return ticks
        value += step


def svg_open(width: int, height: int, label: str) -> list[str]:
    return [
        f'<svg viewBox="0 0 {width} {height}" role="img" aria-label="{esc(label)}" '
        f'class="chart" preserveAspectRatio="xMidYMid meet">'
    ]


# --------------------------------------------------------------------------- #
# Charts
# --------------------------------------------------------------------------- #


def scatter_lines_vs_seconds(costs: list[CrateCost], *, title: str) -> str:
    """Log-log lines vs seconds, with constant-ms/line diagonals.

    The diagonals are the point: a crate BELOW one is cheap for its size and a
    crate above it is dear, which is the question "which crate is expensive for
    what it is" drawn rather than argued.
    """
    points = [c for c in costs if c.lines > 0 and c.seconds > 0]
    if not points:
        return '<p class="empty">No first-party units in this build.</p>'

    width, height = 760, 420
    pad_l, pad_r, pad_t, pad_b = 58, 18, 18, 44
    plot_w, plot_h = width - pad_l - pad_r, height - pad_t - pad_b

    xs = [c.lines for c in points]
    ys = [c.seconds for c in points]
    x_lo, x_hi = min(xs) / 1.6, max(xs) * 1.6
    y_lo, y_hi = min(ys) / 1.8, max(ys) * 1.8

    def px(v):
        return pad_l + (math.log10(v) - math.log10(x_lo)) / (math.log10(x_hi) - math.log10(x_lo)) * plot_w

    def py(v):
        return pad_t + plot_h - (math.log10(v) - math.log10(y_lo)) / (math.log10(y_hi) - math.log10(y_lo)) * plot_h

    out = svg_open(width, height, title)
    out.append(f'<rect x="{pad_l}" y="{pad_t}" width="{plot_w}" height="{plot_h}" class="plot-bg"/>')

    # Iso-cost diagonals: seconds = lines * ms_per_line / 1000.
    for ms in (0.3, 1, 3, 10, 30, 100):
        seg = []
        for lines in (x_lo, x_hi):
            seconds = lines * ms / 1000.0
            if y_lo <= seconds <= y_hi:
                seg.append((px(lines), py(seconds)))
            else:
                clamped = min(max(seconds, y_lo), y_hi)
                lines_at = clamped / ms * 1000.0
                if x_lo <= lines_at <= x_hi:
                    seg.append((px(lines_at), py(clamped)))
        if len(seg) == 2:
            (x1, y1), (x2, y2) = seg
            out.append(f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" class="iso"/>')
            out.append(
                f'<text x="{x2 - 4:.1f}" y="{y2 - 5:.1f}" class="iso-label" text-anchor="end">'
                f"{ms:g} ms/line</text>"
            )

    for tick in _log_ticks(x_lo, x_hi):
        x = px(tick)
        out.append(f'<line x1="{x:.1f}" y1="{pad_t}" x2="{x:.1f}" y2="{pad_t + plot_h}" class="grid"/>')
        label = f"{tick / 1000:g}k" if tick >= 1000 else f"{tick:g}"
        out.append(f'<text x="{x:.1f}" y="{pad_t + plot_h + 16}" class="tick" text-anchor="middle">{label}</text>')
    for tick in _log_ticks(y_lo, y_hi):
        y = py(tick)
        out.append(f'<line x1="{pad_l}" y1="{y:.1f}" x2="{pad_l + plot_w}" y2="{y:.1f}" class="grid"/>')
        out.append(f'<text x="{pad_l - 8}" y="{y + 4:.1f}" class="tick" text-anchor="end">{tick:g}</text>')

    out.append(
        f'<text x="{pad_l + plot_w / 2:.0f}" y="{height - 6}" class="axis-title" text-anchor="middle">'
        "physical lines of src/**/*.rs (log)</text>"
    )
    out.append(
        f'<text x="14" y="{pad_t + plot_h / 2:.0f}" class="axis-title" text-anchor="middle" '
        f'transform="rotate(-90 14 {pad_t + plot_h / 2:.0f})">compile seconds (log)</text>'
    )

    labelled = {MONOLITH, max(points, key=lambda c: c.ms_per_line).crate, max(points, key=lambda c: c.seconds).crate}
    for cost in points:
        x, y = px(cost.lines), py(cost.seconds)
        emphasised = cost.crate in labelled
        out.append(
            f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{5.5 if emphasised else 4}" '
            f'class="{"dot dot-key" if emphasised else "dot"}">'
            f"<title>{esc(cost.crate)}\n{cost.lines:,} lines · {cost.seconds:.2f}s · "
            f"{cost.ms_per_line:.2f} ms/line</title></circle>"
        )
    for cost in points:
        if cost.crate not in labelled:
            continue
        x, y = px(cost.lines), py(cost.seconds)
        anchor = "end" if x > pad_l + plot_w * 0.6 else "start"
        dx = -9 if anchor == "end" else 9
        out.append(
            f'<text x="{x + dx:.1f}" y="{y + 4:.1f}" class="point-label" text-anchor="{anchor}">'
            f"{esc(cost.crate.replace('ambition_', ''))} · {cost.ms_per_line:.2f}</text>"
        )

    out.append("</svg>")
    return "".join(out)


def stacked_bars(rows: list[dict], *, segments: list[tuple[str, str, str]], title: str, unit: str = "s") -> str:
    """Horizontal stacked bars. `rows` = [{label, values: {key: v}, note}].

    A 2px surface gap separates adjacent fills rather than a stroke.
    """
    if not rows:
        return '<p class="empty">Nothing to draw.</p>'

    bar_h, gap, pad_t, pad_b = 22, 12, 10, 34
    width, pad_r = 760, 12
    height = pad_t + len(rows) * (bar_h + gap) + pad_b
    char_w = 6.2  # 11.5px system sans, measured generously

    # size the gutters to the TEXT, then truncate to what is left.
    totals = [sum(r["values"].get(k, 0.0) for k, _, _ in segments) for r in rows]
    values = [f"{t:,.0f}{unit}{r.get('note') or ''}" for t, r in zip(totals, rows)]
    value_w = min(max((len(v) for v in values), default=0) * char_w + 14, 210)
    label_w = 268
    plot_w = width - label_w - value_w - pad_r
    max_label = int((label_w - 12) / char_w)
    labels = [
        (r["label"] if len(r["label"]) <= max_label else r["label"][: max_label - 1] + "…") for r in rows
    ]

    hi = max(totals) or 1.0
    ticks = _nice_ticks(hi)
    scale = plot_w / (ticks[-1] or 1.0)

    out = svg_open(width, height, title)
    for tick in ticks:
        x = label_w + tick * scale
        out.append(f'<line x1="{x:.1f}" y1="{pad_t}" x2="{x:.1f}" y2="{height - pad_b + 4}" class="grid"/>')
        out.append(f'<text x="{x:.1f}" y="{height - pad_b + 20}" class="tick" text-anchor="middle">{tick:,.0f}</text>')

    for index, row in enumerate(rows):
        y = pad_t + index * (bar_h + gap)
        out.append(
            f'<text x="{label_w - 10}" y="{y + bar_h * 0.72:.1f}" class="bar-label" text-anchor="end">'
            f"{esc(labels[index])}<title>{esc(row['label'])}</title></text>"
        )
        cursor = float(label_w)
        for key, css, human in segments:
            value = row["values"].get(key, 0.0)
            w = value * scale
            if w <= 0:
                continue
            drawn = max(w - 2, 0.6)  # the 2px surface gap between fills
            out.append(
                f'<rect x="{cursor:.1f}" y="{y}" width="{drawn:.1f}" height="{bar_h}" rx="2" class="seg {css}">'
                f"<title>{esc(row['label'])} — {human}: {value:,.1f}{unit}</title></rect>"
            )
            cursor += w
        out.append(
            f'<text x="{label_w + plot_w + 8}" y="{y + bar_h * 0.72:.1f}" class="bar-value">'
            f"{esc(values[index])}</text>"
        )
    out.append("</svg>")
    return "".join(out)


def stacked_area_over_time(points: list[dict], *, title: str) -> str:
    """Suite wall clock over 75 runs: executed at the bottom, build stacked on.

    One axis, one unit (seconds). The stack sums to the wall clock, so the top
    edge IS the total — no second scale is invented.
    """
    if len(points) < 2:
        return '<p class="empty">Fewer than two runs — nothing to trend.</p>'

    width, height = 760, 300
    pad_l, pad_r, pad_t, pad_b = 58, 14, 14, 40
    plot_w, plot_h = width - pad_l - pad_r, height - pad_t - pad_b

    hi = max(p["seconds"] for p in points) or 1.0
    ticks = _nice_ticks(hi)
    top = ticks[-1] or 1.0
    n = len(points)

    def px(i):
        return pad_l + (i / max(n - 1, 1)) * plot_w

    def py(v):
        return pad_t + plot_h - (v / top) * plot_h

    out = svg_open(width, height, title)
    out.append(f'<rect x="{pad_l}" y="{pad_t}" width="{plot_w}" height="{plot_h}" class="plot-bg"/>')
    for tick in ticks:
        y = py(tick)
        out.append(f'<line x1="{pad_l}" y1="{y:.1f}" x2="{pad_l + plot_w}" y2="{y:.1f}" class="grid"/>')
        out.append(f'<text x="{pad_l - 8}" y="{y + 4:.1f}" class="tick" text-anchor="end">{tick:,.0f}</text>')

    lower = " ".join(f"{px(i):.1f},{py(p['executed']):.1f}" for i, p in enumerate(points))
    upper = " ".join(f"{px(i):.1f},{py(p['seconds']):.1f}" for i, p in enumerate(points))
    base = f"{px(n - 1):.1f},{py(0):.1f} {px(0):.1f},{py(0):.1f}"
    out.append(f'<polygon points="{lower} {base}" class="area area-exec"/>')
    reverse_lower = " ".join(f"{px(i):.1f},{py(p['executed']):.1f}" for i, p in reversed(list(enumerate(points))))
    out.append(f'<polygon points="{upper} {reverse_lower}" class="area area-build"/>')
    out.append(f'<polyline points="{upper}" class="line line-total"/>')

    for i, point in enumerate(points):
        out.append(
            f'<rect x="{px(i) - plot_w / (2 * max(n - 1, 1)):.1f}" y="{pad_t}" '
            f'width="{plot_w / max(n - 1, 1):.1f}" height="{plot_h}" class="hit">'
            f"<title>{esc(point['when'])} UTC\n{point['seconds']:,.0f}s wall · "
            f"{point['build']:,.0f}s build · {point['executed']:,.0f}s running\n"
            f"{point['jobs']} job(s){' · exhaustive' if point['exhaustive'] else ''}</title></rect>"
        )

    for i in (0, n // 2, n - 1):
        out.append(
            f'<text x="{px(i):.1f}" y="{height - 14}" class="tick" '
            f'text-anchor="{"start" if i == 0 else "end" if i == n - 1 else "middle"}">'
            f"{esc(points[i]['when'])}</text>"
        )
    out.append(
        f'<text x="14" y="{pad_t + plot_h / 2:.0f}" class="axis-title" text-anchor="middle" '
        f'transform="rotate(-90 14 {pad_t + plot_h / 2:.0f})">seconds</text>'
    )
    out.append("</svg>")
    return "".join(out)


def single_point_chart(label: str, value: float, unit: str, caption: str) -> str:
    """⛔ ONE sample. This draws one dot on a bare axis and says so.

    A line chart here would be a lie with a trend in it; the whole purpose of
    this function is to be visibly not that.
    """
    width, height = 760, 96
    pad_l, pad_r = 58, 18
    out = svg_open(width, height, f"{label}: one sample")
    y = 44
    out.append(f'<line x1="{pad_l}" y1="{y}" x2="{width - pad_r}" y2="{y}" class="baseline"/>')
    out.append(f'<circle cx="{pad_l}" cy="{y}" r="6" class="dot dot-key"/>')
    out.append(f'<text x="{pad_l + 16}" y="{y - 8}" class="point-label">{esc(label)}: {value:,.0f}{esc(unit)}</text>')
    out.append(f'<text x="{pad_l + 16}" y="{y + 16}" class="tick">{esc(caption)}</text>')
    out.append(
        f'<text x="{width - pad_r}" y="{y + 16}" class="tick one-point" text-anchor="end">'
        "← 1 sample. No trend exists.</text>"
    )
    out.append("</svg>")
    return "".join(out)


# --------------------------------------------------------------------------- #
# Page assembly
# --------------------------------------------------------------------------- #

CSS = """
:root{color-scheme:light dark;
--page:#f9f9f7;--surface:#fcfcfb;--ink:#0b0b0b;--ink2:#52514e;--muted:#898781;
--grid:#e1e0d9;--axis:#c3c2b7;--rule:rgba(11,11,11,.10);
--s1:#2a78d6;--s2:#eb6834;--s3:#1baf7a;--warn:#d03b3b;--warnbg:rgba(208,59,59,.08);
--notebg:rgba(42,120,214,.07);}
@media (prefers-color-scheme:dark){:root:where(:not([data-theme="light"])){
--page:#0d0d0d;--surface:#1a1a19;--ink:#fff;--ink2:#c3c2b7;--muted:#898781;
--grid:#2c2c2a;--axis:#383835;--rule:rgba(255,255,255,.10);
--s1:#3987e5;--s2:#d95926;--s3:#199e70;--warn:#e66767;--warnbg:rgba(230,103,103,.12);
--notebg:rgba(57,135,229,.12);}}
:root[data-theme="dark"]{--page:#0d0d0d;--surface:#1a1a19;--ink:#fff;--ink2:#c3c2b7;--muted:#898781;
--grid:#2c2c2a;--axis:#383835;--rule:rgba(255,255,255,.10);
--s1:#3987e5;--s2:#d95926;--s3:#199e70;--warn:#e66767;--warnbg:rgba(230,103,103,.12);
--notebg:rgba(57,135,229,.12);}
*{box-sizing:border-box}
body{margin:0;background:var(--page);color:var(--ink);
font:15px/1.55 system-ui,-apple-system,"Segoe UI",sans-serif;}
.wrap{max-width:900px;margin:0 auto;padding:32px 20px 80px}
h1{font-size:1.75rem;line-height:1.2;margin:0 0 6px;letter-spacing:-.02em}
h2{font-size:1.2rem;margin:0 0 4px;letter-spacing:-.01em}
h3{font-size:.98rem;margin:22px 0 6px;color:var(--ink2)}
.sub{color:var(--ink2);margin:0 0 26px}
section{background:var(--surface);border:1px solid var(--rule);border-radius:10px;
padding:20px;margin:0 0 20px}
.n{color:var(--muted);font-size:.82rem;font-weight:600;letter-spacing:.04em;
text-transform:uppercase;margin:0 0 12px}
p{margin:0 0 12px;color:var(--ink2)}
p.lede{color:var(--ink)}
.note,.warn{border-radius:8px;padding:11px 13px;margin:12px 0;font-size:.9rem;color:var(--ink2)}
.note{background:var(--notebg)}
.warn{background:var(--warnbg);border-left:3px solid var(--warn);color:var(--ink)}
.warn strong,.note strong{color:var(--ink)}
.chart{width:100%;height:auto;display:block;margin:8px 0 4px;overflow:visible}
.plot-bg{fill:none}
.grid{stroke:var(--grid);stroke-width:1}
.baseline{stroke:var(--axis);stroke-width:1}
.iso{stroke:var(--axis);stroke-width:1;stroke-opacity:.75}
.iso-label{fill:var(--muted);font-size:10px}
.tick{fill:var(--muted);font-size:11px;font-variant-numeric:tabular-nums}
.axis-title{fill:var(--ink2);font-size:11.5px}
.dot{fill:var(--s1);fill-opacity:.72}
.dot-key{fill:var(--s2);fill-opacity:1;stroke:var(--surface);stroke-width:2}
.point-label{fill:var(--ink);font-size:11.5px;font-weight:600}
.one-point{fill:var(--warn);font-weight:600}
.bar-label{fill:var(--ink2);font-size:11.5px}
.bar-value{fill:var(--ink);font-size:11.5px;font-variant-numeric:tabular-nums}
.seg-1{fill:var(--s1)}.seg-2{fill:var(--s2)}.seg-3{fill:var(--s3)}
.seg-muted{fill:var(--axis)}
.area-exec{fill:var(--s3);fill-opacity:.85}
.area-build{fill:var(--s1);fill-opacity:.55}
.line-total{fill:none;stroke:var(--s1);stroke-width:2}
.hit{fill:transparent}
.legend{display:flex;flex-wrap:wrap;gap:14px;margin:6px 0 2px;font-size:.85rem;color:var(--ink2)}
.legend span{display:inline-flex;align-items:center;gap:6px}
.sw{width:11px;height:11px;border-radius:2px;display:inline-block}
.sw1{background:var(--s1)}.sw2{background:var(--s2)}.sw3{background:var(--s3)}
.swm{background:var(--axis)}
.scroll{overflow-x:auto;-webkit-overflow-scrolling:touch;margin:10px 0}
table{border-collapse:collapse;width:100%;font-size:.86rem;min-width:520px}
th,td{text-align:right;padding:6px 9px;border-bottom:1px solid var(--rule);white-space:nowrap}
th{color:var(--muted);font-weight:600;font-size:.78rem;text-transform:uppercase;
letter-spacing:.03em;position:sticky;top:0;background:var(--surface)}
td:first-child,th:first-child{text-align:left;white-space:normal}
td.num{font-variant-numeric:tabular-nums}
tr.hi td{background:rgba(235,104,52,.10);font-weight:600}
tr.bad td{color:var(--warn)}
.tiles{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:12px;margin:14px 0}
.tile{border:1px solid var(--rule);border-radius:8px;padding:11px 13px}
.tile .v{font-size:1.5rem;font-weight:650;letter-spacing:-.02em;line-height:1.1}
.tile .k{color:var(--muted);font-size:.76rem;text-transform:uppercase;letter-spacing:.04em;margin-top:3px}
.tile .x{color:var(--ink2);font-size:.8rem;margin-top:4px}
.picker{display:flex;flex-wrap:wrap;gap:7px;margin:14px 0 4px}
.picker button{font:inherit;font-size:.82rem;padding:5px 11px;border-radius:99px;cursor:pointer;
border:1px solid var(--rule);background:transparent;color:var(--ink2)}
.picker button[aria-pressed="true"]{background:var(--s1);border-color:var(--s1);color:#fff}
.buildblock+.buildblock{border-top:1px solid var(--rule);margin-top:22px;padding-top:14px}
code{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:.88em;
background:var(--notebg);padding:1px 5px;border-radius:4px}
.empty{color:var(--muted);font-style:italic}
footer{color:var(--muted);font-size:.82rem;margin-top:26px;text-align:center}
.carve{border:1px solid var(--rule);border-radius:8px;padding:14px;margin:12px 0}
.carve .path{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:.84rem;
color:var(--ink);word-break:break-all}
.carve .arrow{color:var(--s2);font-weight:700;margin:6px 0}
.carve .why{color:var(--ink2);font-size:.9rem;margin-top:9px}
"""

JS = """
// The ONLY script on this page, and it is a progressive enhancement: every
// build block is rendered visible, and this hides all but the selected one.
// If it never runs, the page is longer and completely intact.
(function () {
  document.querySelectorAll('[data-picker]').forEach(function (picker) {
    var blocks = document.querySelectorAll('[data-build-block="' + picker.dataset.picker + '"]');
    var buttons = picker.querySelectorAll('button');
    function show(key) {
      blocks.forEach(function (b) { b.hidden = b.dataset.buildKey !== key; });
      buttons.forEach(function (b) { b.setAttribute('aria-pressed', String(b.dataset.buildKey === key)); });
    }
    buttons.forEach(function (b) {
      b.addEventListener('click', function () { show(b.dataset.buildKey); });
    });
    if (buttons.length) { show(picker.dataset.default || buttons[0].dataset.buildKey); }
  });
})();
"""


def tile(value: str, key: str, extra: str = "") -> str:
    return (
        f'<div class="tile"><div class="v">{esc(value)}</div><div class="k">{esc(key)}</div>'
        + (f'<div class="x">{extra}</div>' if extra else "")
        + "</div>"
    )


def table(headers: list[str], rows: list[list[str]], *, classes: list[str] | None = None) -> str:
    head = "".join(f"<th>{esc(h)}</th>" for h in headers)
    body = []
    for index, row in enumerate(rows):
        css = f' class="{classes[index]}"' if classes and classes[index] else ""
        cells = "".join(
            f"<td{' class=\"num\"' if i else ''}>{cell}</td>" for i, cell in enumerate(row)
        )
        body.append(f"<tr{css}>{cells}</tr>")
    return f'<div class="scroll"><table><thead><tr>{head}</tr></thead><tbody>{"".join(body)}</tbody></table></div>'


# --------------------------------------------------------------------------- #
# Sections
# --------------------------------------------------------------------------- #


def section_coverage(loads: dict[str, LedgerLoad], builds: list[Build], series: list[dict]) -> str:
    rows, classes = [], []
    facts = {
        "compile_units": ("unit", "per-rustc-invocation wall time"),
        "run_tests_cost": ("job", "one suite invocation, with per-command rows"),
        "compile_cost": ("scenario", "edit → rebuild stopwatch"),
        "compile_graph": ("graph", "dependency-graph snapshot"),
        "carve_lineage": ("carve", "what a module split out of, and why"),
    }
    for name, load in loads.items():
        kind, what = facts[name]
        if load.missing:
            verdict, css = "file absent — a fresh clone has none of these", "bad"
        elif load.n == 0:
            verdict, css = "file present, no rows", "bad"
        elif load.n == 1:
            verdict, css = "⛔ 1 row — a snapshot. NO trend is available.", "bad"
        elif load.n < 10:
            verdict, css = f"⚠ {load.n} rows — too thin to trend; read them individually.", ""
        else:
            verdict, css = f"{load.n} rows", ""
        rows.append([f"<code>dev/{name}.jsonl</code>", esc(kind), esc(what), verdict])
        classes.append(css)

    span_units = sorted(b.started_at or "" for b in builds if b.started_at)
    span_tests = (series[0]["when"], series[-1]["when"]) if series else ("—", "—")
    units_span = (
        f"{span_units[0][:10]} → {span_units[-1][:10]}" if span_units else "no dated builds"
    )

    return f"""
<section id="coverage">
  <h2>What this page can and cannot say</h2>
  <p class="n">read from five append-only ledgers · no build was run to produce it</p>
  <p class="lede">Every number below comes from a file that already existed.
  This reader invokes no cargo, measures nothing, and appends nothing.</p>
  {table(["ledger", "kind", "grain", "what the row count supports"], rows, classes=classes)}
  <div class="warn">
    <strong>⛔ "over time" is a promise two of these ledgers cannot keep.</strong>
    <code>compile_graph</code> and <code>carve_lineage</code> hold one row each.
    Their sections draw one point and label it as one point. Nothing on this page
    fits a line through a single sample.
  </div>
  <div class="note">
    <strong>Spans.</strong> Compile units: {esc(len(builds))} builds, {esc(units_span)} —
    {esc(len({s[:10] for s in span_units}))} distinct day(s) of collection, so the unit
    ledger compares <em>configurations</em> honestly and <em>dates</em> barely.
    Test jobs: {esc(len(series))} suite runs, {esc(span_tests[0])} → {esc(span_tests[1])} UTC —
    the longest genuine series here.
  </div>
</section>"""


def section_cost_per_line(builds: list[Build], costs: dict[str, list[CrateCost]]) -> str:
    ordered = [b for b in builds if costs.get(b.source)]
    if not ordered:
        return ""

    # The default view is the honest warm dev rebuild — what an agent actually pays.
    def rank(build: Build) -> tuple:
        return (
            0 if (build.cache_state == "warm" and build.phase == "first-party" and build.config == "dev") else 1,
            build.started_at or "",
        )

    default = sorted(ordered, key=rank)[0]

    buttons = "".join(
        f'<button type="button" data-build-key="{esc(b.source)}" aria-pressed="false">'
        f"{esc(b.config or 'backfill')} · {esc(b.phase or 'n/a')}"
        f"{' ⚠' if b.label_disputed else ''} · {esc(b.name)}</button>"
        for b in ordered
    )

    blocks = []
    for build in ordered:
        crate_costs = costs[build.source]
        by_cost = sorted(crate_costs, key=lambda c: c.ms_per_line)
        total_s = sum(c.seconds for c in crate_costs)
        total_l = sum(c.lines for c in crate_costs)
        mean = total_s / total_l * 1000 if total_l else float("nan")
        mono = next((c for c in by_cost if c.crate == MONOLITH), None)
        mono_rank = by_cost.index(mono) + 1 if mono else None

        rows, classes = [], []
        for index, cost in enumerate(by_cost):
            rows.append(
                [
                    esc(cost.crate),
                    f"{index + 1}",
                    f"{cost.lines:,}",
                    f"{cost.seconds:,.2f}",
                    f"{cost.ms_per_line:,.2f}",
                    _pct(cost.codegen_share),
                ]
            )
            classes.append("hi" if cost.crate == MONOLITH else "")

        disputed = (
            '<div class="warn"><strong>⚠ this build\'s <code>phase</code> says '
            "<code>first-party</code> and its cache counters say otherwise.</strong> "
            f"{esc(build.fresh_units)} of {esc(build.total_units)} units were cached, so it "
            "recompiled everything, third-party included. Read it as a cold build; do not "
            "compare its seconds to the warm rebuilds.</div>"
            if build.label_disputed
            else ""
        )

        verdict = (
            f"<p><strong>The monolith is the {esc(_ordinal(mono_rank))} "
            f"cheapest of {esc(len(by_cost))}</strong> at {mono.ms_per_line:.2f} ms/line, against a "
            f"population mean of {mean:.2f}. Cheapest: <code>{esc(by_cost[0].crate)}</code> "
            f"at {by_cost[0].ms_per_line:.2f}. Dearest: <code>{esc(by_cost[-1].crate)}</code> "
            f"at {by_cost[-1].ms_per_line:.2f} — <strong>{by_cost[-1].ms_per_line / mono.ms_per_line:.0f}×</strong> "
            f"the monolith's rate on {by_cost[-1].lines / mono.lines * 100:.1f}% of its lines.</p>"
            if mono
            else ""
        )

        blocks.append(
            f"""<div class="buildblock" data-build-block="costline" data-build-key="{esc(build.source)}">
  <h3>{esc(build.config or "backfill")} · {esc(build.phase or "n/a")} · {esc(build.name)} —
  {esc(build.cache_state)} cache, {esc(len(by_cost))} first-party crates</h3>
  {disputed}
  {verdict}
  {scatter_lines_vs_seconds(crate_costs, title=f"cost per line, {build.name}")}
  <p class="tick">Each dot is one crate. The diagonals are constant ms/line — a dot
  below a diagonal is cheap for its size, a dot above it is dear.</p>
  {table(["crate", "rank", "lines", "seconds", "ms/line", "codegen share"], rows, classes=classes)}
</div>"""
        )

    return f"""
<section id="cost-per-line">
  <h2>Cost per line, per crate</h2>
  <p class="n">the question is "expensive for its size", not "big"</p>
  <p class="lede">Which crates cost more than their line count buys? This is the
  view that answers it, and it has already overturned one claim — see the note below.</p>
  <div class="warn">
    <strong>⚠ lines are a proxy that is wrong by roughly an order of magnitude
    BETWEEN crates.</strong> The schema says it and it is worth repeating beside every
    number here: ms/line is reliable for one crate against <em>itself over time</em>,
    and between crates it is a hypothesis generator, not a verdict. A crate full of
    generics pays per <em>instantiation</em>, which no line count can see.
  </div>
  <div class="picker" data-picker="costline" data-default="{esc(default.source)}">{buttons}</div>
  {"".join(blocks)}
</section>"""


def default_build(builds: list[Build], costs: dict[str, list[CrateCost]]) -> Build | None:
    """The honest warm dev first-party rebuild — what an agent actually pays.

    Preferred over a cold build because it is the loop the repo lives in, and
    over the mislabelled one because its cache counters agree with its label.
    """
    candidates = [b for b in builds if costs.get(b.source)]
    if not candidates:
        return None
    return sorted(
        candidates,
        key=lambda b: (
            0 if (b.cache_state == "warm" and b.phase == "first-party" and b.config == "dev") else 1,
            b.started_at or "",
        ),
    )[0]


def section_monolith_claim(builds: list[Build], costs: dict[str, list[CrateCost]]) -> str:
    rows, classes = [], []
    for build in builds:
        crate_costs = costs.get(build.source) or []
        if not crate_costs:
            continue
        by_cost = sorted(crate_costs, key=lambda c: c.ms_per_line)
        mono = next((c for c in by_cost if c.crate == MONOLITH), None)
        if not mono:
            continue
        position = by_cost.index(mono) + 1
        total_s = sum(c.seconds for c in crate_costs)
        total_l = sum(c.lines for c in crate_costs)
        rows.append(
            [
                f"{esc(build.config or 'backfill')} · {esc(build.phase or 'n/a')}",
                esc(build.cache_state + (" ⚠" if build.label_disputed else "")),
                f"{len(by_cost)}",
                f"{mono.ms_per_line:.2f}",
                f"{position} / {len(by_cost)}",
                f"{total_s / total_l * 1000:.2f}" if total_l else "—",
                esc(by_cost[0].crate.replace("ambition_", "")),
            ]
        )
        classes.append("hi" if position == 1 else "")

    # The two crates worth naming, and their multiples — computed against the
    # build a person actually waits on, never typed in.
    focus = default_build(builds, costs)
    by_crate = {c.crate: c for c in (costs.get(focus.source) if focus else []) or []}
    mono = by_crate.get(MONOLITH)
    # "dear per line" alone names a 250-line crate nobody waits on. The crates
    # worth naming are dear per line AND materially expensive in absolute terms,
    # so this ranks the top-15 absolute costs by their rate.
    expensive = sorted(by_crate.values(), key=lambda c: -c.seconds)[:15]
    dearest_big = sorted(
        (c for c in expensive if c.crate != MONOLITH), key=lambda c: -c.ms_per_line
    )[:2]
    multiples = (
        ", ".join(
            f"<code>{esc(c.crate)}</code> ({c.lines:,} lines, {c.seconds:.0f}s) at "
            f"<strong>{c.ms_per_line / mono.ms_per_line:.0f}×</strong>"
            for c in dearest_big
        )
        if mono and dearest_big
        else "—"
    )
    mean = (
        sum(c.seconds for c in by_crate.values()) / sum(c.lines for c in by_crate.values()) * 1000
        if by_crate
        else float("nan")
    )
    biggest = max(by_crate.values(), key=lambda c: c.seconds) if by_crate else None

    # The headline: where the monolith actually places, split by cache state,
    # computed so it cannot drift as builds are appended.
    placings: dict[str, list[int]] = defaultdict(list)
    rank_one: list[Build] = []
    measured = 0
    for build in builds:
        ranked = sorted(costs.get(build.source) or [], key=lambda c: c.ms_per_line)
        entry = next((c for c in ranked if c.crate == MONOLITH), None)
        if not entry:
            continue
        measured += 1
        position = ranked.index(entry) + 1
        if position == 1:
            rank_one.append(build)
        # the back-filled build is excluded from the spans on purpose: the
        # sentence they feed is "once the collector recompiled ALL of them",
        # and folding a 17-crate sample back in would put a 1st place in the
        # very range built to show the claim does not hold there.
        if not build.backfilled:
            placings["warm" if build.cache_state == "warm" else "cold"].append(position)

    def span(key: str) -> str:
        values = placings.get(key) or []
        if not values:
            return "—"
        return _ordinal(values[0]) if min(values) == max(values) else f"{_ordinal(min(values))} to {_ordinal(max(values))}"

    winner = rank_one[0] if rank_one else None
    winner_crates = len(costs.get(winner.source) or []) if winner else 0
    all_crates = max((len(v) for v in costs.values()), default=0)

    return f"""
<section id="monolith">
  <h2>The claim: "the monolith is the cheapest crate per line"</h2>
  <p class="n">tested against every recorded build · verdict: true of the sample it came from, false of the population</p>
  {table(
      ["build", "cache", "crates measured", "monolith ms/line", "its rank", "population mean", "actual cheapest"],
      rows, classes=classes,
    )}
  <div class="warn">
    <strong>The claim is rank 1 in {esc(len(rank_one))} of {esc(measured)}
    builds, and that build measured {esc(winner_crates)} crates — not {esc(all_crates)}.</strong>
    The back-filled report had {esc(next((b.fresh_units for b in builds if b.backfilled), "?"))} of
    {esc(next((b.total_units for b in builds if b.backfilled), "?"))} units already cached, so only
    {esc(winner_crates)} first-party crates were dirty — and among <em>those</em> the monolith was
    indeed cheapest. Once the collector recompiled all {esc(all_crates)}, it placed
    <strong>{esc(span("warm"))} in the warm rebuilds and {esc(span("cold"))} in the cold builds</strong>.
    The claim was true of its sample and does not survive the population.
  </div>
  <div class="note">
    <strong>What survives, and it is the useful half:</strong> in the rebuild an agent
    actually waits on ({esc(focus.config if focus else "—")} · {esc(focus.phase if focus else "—")},
    {esc(focus.name if focus else "—")}) the monolith runs
    {mono.ms_per_line:.2f} ms/line against a population mean of {mean:.2f} —
    <strong>{mono.ms_per_line / mean:.0%} of the mean rate</strong>. Its
    {mono.lines:,} lines are not what the build is paying for. The crates worth
    looking at are the ones dear <em>for their size</em>: {multiples} the monolith's rate.
    ⚠ <strong>and "cheap per line" is not "cheap".</strong>
    {esc(biggest.crate) if biggest else "—"} is still the single largest absolute cost in
    that rebuild at {biggest.seconds:.0f}s — being efficient per line does not stop
    {mono.lines:,} lines from being a lot of lines.
  </div>
</section>"""


def section_dimensions(builds: list[Build], groups: list[DimensionGroup], unit_rows: list[dict]) -> str:
    group_rows = [
        [esc(g.profile), esc(g.opt_level), esc(g.incremental), f"{g.units:,}", f"{g.crates:,}", f"{g.seconds:,.0f}"]
        for g in groups
    ]

    bars = []
    for build in builds:
        if build.wall_seconds is None:
            continue
        bars.append(
            {
                "label": f"{build.config or 'backfill'} · {build.phase or 'n/a'} · {build.cache_state}",
                "values": {"wall": build.wall_seconds},
                "note": f"  ({build.units} units{'  ⚠label' if build.label_disputed else ''})",
            }
        )

    seen = sorted({("on" if r["incremental"] else "off") for r in unit_rows if r.get("incremental") is not None})
    n_incremental = sum(1 for r in unit_rows if r.get("incremental") is not None)

    loads = [b.load_mean for b in builds if b.load_mean is not None]
    load_note = (
        f"ranged from {min(loads):.1f} to {max(loads):.1f} mean load on "
        f"{esc(next((b.cores for b in builds if b.cores), '?'))} cores"
        if loads
        else "was not recorded for these builds"
    )

    def ms_per_line(build: Build) -> float | None:
        crates = [r for r in unit_rows if r.get("build_source") == build.source and r.get("first_party")]
        by_crate: dict[str, tuple[float, int]] = {}
        for row in crates:
            seconds, lines = by_crate.get(row["unit"], (0.0, 0))
            by_crate[row["unit"]] = (seconds + (row.get("seconds") or 0.0), max(lines, row.get("lines") or 0))
        total_l = sum(lines for _, lines in by_crate.values())
        return sum(s for s, _ in by_crate.values()) / total_l * 1000 if total_l else None

    def pick(config: str, phase: str, *, cache: str | None = None) -> list[Build]:
        return [
            b
            for b in builds
            if b.config == config and b.phase == phase and (cache is None or b.cache_state == cache)
        ]

    rel_warm = pick("release", "first-party")
    dev_warm = pick("dev", "first-party", cache="warm")
    rel_cold = pick("release", "cold")
    dev_cold = pick("dev", "cold")

    def rate_span(group: list[Build]) -> str:
        rates = [r for r in (ms_per_line(b) for b in group) if r is not None]
        if not rates:
            return "—"
        return f"{min(rates):.2f}" if len(rates) == 1 else f"{min(rates):.2f}–{max(rates):.2f}"

    def wall_span(group: list[Build]) -> str:
        walls = [b.wall_seconds for b in group if b.wall_seconds is not None]
        return "—" if not walls else (f"{walls[0]:,.0f}s" if len(walls) == 1 else f"{min(walls):,.0f}–{max(walls):,.0f}s")

    # `dev-app-rlib-only` is a manifest-edit configuration: it drops the app's
    # cdylib to price that one knob against the base config's target dir.
    rlib = pick("dev-app-rlib-only", "first-party")
    base_for_rlib = [b for b in dev_warm if b.started_at and rlib and b.started_at < rlib[0].started_at]
    rlib_note = ""
    if rlib and base_for_rlib:
        nearest = max(base_for_rlib, key=lambda b: b.started_at or "")
        if rlib[0].wall_seconds and nearest.wall_seconds:
            delta = (rlib[0].wall_seconds - nearest.wall_seconds) / nearest.wall_seconds
            rlib_note = f"""
  <div class="note">
    <strong>The one manifest knob that was priced, and it came back flat.</strong>
    <code>dev-app-rlib-only</code> drops <code>ambition_app</code>'s <code>cdylib</code>
    — a dimension no cargo flag can express — and shares the base config's target dir so
    the comparison is warm. It ran {rlib[0].wall_seconds:,.0f}s against the immediately
    preceding {esc(nearest.config)} rebuild's {nearest.wall_seconds:,.0f}s:
    <strong>{delta:+.1%}</strong>. ⚠ that is inside the run-to-run spread of the two
    identical dev rebuilds ({wall_span(dev_warm)}), so the honest reading is
    <em>no measurable effect</em>, on one pair of runs — not "no effect".
  </div>"""

    return f"""
<section id="dimensions">
  <h2>Debug vs release, opt-level, incremental</h2>
  <p class="n">a group-by over columns that already exist — including the one that turned out to be empty</p>

  <h3>Wall clock per build</h3>
  {stacked_bars(bars, segments=[("wall", "seg-1", "wall clock")], title="wall clock per build")}
  <p class="tick">⚠ absolute seconds are comparable only within one load level.
  Machine contention is recorded per build in the table below and {load_note}.</p>

  <h3>Unit-seconds by profile × opt-level × incremental</h3>
  {table(["build profile", "opt-level", "incremental", "units", "distinct crates", "unit-seconds"], group_rows)}

  <div class="warn">
    <strong>⛔ the incremental axis has exactly one value, so it is not answerable here.</strong>
    All {n_incremental:,} collector rows that record the setting record it as
    <strong>{esc(" / ".join(seen)) or "nothing"}</strong>. The collector <em>sets</em> the
    variable rather than inheriting it — that is deliberate and documented, because
    <code>run_tests.py</code> forces <code>CARGO_INCREMENTAL=0</code> for its children and a
    collector reading its own environment would report the opposite — but it has only ever
    been set one way. The only incremental-<em>on</em> measurements in this repo are three
    of the four rows in <code>compile_cost.jsonl</code>, below. A chart here would be a
    one-bar comparison, so there is no chart here.
  </div>

  <div class="note">
    <strong>What the profile axis does say.</strong> Release is not uniformly slower —
    it is slower cold and <em>faster warm</em>. The release first-party rebuild ran
    {esc(rate_span(rel_warm))} ms/line against dev's {esc(rate_span(dev_warm))}, while the
    release cold build cost {esc(wall_span(rel_cold))} wall against dev's
    {esc(wall_span(dev_cold))}. Optimised codegen costs more the first time and buys
    smaller, faster-to-relink artifacts afterwards.
    ⚠ <code>build_profile</code> reads <code>test</code> where a reader expects
    <code>dev</code>: cargo names the profile after the target it built, and the collector
    records cargo's word rather than inventing one.
  </div>
  {rlib_note}

  <h3>Per-build detail</h3>
  {table(
      ["build", "config", "phase", "cache", "profile", "opt", "units", "cached", "wall s", "unit s", "load mean", "cores"],
      [
          [
              esc(b.name), esc(b.config or "—"),
              esc(b.phase or "—") + (" ⚠" if b.label_disputed else ""),
              esc(b.cache_state), esc(b.profile or "—"), esc(b.dominant_opt_level),
              f"{b.units:,}", f"{b.fresh_units if b.fresh_units is not None else '—'}",
              _fmt(b.wall_seconds, 0), f"{b.unit_seconds:,.0f}",
              _fmt(b.load_mean, 1), esc(b.cores or "—"),
          ]
          for b in builds
      ],
      classes=["bad" if b.label_disputed else "" for b in builds],
    )}
</section>"""


def section_codegen(builds: list[Build], unit_rows: list[dict]) -> str:
    by_build: dict[str, list[dict]] = defaultdict(list)
    for row in unit_rows:
        by_build[row.get("build_source") or "?"].append(row)

    bars, rows = [], []
    for build in builds:
        split = split_totals(by_build.get(build.source) or [])
        bars.append(
            {
                "label": f"{build.config or 'backfill'} · {build.phase or 'n/a'} · {build.cache_state}",
                "values": {
                    "frontend": split.frontend_seconds,
                    "codegen": split.codegen_seconds,
                    "other": max(split.unattributed_seconds, 0.0),
                },
                "note": "",
            }
        )
        rows.append(
            [
                f"{esc(build.config or 'backfill')} · {esc(build.phase or 'n/a')}",
                f"{split.units:,}",
                f"{split.units_with_split:,}",
                f"{split.frontend_seconds:,.0f}",
                f"{split.codegen_seconds:,.0f}",
                f"{split.unattributed_seconds:,.0f}",
                _pct(split.codegen_share_of_split),
                _pct(split.codegen_share_of_all),
            ]
        )

    overall = split_totals([r for r in unit_rows if not r.get("backfilled")])
    gap = (overall.codegen_share_of_split or 0) - (overall.codegen_share_of_all or 0)
    collector_shares = [
        split_totals(by_build.get(b.source) or []).codegen_share_of_split
        for b in builds
        if not b.backfilled and by_build.get(b.source)
    ]
    collector_shares = [s for s in collector_shares if s is not None]
    span = (
        f"{min(collector_shares) * 100:.0f}–{max(collector_shares) * 100:.0f}%"
        if collector_shares
        else "—"
    )
    backfill = next((b for b in builds if b.backfilled), None)
    backfill_note = ""
    if backfill:
        share = split_totals(by_build.get(backfill.source) or []).codegen_share_of_split
        backfill_note = (
            f" The one back-filled report reads {_pct(share)}, on "
            f"{len(by_build.get(backfill.source) or [])} units — a different, much smaller sample."
        )
    return f"""
<section id="codegen">
  <h2>Codegen vs frontend</h2>
  <p class="n">"the cost is codegen-bound, not link or frontend" — now something you can look at</p>

  <div class="tiles">
    {tile(_pct(overall.codegen_share_of_split), "codegen", "of the seconds that carry a split")}
    {tile(_pct(overall.codegen_share_of_all), "codegen", "of ALL unit-seconds")}
    {tile(f"{overall.units_with_split:,} / {overall.units:,}", "units split", "the rest emit no metadata")}
    {tile(f"{overall.unattributed_seconds:,.0f}s", "unattributed", "build scripts, bins, the app cdylib")}
  </div>

  <div class="warn">
    <strong>⛔ two denominators, both true, and they are {gap * 100:.1f} points apart.</strong>
    Codegen is <strong>{_pct(overall.codegen_share_of_split)}</strong> of the time that is
    <em>split into phases</em>, and <strong>{_pct(overall.codegen_share_of_all)}</strong> of
    <em>every unit-second recorded</em>. The gap is
    {overall.units - overall.units_with_split:,} units with no split at all — proc-macro
    crates, build scripts, bins, tests, and <code>ambition_app</code>, the one lib here
    declaring a <code>cdylib</code>. They emit no metadata, so cargo reports no phases for
    them. Quote whichever you like; name which one it is.
  </div>

  <div class="legend">
    <span><i class="sw sw1"></i>frontend</span>
    <span><i class="sw sw2"></i>codegen</span>
    <span><i class="sw swm"></i>no phase split recorded</span>
  </div>
  {stacked_bars(bars, segments=[("frontend", "seg-1", "frontend"), ("codegen", "seg-2", "codegen"), ("other", "seg-muted", "unsplit")], title="frontend vs codegen per build")}

  {table(["build", "units", "with split", "frontend s", "codegen s", "unsplit s", "codegen % of split", "codegen % of all"], rows)}

  <div class="note">
    <strong>The claim holds, in every configuration measured.</strong> Codegen is {esc(span)}
    of split time across the {esc(len(collector_shares))} collector builds — cold or warm,
    dev or release, the answer does not move.{esc(backfill_note)} That is what makes
    <code>opt-level</code> and <code>codegen-units</code> the levers and the frontend a
    rounding error by comparison — and it is also why
    <code>cargo check</code> (frontend only) costs 8.2s where the equivalent
    <code>cargo test --no-run</code> costs 20–21s for the same edit.
  </div>
</section>"""


def section_tests(load: LedgerLoad, jobs: list[JobCost], totals: SuiteTotals, series: list[dict]) -> str:
    if not load.rows:
        return f"""
<section id="tests"><h2>Test time as a first-class cost</h2>
<p class="empty">{html.escape(load.label)} has no rows.</p></section>"""

    top = jobs[:14]
    bars = [
        {
            "label": j.job,  # `stacked_bars` truncates to what its gutter fits
            "values": {"build": j.build_seconds, "exec": j.executed_seconds},
            "note": f"  ×{j.runs}",
        }
        for j in top
    ]
    rows, classes = [], []
    for j in jobs:
        rows.append(
            [
                esc(j.job),
                f"{j.runs}",
                f"{j.seconds:,.0f}",
                f"{j.build_seconds:,.0f}",
                f"{j.executed_seconds:,.0f}",
                _pct(j.build_share),
                f"{j.seconds / j.runs:,.1f}" if j.runs else "—",
                f"{j.failures}" if j.failures else "",
            ]
        )
        classes.append("bad" if j.failures else "")

    hours = totals.seconds / 3600
    return f"""
<section id="tests">
  <h2>Test time as a first-class cost</h2>
  <p class="n">{esc(totals.runs)} suite invocations · {esc(sum(j.runs for j in jobs))} command runs · {esc(len(jobs))} distinct jobs</p>

  <div class="tiles">
    {tile(f"{hours:,.1f} h", "total suite wall clock", f"across {totals.runs} invocations")}
    {tile(_pct(totals.build_share), "spent BUILDING", f"{totals.build_seconds / 3600:,.1f} h of build graph")}
    {tile(_pct(1 - (totals.build_share or 0)), "spent RUNNING", f"{totals.executed_seconds / 3600:,.1f} h in libtest")}
    {tile(f"{totals.seconds / totals.runs:,.0f}s", "mean per invocation", "median "
          f"{statistics.median(p['seconds'] for p in series):,.0f}s")}
  </div>

  <p class="lede"><strong>{_pct(totals.build_share)} of the time the suite costs is not
  running tests.</strong> <code>seconds − executed_seconds</code> is the split that makes
  "quantify test time like this too" answerable rather than one wall clock, and it says the
  build graph dominates the thing everyone calls "the tests".</p>

  <div class="warn">
    <strong>⚠ that percentage is an overstatement, by a knowable amount.</strong>
    <code>executed_seconds</code> comes from libtest's own "finished in Xs", so a job that
    is not libtest reports <code>0.0</code> and its entire wall clock lands in the build
    column. Every pytest job, every <code>cargo check</code> job, and the acceptance job
    are in that category — <code>acceptance: the render composition draws a frame</code>
    alone contributes {esc(next((f"{j.seconds:,.0f}s" for j in jobs if j.job.startswith("acceptance")), "—"))}
    of pure "build". The honest reading: of the one job that <em>does</em> report both,
    <code>workspace (default features)</code>,
    {esc(next((_pct(j.build_share) for j in jobs if j.job.startswith("workspace (default")), "—"))}
    is build. That is the number to trust.
  </div>

  <h3>Where the suite's hours go</h3>
  <div class="legend">
    <span><i class="sw sw1"></i>build graph (seconds − executed)</span>
    <span><i class="sw sw3"></i>running tests (libtest)</span>
  </div>
  {stacked_bars(bars, segments=[("build", "seg-1", "build"), ("exec", "seg-3", "running")], title="test job cost")}

  <h3>Every suite invocation, oldest to newest</h3>
  <div class="legend">
    <span><i class="sw sw1"></i>build graph</span>
    <span><i class="sw sw3"></i>running tests</span>
  </div>
  {stacked_area_over_time(series, title="suite wall clock over time")}
  <p class="tick">{esc(len(series))} runs, {esc(series[0]["when"])} → {esc(series[-1]["when"])} UTC.
  The spikes are the exhaustive plan ({esc(sum(1 for p in series if p["exhaustive"]))} of
  {esc(len(series))} runs); the floor is a single filtered job.</p>

  <h3>Per job</h3>
  {table(["job", "runs", "total s", "build s", "running s", "build %", "mean s", "failures"], rows, classes=classes)}
</section>"""


def section_scenarios(load: LedgerLoad) -> str:
    if not load.rows:
        return f"""
<section id="scenarios"><h2>The edit → rebuild stopwatch</h2>
<p class="empty">{html.escape(load.label)} has no rows.</p></section>"""

    scenarios = [normalise_scenario(r) for r in load.rows]
    rows, classes = [], []
    for s in scenarios:
        rows.append(
            [
                esc(s.get("scenario")),
                esc(s.get("label") or "—"),
                esc(s.get("profile") or "?"),
                esc(s.get("opt_level") or "?"),
                "on" if s.get("incremental") else ("off" if s.get("incremental") is False else "?"),
                _fmt(s.get("warm_noop_seconds"), 2),
                _fmt(s.get("after_edit_seconds"), 2),
                _fmt(s.get("edit_cost_seconds"), 2) if s.get("edit_cost_seconds") is not None else "⚠ n/a",
                esc(s.get("dimension_source")),
            ]
        )
        classes.append("bad" if s.get("warm_pass_suspect") else "")

    suspect = sum(1 for s in scenarios if s.get("warm_pass_suspect"))
    return f"""
<section id="scenarios">
  <h2>The edit → rebuild stopwatch</h2>
  <p class="n">⚠ {esc(len(scenarios))} rows. That is the whole ledger.</p>

  <div class="warn">
    <strong>⛔ four rows, all from one evening, {esc(suspect)} of them with a warm pass that was not warm.</strong>
    This is not a series and nothing here should be read as a trend. Each row is a single
    stopwatch reading, and the rows flagged in red have a <code>warm_noop_seconds</code> at
    or above half their <code>after_edit_seconds</code> — meaning the "warm" pass paid for
    work the tree already owed, so subtracting it yields a number that measures nothing.
    Those rows show <code>⚠ n/a</code> rather than a computed edit cost. The tell is the
    two <code>test-build</code> rows: same scenario, same commit, same environment, warm
    passes of 14.81s and 0.50s. Only the second was warm.
  </div>

  {table(["scenario", "label", "profile", "opt", "incremental", "warm no-op s", "after edit s", "edit cost s", "dimensions"], rows, classes=classes)}

  <div class="note">
    <strong>The dimension columns are reconstructed, and the ledger was not rewritten.</strong>
    All four rows predate schema 1, so <code>profile</code>, <code>opt_level</code> and
    <code>incremental</code> were never recorded as columns. They are recovered at read time
    from the mapping table in <code>dev/compile_telemetry_schema.md</code> §4 — which exists
    because <code>machine_cargo_incremental: "(config default)"</code> means <strong>off</strong>
    before <code>.cargo/config.toml</code> turned incremental on and <strong>on</strong> after.
    One string, two opposite meanings; reading it as a boolean gets a row wrong.
  </div>

  <div class="note">
    <strong>The one comparison these four rows do support</strong> is the headline they were
    taken for: the same edit costs <strong>8.2s</strong> through <code>cargo check</code>
    and <strong>20–21s</strong> through <code>cargo test --no-run</code>. Frontend versus
    frontend-plus-codegen, on one machine, in one evening — the same story the codegen
    share tells above, arrived at by a completely different instrument.
  </div>
</section>"""


def section_graph(load: LedgerLoad) -> str:
    if not load.rows:
        return f"""
<section id="graph"><h2>Dependency-graph shape</h2>
<p class="empty">{html.escape(load.label)} has no rows.</p></section>"""

    snapshot = load.rows[-1]
    crate_lines: dict[str, int] = snapshot.get("crate_lines") or {}
    largest = snapshot.get("largest_unit") or {}
    worst = snapshot.get("worst_edit_cost") or {}
    watched = snapshot.get("watched_edit_cost") or {}
    total = snapshot.get("first_party_lines") or sum(crate_lines.values())

    top = sorted(crate_lines.items(), key=lambda kv: -kv[1])[:12]
    bars = [
        {"label": name.replace("ambition_", ""), "values": {"lines": value}, "note": f"  ({value / total * 100:.0f}%)"}
        for name, value in top
    ]

    watch_rows = [
        [
            esc(name),
            f"{payload.get('crates', 0):,}",
            f"{payload.get('lines', 0):,}",
            f"{payload.get('lines', 0) / total * 100:.0f}%",
        ]
        for name, payload in watched.items()
    ]

    one_row = len(load.rows) == 1
    return f"""
<section id="graph">
  <h2>Dependency-graph shape</h2>
  <p class="n">⛔ {esc(len(load.rows))} snapshot{"" if one_row else "s"} · commit
  <code>{esc(snapshot.get("commit"))}</code> · {esc(snapshot.get("recorded_at", "")[:10])}</p>

  {single_point_chart("first-party lines", float(total), " lines", f"{esc(snapshot.get('first_party_crates'))} crates, deterministic — no build required")
   if one_row else ""}

  <div class="warn">
    <strong>⛔ this is a snapshot, not a series.</strong> One row exists. Nothing on this
    page can say whether any of these numbers moved, because there is no second measurement
    to move from. The ledger is deterministic and cheap
    (<code>scripts/compile_ratchet.py --update</code> needs no build), so a second row is
    one command away — but until it is taken, every figure below is a single reading of a
    single commit.
  </div>

  <div class="tiles">
    {tile(f"{snapshot.get('first_party_crates')}", "first-party crates")}
    {tile(f"{total:,}", "first-party lines", esc(snapshot.get("line_unit", "")))}
    {tile(_count(snapshot.get("critical_path_crates")), "critical path", "longest serial chain of first-party crates")}
    {tile(f"{largest.get('lines', 0):,}", "largest unit", esc(largest.get("crate", "")))}
  </div>

  <h3>Where the lines are — top 12 of {esc(len(crate_lines))} crates</h3>
  {stacked_bars(bars, segments=[("lines", "seg-1", "lines")], title="lines per crate", unit="")}
  <p class="tick">The largest crate holds {largest.get("lines", 0) / total * 100:.0f}% of
  first-party lines. ⚠ per the section above, it does <em>not</em> hold a proportional share
  of the cost per line — it is well under the population mean.</p>

  <h3>What one edit can force</h3>
  {table(["edit here", "crates rebuilt", "lines rebuilt", "share of tree"],
         [[esc(worst.get("crate")), f"{worst.get('crates', 0):,}", f"{worst.get('lines', 0):,}",
           f"{worst.get('lines', 0) / total * 100:.0f}%"]] + watch_rows)}
  <p class="tick">The first row is the worst case in the graph; the rest are the crates the
  ratchet watches. ⚠ this is <em>graph shape</em>, in lines — not seconds. Only
  <code>compile_units.jsonl</code> prices it, and the two ledgers agree the expensive thing
  about the monolith is that 17 crates sit downstream of it, not its size.</p>
</section>"""


def section_carve(load: LedgerLoad) -> str:
    if not load.rows:
        return f"""
<section id="carve"><h2>Carve lineage</h2>
<p class="empty">{html.escape(load.label)} has no rows — no carve has recorded itself yet.</p></section>"""

    cards = []
    for row in load.rows:
        same_crate = row.get("from_crate") == row.get("to_crate")
        cards.append(
            f"""<div class="carve">
  <div class="path">{esc(row.get("from_path"))}</div>
  <div class="arrow">↓ {esc("intra-crate module move" if same_crate else "crate carve")} ·
  {esc(f"{row.get('lines_at_split', 0):,}")} lines at split ·
  <code>{esc(row.get("from_crate"))}</code> → <code>{esc(row.get("to_crate"))}</code></div>
  <div class="path">{esc(row.get("to_path"))}</div>
  <div class="why">{esc(row.get("why"))}</div>
  <p class="tick" style="margin-top:9px">
    happened in <code>{esc(row.get("happened_in") or "?")}</code> ·
    recorded {esc((row.get("recorded_at") or "")[:10])} ·
    provenance: <strong>{esc(row.get("recorded_from") or "?")}</strong>
  </p>
</div>"""
        )

    live = sum(1 for r in load.rows if r.get("recorded_from") == "live")
    return f"""
<section id="carve">
  <h2>Carve lineage</h2>
  <p class="n">⛔ {esc(len(load.rows))} row · {esc(live)} recorded live by the carve's own commit</p>

  <div class="warn">
    <strong>⛔ one row, and it is a transcription rather than a live record.</strong>
    There is no lineage graph here because there is no lineage yet — one entry cannot be a
    tree. This ledger is the only dimension in the whole schema with <em>no other source</em>:
    <code>git log --follow</code> approximates a file move, gives up on a module split across
    two homes, and records nothing about why. It is also deliberately <strong>not
    back-filled</strong> — a reconstructed lineage that reads like a recorded one is worse
    than a gap, because the next reader cannot tell which is which. The
    <code>recorded_from</code> field on the row below says exactly what it came from.
  </div>

  {"".join(cards)}

  <div class="note">
    <strong>What would make this section a graph.</strong> Each carve appends its own row at
    the moment it splits, via
    <code>scripts/compile_ratchet.py --record-carve</code>. Cross-referenced against
    <code>compile_units.jsonl</code>, a populated lineage answers the question none of the
    other four ledgers can: whether splitting a module actually moved the seconds, or only
    moved the lines.
  </div>
</section>"""


# --------------------------------------------------------------------------- #
# main
# --------------------------------------------------------------------------- #


def render(loads: dict[str, LedgerLoad]) -> str:
    unit_rows = loads["compile_units"].rows
    builds = summarise_builds(unit_rows)
    costs = crate_costs_by_build(unit_rows)
    jobs = job_costs(loads["run_tests_cost"].rows)
    totals = suite_totals(loads["run_tests_cost"].rows)
    series = suite_series(loads["run_tests_cost"].rows)

    generated = datetime.now().astimezone().strftime("%Y-%m-%d %H:%M %z")
    body = "\n".join(
        part
        for part in [
            section_coverage(loads, builds, series),
            section_cost_per_line(builds, costs) if builds else "",
            section_monolith_claim(builds, costs) if builds else "",
            section_dimensions(builds, dimension_groups(unit_rows), unit_rows) if builds else "",
            section_codegen(builds, unit_rows) if builds else "",
            section_tests(loads["run_tests_cost"], jobs, totals, series),
            section_scenarios(loads["compile_cost"]),
            section_graph(loads["compile_graph"]),
            section_carve(loads["carve_lineage"]),
        ]
        if part
    )

    return f"""<!doctype html>
<html lang="en"><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Ambition compile telemetry</title>
<style>{CSS}</style>
</head><body>
<div class="wrap">
<h1>Ambition compile telemetry</h1>
<p class="sub">A reader over five append-only ledgers. Generated {esc(generated)} —
no build was run, no cargo was invoked, nothing was appended.</p>
{body}
<footer>
  Written by <code>scripts/compile_report.py</code>.
  Column meanings are governed by <code>dev/compile_telemetry_schema.md</code>; where this
  page and that document disagree, the document is right.
</footer>
</div>
<script>{JS}</script>
</body></html>
"""


def print_summary(loads: dict[str, LedgerLoad]) -> None:
    unit_rows = loads["compile_units"].rows
    builds = summarise_builds(unit_rows)
    costs = crate_costs_by_build(unit_rows)

    print("LEDGERS")
    for name, load in loads.items():
        state = "MISSING" if load.missing else f"{load.n} rows"
        extra = f" ({load.malformed} malformed)" if load.malformed else ""
        print(f"  {name:20} {state}{extra}")

    if builds:
        print("\nBUILDS  (⚠ = phase label disputed by the cache counters)")
        for build in builds:
            flag = " ⚠" if build.label_disputed else ""
            print(
                f"  {build.name:12} {str(build.config):18} {str(build.phase):12} "
                f"{build.cache_state:5}{flag:2} units={build.units:4} "
                f"wall={_fmt(build.wall_seconds, 0):>7}s"
            )

        print("\nMONOLITH COST PER LINE")
        for build in builds:
            ranked = sorted(costs.get(build.source) or [], key=lambda c: c.ms_per_line)
            mono = next((c for c in ranked if c.crate == MONOLITH), None)
            if mono:
                print(
                    f"  {build.name:12} {str(build.config):18} {mono.ms_per_line:6.2f} ms/line "
                    f"rank {ranked.index(mono) + 1:2}/{len(ranked)}  cheapest={ranked[0].crate}"
                )

        overall = split_totals([r for r in unit_rows if not r.get("backfilled")])
        print(
            f"\nCODEGEN  {_pct(overall.codegen_share_of_split)} of split seconds · "
            f"{_pct(overall.codegen_share_of_all)} of all unit-seconds"
        )

    totals = suite_totals(loads["run_tests_cost"].rows)
    if totals.runs:
        print(
            f"\nTESTS    {totals.runs} runs · {totals.seconds / 3600:.1f}h wall · "
            f"{_pct(totals.build_share)} building, {_pct(1 - totals.build_share)} running"
        )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "-o", "--output", type=Path, default=DEFAULT_OUTPUT, help=f"default: {DEFAULT_OUTPUT}"
    )
    parser.add_argument("--print-summary", action="store_true", help="text digest to stdout; writes no file")
    args = parser.parse_args(argv)

    loads = {
        "compile_units": load_jsonl(UNITS_LEDGER),
        "run_tests_cost": load_jsonl(JOBS_LEDGER),
        "compile_cost": load_jsonl(SCENARIO_LEDGER),
        "compile_graph": load_jsonl(GRAPH_LEDGER),
        "carve_lineage": load_jsonl(CARVE_LEDGER),
    }

    if all(load.missing for load in loads.values()):
        print("⛔ none of the five ledgers exist. A fresh clone has none of them; run")
        print("   `python3 scripts/compile_collect.py` to record some, then come back.")
        return 1

    if args.print_summary:
        print_summary(loads)
        return 0

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(render(loads), encoding="utf-8")

    thin = [name for name, load in loads.items() if 0 < load.n < 5]
    empty = [name for name, load in loads.items() if load.n == 0]
    print(f"wrote {args.output.relative_to(ROOT) if args.output.is_relative_to(ROOT) else args.output} "
          f"({args.output.stat().st_size / 1024:,.0f} KB)")
    if thin:
        print(f"⚠ thin ledgers, labelled as such on the page: {', '.join(thin)}")
    if empty:
        print(f"⚠ no rows, section says so: {', '.join(empty)}")
    print(f"  file://{args.output.resolve()}\n  file://{args.output.resolve().parent}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
