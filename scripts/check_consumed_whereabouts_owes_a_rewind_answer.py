#!/usr/bin/env python3
"""A `Consumed` producer obliges the occurrence ledger to become VALUE state.

⛔⛔ THE RULE IS ALREADY WRITTEN, AND NOTHING ENFORCED IT.
`AuthoredOccurrences::rewind_argument` (a `const fn` that exists only to carry
its doc) says:

    Live custody and placement producers republish their rows every tick, while
    room transitions commit beyond the frame-rollback boundary. If a
    non-rederived whereabouts state (such as `Consumed`) gains a producer, this
    ledger must become registered value state with a value-sensitive probe.

⇒ The ledger is registered `declare_rollback_derived_resource`, which is correct
ONLY while every row is republished from live state each tick. `Consumed` is the
one variant that would be written mid-frame from an event and never re-derived,
so the day it gains a producer a rewind can strand it — and the person adding
that producer has no reason to read a doc comment on a `const fn` in another
crate.

⭐ WHY A CENSUS AND NOT A PRODUCER DETECTOR. Telling a CONSTRUCTION from a MATCH
ARM textually is exactly the mistake this guard was written after making: a grep
for `OccurrenceWhereabouts::Consumed` reports `held_items`' refusing match arm as
though it were a write. Rather than encode a fragile heuristic, this pins the
POPULATION of files that mention the variant at all. A new mention anywhere
reddens and asks its author one question. That is a slightly wider net than the
rule needs, and the failure text says so, so a legitimate new READER can be added
to the census in one line with its reason.

⚠ THE FLOOR IS A PRESENCE, deliberately: the variant must still exist and the
ledger must still be registered DERIVED. If either stops being true the guard
says so instead of passing quietly — a rename would otherwise empty the census
and certify nothing.
"""
from __future__ import annotations

import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
VARIANT = "OccurrenceWhereabouts::Consumed"
LEDGER = "AuthoredOccurrences"
DERIVED_REGISTRATION = f"declare_rollback_derived_resource::<crate::lifecycle::{LEDGER}>"
REGISTRATION_FILE = "crates/ambition_platformer2d_shared_tangle/src/rollback_registration.rs"

#: Every non-test file that may mention the variant, and WHY. A file here has
#: been read and judged not to be a live mid-frame producer.
CENSUS = {
    "crates/ambition_platformer2d_shared_tangle/src/lifecycle/continuity.rs":
        "defines the variant, the ledger's own rules, and the rewind argument",
    "crates/ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs":
        "translates the SAVED row back to the runtime enum on load, and back "
        "again on save — a load-path reconstruction, not a mid-frame write",
    "crates/ambition_held_items/src/lib.rs":
        "a match arm that REFUSES a consumed occurrence as a resting place; "
        "reads the variant, never writes it",
}


def git_grep(pattern: str, *paths: str) -> list[str]:
    proc = subprocess.run(
        ["git", "grep", "-n", pattern, "--", *paths],
        cwd=REPO, capture_output=True, text=True,
    )
    if proc.returncode not in (0, 1):
        raise SystemExit(f"git grep failed: {proc.stderr.strip()}")
    return proc.stdout.splitlines()


def main() -> int:
    hits = [
        line for line in git_grep(VARIANT, "crates/", "game/")
        if "/tests/" not in line.split(":")[0]
        and not line.split(":")[0].endswith("tests.rs")
    ]
    files = {line.split(":")[0] for line in hits}

    # ⭐ FLOOR FIRST: both halves of the premise must still be findable, or every
    # claim below is trivially true.
    if not files:
        print(
            f"FAIL: no non-test mention of `{VARIANT}` at all.\n"
            "  The variant was renamed or removed, and this guard now certifies "
            "an empty set.\n  Re-point it or delete it — do not leave it green.",
            file=sys.stderr,
        )
        return 1
    registration = git_grep(DERIVED_REGISTRATION, REGISTRATION_FILE)
    if not registration:
        print(
            f"FAIL: `{LEDGER}` is no longer registered with "
            f"`declare_rollback_derived_resource`.\n"
            "  That is the premise this guard defends. If the ledger became "
            "registered VALUE state\n  with a value-sensitive probe, the "
            "obligation is DISCHARGED and this guard should be\n  deleted along "
            "with the deferral in `rewind_argument`. If it became something "
            "else,\n  say which and why.",
            file=sys.stderr,
        )
        return 1

    unexpected = sorted(files - set(CENSUS))
    if unexpected:
        print(
            f"FAIL: {len(unexpected)} file(s) mention `{VARIANT}` and are not in "
            "the census:", file=sys.stderr,
        )
        for path in unexpected:
            print(f"  {path}", file=sys.stderr)
        print(
            "\n  ⛔ ANSWER ONE QUESTION: does this file PRODUCE a `Consumed` row "
            "mid-frame?\n"
            "  * NO (it reads the variant, or translates a saved row): add it to "
            "CENSUS with\n    that reason — one line, and the net stays tight.\n"
            "  * YES: the ledger's rollback registration is now WRONG. "
            f"`{LEDGER}` is registered\n    as a DERIVED resource, which holds "
            "only while every row is republished from live\n    state each tick. "
            "A `Consumed` row is written once and never re-derived, so a rewind\n"
            "    can strand it. Make the ledger registered VALUE state with a "
            "value-sensitive\n    probe, then retire this guard and the deferral "
            "in `AuthoredOccurrences::rewind_argument`.",
            file=sys.stderr,
        )
        return 1

    missing = sorted(set(CENSUS) - files)
    if missing:
        # ⭐ THE OTHER DIRECTION, so the census cannot rot into a list of files
        # that no longer say anything.
        print(
            f"FAIL: {len(missing)} census entr(y/ies) no longer mention "
            f"`{VARIANT}`:", file=sys.stderr,
        )
        for path in missing:
            print(f"  {path}  — {CENSUS[path]}", file=sys.stderr)
        print(
            "\n  A stale census makes the guard look tighter than it is. Drop "
            "the entry.", file=sys.stderr,
        )
        return 1

    print(
        f"ok: {len(files)} file(s) mention `{VARIANT}`, all censused as readers "
        f"or load-path translations; `{LEDGER}` is still registered derived."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
