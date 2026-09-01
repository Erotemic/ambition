#!/usr/bin/env python3
"""Query and compare the runtime frame-cost series.

`dev/ambition_dev_measurements/runtime_frame_cost.jsonl` is one normalized row
per profiling bundle, written by `scripts/lib/profile_bundle_to_history.py`.
This is the tool that reads it back and answers the only question the raw
bundles never could: **did this commit make the frame slower than last week's?**

    scripts/perf_history.py list
    scripts/perf_history.py compare 70898d77b75e HEAD
    scripts/perf_history.py latest --against desktop-timeline-run-20260829T000517Z
    scripts/perf_history.py scenario sandbox
    scripts/perf_history.py report -o docs/planning/engine/runtime-frame-history.md

⛔ **THE REFUSAL IS THE POINT.** A frame time is a property of a commit *on a
scenario, on a machine, at a renderer, under a set of instruments*. Two rows
whose `comparable_key` differs describe different experiments, and subtracting
them produces a number that looks like a regression and is an artefact. Every
comparison here checks the key first and refuses by NAME of the field that
differs. On this project the two seeded baseline rows differ by exactly
`instruments.tracy` and are 9x apart; that is what the check is defending
against.

Plain JSONL in, Markdown out. No database, no server, no index."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

import profile_bundle_to_history as history  # noqa: E402
from profile_bundle_to_history import (  # noqa: E402
    ADVISORY_FIELDS,
    COMPARABILITY_FIELDS,
    dig,
)

# The metrics a comparison reports on, in the order a reader wants them: what
# the frame cost, then how spiky it was, then the structural work behind it.
# `higher_is_worse` is False only where the metric is a floor.
METRICS: list[tuple[str, str, str]] = [
    ("frame_ms.mean", "frame mean", "ms"),
    ("frame_ms.p50", "frame p50", "ms"),
    ("frame_ms.p95", "frame p95", "ms"),
    ("frame_ms.p99", "frame p99", "ms"),
    ("frame_ms.max", "frame max", "ms"),
    ("spikes.per_1000_frames", "spikes /1000 frames", ""),
    ("phases_ms._budget_ms", "phase budget", "ms"),
    ("scheduler.registered_systems", "registered systems", ""),
    ("scheduler.system_executions_per_frame", "system exec/frame", ""),
    ("scheduler.run_condition_evaluations_per_frame", "run-condition evals/frame", ""),
    ("scheduler.command_flushes_per_frame", "command flushes/frame", ""),
    ("scene.entities", "entities", ""),
    ("scene.archetypes", "archetypes", ""),
]

DEFAULT_THRESHOLD_PCT = 5.0


class Refusal(SystemExit):
    """A comparison the tool will not make.

    Exit code 2, deliberately distinct from the 1 that a real regression
    returns: a CI job must be able to tell "the frame got slower" from "you
    asked me to subtract two different experiments".
    """

    def __init__(self, message: str) -> None:
        print(f"⛔ {message}", file=sys.stderr)
        super().__init__(2)


# ── selecting rows ────────────────────────────────────────────────────────


def load(ledger: Path) -> list[dict]:
    rows = history.load(ledger)
    if not rows:
        raise SystemExit(
            f"⛔ no rows in {ledger}\n"
            "   Record one with: scripts/lib/profile_bundle_to_history.py <bundle-dir>"
        )
    # ⛔⛔ RE-KEY ON READ. The key stored at ingest is a hash over whatever
    # `COMPARABILITY_FIELDS` contained THAT DAY, so adding a field silently
    # splits every existing group away from every new row — and the split is
    # invisible, because the two groups print the SAME label.
    #
    # Measured 2026-09-01: two hall captures whose `comparable_fields` were
    # byte-identical sat in different groups under one identical heading,
    # because `workload.brain_profile` was added between them. Recomputing here
    # means the field set is whatever the CURRENT code says, uniformly, for
    # every row — which is the only way a series survives its own schema
    # growing.
    for row in rows:
        key, fields = history.comparability(row)
        row["comparable_key"] = key
        row["comparable_fields"] = fields
        row["comparable_label"] = history.comparable_label(fields)
    return rows


def order_key(row: dict) -> str:
    """Rows sort by WHEN THEY WERE MEASURED, not when they were written.

    A backfilled row is written months after the run it describes, and ordering
    the series by `recorded_at` would put an old baseline at the head of it.
    """
    return row.get("measured_at") or row.get("recorded_at") or ""


def resolve(rows: list[dict], token: str) -> dict:
    """One row, named by `record_id`, by commit (prefix), or by `latest`.

    A commit may have several rows — different scenarios, machines, or
    instrument sets. Resolving to the newest of them is only safe because the
    comparison that follows re-checks comparability; say which one was picked.
    """
    if token in ("latest", "HEAD"):
        return max(rows, key=order_key)
    exact = [row for row in rows if row.get("record_id") == token]
    if exact:
        return exact[0]
    by_commit = [
        row
        for row in rows
        if (row.get("commit") or "").startswith(token)
        or (row.get("commit_full") or "").startswith(token)
    ]
    if not by_commit:
        raise Refusal(
            f"no record matches {token!r}.\n"
            "   Known record ids and commits: run `scripts/perf_history.py list`."
        )
    chosen = max(by_commit, key=order_key)
    if len(by_commit) > 1:
        print(
            f"⚠ {token!r} matches {len(by_commit)} records; taking the newest, "
            f"{chosen['record_id']} ({chosen.get('comparable_label')}). "
            "Name a record id to pick another.",
            file=sys.stderr,
        )
    return chosen


# ── the refusal ───────────────────────────────────────────────────────────


def differing_fields(left: dict, right: dict, fields) -> list[tuple[str, object, object]]:
    out = []
    for field in fields:
        a, b = dig(left, field), dig(right, field)
        # The stored `comparable_fields` snapshot is authoritative when present:
        # it is what the key was hashed from, and a later schema change to where
        # a value lives must not silently re-key an old row.
        a = (left.get("comparable_fields") or {}).get(field, a)
        b = (right.get("comparable_fields") or {}).get(field, b)
        if a != b:
            out.append((field, a, b))
    return out


def require_comparable(left: dict, right: dict) -> None:
    if left.get("comparable_key") == right.get("comparable_key"):
        return
    differences = differing_fields(left, right, COMPARABILITY_FIELDS)
    lines = [
        f"refusing to compare {left['record_id']} with {right['record_id']}:",
        "   these rows are not the same experiment.",
        "",
    ]
    if differences:
        lines.append("   The comparability fields that differ:")
        for field, a, b in differences:
            lines.append(f"     {field}:  {a!r}  ≠  {b!r}")
    else:
        # The keys disagree but no listed field does: the row was written by a
        # different schema version. Say that rather than implying equality.
        lines.append(
            "   No listed field differs, so one of these rows was keyed by a "
            "different schema.\n"
            f"     {left['record_id']}: schema {left.get('schema')} key "
            f"{left.get('comparable_key')}\n"
            f"     {right['record_id']}: schema {right.get('schema')} key "
            f"{right.get('comparable_key')}"
        )
    lines += [
        "",
        "   A difference in any one of these makes the delta an artefact of the",
        "   setup rather than a change in the engine. Re-run the newer commit",
        "   under the older row's conditions, or compare within one group",
        f"   (`scripts/perf_history.py scenario {dig(left, 'scenario.id')}`).",
    ]
    raise Refusal("\n".join(lines))


def advisories(left: dict, right: dict) -> list[str]:
    """Differences that do not forbid a comparison but colour it.

    ⚠ A kernel upgrade or a rustc bump genuinely moves frame time. They are NOT
    in the comparability key because putting them there would orphan every row
    on the next `apt upgrade`, leaving a series that can never compare anything.
    So they are printed instead.
    """
    out = []
    for field, a, b in differing_fields(left, right, ADVISORY_FIELDS):
        out.append(f"{field}: {a!r} → {b!r}")
    return out


# ── comparing ─────────────────────────────────────────────────────────────


def delta(base, new) -> tuple[float | None, float | None]:
    if base is None or new is None:
        return None, None
    absolute = new - base
    percent = (absolute / base * 100.0) if base else None
    return absolute, percent


def compare_rows(base: dict, new: dict, threshold: float) -> tuple[list[dict], bool]:
    require_comparable(base, new)
    rows = []
    regressed = False
    for path, name, unit in METRICS:
        before, after = dig(base, path), dig(new, path)
        absolute, percent = delta(before, after)
        flag = ""
        if percent is not None and percent >= threshold:
            flag = "REGRESSION"
            regressed = True
        elif percent is not None and percent <= -threshold:
            flag = "improved"
        rows.append(
            {
                "metric": name,
                "path": path,
                "unit": unit,
                "before": before,
                "after": after,
                "delta": absolute,
                "percent": percent,
                "flag": flag,
            }
        )
    return rows, regressed


def fmt(value, unit: str = "") -> str:
    if value is None:
        return "—"
    if isinstance(value, float):
        text = f"{value:,.3f}".rstrip("0").rstrip(".") if abs(value) < 1000 else f"{value:,.0f}"
    else:
        text = str(value)
    return f"{text}{unit}"


def render_comparison(base: dict, new: dict, rows: list[dict], threshold: float) -> list[str]:
    lines = [
        f"### {base['record_id']} → {new['record_id']}",
        "",
        f"- group: `{new.get('comparable_label')}` (`{new.get('comparable_key')}`)",
        f"- commits: `{base.get('commit')}` → `{new.get('commit')}`",
        f"- measured: {base.get('measured_at')} → {new.get('measured_at')}",
        f"- regression threshold: {threshold:g}%",
    ]
    for row in (base, new):
        if row.get("dirty"):
            lines.append(
                f"- ⚠ `{row['record_id']}` was measured on a DIRTY tree; its binary "
                "is not that commit alone."
            )
        if dig(row, "instruments.tracy"):
            share = dig(row, "instruments.profiler_cycle_share_pct")
            lines.append(
                f"- ⚠ `{row['record_id']}` ran under Tracy"
                + (f" ({share}% of sampled cycles were the profiler's)" if share else "")
                + "; its absolute times are an upper bound."
            )
        if row.get("provenance", {}).get("backfilled"):
            lines.append(
                f"- ⚠ `{row['record_id']}` is TRANSCRIBED FROM PROSE, not extracted "
                f"from a bundle ({row['provenance'].get('recorded_from')})."
            )
    notes = advisories(base, new)
    if notes:
        lines += ["", "⚠ Outside the comparability key, but they move frame time:"]
        lines += [f"  - {note}" for note in notes]
    lines += [
        "",
        "| metric | before | after | Δ | Δ% | |",
        "| --- | ---: | ---: | ---: | ---: | --- |",
    ]
    for row in rows:
        lines.append(
            f"| {row['metric']} | {fmt(row['before'], row['unit'])} | "
            f"{fmt(row['after'], row['unit'])} | {fmt(row['delta'], row['unit'])} | "
            + (f"{row['percent']:+.1f}%" if row["percent"] is not None else "—")
            + f" | {row['flag']} |"
        )
    missing = [row["metric"] for row in rows if row["before"] is None or row["after"] is None]
    if missing:
        lines += [
            "",
            "Not comparable — one side never measured it (an instrument was off, or "
            "the row was transcribed): " + ", ".join(missing) + ".",
        ]
    lines.append("")
    return lines


# ── subcommands ───────────────────────────────────────────────────────────


def cmd_list(args) -> int:
    rows = sorted(load(args.ledger), key=order_key)
    groups: dict[str, list[dict]] = {}
    for row in rows:
        groups.setdefault(row.get("comparable_key", "?"), []).append(row)
    print(f"{len(rows)} record(s) in {args.ledger}, in {len(groups)} comparability group(s).\n")
    for key, members in groups.items():
        print(f"── {members[0].get('comparable_label')}  [{key}]  {len(members)} record(s)")
        for row in members:
            marks = "".join(
                [
                    "D" if row.get("dirty") else " ",
                    "T" if dig(row, "instruments.tracy") else " ",
                    "P" if row.get("provenance", {}).get("backfilled") else " ",
                ]
            )
            print(
                f"   {marks}  {row.get('measured_at') or '?':20s} {row.get('commit', '?'):>13s} "
                f"  mean {fmt(dig(row, 'frame_ms.mean')):>9s}ms  {row.get('record_id')}"
            )
        print()
    print("marks: D=dirty tree  T=Tracy attached (times inflated)  P=transcribed from prose")
    return 0


def cmd_compare(args) -> int:
    rows = load(args.ledger)
    base, new = resolve(rows, args.base), resolve(rows, args.new)
    if base["record_id"] == new["record_id"]:
        raise Refusal(f"{args.base!r} and {args.new!r} both resolve to {base['record_id']}")
    table, regressed = compare_rows(base, new, args.threshold)
    print("\n".join(render_comparison(base, new, table, args.threshold)))
    return 1 if regressed else 0


def cmd_latest(args) -> int:
    rows = load(args.ledger)
    baseline = resolve(rows, args.against)
    # ⭐ The latest row IN THE BASELINE'S GROUP. Taking the globally newest row
    # would hand `require_comparable` a guaranteed refusal whenever anyone ran a
    # different scenario in between, which is most of the time.
    peers = [
        row
        for row in rows
        if row.get("comparable_key") == baseline.get("comparable_key")
        and row["record_id"] != baseline["record_id"]
    ]
    if not peers:
        raise Refusal(
            f"no record after {baseline['record_id']} shares its group "
            f"`{baseline.get('comparable_label')}`.\n"
            "   Nothing measured since is comparable to it. Re-run the profiler "
            "under the same conditions."
        )
    newest = max(peers, key=order_key)
    table, regressed = compare_rows(baseline, newest, args.threshold)
    print("\n".join(render_comparison(baseline, newest, table, args.threshold)))
    return 1 if regressed else 0


def cmd_scenario(args) -> int:
    rows = [row for row in load(args.ledger) if dig(row, "scenario.id") == args.scenario]
    if not rows:
        raise Refusal(f"no records for scenario {args.scenario!r}")
    groups: dict[str, list[dict]] = {}
    for row in sorted(rows, key=order_key):
        groups.setdefault(row.get("comparable_key", "?"), []).append(row)
    print(f"scenario `{args.scenario}`: {len(rows)} record(s) in {len(groups)} group(s)\n")
    for members in groups.values():
        print(f"── {members[0].get('comparable_label')}")
        print(f"   {'measured':20s} {'commit':>13s} {'mean':>9s} {'p99':>9s} {'systems':>9s}  record")
        for row in members:
            print(
                f"   {row.get('measured_at') or '?':20s} {row.get('commit', '?'):>13s} "
                f"{fmt(dig(row, 'frame_ms.mean')):>9s} {fmt(dig(row, 'frame_ms.p99')):>9s} "
                f"{fmt(dig(row, 'scheduler.registered_systems')):>9s}  {row.get('record_id')}"
            )
        if len(members) > 1:
            first, last = members[0], members[-1]
            _, percent = delta(dig(first, "frame_ms.mean"), dig(last, "frame_ms.mean"))
            if percent is not None:
                print(f"   → frame mean moved {percent:+.1f}% across this group")
        print()
    if len(groups) > 1:
        print(
            "⚠ More than one group. Rows in different groups are NOT comparable — "
            "`compare` will refuse and name the field."
        )
    return 0


def cmd_phase(args) -> int:
    """One simulation phase across time, inside each comparability group.

    ⭐ THE QUESTION THE FRAME SERIES COULD NOT ANSWER. `frame_ms` says the frame
    got slower; it never says which phase did. These numbers were living in
    planning prose and journals, where they cannot be plotted and a remote agent
    cannot query them at all.
    """
    rows = [row for row in load(args.ledger) if dig(row, "sim_phases_ms")]
    if not rows:
        raise Refusal(
            "no record carries sim-phase data. Rows ingested before "
            "`sim_phases_ms` existed can be backfilled only while their raw "
            "bundle is still in profiles/."
        )
    if args.phase is None:
        names = sorted({name for row in rows for name in dig(row, "sim_phases_ms")})
        print(f"{len(rows)} record(s) carry sim phases. Names:\n")
        for name in names:
            print(f"  {name}")
        return 0

    groups: dict[str, list[dict]] = {}
    for row in sorted(rows, key=order_key):
        if dig(row, f"sim_phases_ms.{args.phase}") is not None:
            groups.setdefault(row.get("comparable_key", "?"), []).append(row)
    if not groups:
        raise Refusal(f"no record carries a sim phase named {args.phase!r}")

    print(f"sim phase `{args.phase}`, ms per TICK\n")
    for members in groups.values():
        print(f"── {members[0].get('comparable_label')}")
        print(f"   {'measured':20s} {'commit':>13s} {'bodies':>7s} {'ms/tick':>9s}  record")
        for row in members:
            print(
                f"   {row.get('measured_at') or '?':20s} {row.get('commit', '?'):>13s} "
                f"{fmt(dig(row, 'scene.bodies')):>7s} "
                f"{fmt(dig(row, f'sim_phases_ms.{args.phase}')):>9s}  {row.get('record_id')}"
            )
        if len(members) > 1:
            first = dig(members[0], f"sim_phases_ms.{args.phase}")
            last = dig(members[-1], f"sim_phases_ms.{args.phase}")
            _, percent = delta(first, last)
            if percent is not None:
                print(f"   → moved {percent:+.1f}% across this group")
        print()
    # ⚠ Per TICK, and the frame series is per FRAME. A frame may run zero or
    # several ticks, so these two numbers do not divide into one another.
    print("⚠ ms per TICK. `frame_ms` is per FRAME; they are not the same denominator.")
    return 0


def cmd_report(args) -> int:
    rows = sorted(load(args.ledger), key=order_key)
    lines = [
        "# Runtime frame cost — measured history",
        "",
        "Generated by `scripts/perf_history.py report` from",
        f"`{args.ledger}`. ⛔ Do not hand-edit; append with",
        "`scripts/lib/profile_bundle_to_history.py <bundle-dir>`.",
        "",
        f"{len(rows)} record(s).",
        "",
    ]
    groups: dict[str, list[dict]] = {}
    for row in rows:
        groups.setdefault(row.get("comparable_key", "?"), []).append(row)

    lines += [
        "## Comparability groups",
        "",
        "Rows may only be compared WITHIN a group. The group is the scenario, its",
        "content version, the build, the machine, the renderer and the instruments —",
        "everything that changes a frame time without the engine changing.",
        "",
        "| group | records | first | last |",
        "| --- | ---: | --- | --- |",
    ]
    for key, members in groups.items():
        lines.append(
            f"| `{members[0].get('comparable_label')}` | {len(members)} | "
            f"{members[0].get('measured_at')} | {members[-1].get('measured_at')} |"
        )
    lines.append("")

    for key, members in groups.items():
        lines += [
            f"## {members[0].get('comparable_label')}",
            "",
            f"`{key}`",
            "",
            "| measured | commit | frame mean | p99 | max | spikes/1k | systems | exec/frame | record |",
            "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |",
        ]
        for row in members:
            lines.append(
                f"| {row.get('measured_at')} | `{row.get('commit')}`"
                + ("*" if row.get("dirty") else "")
                + f" | {fmt(dig(row, 'frame_ms.mean'))} | {fmt(dig(row, 'frame_ms.p99'))} "
                f"| {fmt(dig(row, 'frame_ms.max'))} | {fmt(dig(row, 'spikes.per_1000_frames'))} "
                f"| {fmt(dig(row, 'scheduler.registered_systems'))} "
                f"| {fmt(dig(row, 'scheduler.system_executions_per_frame'))} "
                f"| `{row.get('record_id')}` |"
            )
        lines.append("")
        caveats = [
            f"- `{row['record_id']}`: {caveat}"
            for row in members
            for caveat in row.get("provenance", {}).get("caveats", [])
        ]
        if caveats:
            lines += ["Caveats carried by these rows:", ""] + caveats + [""]
        if len(members) > 1:
            table, _ = compare_rows(members[0], members[-1], args.threshold)
            lines += ["Oldest to newest in this group:", ""]
            lines += render_comparison(members[0], members[-1], table, args.threshold)
        lines.append("")

    text = "\n".join(lines) + "\n"
    if args.out:
        Path(args.out).write_text(text, encoding="utf-8")
        print(f"wrote {args.out}")
    else:
        print(text)
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--ledger", type=Path, default=history.LEDGER)
    parser.add_argument(
        "--threshold",
        type=float,
        default=DEFAULT_THRESHOLD_PCT,
        help=f"percent worse before a metric is flagged a regression (default {DEFAULT_THRESHOLD_PCT})",
    )
    subs = parser.add_subparsers(dest="command", required=True)

    subs.add_parser("list", help="every record, grouped by what it may be compared with")

    compare = subs.add_parser("compare", help="two commits or two record ids")
    compare.add_argument("base")
    compare.add_argument("new")

    latest = subs.add_parser("latest", help="the newest record in a baseline's group, against it")
    latest.add_argument("--against", required=True, help="a record id or commit")

    scenario = subs.add_parser("scenario", help="one scenario across time")
    scenario.add_argument("scenario")

    phase = subs.add_parser("phase", help="one simulation phase across time")
    phase.add_argument(
        "phase",
        nargs="?",
        help="phase name, e.g. Decide or Integrate. Omit to list what is recorded.",
    )
    phase.set_defaults(func=cmd_phase)
    report = subs.add_parser("report", help="Markdown over the whole series")
    report.add_argument("-o", "--out", help="write here instead of stdout")

    args = parser.parse_args(argv)
    return {
        "list": cmd_list,
        "compare": cmd_compare,
        "latest": cmd_latest,
        "scenario": cmd_scenario,
        "phase": cmd_phase,
        "report": cmd_report,
    }[args.command](args)


if __name__ == "__main__":
    raise SystemExit(main())
