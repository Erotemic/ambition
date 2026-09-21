#!/usr/bin/env python3
"""Every live `GenerationMechanics` construction declares WHICH road it is on.

⛔⛤ **THIS EXISTS BECAUSE `Q144`'S OPTION 1 NAMED ITS OWN WEAKNESS AND NOTHING
HELD IT** — *"nothing but review enforces that choice at a new call site."* A
generation's frozen mechanics and the App registries were two owners of one
fact, discriminated by which CONSTRUCTOR a road picks, and this is the
enforcement of that choice.

⭐⭐ **AND THE FAMILY CLOSED ON 2026-09-20, WHICH CHANGES WHAT THIS SCRIPT IS
FOR RATHER THAN RETIRING IT.** The App-registry fallback is deleted:
`for_live_session` now refuses whenever there is no generation, and the hot
reload states its candidate registries at a constructor that names the road.
So the type no longer ranks two authorities — but the CHOICE is still made per
call site, and a new site can still pick the wrong one. What this holds is
that somebody looked; what it never could hold is whether they were right.

⛔⛤ **WHAT THIS CANNOT WITNESS, AND IT USED TO CLAIM OTHERWISE.** Until
2026-09-18 the three places below said option 2 drives the `new`/`of` rows to
zero. Both halves are wrong, and `session/mechanics.rs` — the constructors' own
doc comments — already said so:

  * `::of` is *"a generation's values with NO App fallback"* (`mechanics.rs:206`),
    for activation, which *"cannot be without a generation"*. Under option 2
    every composition has one, so `::of` is the TARGET STATE, not the thing to
    delete.
  * the one production `::new` is the hot reload, and `mechanics.rs:174-177`
    carves it out by name: a reload *"legitimately has no active generation to
    read: it is building the one that replaces it, and states `None` on
    purpose. Those keep [`Self::new`]."* Option 2 deletes a FALLBACK, and that
    `None` is not one.

⇒ What option 2 actually drives to zero is the `shell_routed == false &&
active.is_none()` branch INSIDE `for_live_session` (`mechanics.rs:185-188`),
which returns `Some(Self::new(None, ..))` and is how a direct-entry demo or a
headless harness reaches the App registries. This script counts which
CONSTRUCTOR a site picked; it does not and statically cannot count which BRANCH
that constructor took at runtime, because every `for_live_session` site passes
an `active` whose emptiness is a property of the composition, not of the call.
⇒ The `::new` row is a reading of the reload's intent, not a progress meter.
Q144's option-2 cost is sized by the composition population (see `Q144`), not by
these rows.

⭐ **THE POPULATION IS FIVE, WHICH IS WHY A DECLARED BASELINE IS THE RIGHT
INSTRUMENT.** MEASURED 2026-09-18 over the production corpus, comments and
`#[cfg(test)]` modules stripped by `lib/test_paths`:

    GenerationMechanics::for_live_session   3   reset, and room transition x2
    GenerationMechanics::of                1   provider activation
    GenerationMechanics::new               1   hot reload, and it states `None`

⭐ RE-MEASURED 2026-09-21 and it is still five, with one road having changed
constructor rather than the population having grown:

    GenerationMechanics::for_live_session   4   reset, room transition x2, world-only reload
    GenerationMechanics::of                1   provider activation

⚠ Both drifts were found by RUNNING this guard, not by reading it, and the
second was hidden behind the first: `undeclared` returns before `vanished` is
printed, so the world-only reload's new `for_live_session` masked the fact that
`for_the_generation_being_built` had no caller left. A guard that reports one
finding per run tells you how many runs you need, not how many findings there
are.

⛔⛤ **AND THAT MEASUREMENT CORRECTED `Q144`'S OWN TABLE, WHICH COUNTED FOUR
`for_live_session` ROADS.** The fourth was *"room stage"*, and
`world/rooms/stage.rs` never constructs mechanics: it owns the ERROR VARIANT
`RoomConstructionError::LiveGenerationMechanicsMissing`, whose doc comment names
the constructor. `construct_room_candidate` is HANDED an
`ActorConstructionContext` built by its caller, and the only producer of that
variant is `room_transition/loading.rs`, which is already one of the three. ⇒ A
file that names a constructor in prose reads exactly like a file that calls it,
and the table was assembled by grep. This script strips comments first, so the
same mistake cannot be made through it.

# # What a row is, and what it is not

A row is a reading: *"this road picks this constructor, and here is the
argument."* It is never a waiver. Deleting a row does not silence the site — it
turns the site into an undeclared one and this check fails.

⚠ **THE KEY IS `(file, constructor)` AND CARRIES A COUNT**, so a second call to
the same constructor in a file that already has one is a new site, not a match.
Keying on a line number would make every unrelated edit above it a failure, and
keying on the file alone would let a road quietly double.

⛔ This check cannot tell whether a road picked the RIGHT constructor. It can
only tell whether a human looked. The argument in each row is the part a
reviewer must re-derive; the floor below is what says the corpus did not
silently empty.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from lib.rust_source import strip_comments  # noqa: E402
from lib.test_paths import is_test_path, strip_test_modules  # noqa: E402

REPO = Path(__file__).resolve().parent.parent

#: The three constructors, and what each one MEANS about its caller's
#: composition. Taken from their doc blocks in
#: `crates/ambition_platformer2d_actor_monolith/src/session/mechanics.rs`.
CONSTRUCTORS: dict[str, str] = {
    "for_live_session": (
        "the caller is inside a shell-routed session and REFUSES rather than "
        "falling back: returns `None` when `SessionGatedSimulation` is present "
        "with no active generation"
    ),
    "of": "the caller already holds the generation, so no fallback is offered",
    "for_the_generation_being_built": (
        "the caller is ASSEMBLING the next generation, so the registries it "
        "was handed are the candidate's and it states them outright — the one "
        "road that is not reading an activated generation, and not a fallback"
    ),
}

#: A call site's own regex.
#:
#: ⚠ **NO PART OF THE FLEXIBILITY HERE IS LOAD-BEARING TODAY, AND SAYING SO IS
#: THE POINT** — my first draft of this comment claimed the `\s*\(` was needed
#: because *"two of the five sites wrap the argument list onto the next line"*.
#: They do wrap, but INSIDE the parens; the `(` is flush against the name at all
#: five. MEASURED 2026-09-18, every relaxation still finds exactly 5:
#: dropping `\s*` around `::`, dropping `\s*` before `(`, dropping the trailing
#: `\(` entirely, dropping the leading `\b`. ⇒ The tolerance is defensive
#: against spellings that do not exist yet, not a fix for one that does.
#:
#: ⭐ The last of those relaxations is itself a reading: with `\(` dropped the
#: count does not RISE, which says every prose mention of these constructors in
#: production code lives in a comment — and `strip_comments` has already removed
#: them before this pattern runs. That is the property `Q144`'s own table lacked.
CALL = re.compile(r"\bGenerationMechanics\s*::\s*(for_live_session|of|for_the_generation_being_built)\s*\(")

#: `(path, constructor) -> (how many calls, the reading)`.
DECLARED: dict[tuple[str, str], tuple[int, str]] = {
    (
        "crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs",
        "for_live_session",
    ): (
        1,
        "RESET. A reset rebuilds the world the live session is already playing, "
        "so the generation's frozen values are the only correct input and the "
        "`None` return is a refusal the caller propagates (read 2026-09-18)",
    ),
    (
        "crates/ambition_platformer2d_runtime/src/room_transition/loading.rs",
        "for_live_session",
    ): (
        1,
        "ROOM TRANSITION, loading half. Same argument as reset, and this is the "
        "one site that turns the `None` into "
        "`RoomConstructionError::LiveGenerationMechanicsMissing` — so it is the "
        "only producer of the variant `world/rooms/stage.rs` declares "
        "(read 2026-09-18)",
    ),
    (
        "game/ambition_app/src/app/world_flow/room_transition_assets.rs",
        "for_live_session",
    ): (
        1,
        "ROOM TRANSITION, asset half. The second of the two transition roads; "
        "it builds the same room from the same generation and must not read a "
        "different authority than the loading half (read 2026-09-18)",
    ),
    ("crates/ambition_platformer2d_provider/src/lifecycle.rs", "of"): (
        1,
        "PROVIDER ACTIVATION. It is holding the `mechanical` generation it is "
        "activating, so there is nothing to fall back to and `of` is the "
        "signature that says so (read 2026-09-18)",
    ),
    (
        "game/ambition_app/src/app/dev_runtime.rs",
        "for_live_session",
    ): (
        1,
        "WORLD-ONLY RELOAD, and it is the OPPOSITE road to the hot reload three "
        "lines below in the same file — which is the whole reason this pair has "
        "to be declared per constructor rather than per file. "
        "`prepare_world_replacement_candidate` copies every non-`world.` "
        "fingerprint section out of the ACTIVE `PreparedContent`, so the "
        "candidate CLAIMS the live session's cast, sheets, bosses and forced "
        "brains; building the room from whatever the App happens to hold would "
        "publish identity A over a world built from mechanics B, and App "
        "registries differing from the frozen generation is a SUPPORTED state. "
        "⇒ A mechanical change is a full generation replacement and is not "
        "something a world reload may smuggle. Added by `a49ae6654` on "
        "2026-09-20 and undeclared until this guard was run on 2026-09-21 "
        "(read 2026-09-21)",
    ),
}

# ⛔⛤ **THE `for_the_generation_being_built` ROW IS DELETED, AND SO IS ITS
# CONSTRUCTOR.** It read *"HOT RELOAD. It is building the generation that
# REPLACES the live one"* — and `a49ae6654` (2026-09-20) found that argument was
# made from the word "replacement" rather than from what the candidate CLAIMS:
# `prepare_world_replacement_candidate` copies every non-`world.` fingerprint
# section out of the ACTIVE content, so the reload published "same mechanics,
# new world" over a room built from whatever the App was holding. The road now
# takes the live generation's frozen mechanics through `for_live_session`, and
# that commit says in its own words: *"`for_the_generation_being_built` is
# deleted with its last caller"*. ⚠ `mechanics.rs` today declares only `of` and
# `for_live_session`.
#
# ⛔ THE NAME STAYS IN `CALL` ON PURPOSE. A pattern that can still see a
# constructor nobody defines costs one alternation and catches its
# re-introduction as UNDECLARED; deleting it would narrow the corpus so that
# bringing the road back is invisible, which is the failure this file's own
# floors exist against.

#: ⛔ THE FLOORS. `production files` is the load-bearing one: widening
#: `lib/test_paths` makes every consumer see less, and a consumer that sees less
#: reports cleaner, so a stripper that dropped these five files would otherwise
#: print a serene OK forever.
#:
#: ⛔⛤ **`construction sites` IS 1 AND NOT 5, AND THE FIRST DRAFT'S 5 MADE THE
#: `vanished` BRANCH UNREACHABLE.** Poisoned 2026-09-18 by renaming one real
#: call: the floor fired first and reported *"this check's own population
#: collapsed, suspect a stripper widening"* for a site that had simply gone
#: away. Two things were wrong with that. The message accused the instrument of
#: a defect the tree had, and — worse — **a falling count is what `Q144` option
#: 2 SUCCEEDING looks like.** A floor pinned to today's reading turns the
#: progress this check is meant to measure into an instrument failure.
#:
#: ⇒ So the floor here is pure non-vacuity: *"the pattern matched anything at
#: all."* Every real drop is owned by `vanished`, which names the rows and says
#: what to do about each.
FLOORS = {"production files": 900, "construction sites": 1}


def production_sources() -> list[tuple[str, str]]:
    out: list[tuple[str, str]] = []
    for root in ("crates", "game"):
        for path in sorted((REPO / root).rglob("*.rs")):
            rel = path.relative_to(REPO)
            if any(part == "target" for part in rel.parts):
                continue
            text = path.read_text(errors="replace")
            if is_test_path(rel, text):
                continue
            out.append((rel.as_posix(), strip_test_modules(strip_comments(text))))
    return out


def construction_sites(
    sources: list[tuple[str, str]] | None = None,
) -> dict[tuple[str, str], int]:
    """`(path, constructor) -> call count` over production code only."""
    if sources is None:
        sources = production_sources()
    found: dict[tuple[str, str], int] = {}
    for rel, text in sources:
        for match in CALL.finditer(text):
            key = (rel, match.group(1))
            found[key] = found.get(key, 0) + 1
    return found


def main() -> int:
    sources = production_sources()
    found = construction_sites(sources)

    sizes = {"production files": len(sources), "construction sites": sum(found.values())}
    thin = {k: v for k, v in sizes.items() if v < FLOORS[k]}
    if thin:
        print("FAIL: this check's own population collapsed, so its OK would mean nothing")
        for name, value in sorted(thin.items()):
            print(f"  {name}: {value} < floor {FLOORS[name]}")
        print("  ⇒ suspect `lib/test_paths` widening or a renamed constructor, not a clean tree")
        return 1

    undeclared = sorted(k for k in found if k not in DECLARED)
    vanished = sorted(k for k in DECLARED if k not in found)
    moved = sorted(k for k in found if k in DECLARED and found[k] != DECLARED[k][0])

    if undeclared:
        print("FAIL: a `GenerationMechanics` construction nobody has read:")
        for path, ctor in undeclared:
            print(f"  {path}  ::{ctor}  (x{found[(path, ctor)]})")
            print(f"      that constructor means: {CONSTRUCTORS[ctor]}")
        print(
            "  ⇒ Q144 is open BECAUSE the second authority is reachable. Add a row saying "
            "which composition this road is, and why that constructor is the right one."
        )
        return 1

    if moved:
        print("FAIL: a declared road changed how many times it constructs:")
        for key in moved:
            print(f"  {key[0]}  ::{key[1]}  declared x{DECLARED[key][0]}, found x{found[key]}")
        print("  ⇒ a road that constructs twice is two readings, even in one file")
        return 1

    if vanished:
        print("FAIL: a declared construction site no longer exists:")
        for path, ctor in vanished:
            print(f"  {path}  ::{ctor}")
        print(
            "  ⇒ if Q144 chose option 2 this is progress and the row should go, with the "
            "census row updated. If it moved, the row moves with it. Silence is the one "
            "wrong answer."
        )
        return 1

    print(
        f"ok: {sizes['construction sites']} live `GenerationMechanics` construction(s) "
        f"across {sizes['production files']} production file(s), every one declared"
    )
    by_ctor: dict[str, int] = {}
    for (_path, ctor), count in found.items():
        by_ctor[ctor] = by_ctor.get(ctor, 0) + count
    for ctor in CONSTRUCTORS:
        print(f"  ::{ctor:<17} {by_ctor.get(ctor, 0)}")
    print(
        "  ⚠ these are CONSTRUCTOR choices. The fallback branch they used to be a\n"
        "    proxy for is gone (2026-09-20): a live rebuild with no generation\n"
        "    refuses. What this holds now is that each site's choice was looked at."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
