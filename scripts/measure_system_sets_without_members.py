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

## ⛔⛤ A THIRD WAY IT REPORTED THE OPPOSITE OF THE TRUTH — found by a reviewer

**`git ls-files`'s return code was never checked.** In a checkout where git
refused (a missing `safe.directory` entry) this script printed **"0 SystemSet
types declared, 0 with ZERO members"** and **exited 0**. ⇒ A clean bill from an
empty corpus, in a file whose own docstring already records two other ways it
lied.

⚠ **THE FINDING SURVIVED THE INSTRUMENT.** 127/1 reproduced once git worked, so
`PlatformerRuntimeSet` stands. **The instrument was the problem, not the result**
— which is the only reason a false clean here was recoverable.

⭐ **Both refusals now exit 1 and say why**, poison-verified:

    git fails                -> exit 1, names the git error
    corpus has no SystemSet  -> exit 1, names the scanned file count
    healthy                  -> exit 0

⚠ And the scope was narrower than the label. The regex required `pub`, while the
output said *"SystemSet types declared"*: **134 derives exist, 127 public.** A
private set is still a set — a system can join it and `.after` it inside its own
crate, which is precisely the no-op this script hunts. **Visibility is now
printed, not filtered on.**

## What it does not answer

⛔⛔ **IT COUNTS NAMES, AND A REPEATED NAME POOLS ITS MEMBERSHIPS.** Membership is
matched as `in_set(...Name)` across the corpus, so two sets sharing a name cannot
be told apart: an EMPTY one is hidden by a populated sibling, which is precisely
the no-op this script hunts. **The collapse is now PRINTED rather than silent** —
measured 2026-09-10, 135 declarations against 134 names, the repeat being two
function-local `Umbrella` sets in one test file, both populated. ⇒ The run is not
refused, because a repeat is not by itself a defect; it is named so the reader can
check the pair.

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
#: ⛔⛔ ALL `SystemSet` DERIVES, NOT ONLY `pub` ONES. A first version required
#: `pub` while the output said *"SystemSet types declared"* — 134 derives exist
#: and 127 are public, so it under-reported the population by seven and named it
#: as if it were complete. ⇒ A private set is still a set: a system can join it
#: and `.after` it within its own crate, which is exactly the no-op this script
#: hunts. **The visibility is captured and printed, not used as a filter.**
DECL = re.compile(
    r"#\[derive\([^)]*\bSystemSet\b[^)]*\)\]\s*(?:#\[[^\]]*\]\s*)*"
    r"(pub(?:\([^)]*\))?\s+)?(?:enum|struct)\s+([A-Za-z0-9_]+)"
)
LINE_COMMENT = re.compile(r"//[^\n]*")


def main() -> int:
    # ⛔⛔ A FAILED `git ls-files` USED TO PRODUCE A CONFIDENT ZERO. The return
    # code was never checked, so in a checkout git refused (a missing
    # `safe.directory` entry) this script printed **"0 SystemSet types declared,
    # 0 with ZERO members"** and exited 0 — a clean bill from an empty corpus.
    #
    # ⚠ THAT IS THIS FILE'S OWN SIGNATURE, TWICE OVER. Its docstring already
    # records counting prose and missing qualified paths; both were caught by a
    # number disagreeing with a fact measured another way. **A total collapse to
    # zero across every bucket is the empty-corpus tell, not a result** — and the
    # one reader who cannot apply that judgement is the script itself.
    listing = subprocess.run(
        ["git", "ls-files", "*.rs"], cwd=REPO, capture_output=True, text=True
    )
    if listing.returncode != 0:
        raise SystemExit(
            "git ls-files failed (exit "
            f"{listing.returncode}) in {REPO}: {listing.stderr.strip()}\n"
            "⇒ REFUSING TO REPORT. Without a file list this script finds no "
            "declarations and no members, and every count it prints is zero — "
            "which reads as `every set is wired`, the opposite of a warning."
        )
    tracked = [p for p in listing.stdout.split("\n") if p]
    if not tracked:
        raise SystemExit(
            f"git ls-files succeeded but listed no .rs files in {REPO}. "
            "⇒ REFUSING TO REPORT: an empty corpus cannot find an unwired set."
        )

    declared: dict[str, tuple[str, bool]] = {}
    sources: dict[str, str] = {}
    # ⛔⛔ THE POPULATION IS NAMES, NOT DECLARATIONS, AND A REPEAT COLLAPSES.
    # `declared` is keyed on the bare name because MEMBERSHIP is: the counter
    # below matches `in_set(...Name)` across the whole corpus and cannot tell two
    # same-named sets apart. So a second declaration of a name is dropped here
    # rather than re-keyed -- re-keying would print two rows sharing one pooled
    # count, which is a worse lie than one row.
    # ⚠ AND THE POOLING IS THIS TOOL'S OWN FAILURE MODE: an EMPTY set can be
    # hidden by a populated same-named sibling, which is exactly the no-op this
    # script hunts. Measured 2026-09-10: 135 declarations, 134 names.
    repeats: collections.defaultdict[str, list[str]] = collections.defaultdict(list)
    for rel in tracked:
        try:
            text = (REPO / rel).read_text(errors="replace")
        except OSError:
            continue
        sources[rel] = text
        for m in DECL.finditer(text):
            name = m.group(2)
            repeats[name].append(rel)
            declared.setdefault(name, (rel, bool(m.group(1))))

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

    empty = sorted((n, v) for n, v in declared.items() if members[n] == 0)
    public = sum(1 for _, is_pub in declared.values() if is_pub)
    # ⛔ AND THE FLOOR IS NONZERO. "No unwired set" over zero declarations is a
    # true statement about nothing; the population must exist before its
    # emptiness means anything.
    if not declared:
        raise SystemExit(
            f"{len(tracked)} .rs files scanned and NOT ONE declares a `SystemSet`. "
            "⇒ REFUSING TO REPORT: the declaration regex found nothing, so the "
            "membership counts below would all be zero for want of a subject."
        )
    print(f"{len(declared)} SystemSet types declared ({public} public, "
          f"{len(declared) - public} private), across {len(tracked)} tracked .rs files\n")
    collapsed = {n: w for n, w in repeats.items() if len(w) > 1}
    if collapsed:
        total = sum(len(w) for w in repeats.values())
        print(
            f"⚠ {total} declarations collapsed to {len(declared)} names. A repeated "
            "name pools its memberships, so an EMPTY one is hidden by a populated "
            "sibling — the very no-op this script hunts:"
        )
        for name, where in sorted(collapsed.items()):
            print(f"   {name:42} {len(where)}x  {', '.join(sorted(set(where)))}")
        print()

    print(f"{len(empty)} with ZERO `in_set(` members — ordering against these is a NO-OP:")
    for name, (where, is_pub) in empty:
        print(f"   {name:42} {'pub ' if is_pub else 'priv'} {where}")
    if not empty:
        print("   (none — every declared set has at least one member)")
    print(f"\n{len(declared) - len(empty)} with members, largest first:")
    for name, count in members.most_common(8):
        if count:
            print(f"   {name:42} {count}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
