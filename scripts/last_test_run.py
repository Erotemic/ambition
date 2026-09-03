#!/usr/bin/env python3
"""Read the last `run_tests.py` result and reject stale answers.

Exit status is the verdict: 0 when every job passed, 1 when a job failed, and 2
when the status is missing, unfinished, stale, or claims a dead process is still
running.

Usage::

    python3 scripts/last_test_run.py
    python3 scripts/last_test_run.py --max-age 5"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
STATUS = REPO / "target" / "run_tests_status.json"

#: How old a run may be and still answer "how is the suite doing?".
#:
#: this is a claim about RELEVANCE, not correctness. A 40-minute-old green run
#: is a true statement about a tree that has probably moved; the point is to make
#: the reader say so instead of implying it is current.
DEFAULT_MAX_AGE_MIN = 30.0


def alive(pid: int) -> bool:
    """Is that process still around? Signal 0 tests existence without sending."""
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        # Owned by somebody else, therefore alive.
        return True
    return True


#: Extensions whose change invalidates a suite result.
SOURCE_SUFFIXES = (".rs", ".py", ".toml", ".ron")


def newest_source(root: Path) -> tuple[str | None, float | None]:
    """The most recently modified tracked source file, and when.

    ⚠ **git's file list, not a walk.** A walk sees `target/` (rewritten by the
    very run being judged, so every result would look stale) and the renderer's
    gitignored `generated/` tree. What invalidates a verdict is a change to
    SOURCE, which is exactly what git tracks.
    """
    import subprocess

    try:
        out = subprocess.run(
            ["git", "ls-files", "-z"],
            cwd=root,
            capture_output=True,
            check=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError):
        # No git, no claim — the age check below is what is left.
        return None, None
    newest_name: str | None = None
    newest_at: float | None = None
    for raw in out.split(b"\0"):
        if not raw:
            continue
        name = raw.decode("utf-8", "replace")
        if not name.endswith(SOURCE_SUFFIXES):
            continue
        try:
            at = (root / name).stat().st_mtime
        except OSError:
            continue
        if newest_at is None or at > newest_at:
            newest_at, newest_name = at, name
    return newest_name, newest_at


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--max-age",
        type=float,
        default=DEFAULT_MAX_AGE_MIN,
        help=f"minutes before a finished run is called stale (default {DEFAULT_MAX_AGE_MIN:g})",
    )
    ap.add_argument("--status-json", default=str(STATUS))
    args = ap.parse_args()

    path = Path(args.status_json)
    if not path.exists():
        print(f"REFUSED: no status at {path} — no run has written one here.")
        return 2
    try:
        status = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        print(f"REFUSED: could not read {path}: {exc}")
        return 2

    started = status.get("started")
    age_min = (time.time() - started) / 60.0 if started else None
    state = status.get("state", "<unstated>")
    jobs = status.get("completed", [])

    stamp = f"{age_min:.1f} min old" if age_min is not None else "no start time"
    print(f"state={state}  jobs={len(jobs)}/{status.get('jobs', '?')}  ({stamp})")
    for job in jobs:
        mark = "ok  " if job.get("ok") else "FAIL"
        print(f"  {mark} {job.get('job')}  {job.get('seconds', 0):.1f}s")

    if state == "running":
        pid = status.get("pid")
        if pid and not alive(pid):
            print(
                f"REFUSED: marked running, but pid {pid} is gone — that run died "
                "without writing a verdict. Nothing here is a result."
            )
            return 2
        print("a run is IN PROGRESS; this is not a verdict yet.")
        return 2

    # THE CHECK THAT ACTUALLY CATCHES IT, and the age check does not.
    #
    # Being honest about the motivating case: the stale run that fooled me was
    # ~10 minutes old, so any sane `--max-age` would have waved it through. Age
    # measures how long ago, and the question is whether the CODE MOVED SINCE —
    # a result is only a statement about the tree it ran against.
    #
    # I nearly shipped this file with the age check alone, which would have
    # been a guard green through its own motivating case: the exact failure this
    # repo has recorded five times in one day.
    newest, newest_at = newest_source(REPO)
    if started and newest_at and newest_at > started:
        drift = (newest_at - started) / 60.0
        print(
            f"REFUSED: {newest} changed {drift:.1f} min AFTER this run started. "
            "The verdict above describes a tree that no longer exists."
        )
        return 2

    if age_min is not None and age_min > args.max_age:
        print(
            f"REFUSED: this result is {age_min:.1f} min old (limit {args.max_age:g}). "
            "It may predate the code you are asking about — re-run the suite, or "
            "pass --max-age deliberately if you know it is still the tree you mean."
        )
        return 2

    if not jobs:
        print("REFUSED: finished with no jobs recorded — nothing ran.")
        return 2

    # ⛔⛔ A RUN THAT STOPPED EARLY IS NOT A RUN THAT PASSED, and every check
    # above this line would wave it through: the jobs it DID finish are all
    # `ok`, the file is fresh, the tree has not moved. Without this the last
    # line of this script prints `all 31 jobs passed` for a 49-job plan that
    # died of ENOSPC at job 32 — which is the whole failure this file exists to
    # refuse, arriving in the one shape it was not looking for.
    # ⚠ Found 2026-09-03 in review of the between-jobs disk abort: the abort
    # returned 1 to ITS shell but serialized `done`/`0`, and this reader
    # believed the file. Both ends are fixed; this is the end that agents read.
    if state != "done":
        detail = status.get("aborted_on_disk")
        never = status.get("never_ran")
        where = f" before `{detail}`" if detail else ""
        short = f", {never} job(s) never ran" if never else ""
        print(
            f"REFUSED: this run is `{state}`, not `done` — it stopped{where}"
            f"{short}. The jobs listed above really did pass; the SUITE did not "
            "finish, so this is not a verdict on the tree."
        )
        return 2

    failed = [job.get("job") for job in jobs if not job.get("ok")]
    if failed:
        print(f"\n{len(failed)} job(s) FAILED: {', '.join(str(f) for f in failed)}")
        return 1
    print(f"\nall {len(jobs)} jobs passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
