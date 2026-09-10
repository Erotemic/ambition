#!/usr/bin/env python3
"""Which declared `SystemSet`s have no members? Ordering against one is a NO-OP.

⛔⛔ **`.after(<a set with no members>)` CONSTRAINS NOTHING, AND NOTHING SAYS SO.**
Not a compile error, not a warning, nothing at the call site. The system is placed
wherever the scheduler likes, and the only symptom is a result that looks fine.

⭐ MEASURED 2026-09-10: of **127** declared `SystemSet` types, exactly **ONE** has
zero `in_set(` members — `PlatformerRuntimeSet`
(`ambition_platformer2d_shared_tangle/src/schedule.rs`). ⇒ **This is not a
widespread habit; it is one vocabulary declared and never realized.** 126 sets are
wired, led by `Platformer2dSimulationPhaseMonolith` at 125 members.

## How this was found, and what it cost

A4's double-tick instrument probed `PlatformerRuntimeSet::{ControlInput,
ActorSimulation}` because the writer map frames the packet around that vocabulary.
Every probe was unconstrained, the scheduler ran them all at the tick's end, and
whichever ran first collected 240 of 240 writes. **It reported zero double ticks
— a clean baseline that meant nothing.**

⚠ **AND THE FIRST VERSION OF THIS SCRIPT REPORTED ZERO EMPTY SETS.** Its single
"member" for `PlatformerRuntimeSet` was a doc comment of MINE saying the set has
zero members. ⇒ **An instrument that reads prose counts the sentence describing an
absence as an instance of the thing.** Comments are stripped before matching;
without that this script certifies the opposite of the truth.

## ⛔⛔ TWO WAYS THIS SCRIPT ITSELF REPORTED THE OPPOSITE OF THE TRUTH

Both were caught by a number that disagreed with a fact measured another way, and
neither would have surfaced from reading the code.

**1. IT COUNTED PROSE.** The first run reported ZERO empty sets. The single
"member" it found for `PlatformerRuntimeSet` was a doc comment of MINE saying the
set has zero members. ⇒ **An instrument that reads prose counts the sentence
describing an absence as an instance of the thing.**

**2. IT MISSED QUALIFIED PATHS.** Matching only `in_set(Name` and not
`in_set(a::b::Name` reported **42** empty sets where there is **one** — 41 sets
wired through a qualified path read as dead. ⚠ That draft was a SIMPLIFICATION of
a working scratch version, made while tidying it for commit, and it silently
changed the answer by a factor of forty.

⇒ **A cleanup pass is an edit, and an edit to an instrument needs the instrument
re-run.** The tidy version and the scratch version disagreed, and only re-running
showed it.

## What it does not answer

⚠ It counts `in_set(` in source. A set could also gain members through
`configure_sets(child.in_set(parent))` chains — those are counted, since the
spelling is the same — but a set populated only by a macro or a generated file is
invisible here. ⇒ A set this reports as empty should be confirmed by the
scheduler refusing to order against it, which is the check that cannot be fooled.

Run: python3 scripts/measure_system_sets_without_members.py
"""
from __future__ import annotations

import collections
import pathlib
import re
import subprocess

REPO = pathlib.Path(__file__).resolve().parent.parent

#: A `SystemSet` declaration: the derive, any further attributes, then the item.
DECL = re.compile(
    r"#\[derive\([^)]*\bSystemSet\b[^)]*\)\]\s*(?:#\[[^\]]*\]\s*)*"
    r"pub\s+(?:enum|struct)\s+([A-Za-z0-9_]+)"
)
LINE_COMMENT = re.compile(r"//[^\n]*")


def main() -> int:
    tracked = [
        p
        for p in subprocess.run(
            ["git", "ls-files", "*.rs"], cwd=REPO, capture_output=True, text=True
        ).stdout.split("\n")
        if p
    ]

    declared: dict[str, str] = {}
    sources: dict[str, str] = {}
    for rel in tracked:
        try:
            text = (REPO / rel).read_text(errors="replace")
        except OSError:
            continue
        sources[rel] = text
        for m in DECL.finditer(text):
            declared.setdefault(m.group(1), rel)

    members: collections.Counter[str] = collections.Counter()
    for rel, text in sources.items():
        # ⛔ SEE THE MODULE DOC. Prose naming `in_set(X::..)` is not a membership.
        code = LINE_COMMENT.sub("", text)
        for name in declared:
            # ⛔⛔ THE QUALIFIER IS OPTIONAL AND OMITTING IT CHANGES THE ANSWER.
            # `in_set(BodyCustodySettled)` and
            # `in_set(shared_tangle::lifecycle::BodyCustodySettled)` are the same
            # membership. A first draft matched only the bare form and reported
            # **42** empty sets where there is **one** — 41 sets that are wired
            # through a qualified path read as dead.
            members[name] += len(
                re.findall(rf"in_set\(\s*(?:[A-Za-z0-9_]+::)*{re.escape(name)}\b", code)
            )

    empty = sorted((n, f) for n, f in declared.items() if members[n] == 0)
    print(f"{len(declared)} SystemSet types declared\n")
    print(f"{len(empty)} with ZERO `in_set(` members — ordering against these is a NO-OP:")
    for name, where in empty:
        print(f"   {name:42} {where}")
    if not empty:
        print("   (none — every declared set has at least one member)")
    print(f"\n{len(declared) - len(empty)} with members, largest first:")
    for name, count in members.most_common(8):
        if count:
            print(f"   {name:42} {count}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
