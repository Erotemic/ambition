#!/usr/bin/env python3
"""**WHERE THIS REPO KEEPS ITS MEASUREMENT LEDGERS — declared once.**

The five append-only telemetry ledgers live in `dev/ambition_dev_measurements`,
a git submodule (`https://github.com/Erotemic/ambition_dev_measurements.git`),
because they grow monotonically and every checkout pays for the growth. On
2026-08-08 those five files were **3.88 MB of a 43.95 MB tracked tree**, and a
cost ledger only ever gets bigger — its history IS its value, so there is no
pruning policy that would fix this. A submodule keeps the whole history without
charging every clone, worktree and CI checkout for it.

⛔ **these same five paths used to be declared THREE times.**
`scripts/compile_ratchet.py`, `scripts/compile_report.py` and
`scripts/compile_cost.py` each spelled them out independently, and
`scripts/run_tests.py` inlined a fourth copy. That is the *two readers of one
declaration, never two declarations* defect: relocating the directory means
finding every copy, and the copy that gets missed goes on writing to the old
place while everything else reads the new one — both halves correct, disagreeing,
with nothing in either output saying so. Same shape as
`scripts/lib/asset_roots.sh`, same fix: the consumer declares, and the tools are
told.

## What happens when the submodule is absent, and why writers must refuse

⛔ **a clone without `--recursive` still HAS `dev/ambition_dev_measurements/`.**
Git materialises the mount point as an EMPTY DIRECTORY. So every
`open(..., "a")` in this repo would succeed there: a collector would CREATE
`compile_units.jsonl`, write a real measurement into it, print a `file://` link,
and exit 0. That file is then an untracked stray inside an uninitialised
submodule — removed without comment by the next `git submodule update --init`,
never seen by `git status` in the parent, and never in the ledger it was meant
for. A measurement that is silently discarded is worse than one never taken,
because the run that took it reported success.

⭐ So **every writer calls `require_writable()` before opening a ledger for
append**, and readers do not:

* a READER on an absent submodule sees files that do not exist, which is exactly
  the fresh-clone case `scripts/compile_report.py` already renders as
  *"has no rows"*. Hard-failing there would break a page that is designed to be
  honest about thin data.
* a WRITER has nowhere to put its row and must say so, naming the one command
  that fixes it.

⚠ **`dev/compile_ratchet_baseline.json` deliberately did NOT move** and is
declared here so that the reason travels with the paths. It is a GATE INPUT —
`compile_ratchet.py --check` reads it on every run — and a gate whose baseline
sits behind an uninitialised submodule cannot run at all. It is also bounded: one
frozen snapshot, rewritten rather than appended, so it never had the problem the
ledgers have.

⚠ **this module imports nothing but the standard library and must stay that
way.** `scripts/run_tests.py` is the suite's own entry point and cannot afford to
pull in a 1,500-line checker in order to learn a file path.
"""

from __future__ import annotations

from pathlib import Path

#: `<repo>/scripts/lib/measurement_paths.py` -> `<repo>`.
REPO = Path(__file__).resolve().parents[2]

#: Repo-relative path of the submodule, as `.gitmodules` spells it. Used in the
#: fix instruction so the message can be pasted verbatim.
SUBMODULE = "dev/ambition_dev_measurements"

#: The directory the ledgers live in. ⚠ THE ONE LINE TO EDIT if it moves again.
MEASUREMENTS = REPO / SUBMODULE

#: The command that turns an empty mount point into a working checkout.
INIT_COMMAND = f"git submodule update --init {SUBMODULE}"

UNIT_LEDGER = MEASUREMENTS / "compile_units.jsonl"
GRAPH_LEDGER = MEASUREMENTS / "compile_graph.jsonl"
SCENARIO_LEDGER = MEASUREMENTS / "compile_cost.jsonl"
JOBS_LEDGER = MEASUREMENTS / "run_tests_cost.jsonl"
CARVE_LEDGER = MEASUREMENTS / "carve_lineage.jsonl"

#: Every ledger, keyed by the stem the schema doc and the report use. The five
#: are enumerated ONCE; a reader that wants "all of them" asks for this.
LEDGERS: dict[str, Path] = {
    "compile_units": UNIT_LEDGER,
    "compile_graph": GRAPH_LEDGER,
    "compile_cost": SCENARIO_LEDGER,
    "run_tests_cost": JOBS_LEDGER,
    "carve_lineage": CARVE_LEDGER,
}

#: ⛔ NOT a ledger and NOT in the submodule — see the module docstring. A gate
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
