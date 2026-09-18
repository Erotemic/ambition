#!/usr/bin/env python3
"""Every live `GenerationMechanics` construction declares WHICH road it is on.

⛔⛤ **THIS EXISTS BECAUSE `Q144`'S OPTION 1 NAMES ITS OWN WEAKNESS AND NOTHING
HELD IT.** `DUP-GENERATION-MECHANICS` is the last open duplicate-authority
family in the consolidation census: a generation's frozen mechanics and the App
registries are two owners of one fact, discriminated by which CONSTRUCTOR a
road picks. Option 1 keeps that separation as permanent vocabulary and says
outright that *"nothing but review enforces that choice at a new call site."*
This is the enforcement, and it is deliberately option-INDEPENDENT:

  * under option 1 it is the missing guard;
  * under option 2 (every composition prepares a generation) it is the progress
    meter — the `new`/`of` rows are exactly what has to reach zero;
  * under option 3 (split the type) it is the list of signatures to change.

⭐ **THE POPULATION IS FIVE, WHICH IS WHY A DECLARED BASELINE IS THE RIGHT
INSTRUMENT.** MEASURED 2026-09-18 over the production corpus, comments and
`#[cfg(test)]` modules stripped by `lib/test_paths`:

    GenerationMechanics::for_live_session   3   reset, and room transition x2
    GenerationMechanics::of                1   provider activation
    GenerationMechanics::new               1   hot reload, and it states `None`

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
    "new": (
        "the caller states its generation and its App registry explicitly, "
        "including stating `None` for either — the only road that can read the "
        "App registries"
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
CALL = re.compile(r"\bGenerationMechanics\s*::\s*(for_live_session|of|new)\s*\(")

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
    ("game/ambition_app/src/app/dev_runtime.rs", "new"): (
        1,
        "⚠ HOT RELOAD, AND THE ONLY LIVE READER OF THE APP REGISTRIES. It "
        "passes `None` for the generation ON PURPOSE: it is building the "
        "generation that REPLACES the live one, and reading the session's "
        "frozen mechanics here would rebuild the world from the generation "
        "being replaced. ⇒ This is the row `Q144`'s option 2 has to delete, and "
        "it is the row that makes the duplicate authority reachable rather than "
        "theoretical (read 2026-09-18)",
    ),
}

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
        "  ⚠ the `::new` count is Q144's live duplicate authority: option 2 drives it to 0"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
