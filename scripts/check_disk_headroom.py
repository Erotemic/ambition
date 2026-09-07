#!/usr/bin/env python3
"""Check free space on the filesystem Cargo actually writes to.

The target directory follows `CARGO_TARGET_DIR`, then `.cargo/config.toml`, then
the repository-local `target/`. The default threshold is sized for a full suite;
callers performing smaller work may request a lower floor.

Usage::

    python3 scripts/check_disk_headroom.py
    python3 scripts/check_disk_headroom.py --min-gb 5
    python3 scripts/check_disk_headroom.py --quiet"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# A floor with a little room, not a precise budget — the point is to refuse BEFORE a job dies of
# ENOSPC and reports it as a compile error.
#
# ⚠ MEASURED COUNTER-EXAMPLE, calculex VM, 2026-09-03: this floor PASSED at 42 GB free and
# `./run_tests.sh --rust` still died of ENOSPC — after all 6957 tests had passed, while writing its
# own status file, so the traceback pointed at `run_tests.py` rather than at the disk. The run
# needed more than 42 GB on top of a `target/` that a day of multi-profile work had grown to 267 GB
# (`target/debug/incremental` alone was 156 GB and is safe to delete).
# ⇒ Two honest readings, and this comment does not pick between them: the floor is low for a full
# nextest + doc-test pass, or a `target/` that large inflates what one costs. Whoever raises it
# should measure a run on a pruned tree first, because the second reading would make a higher floor
# treat a disposable cache as a requirement.
MIN_FREE_GB = 40.0


def target_dir() -> Path:
    """Resolve the Cargo target directory from environment, config, or repo default."""
    if env := os.environ.get("CARGO_TARGET_DIR"):
        return Path(env)
    config = REPO / ".cargo" / "config.toml"
    if config.is_file():
        for line in config.read_text().splitlines():
            line = line.strip()
            if line.startswith("target-dir"):
                _, _, value = line.partition("=")
                value = value.strip().strip('"').strip("'")
                if value:
                    return Path(value)
    return REPO / "target"


BINDMOUNT = os.path.join(REPO, "scripts", "setup", "target_bindmount.sh")


def _bindmount_check_exit_code() -> int | None:
    """Ask the bind-mount script whether `target/` is the volume to build on.

    That script is the one authority for the mount fact; this does not re-derive
    it. Returns its exit code, or `None` when the question could not be ASKED —
    the script missing, not executable, or dying on something other than its own
    verdict.
    """
    if not os.path.isfile(BINDMOUNT):
        return None
    try:
        done = subprocess.run(["bash", BINDMOUNT, "--check"], check=False)
    except OSError:
        return None
    return done.returncode


def require_verified_target_volume() -> None:
    """Refuse unless `target/` is the volume cargo SHOULD be writing to.

    ⭐ THIS IS THE PRECONDITION OF THE NUMBER BELOW, WHICH IS WHY IT LIVES INSIDE
    IT. `free_gb_on_target()` measures whatever `target/` currently resolves to.
    On a virtiofs checkout whose bind mount is absent — the state every reboot
    leaves behind, since the bind does not survive one — that is the SHARED
    volume, so the floor answers about the wrong filesystem and passing it
    licenses the build onto the mount AGENTS.md spends sixty lines forbidding.
    Measured 2026-09-07 on this box after a reboot: unbound, `df` on `target/`
    read 33 GB free on the 1.8 TB shared volume while the intended store had 13.

    ⛔ IT IS NOT A CHECK THE CALLER MUST REMEMBER TO RUN FIRST. It used to be:
    `run_tests.sh` ran `target_bindmount.sh --check` and refused, and
    `scripts/run_tests.py` — the door agents actually use — did not, so whether
    the precondition held depended on which entry point you came through. Making
    it part of the read removes the ordering question rather than testing it: no
    caller can obtain the number from an unverified volume, in any order.

    ⛔ REFUSE RATHER THAN WARN, and refuse here rather than in a doc, because a
    doc is what got skipped: an agent built all day unbound on 2026-08-27, was
    asked to reclaim the space, and deleted 205 GB of the LIVE target instead of
    restoring the mount. A warning at the top of a job that prints for several
    minutes is a warning nobody reads, and the fix is one command.

    ⛔ AN INDETERMINATE ANSWER REFUSES. A probe that could not run is not a
    passing probe; treating "I could not tell" as "fine" is how a guard reports a
    clean tree it never looked at.
    """
    code = _bindmount_check_exit_code()
    if code == 0:
        return
    if code is None:
        print(
            f"REFUSING: could not ask {BINDMOUNT} whether target/ is bound.\n"
            "  A probe that cannot run is not a probe that passed. Fix the "
            "script or the shell before building.",
            file=sys.stderr,
        )
    else:
        # The script has already printed its own diagnosis and remedy to stderr.
        print(
            f"REFUSING: {BINDMOUNT} --check exited {code}. Not building until "
            "target/ is the volume it should be.",
            file=sys.stderr,
        )
    sys.exit(2)


def free_gb_on_target() -> float:
    """Free space on the volume cargo writes to, once that volume is verified.

    Falls back to the repo's own volume only when the target directory's parent
    does not exist yet.
    """
    require_verified_target_volume()
    path = target_dir()
    while not path.exists() and path != path.parent:
        path = path.parent
    return shutil.disk_usage(path).free / 1024**3


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--min-gb",
        type=float,
        default=MIN_FREE_GB,
        help=f"required free GB (default {MIN_FREE_GB}, the full-suite floor)",
    )
    parser.add_argument("--quiet", action="store_true", help="exit code only")
    args = parser.parse_args()

    free = free_gb_on_target()
    where = target_dir()
    if free < args.min_gb:
        print(
            f"REFUSING: {free:.1f} GB free on {where}, need {args.min_gb:.0f} GB.\n\n"
            "⚠ do not start a build. A mid-build ENOSPC does not say 'disk full' — "
            "it surfaces as unrelated compile errors in whichever crate was "
            "unlucky, and the real cause appears nowhere.\n\n"
            # ⛔⛔⛔ THIS USED TO PRINT A DELETION LADDER -- incremental, then
            # the measurement targets, then `cargo clean` -- and AGENTS.md's
            # standing rule forbids every rung of it: "NEVER `rm -rf` anything
            # under a `target/`. NOT `incremental`, NOT `deps`, NOT AS A FAVOUR
            # WHEN THE DISK IS FULL … the reclaim is Jon's call, on Jon's
            # machine, and `cargo clean` is his to run." A refusal message is
            # read at the exact moment someone is under pressure to free space,
            # so it is the LAST place to leave advice that contradicts the rule.
            "  1. scripts/setup/target_bindmount.sh --status\n"
            "     An enormous target/ is almost always an ABSENT BIND, and\n"
            "     repairing it returns the space without deleting anything --\n"
            "     the duplicate underneath was never supposed to exist.\n"
            f"  2. If it is bound and {where} is genuinely full: SAY SO AND STOP.\n"
            "     Report the numbers and hand it to Jon. Do not delete, do not\n"
            "     prune by mtime, do not run `cargo clean` -- that reclaim is\n"
            "     his call on his machine.\n"
            f"  Reading only (safe): du -sh {where}/debug/* | sort -h\n\n"
            "⚠ THE VOLUME IS SHARED with the main checkout and every other agent "
            "worktree, so a number you read here is not yours alone: on "
            "2026-09-03 the main tree held 270 GB and one worktree 99 GB. That "
            "is a reason to report rather than reclaim -- the biggest directory "
            "is usually someone else's live build.",
            file=sys.stderr,
        )
        return 1

    if not args.quiet:
        print(f"OK: {free:.1f} GB free on {where} (floor {args.min_gb:.0f}).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
