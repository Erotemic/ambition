#!/usr/bin/env python3
"""Which id CONVENTIONS are built in one place and taken apart in another?

⭐ THE SHAPE THIS LOOKS FOR. A code that names things — `room_visited_<id>`,
`encounter_chest_<id>`, `npc_<id>_talked` — is a FACT, and it is spelled at least
twice whenever a producer writes it with `format!` and a consumer reads it back
with `strip_prefix` / `starts_with`. The two literals must agree, nothing compares
them, and the failure is silent in the worst direction: the producer keeps making
ids the consumer no longer recognises, both sides keep passing their own tests,
and the behaviour keyed on the parse just stops happening.

⛔⛔ MEASURED 2026-09-07 AND IT IS NOT HYPOTHETICAL. `encounter_chest_<id>` was
spelled THREE times in TWO crates — two producers (`ambition_boss_encounter::rewards`,
the monolith's `encounter_rewards`) and one consumer (`features/ecs/chests.rs`),
whose parse is what sets the "this chest was looted" flag. Poisoning ONLY the
consumer's literal — exactly the drift a rename produces — left **all 590 `app_it`
tests green**. A chest that never records being looted refills on every load, and
nothing in the suite could see it.

⇒ THE FIX IS NOT A GUARD, it is one const and two functions in the crate that owns
the fact, so the producer and the consumer cannot disagree. This file exists to
FIND the next one, not to police them: a reported pair is a question ("who owns
this convention?"), and a crate that answers it stops appearing here.

⚠ IT READS SOURCE TEXT, so it sees only the literal spellings — a prefix assembled
from a const, or built through a helper, is invisible to it AND is exactly what the
fix looks like. ⇒ A shrinking report is the intended direction; an EMPTY one is not
proof of anything, which is why the floor below is on the CORPUS and not on the
findings.

⚠ A COARSE PREFIX CAN COLLIDE ACROSS DOMAINS, and the report says so rather than
exempting it. `npc_` is reported today because the save-flag helpers build
`npc_<id>_hostile` / `npc_<id>_talked` while `ambition_sprite_sheet` strips a
`npc_` off a CHARACTER ID — two different conventions that happen to share four
characters. That is a false pair, and the honest place for it is a reader's glance
at the two paths, not an amnesty list in this file: a list of exempt names is how
you stop looking at the thing it exempts.

⚠ COMMENT LINES ARE SKIPPED. The first version of this sweep reported
`room_visited_` — from the doc comment that RECORDS the repair, quoting both
literals it removed. A sweep that reads prose finds the sentence about the defect
and calls it the defect.
"""

from __future__ import annotations

import collections
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
ROOTS = ("crates", "game")

#: `format!("some_prefix_{...}` — a producer building an id.
WRITE = re.compile(r'format!\(\s*"([a-z0-9_:]*[a-z0-9]_)\{')
#: `strip_prefix("some_prefix_")` / `starts_with("some_prefix_")` — a consumer
#: taking one apart.
READ = re.compile(r'(?:strip_prefix|starts_with)\(\s*"([a-z0-9_:]*[a-z0-9]_)"')

#: A prefix both spelled and parsed inside ONE file is still two literals, but it
#: is one author's problem and one edit's blast radius. Reported, not excused.
Site = collections.namedtuple("Site", "path line kind")


def scan() -> dict[str, list[Site]]:
    sites: dict[str, list[Site]] = collections.defaultdict(list)
    for root in ROOTS:
        for path in sorted((REPO / root).rglob("*.rs")):
            try:
                text = path.read_text(errors="replace")
            except OSError as exc:
                # ⛔ A read error is a FINDING, not a silent zero: a corpus this
                # sweep could not open is a corpus it cannot make claims about.
                print(f"FAIL: cannot read {path}: {exc}", file=sys.stderr)
                raise SystemExit(1) from exc
            rel = str(path.relative_to(REPO))
            for number, line in enumerate(text.splitlines(), 1):
                if line.lstrip().startswith("//"):
                    continue
                for match in WRITE.finditer(line):
                    sites[match.group(1)].append(Site(rel, number, "WRITE"))
                for match in READ.finditer(line):
                    sites[match.group(1)].append(Site(rel, number, "READ"))
    return sites


def main() -> int:
    sites = scan()
    # ⚠ ANTI-VACUITY ON THE CORPUS, not on the findings. Zero findings is the goal
    # state; zero SPELLINGS means the regexes stopped matching Rust.
    if len(sites) < 20:
        print(
            f"FAIL: only {len(sites)} id prefix spelling(s) found in the whole "
            "tree.\n  This sweep is looking at source it no longer understands — "
            "that is a broken\n  reader, not a tree that stopped naming things.",
            file=sys.stderr,
        )
        return 1

    both = {
        prefix: rows
        for prefix, rows in sites.items()
        if {row.kind for row in rows} == {"WRITE", "READ"}
    }
    print(f"id prefixes spelled as a literal anywhere: {len(sites)}")
    print(f"  spelled in BOTH a producer and a consumer: {len(both)}")
    for prefix, rows in sorted(both.items()):
        crates = {row.path.split("/")[1] for row in rows}
        print(f"    {prefix!r}  {len(rows)} site(s) in {len(crates)} crate(s)")
        for row in sorted(rows):
            print(f"        {row.kind:5} {row.path}:{row.line}")
    if not both:
        print("    none — every convention this sweep can see has one owner")
    print(
        "\n⇒ Each row is a QUESTION: which crate owns this naming convention? The "
        "answer is\n  one const plus a build/parse pair there, after which the "
        "prefix stops being a\n  literal anywhere else and this sweep stops seeing "
        "it."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
