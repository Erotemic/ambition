#!/usr/bin/env python3
"""Choose the integration gate required by the current diff.

Only documentation and the append-only measurements submodule may use the small
smoke suite. Any other changed path selects the full integration suite. This is
a whitelist: unknown or newly introduced paths fail toward running more tests,
not fewer.

The smoke modules exist to detect a mistaken classification that broke basic app
composition; they are not intended to cover behavior affected by documentation
changes."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]

#: The ONLY prefix that counts as prose. See the whitelist note above.
PROSE_PREFIXES = ("docs/",)

#: Prose, plus the append-only measurement submodule — everything whose change
#: cannot alter what the suite would report. this is the whole whitelist; a
#: prefix added here is a class of change nobody gates on again.
SKIPPABLE_PREFIXES = PROSE_PREFIXES + ("dev/ambition_dev_measurements",)

#: Named smoke modules, each with the reason it is here. do not add a module
#: because it is fast, and do not remove one because it is slow — the question is
#: only "would this fail if the app stopped composing".
SMOKE_MODULES: dict[str, str] = {
    "composes_through_the_sdk": (
        "the public composition path builds an app at all — if a non-prose "
        "change slipped through the whitelist and broke wiring, this is where "
        "it shows"
    ),
    "direct_and_shell_agree": (
        "the two ways to build the game still agree; one test, and it is the "
        "cheapest proof that neither composition drifted"
    ),
}


def changed_paths(since: str | None) -> list[str]:
    """Every path this turn touched — working tree AND commits since `since`.

    ⚠ **both halves matter.** Uncommitted work is what a Stop hook is usually
    looking at, and committed work is what a turn that already committed left
    behind. Reading only one of them was the first draft's bug: a turn that
    committed a `.rs` change and then edited a doc would have looked prose-only.
    """
    paths: set[str] = set()

    status = subprocess.run(
        ["git", "-C", str(REPO), "status", "--porcelain"],
        capture_output=True, text=True, timeout=60,
    )
    for line in status.stdout.splitlines():
        if len(line) > 3:
            entry = line[3:]
            # a rename reads "old -> new"; both sides count
            for part in entry.split(" -> "):
                paths.add(part.strip().strip('"'))

    if since:
        diff = subprocess.run(
            ["git", "-C", str(REPO), "diff", "--name-only", f"{since}..HEAD"],
            capture_output=True, text=True, timeout=60,
        )
        if diff.returncode == 0:
            paths.update(p for p in diff.stdout.splitlines() if p.strip())

    return sorted(paths)


def is_skippable_only(paths: list[str]) -> bool:
    """True when EVERY changed path is one the full suite cannot have an opinion about.

    ⛔ an empty change set is NOT skippable. "Nothing changed" and "only docs
    changed" are different states, and answering the first with a smoke suite
    would mean a turn that somehow reported no diff silently skips the gate.
    """
    if not paths:
        return False
    return all(p.startswith(SKIPPABLE_PREFIXES) for p in paths)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--since", help="git ref the turn started from (optional)")
    ap.add_argument("--explain", action="store_true",
                    help="print the decision and the paths, run nothing")
    args = ap.parse_args(argv)

    paths = changed_paths(args.since)
    skippable = is_skippable_only(paths)

    if skippable:
        filters = list(SMOKE_MODULES)
        why = (f"{len(paths)} path(s), all under "
               f"{' or '.join(SKIPPABLE_PREFIXES)}")
    else:
        filters = []
        offenders = [p for p in paths if not p.startswith(SKIPPABLE_PREFIXES)]
        why = (f"{len(offenders)} gated path(s), first: "
               f"{offenders[0] if offenders else '(none — empty diff)'}")

    print(f"gate_suite: {'SMOKE' if skippable else 'FULL'} — {why}")
    if skippable:
        for name, reason in SMOKE_MODULES.items():
            print(f"  + {name}: {reason}")

    if args.explain:
        for p in paths:
            print(f"    {p}")
        return 0

    cmd = ["cargo", "test", "-p", "ambition_app", "--test", "app_it", "--quiet"]
    if filters:
        cmd += ["--", *filters]
    print(f"  $ {' '.join(cmd)}", flush=True)
    return subprocess.run(cmd, cwd=str(REPO)).returncode


if __name__ == "__main__":
    raise SystemExit(main())
