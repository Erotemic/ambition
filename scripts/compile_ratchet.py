#!/usr/bin/env python3
"""Guard the DETERMINISTIC cause of compile time, and record the dimensions
that let statistics find the non-obvious wins.

`scripts/compile_cost.py` is the stopwatch. This is the gate and the ledger.
They are deliberately two programs: one runs real cargo builds and must never
run beside another, this one **never builds anything** and is safe to run at any
time, including from the suite while somebody else is compiling.

## Why this is not a timing threshold

Jon, 2026-08-08: *"I want to quantify those compile wins as we do those. And to
guard against compile time regressions."*

⛔ **A wall-clock threshold on a shared machine fails randomly, gets waived, and
then gets ignored.** That is this repo's own "a check that CANNOT FAIL" lesson
arriving from the opposite direction — a guard nobody trusts is worse than no
guard, because it launders a red into a shrug. `compile_cost.py`'s own docstring
records a warm no-op reading **222s** because two builds shared a target dir. A
gate that can read 222s for a correct tree cannot be a gate.

So: **guard the deterministic cause, track the noisy effect.** The numbers below
are exact integers derived from `cargo tree`'s resolved graph and from line
counts. They do not move when the machine is busy. They move precisely when
somebody changes the shape of the build — which is the only thing a carve is
allowed to claim credit for.

## The five guarded numbers, and why these five

Each catches a regression class the others cannot see. That is the whole
selection rule; a sixth number that reddens on the same event as one of these is
a dashboard entry, not a guard.

1. **`largest_unit_lines`** — the biggest single first-party crate, in lines.
   A crate is the recompilation unit: an edit anywhere in it recompiles all of
   it. This is the number a carve exists to move, and the only one that falls
   when the monolith is split.

2. **`worst_edit_cost_lines`** — over every first-party crate, the most lines a
   single edit can force to recompile (the crate plus everything that
   transitively depends on it, inside the consumer's resolved graph).
   ⚠ **this is the one `largest_unit_lines` cannot see.** Making
   `ambition_input` depend on the monolith moves no crate's size at all and
   doubles what an edit to `ambition_input` costs. Blast radius is a property of
   the graph, not of any file.

3. **`watched_edit_cost_lines`** — the same quantity for named crates we have
   decided to watch, because the worst-case crate is a floor crate that no carve
   will ever touch and the interesting one would otherwise hide under it.

4. **`critical_path_crates`** — the longest chain of first-party crates in the
   consumer's graph. ⛔ **this is the trap, and nothing else in the repo would
   notice it.** Parallelism cannot compress a serial chain; the 2026-08-07
   journal measured `user/real = 1.3` on an 8-core box, meaning seven cores idle
   waiting on exactly this. **A carve that inserts a new layer makes the chain
   LONGER** — every crate gets smaller, every number in (1) and (2) improves,
   and the wall clock gets worse. A decomposition campaign with no guard here is
   optimising a number that is not the cost.

5. **`worst_edit_cost_seconds`** — the same dependent closure as (2), summed
   with a MEASURED per-crate weight instead of a line count. ⛔ **this is the
   one the first four let through, and the hole is not hypothetical.** In the
   release rebuild the monolith compiles at **0.61 ms/line** and
   `ambition_platformer2d_runtime` at **14.77** — 24x — so moving 10,000 lines
   out of the monolith into a runtime-shaped crate *lowers* `largest_unit_lines`
   by 9%, leaves `worst_edit_cost_lines` and `critical_path_crates` untouched,
   and makes the build ~2 minutes slower. Every line number reads it as a win.
   `corr(ms/line, lines) = -0.23` across 52 crates: bigger crates are CHEAPER
   per line, so a line count is not merely imprecise here, it points the wrong
   way.

## The seconds number is a WEIGHT, not a stopwatch

⛔ **the weights come from `dev/ambition_dev_measurements/compile_units.jsonl`, a committed ledger, and
never from timing anything at gate time.** Everything §"Why this is not a timing
threshold" says still holds: this file builds nothing, reads no clock, and
returns the same answer on a busy machine as on an idle one. What changed is the
weight each node in the walk contributes, not how the walk is done or when.

⭐ **a stale weight is a known, reviewable number in a file.** That is the trade,
made deliberately. The alternative — recomputing weights from the ledger on every
run — means `scripts/compile_collect.py` appending 57 rows turns the gate red on
a tree nobody edited, which is the false-red this instrument exists to avoid. So
the weights are FROZEN into the baseline by `--update`, alongside the numbers
they price, and they move only when somebody re-freezes and says so.

Three ways to get a plausible wrong weight, each found the hard way on
2026-08-08 and each closed here:

* ⛔ **configurations are not pooled.** The ledger mixes `release/opt-3`,
  `test/opt-1` and `test/opt-0`; averaging across them compares crates that do
  not share an opt level. `unit_weights()` selects ONE config — see
  `WEIGHT_PROFILE` for which and why.
* ⛔ **cache state comes from the counters, never from the label.** The build at
  `2026-08-08T11:17` is labelled `collector: dev/first-party` and has
  `build_fresh_units: 0` — it recompiled all 688 units in 540s against two
  honest first-party rebuilds' 188s and 210s. The label lied; `fresh`/`dirty`
  did not.
* ⚠ **the ledger's `seconds` is a unit's wall duration inside a real parallel
  build**, sharing 8 cores with sibling rustc processes — nightly
  `-Ztime-passes` reads 12.77s for `ambition_relativity2d` where the ledger
  reads 68.1s. **That is the right number for this gate**, chosen on purpose: a
  blast-radius guard asks "what does this cost the build I actually run", and
  the ledger answers exactly that question. The intrinsic cost answers a
  different one.

## ⭐ These four were TESTED against seconds on 2026-08-08, and three held

The premise — that these numbers predict compile cost — went two days without a
test because the seconds did not exist yet. `scripts/compile_collect.py` now
produces them, and `--analyze` prints the regression. Verdicts, over 55 crates
in four builds:

* `largest_unit_lines` — ✅ **rho +0.83 … +0.86** against per-crate seconds.
  Strong in RANK, weak in magnitude: ms/line spans 29x–68x between crates, so
  lines order crates well and price them badly. Which is what §`crate_lines`
  already said, now measured against a clock.
* `worst_edit_cost_lines` / `watched_edit_cost_lines` — ✅ **rho +0.99** against
  the measured seconds over the same dependent closure.
  ⛔ **but the LINE WEIGHTING contributes nothing.** A bare count of crates in
  the closure predicts those seconds equally well (rho +0.988 vs +0.991; by
  Pearson in a rebuild the unweighted count WINS, +0.984 vs +0.977). Both are
  sums over nested closures, so the ordering is carried by closure size. The
  guard is sound and it is really guarding **how many crates an edit reaches**.
* `critical_path_crates` — ⚠️ **right in hops, wrong by 2.2x in seconds.** The
  comment below says "parallelism cannot compress a serial chain". Pipelined
  compilation does exactly that: rustc releases a dependent when the
  predecessor's METADATA lands, so on a pipelined edge only the FRONTEND is
  serial. The naive chain-of-durations reads 377.9s for a build that finished in
  210.5s. The hop count is still the right thing to GUARD — it is deterministic
  and a new layer still makes the chain longer — but do not price it in seconds
  by multiplying. `dev/journals/compile-time-and-disk-2026-08-07.md`, addendum 2.

## ⛔ The line numbers STAY guarded, and that is an arithmetic answer

The obvious reading of the above is "lines do not predict cost, so stop failing
on lines". Measured against this tree, that would LOSE sensitivity rather than
trade it: the monolith's 2%-of-lines budget is ~2,240 lines, which at its
measured 0.61 ms/line is **1.4 seconds** — far inside the seconds budget. Delete
the line guard and the biggest crate in the workspace can grow by a subsystem a
week without any number moving. The two guards are tight in different places, on
purpose: lines catch growth in a CHEAP crate, seconds catch growth in a DENSE one
and every edge that changes what an edit reaches. Neither is a superset.

## What is deliberately NOT guarded here

* **The set of crates a small consumer links.** Already guarded, by
  `capability-footprint-may-not-grow` in `check_absence_contracts.py`, against
  `fixtures/minimal_game`. A second guard on one fact is how a suite starts
  getting waived.
* **Wall-clock anything.** It lives in `dev/ambition_dev_measurements/compile_cost.jsonl` and
  `dev/ambition_dev_measurements/compile_units.jsonl`, is plotted, and is never a gate. See above.

## ⛔ There is no `--check` flag, and that is deliberate

A guard that prints findings and exits 0 unless somebody remembered an
enforcement flag is green by construction. So a violation here exits 1 by
default, and the flag that makes it advisory is `--report-only`, which has to be
typed deliberately.

## The subject is `ambition_app`

Because AGENTS.md says the gate is `cargo check -p ambition_app`, so that is the
graph whose rebuild anybody actually pays for. Measured on cargo's RESOLVED
graph rather than on which files name which crate — this repo has paid twice for
the second thing.

Usage:
    python3 scripts/compile_ratchet.py                 # gate: exit 1 on a violation
    python3 scripts/compile_ratchet.py --report-only   # never exit non-zero
    python3 scripts/compile_ratchet.py --diff          # what moved since the baseline
    python3 scripts/compile_ratchet.py --update        # re-freeze + append a snapshot
    python3 scripts/compile_ratchet.py --carve crates/<crate>/src/<module>
    python3 scripts/compile_ratchet.py --carve <path> --new-crate-rate 14.77
    python3 scripts/compile_ratchet.py --ingest-timings <cargo-timing.html>
    python3 scripts/compile_ratchet.py --record-carve --from <path> --to <path> --why '...'

Schema: `dev/compile_telemetry_schema.md` — every field, its source, and which
are populated today versus reserved for a collector that has to build.
"""

from __future__ import annotations

import argparse
import functools
import json
import re
import statistics
import subprocess
import sys
import time
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

import measurement_paths  # noqa: E402
from check_absence_contracts import cargo_binary, strip_comments_for  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]

# ⛔ declared in ONE place — `scripts/lib/measurement_paths.py` — because these
# same paths used to be spelled out here, in `compile_report.py` and in
# `compile_cost.py` independently. The ledgers now live in the
# `dev/ambition_dev_measurements` submodule; the BASELINE deliberately does not,
# because it is this gate's input and a gate cannot depend on a submodule
# somebody may not have initialised. The module docstring holds the full reason.
BASELINE = measurement_paths.RATCHET_BASELINE
GRAPH_LEDGER = measurement_paths.GRAPH_LEDGER
UNIT_LEDGER = measurement_paths.UNIT_LEDGER
LINEAGE = measurement_paths.CARVE_LEDGER

SCHEMA = 1

# The consumer whose resolved graph is the subject. AGENTS.md: "`cargo check -p
# ambition_app` is the gate, never `-p <one_crate>`."
CONSUMER = "ambition_app"

# Crates whose blast radius is watched by name. Keep this SHORT — each entry is
# a number somebody has to maintain, and the worst-case number already covers
# the graph as a whole.
#
# * the monolith is the crate most work touches and the subject of every carve
#   proposal, so it is the number a carve has to move;
# * the core is the floor — `compile_cost.py`'s `check-leaf` scenario, the
#   worst-case fan-out an edit can have.
WATCHED = [
    "ambition_platformer2d_actor_monolith",
    "ambition_platformer2d_core",
]

# How far a line-count number may drift before it is a finding.
#
# ⚠ **this is a budget, not a rounding error, and it is the honest half of the
# design.** Zero tolerance on a line count reddens on an ordinary 40-line
# feature, which is noise, and a noisy guard gets waived — the failure mode this
# whole file exists to avoid. 2% of the monolith is ~2,200 lines: a subsystem,
# not a function. Growing past it is a real "this belonged in its own crate"
# conversation.
#
# ⛔ the tolerance is TWO-SIDED and the downward half is not politeness. A carve
# that lands without re-freezing leaves the guard holding that much slack, and
# the next 2,200 lines of growth land silently. `check_absence_contracts.py`
# calls the same rule STALE and demands the prune in the same commit.
HEADROOM_FRACTION = 0.02

# The ONE build configuration the seconds weights are read from.
#
# ⛔ **release, because it is the only config in which all 55 first-party crates
# share an opt level.** `Cargo.toml` pins `ambition_platformer2d_runtime`,
# `ambition_render` and `ambition_app` to `opt-level = 0` under `[profile.dev]`
# and has no `[profile.release]` table at all, so a `test`-profile weight table
# prices three crates on a different setting from the other 52 — comparing them
# is the pooling trap arriving inside a single build. Release also holds no
# override, so it is where the cost this guard exists to see is largest and
# where nobody has already mitigated it by hand.
#
# ⚠ **this is not the profile an agent's `cargo test` runs**, and that is a real
# cost of the choice. It buys mutual comparability of the 55 weights, which is
# what a SUM over a dependency closure needs; a table where three entries are
# priced differently from the rest sums to a number that means nothing.
WEIGHT_PROFILE = "release"

# A build is a REBUILD when most of its units were already cached, and COLD
# otherwise. Derived from the build's own `fresh`/`dirty` counters — see the
# module docstring for the build whose LABEL says the opposite of its counters.
#
# ⭐ **rebuild is the class that matches the question.** Blast radius is "an edit
# to this crate forces these crates to recompile" — dependencies cached, first
# party dirty. That is a rebuild, and the two release builds on record differ by
# more than 4x on the monolith (68.1s rebuild, 309.0s cold) precisely because
# the cold one is also paying for 500 third-party units competing for the same 8
# cores. Averaging the two would describe neither.
WEIGHT_CACHE_CLASS = "rebuild"
REBUILD_DIRTY_FRACTION = 0.5


# ---------------------------------------------------------------------------
# the graph
# ---------------------------------------------------------------------------


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, check=False
    ).stdout.strip()


def envelope(kind: str, *, run_id: str = "", label: str = "") -> dict:
    """The columns EVERY compile-telemetry row carries, whatever its grain.

    ⚠ **this is the part that cannot be back-filled, so it lands before any
    collector does.** `dev/ambition_dev_measurements/run_tests_cost.jsonl` has 75 rows and no commit, no
    machine, no profile and no opt-level; `dev/ambition_dev_measurements/compile_cost.jsonl` has 4 rows
    that encode the incremental setting as `machine_cargo_incremental:
    "(config default)"` in two of them and `"1"` in the other two — the same
    dimension, stringly typed, as a side effect of how the run was invoked. A
    year of that answers no question at all.

    ⛔ **and the grains stay in SEPARATE files, deliberately.** A `run_tests`
    row is one invocation of the suite carrying an array of COMMANDS; a compile
    row is one rustc invocation on one CRATE. One suite job contains hundreds of
    compile units, so nesting them would grow that file ~700x and still be wrong
    for every build that is not a suite run. Forcing one row shape over both
    grains means every row is half nulls, which is a union type with no
    discriminator rather than a schema. What must be shared is the ENVELOPE, and
    `kind` is the discriminator that lets the four files be read as one table.

    ⚠ **duplicated by hand in `run_tests.py` rather than imported.** That script
    is the suite's own entry point and must not gain an import of a module that
    itself imports a 1,500-line checker; eight keys copied is cheaper than a
    coupling that can take the suite down.
    """
    return {
        "schema": SCHEMA,
        "kind": kind,
        "recorded_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "commit": git("rev-parse", "--short=12", "HEAD") or "unknown",
        "dirty": bool(git("status", "--porcelain")),
        "run_id": run_id or uuid.uuid4().hex[:12],
        "label": label,
    }


def workspace_dirs() -> dict[str, Path]:
    """Every workspace member and its directory, from cargo rather than from a glob.

    `--offline` on purpose: this runs from the suite and must not reach the
    network, and it resolves nothing that a lockfile does not already pin.
    """
    raw = subprocess.run(
        [cargo_binary(), "metadata", "--no-deps", "--format-version", "1", "--offline"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return {
        package["name"]: Path(package["manifest_path"]).parent
        for package in json.loads(raw)["packages"]
    }


_TREE_LINE = re.compile(r"^(\d+)(\S+) v")


def resolved_edges(consumer: str) -> tuple[dict[str, set[str]], str]:
    """The consumer's RESOLVED dependency edges, from cargo's own resolver.

    ⚠ **not the manifest graph, and the difference is the whole point.** A
    static walk over `[dependencies]` counts optional edges nobody enabled, so a
    number computed that way cannot move when a feature is switched off — the
    exact trap `capability-footprint-may-not-grow` records having fallen into.
    `cargo tree --edges normal` is cargo's answer for the features actually in
    force, with dev- and build-dependencies excluded because neither is on the
    path to a rebuilt binary.

    `--prefix depth` prints the depth as an integer, so the parent of a line is
    the nearest preceding line one shallower. Cargo prints a package's children
    only at its FIRST occurrence and marks repeats `(*)`, which is fine: the
    union of the parent->child edges observed is still complete, because every
    node's children appear at its first occurrence.
    """
    raw = subprocess.run(
        [
            cargo_binary(),
            "tree",
            "--offline",
            "--edges",
            "normal",
            "--prefix",
            "depth",
            "-p",
            consumer,
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    edges: dict[str, set[str]] = {}
    stack: list[str] = []
    root = ""
    for line in raw.splitlines():
        match = _TREE_LINE.match(line)
        if not match:
            continue
        depth, name = int(match.group(1)), match.group(2)
        del stack[depth:]
        if depth == 0:
            root = name
        else:
            edges.setdefault(stack[depth - 1], set()).add(name)
        stack.append(name)
        edges.setdefault(name, set())
    if not root:
        raise SystemExit(f"⛔ `cargo tree -p {consumer}` produced no root; refusing to guess")
    return edges, root


# `tests.rs`, `tests/` and `test_*.rs` are a PROXY for test lines, not a
# measurement — this repo also writes `#[cfg(test)] mod` inline in production
# files, which no path rule can see. It is recorded as a separate column rather
# than subtracted, because both readings are wanted: `cargo check` cfg's these
# out and `cargo test --no-run` pays for them, and those are different questions.
_TEST_FILE = re.compile(r"(^|/)(tests?)\.rs$|(^|/)tests?/|(^|/)test_[^/]*\.rs$")


def crate_lines(directory: Path) -> dict[str, int]:
    """Physical lines of `src/**/*.rs`, and the test-file share of them.

    ⚠ **UNITS, stated because a LOC ledger that does not state them is a
    liability**: physical lines, blanks and comments and inline `#[cfg(test)]`
    modules included. Not statements, not tokens. The number is a PROXY for
    codegen cost and a good one at this scale — the 2026-08-07 ledger measured
    0.45 ms/line for the monolith against 7.80 ms/line for `relativity2d`, so it
    is a proxy that is wrong by 17x between crates and right within one crate
    over time. It is used here for the second thing only: how a crate moved
    against ITSELF, and how much of a crate a carve removes.
    """
    source = directory / "src"
    lines = files = test_lines = 0
    if not source.exists():
        return {"lines": 0, "files": 0, "test_file_lines": 0}
    for path in source.rglob("*.rs"):
        with path.open("rb") as handle:
            count = sum(1 for _ in handle)
        lines += count
        files += 1
        if _TEST_FILE.search(str(path.relative_to(source))):
            test_lines += count
    return {"lines": lines, "files": files, "test_file_lines": test_lines}


# ---------------------------------------------------------------------------
# the weights
# ---------------------------------------------------------------------------


def cache_class(row: dict) -> str:
    """`"rebuild"` or `"cold"` for the BUILD a unit row came from.

    ⛔ **from the counters, never from `build_label` or `build_profile`'s
    neighbours.** `dev/ambition_dev_measurements/compile_units.jsonl` holds a build labelled
    `collector: dev/first-party` with `build_fresh_units: 0` — it rebuilt all 688
    units and took 540s, against two honestly-labelled first-party rebuilds at
    188s and 210s. Selecting on that label puts a cold build's durations into a
    rebuild's weight table, and every weight comes out 2-4x high with nothing in
    the output saying so.

    The atomic unit is a BUILD, and a build's own `fresh`/`dirty` counters are
    the only thing in the row that cannot be mislabelled by the caller that
    invoked cargo.
    """
    fresh = row.get("build_fresh_units") or 0
    dirty = row.get("build_dirty_units") or 0
    total = fresh + dirty
    if not total:
        return "unknown"
    return "rebuild" if dirty / total <= REBUILD_DIRTY_FRACTION else "cold"


def load_unit_rows(ledger: Path | None = None) -> list[dict]:
    path = ledger or UNIT_LEDGER
    if not path.exists():
        return []
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def unit_weights(rows: list[dict] | None = None) -> dict:
    """The per-crate compile weight, in **ms per line**, from the committed ledger.

    ⭐ **a RATE, not a duration, and that is the whole modelling decision.** A
    frozen duration cannot answer "what if 10,000 lines moved", which is the
    question a carve asks; a frozen rate multiplied by today's line count can.
    It also composes exactly the way this file already believes the two
    measurements behave — see `crate_lines`: lines are wrong by 17x BETWEEN
    crates and right for one crate against ITSELF over time. So the measured
    rate carries the between-crate spread that a line count gets wrong, and the
    line count carries the within-crate movement that a frozen duration gets
    wrong. Each does the half it is good at.

    Selection, in order, and every filter is here because dropping it produces a
    plausible wrong number:

    * `first_party` — third-party units are cached and never in a blast radius.
    * ONE profile (`WEIGHT_PROFILE`) and ONE cache class (`WEIGHT_CACHE_CLASS`),
      because the ledger mixes three configurations and two cache states.
    * ⛔ `backfilled` rows are DROPPED. A backfilled row's `lines` column
      describes the tree at ingest, not the tree that was built, so
      `seconds / lines` divides a measurement by an unrelated number.

    All of a crate's units are summed before dividing — lib, test, bin and build
    script are one crate's contribution to one build, which is also how
    `scripts/compile_report.py` counts so the two readers of this ledger cannot
    disagree. Across builds the per-crate rate is the MEDIAN, so a single odd
    build cannot carry the table.
    """
    rows = load_unit_rows() if rows is None else rows
    selected = [
        row
        for row in rows
        if row.get("first_party")
        and row.get("build_profile") == WEIGHT_PROFILE
        and cache_class(row) == WEIGHT_CACHE_CLASS
        and not row.get("backfilled")
        and row.get("seconds")
        and row.get("lines")
    ]

    per_build: dict[str, dict[str, list[dict]]] = {}
    for row in selected:
        source = row.get("build_source") or ""
        per_build.setdefault(source, {}).setdefault(row["unit"], []).append(row)

    rates: dict[str, list[float]] = {}
    for units in per_build.values():
        for name, unit_rows in units.items():
            seconds = sum(r["seconds"] for r in unit_rows)
            # One crate has ONE line count however many units it compiles into.
            lines = max(r["lines"] for r in unit_rows)
            rates.setdefault(name, []).append(seconds * 1000.0 / lines)

    table = {name: round(statistics.median(v), 4) for name, v in sorted(rates.items())}
    builds = [
        {
            "source": source,
            "started_at": sample["build_build_started_at"],
            "commit": sample["commit"],
            "fresh_units": sample["build_fresh_units"],
            "dirty_units": sample["build_dirty_units"],
            "wall_clock": sample["build_total_seconds"],
            # ⚠ recorded for the reader, NEVER selected on. See `cache_class`.
            "untrusted_label": sample.get("build_label"),
        }
        for source, units in sorted(per_build.items())
        for sample in [next(iter(units.values()))[0]]
    ]
    return {
        "weight_unit": "milliseconds of rustc wall time per physical line of src/**/*.rs",
        "profile": WEIGHT_PROFILE,
        "cache_class": WEIGHT_CACHE_CLASS,
        "opt_levels": sorted({str(r.get("opt_level")) for r in selected}),
        "ledger": str(
            UNIT_LEDGER.relative_to(ROOT)
            if UNIT_LEDGER.is_relative_to(ROOT)
            else UNIT_LEDGER
        ),
        "builds": sorted(builds, key=lambda b: b["started_at"]),
        "crates_measured": len(table),
        # ⛔ the fallback for a crate nobody has measured, and it is a GUESS.
        # A least-squares fit of `seconds ~ a + b*lines` over these 55 crates
        # reads R^2 = 0.12 and predicts 24.7s for a 10,000-line crate against
        # the median rate's 25.6s — the same answer, from a fitted parameter
        # that explains an eighth of the variance. Size does not predict a new
        # crate's cost, so no arithmetic over sizes will. The median is used
        # because the number still has to be computable, and `evaluate` raises
        # an UNPRICED finding so that nobody mistakes it for a measurement.
        "median_ms_per_line": round(statistics.median(table.values()), 4) if table else 0.0,
        "ms_per_line": table,
    }


def snapshot(
    consumer: str = CONSUMER,
    *,
    override_lines: dict[str, int] | None = None,
    extra_edges: dict[str, set[str]] | None = None,
    weights: dict | None = None,
) -> dict:
    """Every number this file guards, plus the per-crate table behind them.

    `override_lines`, `extra_edges` and `weights` exist so `--carve` can ask the
    same question of a graph that does not exist yet. A simulator that
    reimplements the metric is a simulator that answers a different question.

    `weights` is a `unit_weights()` payload. The gate passes the one FROZEN in
    the baseline so that appending to `dev/ambition_dev_measurements/compile_units.jsonl` cannot move a
    guarded number without a re-freeze; `--update` passes None and reads the
    ledger.
    """
    weights = unit_weights() if weights is None else weights
    rate_table = weights.get("ms_per_line") or {}
    median_rate = weights.get("median_ms_per_line") or 0.0
    dirs = workspace_dirs()
    measured = {name: crate_lines(path) for name, path in dirs.items()}
    edges, root = resolved_edges(consumer)

    first_party = {name for name in edges if name in measured} | {root}
    forward = {
        name: {dep for dep in edges.get(name, ()) if dep in first_party}
        for name in first_party
    }
    for name, deps in (extra_edges or {}).items():
        first_party.add(name)
        forward.setdefault(name, set())
        forward[name] |= deps
        for dep in deps:
            first_party.add(dep)
            forward.setdefault(dep, set())

    def line_count(name: str) -> int:
        if override_lines and name in override_lines:
            return override_lines[name]
        return measured.get(name, {}).get("lines", 0)

    reverse: dict[str, set[str]] = {name: set() for name in first_party}
    for name, deps in forward.items():
        for dep in deps:
            reverse.setdefault(dep, set()).add(name)

    def dependents(start: str) -> set[str]:
        """`start` and everything that must recompile when `start` changes."""
        seen, queue = {start}, [start]
        while queue:
            current = queue.pop()
            for parent in reverse.get(current, ()):
                if parent not in seen:
                    seen.add(parent)
                    queue.append(parent)
        return seen

    @functools.lru_cache(maxsize=None)
    def height(name: str) -> int:
        """Longest chain of first-party crates from `name` up to a consumer."""
        return 1 + max((height(p) for p in reverse.get(name, ())), default=0)

    def crate_seconds(name: str) -> float:
        """The measured cost of compiling this crate, at its CURRENT size.

        ⚠ **the fallback is the population median RATE, and it is a placeholder
        rather than an estimate.** A crate nobody has built has no measurement,
        and nothing in the population predicts one — `unit_weights` records why.
        Zero would be worse in the one direction that matters: it makes a new
        crate free, which is exactly the shape of the carve this guard exists to
        price. `unpriced_crates` below carries the names so the report can say
        which numbers rest on the guess.
        """
        return rate_table.get(name, median_rate) * line_count(name) / 1000.0

    table: dict[str, dict] = {}
    for name in sorted(first_party):
        closure = dependents(name)
        table[name] = {
            **{k: v for k, v in measured.get(name, {}).items()},
            "lines": line_count(name),
            "edit_cost_lines": sum(line_count(x) for x in closure),
            "edit_cost_crates": len(closure),
            "ms_per_line": rate_table.get(name, median_rate),
            "seconds": round(crate_seconds(name), 3),
            "seconds_source": "measured" if name in rate_table else "estimated",
            "edit_cost_seconds": round(sum(crate_seconds(x) for x in closure), 3),
            "depth": height(name),
            # DIRECT first-party dependents — not the transitive closure that
            # `edit_cost_crates` counts. Recorded because it is what makes a
            # `--diff` readable: "this crate's edit cost jumped" is the symptom
            # and "this crate gained a dependent" is the cause.
            "direct_dependents": sorted(reverse.get(name, ())),
        }
    table.setdefault(root, {}).setdefault("lines", 0)

    largest = max(table.items(), key=lambda kv: (kv[1]["lines"], kv[0]))
    worst = max(table.items(), key=lambda kv: (kv[1]["edit_cost_lines"], kv[0]))
    # ⚠ maximised INDEPENDENTLY of the lines version, and it has to be: the
    # crate whose closure holds the most lines need not be the crate whose
    # closure costs the most seconds. When they agree that is a fact about this
    # tree, not an invariant.
    worst_seconds = max(
        table.items(), key=lambda kv: (kv[1].get("edit_cost_seconds", 0.0), kv[0])
    )
    dearest = max(table.items(), key=lambda kv: (kv[1].get("seconds", 0.0), kv[0]))

    return {
        "schema": SCHEMA,
        "kind": "graph",
        "consumer": consumer,
        "line_unit": "physical lines of <crate>/src/**/*.rs, inline #[cfg(test)] included",
        "first_party_crates": len(first_party),
        "first_party_lines": sum(table[n]["lines"] for n in table),
        "first_party_seconds": round(sum(table[n].get("seconds", 0.0) for n in table), 3),
        "largest_unit": {"crate": largest[0], "lines": largest[1]["lines"]},
        # ⚠ REPORTED, not guarded. It moves on the same events as
        # `largest_unit_lines` and `worst_edit_cost_seconds` between them, and
        # the file's own selection rule says that makes it a dashboard entry.
        # It is here because it is the single most surprising number in the
        # table: the most expensive recompilation unit is not the biggest one.
        "largest_unit_seconds": {
            "crate": dearest[0],
            "seconds": dearest[1].get("seconds", 0.0),
        },
        "worst_edit_cost": {
            "crate": worst[0],
            "lines": worst[1]["edit_cost_lines"],
            "crates": worst[1]["edit_cost_crates"],
        },
        "worst_edit_cost_seconds": {
            "crate": worst_seconds[0],
            "seconds": worst_seconds[1].get("edit_cost_seconds", 0.0),
            "crates": worst_seconds[1]["edit_cost_crates"],
        },
        "watched_edit_cost": {
            name: {
                "lines": table[name]["edit_cost_lines"],
                "crates": table[name]["edit_cost_crates"],
                "seconds": table[name].get("edit_cost_seconds", 0.0),
            }
            for name in WATCHED
            if name in table
        },
        "critical_path_crates": max(entry["depth"] for entry in table.values()),
        "unpriced_crates": sorted(
            name for name in table if table[name].get("seconds_source") == "estimated"
        ),
        # The whole payload, rate table included, so `freeze` writes a baseline
        # the gate can read back verbatim. Reconstructing it from the per-crate
        # column would silently promote every ESTIMATED crate to measured, since
        # both store a number and only this table knows which are real.
        "unit_weights": weights,
        "crates": table,
    }


# ---------------------------------------------------------------------------
# the gate
# ---------------------------------------------------------------------------


def _lines(value: float) -> str:
    return f"{int(value):,}"


def _seconds(value: float) -> str:
    return f"{value:,.1f}s"


def _compare(
    label: str,
    current: float,
    frozen: float,
    headroom: float,
    fmt=_lines,
) -> tuple[str, str] | None:
    """`(severity, message)` for one number, or None when it is inside budget."""
    if current > frozen + headroom:
        return (
            "REGRESSED",
            f"{label}: {fmt(frozen)} -> {fmt(current)} (+{fmt(current - frozen)}, "
            f"budget +{fmt(headroom)}). Something got bigger or grew a dependency "
            f"edge. If this is a deliberate landing, say so and re-freeze; if it "
            f"is a module that belongs in its own crate, that is the finding.",
        )
    if current < frozen - headroom:
        return (
            "CARVED",
            f"{label}: {fmt(frozen)} -> {fmt(current)} ({fmt(current - frozen)}). "
            f"This is a WIN and the baseline is now stale — re-freeze it in this "
            f"commit (`--update`), or the guard is holding {fmt(frozen - current)} "
            f"of slack and the next regression that size lands silently.",
        )
    return None


def evaluate(current: dict, frozen: dict) -> list[tuple[str, str]]:
    """Every guarded number that is outside its budget, worst class first."""
    fraction = frozen.get("headroom_fraction", HEADROOM_FRACTION)
    findings: list[tuple[str, str]] = []

    def line_check(label: str, now: int, then: int) -> None:
        result = _compare(label, now, then, max(1, int(then * fraction)))
        if result:
            findings.append(result)

    def seconds_check(label: str, now: float, then: float) -> None:
        result = _compare(label, now, then, max(0.1, then * fraction), fmt=_seconds)
        if result:
            findings.append(result)

    # ⛔ a baseline frozen before the seconds guard existed has no number to
    # compare against, and skipping it silently is precisely "a check that
    # CANNOT FAIL". Say so and go red; `--update` is one command.
    if "worst_edit_cost_seconds" not in frozen:
        findings.append(
            (
                "STALE",
                "the frozen baseline predates `worst_edit_cost_seconds`, so the "
                "seconds guard checked nothing this run. Re-freeze with "
                "`--update` and commit it.",
            )
        )
    else:
        seconds_check(
            f"worst_edit_cost_seconds ({current['worst_edit_cost_seconds']['crate']})",
            current["worst_edit_cost_seconds"]["seconds"],
            frozen["worst_edit_cost_seconds"]["seconds"],
        )

    # ⚠ a crate priced by the fallback is a crate this guard cannot see, and the
    # carve that adds one is exactly the carve worth pricing. Loud on purpose.
    new_unpriced = sorted(
        set(current.get("unpriced_crates", ())) - set(frozen.get("unpriced_crates", ()))
    )
    if new_unpriced:
        rate = current["unit_weights"]["median_ms_per_line"]
        findings.append(
            (
                "UNPRICED",
                f"no measured compile cost for {', '.join(new_unpriced)}. They are "
                f"priced at the population median {rate} ms/line, which is a "
                f"PLACEHOLDER — size predicts a crate's compile cost with R^2 = "
                f"0.12, so the seconds numbers above under- or over-state these "
                f"crates by an unknown factor. Run `python3 "
                f"scripts/compile_collect.py` to measure them and re-freeze, or "
                f"say in the commit that the guess is accepted for now.",
            )
        )

    line_check(
        f"largest_unit_lines ({current['largest_unit']['crate']})",
        current["largest_unit"]["lines"],
        frozen["largest_unit"]["lines"],
    )
    line_check(
        f"worst_edit_cost_lines ({current['worst_edit_cost']['crate']})",
        current["worst_edit_cost"]["lines"],
        frozen["worst_edit_cost"]["lines"],
    )
    if current["largest_unit"]["crate"] != frozen["largest_unit"]["crate"]:
        findings.append(
            (
                "MOVED",
                f"the largest recompilation unit is now "
                f"{current['largest_unit']['crate']}, not "
                f"{frozen['largest_unit']['crate']}. Either a carve worked or a "
                f"different crate has become the problem; both need the baseline "
                f"re-frozen and both are worth a sentence in the commit.",
            )
        )

    for name, frozen_entry in frozen["watched_edit_cost"].items():
        if name not in current["watched_edit_cost"]:
            findings.append(
                (
                    "GONE",
                    f"watched crate `{name}` is no longer in {current['consumer']}'s "
                    f"resolved graph. If it was carved or renamed, update WATCHED in "
                    f"this script — a watch list naming a crate that does not exist "
                    f"is a guard that measures nothing.",
                )
            )
            continue
        line_check(
            f"edit_cost_lines ({name})",
            current["watched_edit_cost"][name]["lines"],
            frozen_entry["lines"],
        )
        if "seconds" in frozen_entry:
            seconds_check(
                f"edit_cost_seconds ({name})",
                current["watched_edit_cost"][name]["seconds"],
                frozen_entry["seconds"],
            )

    # ⛔ EXACT, both directions, and deliberately not budgeted. This number only
    # moves when the SHAPE of the graph changes, which never happens by
    # accident. A carve that lengthens the serial chain is the failure this
    # whole guard exists to make visible, and it is invisible in every other
    # number here.
    if current["critical_path_crates"] != frozen["critical_path_crates"]:
        direction = (
            "LONGER — parallelism cannot compress this, so the wall clock gets "
            "worse even if every crate got smaller"
            if current["critical_path_crates"] > frozen["critical_path_crates"]
            else "SHORTER, which is a real win worth recording"
        )
        findings.append(
            (
                "PATH",
                f"critical_path_crates: {frozen['critical_path_crates']} -> "
                f"{current['critical_path_crates']} — {direction}. Re-freeze with "
                f"`--update` and say in the commit which carve did it.",
            )
        )

    order = {
        "REGRESSED": 0,
        "PATH": 1,
        "STALE": 2,
        "UNPRICED": 3,
        "MOVED": 4,
        "GONE": 5,
        "CARVED": 6,
    }
    findings.sort(key=lambda item: order.get(item[0], 9))
    return findings


def _vs_baseline(current: dict, frozen: dict) -> str:
    """`largest_unit_lines` against its frozen value — printed even when inside budget.

    ⛔⛔ **A WIN CAN BE GIVEN BACK IN SILENCE, and this row watched it happen.**
    The monolith went 111,429 (frozen) → 110,932 after a carve, celebrated as
    "under baseline for the first time", and was at 112,357 one day later —
    **+928 OVER the baseline**. The ratchet said nothing, correctly: the number
    carries a 2% growth budget and 112,357 sits inside it.

    ⭐ so the gate is right and the REPORT was incomplete. A budget answers *"is
    this a regression worth failing on"*; it does not answer *"are we where we
    thought we were"*. This annotates the second question and gates nothing —
    ⛔ deliberately NOT a tightened budget, because Jon's ruling stands: *"the
    compile ratchet is an INSTRUMENT, NOT A TARGET"*, and a tighter budget would
    make it more of one.
    """

    frozen_unit = frozen.get("largest_unit") or {}
    was = frozen_unit.get("lines")
    now = current.get("largest_unit", {}).get("lines")
    if not isinstance(was, int) or not isinstance(now, int) or was <= 0:
        return ""
    delta = now - was
    budget = int(was * frozen.get("headroom_fraction", HEADROOM_FRACTION))
    inside = " within budget" if abs(delta) <= budget else " OUTSIDE budget"
    same_crate = frozen_unit.get("crate") == current.get("largest_unit", {}).get("crate")
    note = "" if same_crate else " (different crate than the frozen one)"
    return f"   [frozen {was:,}, {delta:+,}, budget ±{budget:,}{inside}]{note}"


def report(current: dict, frozen: dict) -> None:
    worst_seconds = current.get("worst_edit_cost_seconds", {})
    dearest = current.get("largest_unit_seconds", {})
    weights = current.get("unit_weights", {})
    print(f"  consumer                {current['consumer']}  "
          f"({current['first_party_crates']} first-party crates, "
          f"{current['first_party_lines']:,} lines, "
          f"{current.get('first_party_seconds', 0):,.0f}s)")
    print(f"  worst_edit_cost_seconds {worst_seconds.get('seconds', 0):>8,.1f}s  "
          f"{worst_seconds.get('crate', '?')} "
          f"({worst_seconds.get('crates', 0)} crates)")
    for name, entry in current["watched_edit_cost"].items():
        print(f"  edit_cost_seconds       {entry.get('seconds', 0):>8,.1f}s  {name} "
              f"({entry['crates']} crates)")
    print(f"  critical_path_crates    {current['critical_path_crates']:>9}  "
          f"longest serial chain")
    print(f"  largest_unit_lines      {current['largest_unit']['lines']:>9,}  "
          f"{current['largest_unit']['crate']}{_vs_baseline(current, frozen)}")
    print(f"  worst_edit_cost_lines   {current['worst_edit_cost']['lines']:>9,}  "
          f"{current['worst_edit_cost']['crate']} "
          f"({current['worst_edit_cost']['crates']} crates)"
          f"{_share_of_workspace(current, frozen, current['worst_edit_cost'], current['worst_edit_cost']['crate'])}")
    for name, entry in current["watched_edit_cost"].items():
        print(f"  edit_cost_lines         {entry['lines']:>9,}  {name} "
              f"({entry['crates']} crates)"
              f"{_share_of_workspace(current, frozen, entry, name)}")
    print(f"  · largest_unit_seconds  {dearest.get('seconds', 0):>8,.1f}s  "
          f"{dearest.get('crate', '?')}  (context, not guarded)")
    if weights:
        estimated = current.get("unpriced_crates", [])
        print(f"\n  weights   {weights.get('profile')}/opt-"
              f"{','.join(weights.get('opt_levels') or ['?'])} "
              f"{weights.get('cache_class')}, "
              f"{len(weights.get('builds') or [])} build(s), "
              f"{weights.get('crates_measured', 0)} crates measured"
              + (f", {len(estimated)} at the median {weights.get('median_ms_per_line')} "
                 f"ms/line ({', '.join(estimated)})" if estimated else ""))
    print(f"\n  baseline frozen at {frozen.get('commit', '?')} "
          f"({frozen.get('recorded_at', '?')}), "
          f"headroom {frozen.get('headroom_fraction', HEADROOM_FRACTION):.0%}")



def _share_of_workspace(current: dict, frozen: dict, entry: dict, crate: str) -> str:
    """An edit cost as a FRACTION of the workspace, beside the absolute number.

    ⛔⛔ **THE ABSOLUTE NUMBER ASKS A PROXY QUESTION FOR A FOUNDATION CRATE, and
    on 2026-08-19 that cost a reading of three findings.** `edit_cost_lines` is
    the total lines of a crate's reverse-dependency closure, so for a crate 49 of
    59 crates depend on it is very nearly a measurement of the workspace itself:

    ```text
    workspace              456,072 -> 535,388   +79,316
    ambition_geometry      433,774 -> 511,209   +77,435  = 97.6% of that growth
                own lines                         +197
    ```

    The crate grew 197 lines and its edit cost grew 77,435, and the gate said
    `REGRESSED  Something got bigger or grew a dependency edge. ... if it is a
    module that belongs in its own crate, that is the finding`. Carving
    `ambition_geometry` could not have moved that number by anything.

    ⭐ **the share is the quantity that distinguishes the two stories**: geometry
    went 95.1% -> 95.5% of the workspace (+0.4 pts) — architecturally flat —
    while the actor monolith went 55.1% -> 53.3% (-1.7 pts), i.e. the
    decomposition working, which the absolute number reports as a regression.

    ⚠ **this is REPORT ONLY and deliberately does not touch what the gate fails
    on.** A metric that guards should not change its verdicts in the same commit
    that teaches it a new number; what was wrong here was the reading, and a
    reader who can see both figures gets it right.
    """
    now_total = current.get("first_party_lines") or 0
    was_total = frozen.get("first_party_lines") or 0
    lines = entry.get("lines") or 0
    if not now_total or not lines:
        return ""
    now_share = 100.0 * lines / now_total
    # The frozen side is only comparable when the baseline recorded the same
    # crate; a NEW crate has no share to move from and says so by omission.
    # ⚠ the two shapes differ: `worst_edit_cost` is one record CARRYING its crate
    # name, `watched_edit_cost` is a dict KEYED by it. Reading only the first is
    # why the watched crates printed no delta on the first attempt — and the
    # delta is the whole point, since it is the monolith's −1.7 pts that shows
    # the decomposition working.
    was = None
    worst = frozen.get("worst_edit_cost")
    if isinstance(worst, dict) and worst.get("crate") == crate:
        was = worst.get("lines")
    watched = frozen.get("watched_edit_cost")
    if was is None and isinstance(watched, dict) and crate in watched:
        was = watched[crate].get("lines")
    if not was or not was_total:
        return f"   [{now_share:.1f}% of the workspace]"
    was_share = 100.0 * was / was_total
    return (f"   [{was_share:.1f}% -> {now_share:.1f}% of the workspace, "
            f"{now_share - was_share:+.1f} pts]")

def diff(current: dict, frozen: dict) -> None:
    """Per-crate attribution: what moved since the baseline, and by how much.

    This is the half that answers "carving X moved these numbers by Y". The gate
    says pass or fail; nobody can write a commit message from a pass.
    """
    print(f"since {frozen.get('commit', '?')} ({frozen.get('recorded_at', '?')}):\n")
    rows = []
    names = set(current["crates"]) | set(frozen.get("crates", {}))
    for name in names:
        now = current["crates"].get(name, {})
        then = frozen.get("crates", {}).get(name, {})
        moved = now.get("lines", 0) - then.get("lines", 0)
        cost = now.get("edit_cost_lines", 0) - then.get("edit_cost_lines", 0)
        # ⚠ the seconds column is what makes the other two readable: 10,000
        # lines leaving a cheap crate for a dense one shows as a wash in `lines`
        # and a win in `edit cost`, and only here as the regression it is.
        secs = now.get("edit_cost_seconds", 0.0) - then.get("edit_cost_seconds", 0.0)
        if moved or cost or round(secs, 1):
            rows.append((abs(moved), moved, cost, secs, name, bool(then), bool(now)))
    if not rows:
        print("  nothing moved.")
        return
    print(f"  {'lines':>9}  {'edit cost':>10}  {'edit cost s':>12}  crate")
    for _, moved, cost, secs, name, was, now in sorted(rows, reverse=True):
        tag = "" if (was and now) else ("  [NEW CRATE]" if now else "  [GONE]")
        print(f"  {moved:>+9,}  {cost:>+10,}  {secs:>+11,.1f}s  {name}{tag}")


# ---------------------------------------------------------------------------
# the carve simulator
# ---------------------------------------------------------------------------


_CRATE_REF = re.compile(r"\bcrate::([a-z_][a-z0-9_]*)")


def module_coupling(module: Path) -> tuple[set[str], set[str], int]:
    """`(inward, outward, lines)` for lifting `module` out of its own crate.

    * **inward** — the modules of the owning crate that NAME this one. Nonempty
      means the owner would depend on the new crate, so the new crate lands
      BELOW it and an edit to the module still rebuilds the owner.
    * **outward** — the sibling modules THIS one names. Nonempty means the carve
      is not a `Cargo.toml`; those have to move or be ported first, or the new
      crate cycles.

    ⚠ **comments are stripped before matching**, using the same helper the
    absence contracts use, because this repo went red on PROSE three times: a
    module docstring explaining that an edge was REMOVED reads exactly like the
    edge being present. `conversation/mod.rs` is the live example — every
    `crate::` string in it is in a doc comment about edges it no longer has.
    """
    crate_src = module.parent
    while crate_src.name != "src" and crate_src != crate_src.parent:
        crate_src = crate_src.parent
    name = module.name if module.is_dir() else module.stem

    # ⛔ TOP-LEVEL MODULES ONLY, and the refusal is the honest half of this tool.
    #
    # Coupling is detected by matching `crate::<segment>` after comment
    # stripping, which sees the FIRST path segment and nothing more. For a
    # nested module the callers write `crate::features::npcs`, so that match
    # reports `features` and this function would confidently answer "no inward
    # edges" — the SIBLING verdict, which is the one with the large payoff. A
    # carve sold on a number produced that way is exactly the failure this file
    # exists to prevent, so it refuses rather than guesses.
    #
    # It is not a real restriction either: lifting `features/npcs.rs` alone
    # leaves `features/` behind and is not a crate carve. Resolve the nesting
    # first, then simulate.
    if module.parent != crate_src:
        raise SystemExit(
            f"⛔ {module.relative_to(ROOT)} is a NESTED module "
            f"(under `{module.parent.relative_to(crate_src)}/`), and coupling here "
            f"is detected one path segment at a time. Simulating it would report "
            f"the parent's edges as this module's and very likely answer SIBLING "
            f"when it is not. Simulate the top-level module "
            f"`{module.relative_to(crate_src).parts[0]}` instead, or hoist this "
            f"module to the crate root first."
        )

    def refs(paths) -> set[str]:
        found: set[str] = set()
        for path in paths:
            for number, raw in enumerate(
                path.read_text(errors="replace").splitlines(), start=1
            ):
                text = strip_comments_for(str(path), raw)
                found.update(_CRATE_REF.findall(text))
        return found

    module_files = sorted(module.rglob("*.rs")) if module.is_dir() else [module]
    module_set = set(module_files)
    others = [p for p in sorted(crate_src.rglob("*.rs")) if p not in module_set]

    inward = {
        str(path.relative_to(crate_src))
        for path in others
        if name in refs([path])
    }
    outward = refs(module_files) - {name}
    lines = sum(
        sum(1 for _ in path.open("rb")) for path in module_files
    )
    return inward, outward, lines


def simulate_carve(
    module: Path,
    new_crate: str | None = None,
    new_crate_rate: float | None = None,
) -> None:
    module = module.resolve()
    if not module.exists():
        raise SystemExit(f"⛔ {module} does not exist")
    crate_dir = module
    while not (crate_dir / "Cargo.toml").exists() and crate_dir != crate_dir.parent:
        crate_dir = crate_dir.parent
    owner = crate_dir.name
    new_crate = new_crate or f"ambition_{module.stem if module.is_file() else module.name}"

    inward, outward, lines = module_coupling(module)
    # ⛔ the new crate's ms/line is the one number a simulation cannot measure,
    # and it decides the answer. Left at the population median a carve into a
    # `relativity2d`-shaped crate (22.9 ms/line, 9x the median) reads as nearly
    # free. `--new-crate-rate` is how you ask the honest question — "and if it
    # compiles like the runtime?" — rather than accepting the optimistic default
    # silently.
    weights = unit_weights()
    if new_crate_rate is not None:
        weights = {
            **weights,
            "ms_per_line": {**weights["ms_per_line"], new_crate: new_crate_rate},
        }
    before = snapshot(weights=weights)
    if owner not in before["crates"]:
        raise SystemExit(f"⛔ {owner} is not in {CONSUMER}'s resolved graph")

    sibling = not inward
    override = {owner: before["crates"][owner]["lines"] - lines, new_crate: lines}
    # ⚠ the new crate's placement is DERIVED from the coupling, not chosen. If
    # the owner names the module, the owner depends on the new crate and the new
    # crate lands BELOW it; if it does not, the new crate is a SIBLING and
    # whoever consumed the module through the owner's facade picks it up
    # directly. Those two placements have wildly different payoffs, and getting
    # them backwards is how a carve gets sold on a number it will not deliver.
    #
    # For the sibling case the simulated consumers are the owner's own direct
    # dependents — the WORST case, because the real carve will usually be picked
    # up by fewer of them. An upper bound that is labelled as one is honest; a
    # guess dressed as a measurement is not.
    if sibling:
        extra = {
            parent: {new_crate}
            for parent in before["crates"][owner]["direct_dependents"]
        }
        extra = extra or {before["consumer"]: {new_crate}}
    else:
        extra = {owner: {new_crate}}
    # ⛔ **and the new crate keeps the owner's own dependencies.** Leaving them
    # out — which the first version did — gives the simulated crate NO
    # dependencies at all, so it falls out of every floor crate's dependent
    # closure and `worst_edit_cost` reports the carve as removing its lines from
    # the graph entirely. It does not: a module lifted out of the monolith still
    # names `ambition_geometry`. Inheriting the whole set is an UPPER bound (the
    # module surely uses a subset) and errs toward "a carve is not free", which
    # is the direction this file is deliberately biased.
    extra[new_crate] = {
        name
        for name, entry in before["crates"].items()
        if owner in entry.get("direct_dependents", ())
    }
    after = snapshot(override_lines=override, extra_edges=extra, weights=weights)

    assumed_rate = weights["ms_per_line"].get(new_crate, weights["median_ms_per_line"])
    print(f"CARVE SIMULATION  {module.relative_to(ROOT)}")
    print(f"  owner crate            {owner}  "
          f"({before['crates'][owner]['ms_per_line']} ms/line, measured)")
    print(f"  proposed crate         {new_crate}  ({lines:,} lines at "
          f"{assumed_rate} ms/line, "
          f"{'ASSUMED via --new-crate-rate' if new_crate_rate is not None else 'the population MEDIAN — a placeholder, not a measurement'})")
    print(f"  inward edges           "
          + ("NONE — no file in the owner names it"
             if sibling
             else f"{len(inward)} file(s) in the owner name it: "
                  + ", ".join(sorted(inward)[:4])
                  + (" …" if len(inward) > 4 else "")))
    print(f"  outward edges          "
          f"{sorted(outward) if outward else 'NONE — nothing in the owner is named back'}")
    print(f"  resulting placement    "
          f"{'SIBLING of ' + owner if sibling else 'BELOW ' + owner + ' (owner depends on it)'}")
    if outward:
        print("  ⛔ the outward edges above must move or be ported FIRST; a crate "
              "that names its old owner cannot compile.")
    print()

    def line(label: str, a: int, b: int) -> None:
        delta = b - a
        pct = (delta / a * 100) if a else 0.0
        print(f"  {label:<28} {a:>9,} -> {b:>9,}   {delta:>+9,}  ({pct:+.2f}%)")

    line("largest_unit_lines", before["largest_unit"]["lines"], after["largest_unit"]["lines"])
    line(
        "edit_cost(rest of owner)",
        before["crates"][owner]["edit_cost_lines"],
        after["crates"][owner]["edit_cost_lines"],
    )
    line(
        "edit_cost(the module)",
        before["crates"][owner]["edit_cost_lines"],
        after["crates"][new_crate]["edit_cost_lines"],
    )
    line(
        "critical_path_crates",
        before["critical_path_crates"],
        after["critical_path_crates"],
    )

    def line_seconds(label: str, a: float, b: float) -> None:
        delta = b - a
        pct = (delta / a * 100) if a else 0.0
        print(f"  {label:<28} {a:>8,.1f}s -> {b:>8,.1f}s   {delta:>+8,.1f}s  ({pct:+.2f}%)")

    print()
    # ⭐ the rows the four line numbers above cannot produce. Total seconds is
    # the one that answers "does the BUILD get faster", and it is free to
    # disagree with every percentage above it — that disagreement is the entire
    # reason this section exists.
    line_seconds(
        "first_party_seconds",
        before["first_party_seconds"],
        after["first_party_seconds"],
    )
    line_seconds(
        "worst_edit_cost_seconds",
        before["worst_edit_cost_seconds"]["seconds"],
        after["worst_edit_cost_seconds"]["seconds"],
    )
    line_seconds(
        "edit_cost_s(rest of owner)",
        before["crates"][owner]["edit_cost_seconds"],
        after["crates"][owner]["edit_cost_seconds"],
    )
    line_seconds(
        "edit_cost_s(the module)",
        before["crates"][owner]["edit_cost_seconds"],
        after["crates"][new_crate]["edit_cost_seconds"],
    )
    if after["first_party_seconds"] > before["first_party_seconds"]:
        print(f"\n  ⛔ **this carve makes the BUILD SLOWER** by "
              f"{after['first_party_seconds'] - before['first_party_seconds']:,.1f}s "
              f"of first-party rustc time, whatever the line rows above say. "
              f"{lines:,} lines leave {owner} at "
              f"{before['crates'][owner]['ms_per_line']} ms/line and arrive at "
              f"{assumed_rate} ms/line"
              + ("." if new_crate_rate is not None else
                 ", the population MEDIAN. Re-run with `--new-crate-rate` for a "
                 "denser assumption; the four densest crates in the workspace are "
                 "all small consumers at 12.8-22.9 ms/line, which is the shape a "
                 "carved crate has."))
    print()
    if sibling:
        print("  ⭐ SIBLING carve. An edit to the module no longer rebuilds "
              f"{owner} at all, and the two can compile in parallel. This is the "
              "shape a carve has to have for the compile-time argument to be "
              "worth making.")
    else:
        print("  ⚠ **`edit_cost(the module)` DOES NOT FALL, and that is the "
              f"finding.** {owner} depends on the new crate, so an edit to the "
              "module still rebuilds the owner and everything above it — the "
              "isolation runs one direction only. What the carve buys is the "
              f"`rest of owner` row: edits to the other "
              f"{after['crates'][owner]['lines']:,} lines of {owner} stop "
              f"rebuilding these {lines:,}. Judge this carve on architecture; the "
              "compile-time argument is the percentages above and nothing more.")


# ---------------------------------------------------------------------------
# the per-unit ledger (D9)
# ---------------------------------------------------------------------------


_UNIT_DATA = re.compile(r"const UNIT_DATA = (\[.*?\n\]);", re.S)
_HEAD_ROW = re.compile(r"<td>([^<]{1,40}?):</td>\s*<td>(.*?)</td>", re.S)


def ingest_timings(
    html: Path,
    *,
    label: str = "",
    record: bool = True,
    run_id: str = "",
    profile: str = "dev",
    extra: dict | None = None,
    unit_extra=None,
) -> list[dict]:
    """Turn one `cargo build --timings` report into per-unit ledger rows.

    `extra` is merged into every row and `unit_extra(unit) -> dict` overrides
    per-unit fields. Both exist for `scripts/compile_collect.py`, which owns the
    environment its build ran under and can therefore fill the two columns this
    report cannot carry: `incremental`, and an `opt_level` read off the rustc
    command line rather than modelled from the manifest. ⛔ **that modelling is
    exactly what went wrong once already** — `[profile.dev.package."*"]` does not
    apply to workspace members — so a row carries `opt_level_source` saying which
    of the two it is, and a hand ingest of an old report still gets the modelled
    value rather than a null.

    ⭐ **the HTML is the source, and that is a finding rather than a fallback.**
    `--timings=json` is `-Z unstable-options` on stable cargo, so ADR 0013's
    "quarterly" prescription cannot be automated the obvious way. The stable
    HTML report EMBEDS the identical per-unit JSON as `const UNIT_DATA`,
    including the per-unit `sections` split into `frontend` and `codegen` —
    which is exactly where the 2026-08-07 finding ("255 of 313 unit-seconds are
    codegen") came from. So the collector needs no nightly toolchain.

    ⚠ **only units that did WORK are recorded.** A cargo timing report lists
    every unit in the graph, fresh ones included; the 2026-08-07 report has 688
    units of which 669 were cached at duration 0. "How long did nothing take" is
    not a statistic, and 669 zero rows per build would bury the 19 that matter.
    The fresh/dirty counts survive as build-level columns, so the cache state
    that produced the run is still recoverable.

    ⚠ `mode` is `"todo"` for every non-build-script unit in cargo 1.95 — a
    cargo-side placeholder, not a parse failure. It is recorded verbatim anyway;
    a column that is useless today and correct tomorrow costs one key.
    """
    text = html.read_text(errors="replace")
    match = _UNIT_DATA.search(text)
    if not match:
        raise SystemExit(
            f"⛔ {html} has no `const UNIT_DATA` — that is what a cargo timing "
            "report is, so either this is not one or cargo's report format moved."
        )
    units = json.loads(match.group(1))
    head = dict(_HEAD_ROW.findall(text[: match.start()]))

    def head_int(key: str) -> int | None:
        try:
            return int(head.get(key, "").strip())
        except ValueError:
            return None

    rustc = re.sub(r"<br>.*", "", head.get("rustc", "")).strip() or None
    build = {
        "run_id": run_id or uuid.uuid4().hex[:12],
        "source": str(html),
        "profile": (head.get("Profile") or "").strip() or None,
        "targets": (head.get("Targets") or "").strip() or None,
        "build_started_at": (head.get("Build start") or "").strip() or None,
        "total_seconds": (head.get("Total time") or "").strip() or None,
        "fresh_units": head_int("Fresh units"),
        "dirty_units": head_int("Dirty units"),
        "total_units": head_int("Total units"),
        "max_concurrency": (head.get("Max concurrency") or "").strip() or None,
        "rustc": rustc,
        "label": label,
    }

    dirs = workspace_dirs()
    lines_now = {name: crate_lines(path)["lines"] for name, path in dirs.items()}
    profiles = package_opt_levels(profile)
    shared = envelope("unit", run_id=build["run_id"], label=label)
    shared.update(extra or {})

    # ⚠ **`backfilled` exists because LOC is read at INGEST, not at build.** A
    # report ingested in the commit that produced it has honest `lines` and
    # `commit` columns; one ingested a day later describes a tree the build never
    # saw. Rather than silently mixing the two, the row says which it is, so a
    # regression of seconds against lines can drop the rows that would poison it.
    # Derived rather than asked for: a flag nobody remembers to pass is a column
    # that is wrong exactly when it matters.
    #
    # ⛔ **the timestamp heuristic BREAKS when another session is committing**,
    # which is not hypothetical: on 2026-08-08 a parallel agent landed five
    # `docs/planning/` commits while a collection was running, so HEAD's commit
    # time moved past the build's start and every row would have been marked a
    # backfill despite no `.rs` file having changed. A caller that built the tree
    # itself KNOWS the answer and passes it in `extra`; the heuristic is the
    # fallback for a hand ingest, where it is still right.
    head_epoch = git("log", "-1", "--format=%cI")
    if "backfilled" not in shared:
        shared["backfilled"] = bool(
            build["build_started_at"] and head_epoch
            and build["build_started_at"] < head_epoch
        )
    if shared["backfilled"] and record:
        print(f"⚠ this report predates HEAD ({build['build_started_at']} < "
              f"{head_epoch}); rows are marked backfilled=true and their `lines` "
              f"column describes the tree NOW, not the tree that was built.")

    # ⭐ **the DAG is the dimension that turns durations into a critical path**,
    # and it is only in the report. `unblocked_rmeta_units` are the successors
    # cargo released when this unit's METADATA appeared — rustc's pipelined
    # compilation — and `unblocked_units` are the ones that had to wait for the
    # whole unit. The difference is the entire reason a build whose "serial
    # chain" sums to 242s can finish in 188s: on a pipelined edge only the
    # FRONTEND is serial and the successor's work overlaps the predecessor's
    # codegen. Recorded as unit NAMES rather than the report's local indices,
    # which mean nothing once the row leaves the file.
    def label_of(index: int) -> str:
        other = by_index.get(index)
        if not other:
            return f"?{index}"
        target = (other.get("target") or "").strip()
        return f"{other['name']}{' ' + target.split()[0] if target else ''}"

    by_index = {unit["i"]: unit for unit in units}

    rows: list[dict] = []
    for unit in units:
        if not unit.get("duration"):
            continue
        name = unit["name"]
        sections = {key: value for key, value in (unit.get("sections") or [])}
        rows.append(
            {
                **shared,
                "unit": name,
                "version": unit.get("version"),
                "target": (unit.get("target") or "").strip(),
                "mode": unit.get("mode"),
                "first_party": name in dirs,
                # ⚠ LOC is read at INGEST time, not at build time. It is right
                # when the report is ingested in the commit that produced it and
                # drifts otherwise, so `commit` is the column that makes it
                # trustworthy — join on it, never on `unit` alone.
                "lines": lines_now.get(name),
                "opt_level": profiles.get(
                    name,
                    profiles["_workspace_default"]
                    if name in dirs
                    else profiles["_dependency_default"],
                ),
                "seconds": round(unit["duration"], 3),
                "start_seconds": round(unit.get("start", 0.0), 3),
                "frontend_seconds": round(
                    sections.get("frontend", {}).get("end", 0)
                    - sections.get("frontend", {}).get("start", 0),
                    3,
                )
                or None,
                "codegen_seconds": round(
                    sections.get("codegen", {}).get("end", 0)
                    - sections.get("codegen", {}).get("start", 0),
                    3,
                )
                or None,
                "features": unit.get("features") or [],
                # ⚠ successors released at RMETA vs at COMPLETION. A unit that
                # emits no metadata — a proc-macro, a build script, a bin, a
                # test, or a lib declaring a `cdylib` — appears here with an
                # empty `unblocks_at_rmeta` and is also the unit whose
                # `frontend_seconds`/`codegen_seconds` are null. Same cause.
                "unblocks_at_rmeta": [
                    label_of(i) for i in (unit.get("unblocked_rmeta_units") or [])
                ],
                "unblocks_at_completion": [
                    label_of(i) for i in (unit.get("unblocked_units") or [])
                ],
                **{f"build_{k}": v for k, v in build.items() if k != "run_id"},
                **(unit_extra(unit) if unit_extra else {}),
            }
        )

    if record:
        measurement_paths.require_writable(UNIT_LEDGER)
        UNIT_LEDGER.parent.mkdir(parents=True, exist_ok=True)
        with UNIT_LEDGER.open("a", encoding="utf-8") as handle:
            for row in rows:
                handle.write(json.dumps(row, sort_keys=True) + "\n")
        print(f"appended {len(rows)} unit row(s) to {UNIT_LEDGER.relative_to(ROOT)}")
        print(f"  file://{UNIT_LEDGER}\n  file://{UNIT_LEDGER.parent}")
    return rows


# Cargo's own defaults when a profile table says nothing, plus which profile
# each one inherits from. `test` is `dev` and `bench` is `release`, so a
# `cargo test --no-run` build applies `[profile.dev.package.*]` and a
# `--release` one applies nothing this repo writes — `Cargo.toml` has no
# `[profile.release]` table at all.
_PROFILE_BASE = {"dev": "dev", "test": "dev", "release": "release", "bench": "release"}
_PROFILE_DEFAULT_OPT = {"dev": 0, "release": 3}


def package_opt_levels(profile: str = "dev") -> dict[str, str]:
    """`opt-level` per package for a profile, from `Cargo.toml`.

    ⚠ **this is the MODEL, and the collector does not use it.**
    `scripts/compile_collect.py` reads the level off the rustc command line
    cargo printed, because the model below has been wrong once already. This is
    the fallback for ingesting a report by hand, and rows say which they got via
    `opt_level_source`.

    Recorded per unit because it is the dimension Jon named that nothing else
    captures, and because it is genuinely non-uniform here: `runtime`, `render`
    and `app` are pinned to 0 while every other workspace crate inherits 1, and
    dependencies get 3. A ledger storing one opt-level for a whole build would
    be wrong for three of the most expensive crates in it.

    ⛔ **`[profile.dev.package."*"]` DOES NOT APPLY TO WORKSPACE MEMBERS**, and
    the first draft of this function got it backwards — it reported the monolith
    at opt-level 3 when it builds at 1. The repo's own `Cargo.toml` states the
    rule in prose ("the `package."*"` glob above applies to dependencies only")
    and a ledger that contradicts its own manifest is worse than one with the
    column missing. Two defaults, keyed on membership; the caller says which.
    """
    import tomllib

    base = _PROFILE_BASE.get(profile, profile)
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    dev = manifest.get("profile", {}).get(base, {})
    packages = dev.get("package") or {}
    fallback = _PROFILE_DEFAULT_OPT.get(base, 0)
    levels = {
        "_workspace_default": str(dev.get("opt-level", fallback)),
        "_dependency_default": str((packages.get("*") or {}).get(
            "opt-level", dev.get("opt-level", fallback))),
    }
    for name, table in packages.items():
        if name != "*" and "opt-level" in table:
            levels[name] = str(table["opt-level"])
    return levels


# ---------------------------------------------------------------------------
# lineage (D9)
# ---------------------------------------------------------------------------


def record_carve(args: argparse.Namespace) -> int:
    """Append one carve-lineage row.

    ⛔ **this is the only dimension with NO other source.** `git log --follow`
    approximates a file move and gives up entirely on a module that was split
    across two new homes, and neither git nor cargo records WHY. A carve knows
    what it split from at the moment it splits and never again, so the record
    has to be written then. Rows are appended by the carve's own commit.

    Deliberately NOT back-filled for carves that already happened: a
    reconstructed lineage that reads like a recorded one is worse than a gap,
    because the next reader cannot tell which is which.
    """
    # Measured on the DESTINATION: `lines_at_split` is how much code landed in
    # the new home, which is the number a carve is judged on. Measuring the
    # source would report whatever is left behind — the first draft did, and
    # recorded 233 for a 2,167-line move.
    lines = 0
    destination = ROOT / args.destination
    if destination.exists():
        files = (
            sorted(destination.rglob("*.rs")) if destination.is_dir() else [destination]
        )
        lines = sum(sum(1 for _ in path.open("rb")) for path in files)
    row = {
        **envelope("carve"),
        "from_path": args.origin,
        "to_path": args.destination,
        "from_crate": args.from_crate,
        "to_crate": args.to_crate,
        "lines_at_split": args.lines if args.lines is not None else lines or None,
        "why": args.why,
        # ⚠ "live" means the carve's own commit wrote this row and the numbers
        # were true when it did. Anything else NAMES where the claim came from,
        # so a reader can tell a recorded lineage from a transcribed one without
        # having to trust that they are the same. `happened_in` is the carve's
        # commit; `commit` in the envelope is when the ROW was written, and for a
        # live row they are the same.
        "recorded_from": args.recorded_from,
        "happened_in": args.happened_in,
    }
    measurement_paths.require_writable(LINEAGE)
    LINEAGE.parent.mkdir(parents=True, exist_ok=True)
    with LINEAGE.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(row, sort_keys=True) + "\n")
    print(f"recorded: {row['from_path']} -> {row['to_path']}")
    print(f"  file://{LINEAGE}\n  file://{LINEAGE.parent}")
    return 0


# ---------------------------------------------------------------------------


def freeze(current: dict) -> None:
    # ⛔ checked BEFORE the baseline is written, not between the two writes. This
    # function makes TWO records — the gate's baseline in the parent repo and a
    # trend row in the submodule's ledger — and a freeze that lands the first and
    # loses the second leaves a guarded number with no snapshot behind it.
    measurement_paths.require_writable(GRAPH_LEDGER)

    frozen = {**envelope("graph"), **current, "headroom_fraction": HEADROOM_FRACTION}
    frozen["recorded_at"] = time.strftime("%Y-%m-%dT%H:%M:%S%z")
    BASELINE.parent.mkdir(parents=True, exist_ok=True)
    BASELINE.write_text(json.dumps(frozen, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    row = {k: v for k, v in frozen.items() if k != "crates"}
    row["crate_lines"] = {n: e["lines"] for n, e in frozen["crates"].items()}
    with GRAPH_LEDGER.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(row, sort_keys=True) + "\n")

    print(f"froze {BASELINE.relative_to(ROOT)} and appended a snapshot to "
          f"{GRAPH_LEDGER.relative_to(ROOT)}")
    print(f"  file://{BASELINE}\n  file://{GRAPH_LEDGER}\n  file://{BASELINE.parent}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--report-only", action="store_true",
                        help="print the numbers and exit 0 even on a violation")
    parser.add_argument("--update", action="store_true",
                        help="re-freeze the baseline and append a graph snapshot")
    parser.add_argument("--diff", action="store_true",
                        help="per-crate attribution against the frozen baseline")
    parser.add_argument("--carve", metavar="PATH",
                        help="simulate lifting a module into its own crate")
    parser.add_argument("--new-crate", metavar="NAME",
                        help="name for the simulated crate (default ambition_<module>)")
    parser.add_argument("--new-crate-rate", type=float, metavar="MS_PER_LINE",
                        help="assumed compile density of the simulated crate "
                             "(default: the population median, which small "
                             "consumer crates measure far above)")
    parser.add_argument("--ingest-timings", metavar="HTML",
                        help="append per-unit rows from a `cargo build --timings` report")
    parser.add_argument("--label", default="", help="free-text tag for an ingested run")
    parser.add_argument("--record-carve", action="store_true",
                        help="append a lineage row; needs --from and --to")
    parser.add_argument("--from", dest="origin", metavar="PATH",
                        help="repo-relative path the code came FROM")
    parser.add_argument("--to", dest="destination", metavar="PATH",
                        help="repo-relative path the code went TO")
    parser.add_argument("--from-crate", help="crate the code left")
    parser.add_argument("--to-crate", help="crate the code joined")
    parser.add_argument("--lines", type=int, help="lines moved, if the paths cannot say")
    parser.add_argument("--why", default="", help="one sentence: why this split happened")
    parser.add_argument("--recorded-from", default="live",
                        help="'live' (the carve's own commit) or a path/citation "
                             "for a lineage transcribed from an existing record")
    parser.add_argument("--happened-in", help="the carve's commit, if not this one")
    args = parser.parse_args(argv)

    if args.record_carve:
        if not (args.origin and args.destination):
            raise SystemExit("⛔ --record-carve needs --from and --to")
        return record_carve(args)

    if args.ingest_timings:
        ingest_timings(Path(args.ingest_timings), label=args.label)
        return 0

    if args.carve:
        simulate_carve(Path(args.carve), args.new_crate, args.new_crate_rate)
        return 0

    if args.update:
        # ⚠ the ONLY path that re-reads the ledger. Everything else prices the
        # graph with the weights the baseline froze, so appending a build's rows
        # cannot turn the gate red on a tree nobody touched.
        freeze(snapshot())
        return 0

    if not BASELINE.exists():
        raise SystemExit(
            f"⛔ {BASELINE.relative_to(ROOT)} is missing. Run `--update` to freeze "
            "today's numbers, and commit it — a ratchet with no baseline is a "
            "program that cannot fail."
        )
    frozen = json.loads(BASELINE.read_text(encoding="utf-8"))
    current = snapshot(weights=frozen.get("unit_weights"))

    if args.diff:
        diff(current, frozen)
        return 0

    findings = evaluate(current, frozen)
    report(current, frozen)
    if not findings:
        print("\n  ok   every guarded compile-cost number is inside its budget.")
        return 0

    print()
    for severity, message in findings:
        print(f"  {severity:<10} {message}\n")
    print(f"{len(findings)} compile-cost finding(s). "
          f"`python3 {Path(__file__).relative_to(ROOT)} --diff` says which crate moved.")
    return 0 if args.report_only else 1


if __name__ == "__main__":
    sys.exit(main())
