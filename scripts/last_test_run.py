#!/usr/bin/env python3
"""What did the last `run_tests.py` actually say — and is that answer FRESH?

⛔ **This exists because the status file reported a run that never happened.**
Twice on 2026-08-04 a `run_tests.py` invocation failed before it started (a shell
left inside the renderer submodule, so `scripts/run_tests.py` resolved to a path
that does not exist, exit 2). The one-liner used to read the result —
`json.load(...)["completed"]` — printed the PREVIOUS run's per-job table, and
both times that table said 5/5 green.

⚠ The near-miss was caught by noticing the timings were byte-identical to the run
before. That is luck, not method. A reader that cannot tell a fresh result from
an old one belongs to the same family as a check that cannot fail: it reports the
success condition regardless of what happened.

⭐ **The writer was never the problem.** `run_tests.py` already stamps `pid` and
`started` into the payload. The gap was that no reader existed, so every caller
hand-rolled the query and left the freshness check out — and a fact everyone
re-derives is a fact someone will derive wrong.

So: this is the reader, the exit code is the answer, and it REFUSES rather than
reports when it cannot vouch for what it found.

    python3 scripts/last_test_run.py            # verdict, or a refusal
    python3 scripts/last_test_run.py --max-age 5

Exit codes: 0 every job passed; 1 a job failed; 2 the answer is not trustworthy
(stale, unfinished, missing, or a dead run still marked running).
"""

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

    failed = [job.get("job") for job in jobs if not job.get("ok")]
    if failed:
        print(f"\n{len(failed)} job(s) FAILED: {', '.join(str(f) for f in failed)}")
        return 1
    print(f"\nall {len(jobs)} jobs passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
