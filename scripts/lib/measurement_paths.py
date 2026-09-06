#!/usr/bin/env python3
"""Shared paths for compile and test telemetry.

Append-only ledgers live in the `dev/ambition_dev_measurements` submodule.
Readers may treat an uninitialized submodule as having no rows; writers must call
`require_writable()` before appending so measurements are never written into an
empty submodule mount and later discarded.

`dev/compile_ratchet_baseline.json` remains in the main repository because it is
a bounded gate input required by the compile ratchet. This module intentionally
uses only the standard library so suite entry points can import it cheaply."""

from __future__ import annotations

from pathlib import Path

#: `<repo>/scripts/lib/measurement_paths.py` -> `<repo>`.
REPO = Path(__file__).resolve().parents[2]

#: Repo-relative path of the submodule, as `.gitmodules` spells it. Used in the
#: fix instruction so the message can be pasted verbatim.
SUBMODULE = "dev/ambition_dev_measurements"

#: The directory the ledgers live in. THE ONE LINE TO EDIT if it moves again.
MEASUREMENTS = REPO / SUBMODULE

#: The command that turns an empty mount point into a working checkout.
INIT_COMMAND = f"git submodule update --init {SUBMODULE}"

UNIT_LEDGER = MEASUREMENTS / "compile_units.jsonl"
GRAPH_LEDGER = MEASUREMENTS / "compile_graph.jsonl"
SCENARIO_LEDGER = MEASUREMENTS / "compile_cost.jsonl"
JOBS_LEDGER = MEASUREMENTS / "run_tests_cost.jsonl"
CARVE_LEDGER = MEASUREMENTS / "carve_lineage.jsonl"
#: Runtime frame cost, one row per profiling bundle.
#: `scripts/lib/profile_bundle_to_history.py` writes it,
#: `scripts/perf_history.py` reads it.
#:
#: ⛔ DELIBERATELY NOT IN `LEDGERS` below. That dict is the compile/test
#: telemetry set, and `scripts/compile_report.py` renders every member of it
#: on a page about compile cost. A runtime frame time shares the envelope and
#: nothing else; putting it there would make the compile report claim to
#: explain it.
RUNTIME_LEDGER = MEASUREMENTS / "runtime_frame_cost.jsonl"

#: The compile/test telemetry ledgers, keyed by the stem the schema doc and the
#: report use. The five are enumerated ONCE; a reader that wants "all of them"
#: asks for this.
#:
#: ⚠ `scripts/compile_report.py` keeps a hand-written table of one row per
#: member and indexes it by these keys. ADDING A MEMBER HERE WITHOUT ADDING A
#: ROW THERE IS A KeyError, not a missing section.
LEDGERS: dict[str, Path] = {
    "compile_units": UNIT_LEDGER,
    "compile_graph": GRAPH_LEDGER,
    "compile_cost": SCENARIO_LEDGER,
    "run_tests_cost": JOBS_LEDGER,
    "carve_lineage": CARVE_LEDGER,
}

#: NOT a ledger and NOT in the submodule — see the module docstring. A gate
#: input stays where the gate can always reach it.
RATCHET_BASELINE = REPO / "dev" / "compile_ratchet_baseline.json"


def submodule_reason() -> str | None:
    """Why the measurements submodule cannot be written to, or `None` if it can.

    An initialised submodule has a `.git` inside it — a FILE holding
    `gitdir: ../../.git/modules/…` for a normal `submodule update`, or a
    directory if somebody cloned the measurements repo standalone. Either is
    proof that a real checkout is mounted here. An empty directory with no
    `.git` is git's mount point and nothing more.

    ⛔ do not weaken this to "the directory exists" or "a ledger exists": the
    empty mount point exists in every non-recursive clone, and the very first
    thing a writer does is create the ledger it is looking for.
    """
    if (MEASUREMENTS / ".git").exists():
        return None
    if not MEASUREMENTS.exists():
        return f"{SUBMODULE}/ does not exist"
    return (
        f"{SUBMODULE}/ is an uninitialised submodule mount point "
        f"(no .git inside it)"
    )


def unavailable_reason(path: Path | str) -> str | None:
    """Why appending to `path` would be lost, or `None` if it is safe.

    Paths OUTSIDE the submodule are always safe: `run_tests.py` honours
    `RUN_TESTS_COST_LEDGER`, and a caller that redirected the ledger somewhere
    else has taken the question away from us.
    """
    # Resolved on both branches: `MEASUREMENTS` is built from a resolved
    # `__file__`, so an unresolved argument would compare against a different
    # spelling of the same directory and quietly report "safe".
    target = Path(path)
    target = (target if target.is_absolute() else REPO / target).resolve()
    if not target.is_relative_to(MEASUREMENTS):
        return None
    return submodule_reason()


def require_writable(path: Path | str) -> Path:
    """Refuse to append to `path` unless the submodule is really checked out.

    Raises `SystemExit` — these are all command-line tools, and the operator
    needs the fix, not a traceback.
    """
    reason = unavailable_reason(path)
    if reason is not None:
        raise SystemExit(
            f"⛔ refusing to append to {path}\n"
            f"   {reason}.\n"
            f"   Writing anyway would create a stray file inside an "
            f"uninitialised submodule mount, where the next\n"
            f"   `git submodule update` deletes it and `git status` never "
            f"mentions it.\n"
            f"   Fix: {INIT_COMMAND}"
        )
    return Path(path)
