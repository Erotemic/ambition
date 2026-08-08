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

## The four guarded numbers, and why these four

Each catches a regression class the other three cannot see. That is the whole
selection rule; a fifth number that reddens on the same event as one of these is
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

## What is deliberately NOT guarded here

* **The set of crates a small consumer links.** Already guarded, by
  `capability-footprint-may-not-grow` in `check_absence_contracts.py`, against
  `fixtures/minimal_game`. A second guard on one fact is how a suite starts
  getting waived.
* **Wall-clock anything.** It lives in `dev/compile_cost.jsonl` and
  `dev/compile_units.jsonl`, is plotted, and is never a gate. See above.

## ⛔ There is no `--check` flag, and that is deliberate

This repo has been bitten twice by an optional `--check`: a guard that prints
its findings and exits 0 unless somebody remembered a flag is green by
construction. `check_roadmap_evidence.py` ran that way for its whole life and
caught nothing (fixed 2026-08-07). So a violation here exits 1 by default, and
the flag that makes it advisory is `--report-only`, which has to be typed.

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
import subprocess
import sys
import time
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from check_absence_contracts import cargo_binary, strip_comments_for  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]

BASELINE = ROOT / "dev" / "compile_ratchet_baseline.json"
GRAPH_LEDGER = ROOT / "dev" / "compile_graph.jsonl"
UNIT_LEDGER = ROOT / "dev" / "compile_units.jsonl"
LINEAGE = ROOT / "dev" / "carve_lineage.jsonl"

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
    collector does.** `dev/run_tests_cost.jsonl` has 75 rows and no commit, no
    machine, no profile and no opt-level; `dev/compile_cost.jsonl` has 4 rows
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


def snapshot(
    consumer: str = CONSUMER,
    *,
    override_lines: dict[str, int] | None = None,
    extra_edges: dict[str, set[str]] | None = None,
) -> dict:
    """Every number this file guards, plus the per-crate table behind them.

    `override_lines` and `extra_edges` exist so `--carve` can ask the same
    question of a graph that does not exist yet. A simulator that reimplements
    the metric is a simulator that answers a different question.
    """
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

    table: dict[str, dict] = {}
    for name in sorted(first_party):
        closure = dependents(name)
        table[name] = {
            **{k: v for k, v in measured.get(name, {}).items()},
            "lines": line_count(name),
            "edit_cost_lines": sum(line_count(x) for x in closure),
            "edit_cost_crates": len(closure),
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

    return {
        "schema": SCHEMA,
        "kind": "graph",
        "consumer": consumer,
        "line_unit": "physical lines of <crate>/src/**/*.rs, inline #[cfg(test)] included",
        "first_party_crates": len(first_party),
        "first_party_lines": sum(table[n]["lines"] for n in table),
        "largest_unit": {"crate": largest[0], "lines": largest[1]["lines"]},
        "worst_edit_cost": {
            "crate": worst[0],
            "lines": worst[1]["edit_cost_lines"],
            "crates": worst[1]["edit_cost_crates"],
        },
        "watched_edit_cost": {
            name: {
                "lines": table[name]["edit_cost_lines"],
                "crates": table[name]["edit_cost_crates"],
            }
            for name in WATCHED
            if name in table
        },
        "critical_path_crates": max(entry["depth"] for entry in table.values()),
        "crates": table,
    }


# ---------------------------------------------------------------------------
# the gate
# ---------------------------------------------------------------------------


def _compare(label: str, current: int, frozen: int, headroom: int) -> tuple[str, str] | None:
    """`(severity, message)` for one number, or None when it is inside budget."""
    if current > frozen + headroom:
        return (
            "REGRESSED",
            f"{label}: {frozen:,} -> {current:,} (+{current - frozen:,}, budget "
            f"+{headroom:,}). Something got bigger or grew a dependency edge. If "
            f"this is a deliberate landing, say so and re-freeze; if it is a "
            f"module that belongs in its own crate, that is the finding.",
        )
    if current < frozen - headroom:
        return (
            "CARVED",
            f"{label}: {frozen:,} -> {current:,} ({current - frozen:,}). This is a "
            f"WIN and the baseline is now stale — re-freeze it in this commit "
            f"(`--update`), or the guard is holding {frozen - current:,} lines of "
            f"slack and the next regression that size lands silently.",
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

    order = {"REGRESSED": 0, "PATH": 1, "MOVED": 2, "GONE": 3, "CARVED": 4}
    findings.sort(key=lambda item: order.get(item[0], 9))
    return findings


def report(current: dict, frozen: dict) -> None:
    print(f"  consumer                {current['consumer']}  "
          f"({current['first_party_crates']} first-party crates, "
          f"{current['first_party_lines']:,} lines)")
    print(f"  largest_unit_lines      {current['largest_unit']['lines']:>9,}  "
          f"{current['largest_unit']['crate']}")
    print(f"  worst_edit_cost_lines   {current['worst_edit_cost']['lines']:>9,}  "
          f"{current['worst_edit_cost']['crate']} "
          f"({current['worst_edit_cost']['crates']} crates)")
    for name, entry in current["watched_edit_cost"].items():
        print(f"  edit_cost_lines         {entry['lines']:>9,}  {name} "
              f"({entry['crates']} crates)")
    print(f"  critical_path_crates    {current['critical_path_crates']:>9}  "
          f"longest serial chain")
    print(f"\n  baseline frozen at {frozen.get('commit', '?')} "
          f"({frozen.get('recorded_at', '?')}), "
          f"headroom {frozen.get('headroom_fraction', HEADROOM_FRACTION):.0%}")


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
        if moved or cost:
            rows.append((abs(moved), moved, cost, name, bool(then), bool(now)))
    if not rows:
        print("  nothing moved.")
        return
    print(f"  {'lines':>9}  {'edit cost':>10}  crate")
    for _, moved, cost, name, was, now in sorted(rows, reverse=True):
        tag = "" if (was and now) else ("  [NEW CRATE]" if now else "  [GONE]")
        print(f"  {moved:>+9,}  {cost:>+10,}  {name}{tag}")


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


def simulate_carve(module: Path, new_crate: str | None = None) -> None:
    module = module.resolve()
    if not module.exists():
        raise SystemExit(f"⛔ {module} does not exist")
    crate_dir = module
    while not (crate_dir / "Cargo.toml").exists() and crate_dir != crate_dir.parent:
        crate_dir = crate_dir.parent
    owner = crate_dir.name
    new_crate = new_crate or f"ambition_{module.stem if module.is_file() else module.name}"

    inward, outward, lines = module_coupling(module)
    before = snapshot()
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
    after = snapshot(override_lines=override, extra_edges=extra)

    print(f"CARVE SIMULATION  {module.relative_to(ROOT)}")
    print(f"  owner crate            {owner}")
    print(f"  proposed crate         {new_crate}  ({lines:,} lines)")
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
    LINEAGE.parent.mkdir(parents=True, exist_ok=True)
    with LINEAGE.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(row, sort_keys=True) + "\n")
    print(f"recorded: {row['from_path']} -> {row['to_path']}")
    print(f"  file://{LINEAGE}\n  file://{LINEAGE.parent}")
    return 0


# ---------------------------------------------------------------------------


def freeze(current: dict) -> None:
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
        simulate_carve(Path(args.carve), args.new_crate)
        return 0

    current = snapshot()

    if args.update:
        freeze(current)
        return 0

    if not BASELINE.exists():
        raise SystemExit(
            f"⛔ {BASELINE.relative_to(ROOT)} is missing. Run `--update` to freeze "
            "today's numbers, and commit it — a ratchet with no baseline is a "
            "program that cannot fail."
        )
    frozen = json.loads(BASELINE.read_text(encoding="utf-8"))

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
