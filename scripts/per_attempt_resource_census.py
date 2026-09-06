#!/usr/bin/env python3
"""Which CONTENT resources hold collection state, and what retracts them?

⭐ THE RULE THAT MAKES THIS A QUESTION, measured 2026-09-05. An admitted room
replay records a transition back to the SAME room, and that rebuild despawns
every `RoomScopedEntity`. So:

    ENTITY-shaped per-attempt state    retracted FOR FREE by the rebuild
    RESOURCE-shaped per-attempt state  survives untouched -- it must retract
                                       ITSELF, because nothing despawns a resource

⛔⛔ THE BUG THAT PROVES THE CLASS IS REAL. Sanic's `SpentMonitors` re-armed on
`RoomLoaded` only, and Sanic declares `DeathRules::replay_level_after(0.0)`: a pit
death replays the room IN PLACE and never emits a load. A monitor broken before
the death stayed broken after the respawn and its grant was unreachable for the
rest of the run. ⭐ THE WAY IN WAS A NAME -- `SpentMonitors` reads exactly like
`SpentPowerBlocks`, which did retract -- so a name sweep is the right instrument
and this file is that sweep, committed.

⇒ Since 2026-09-06 the retraction is a TYPE:
`ambition_platformer2d_actor_monolith::session::reset::AttemptScoped`. This census
reports which collection-holding content resources implement it and which do not.

⛔⛔ IT DOES NOT FAIL ON "DOES NOT IMPLEMENT IT", AND THAT IS DELIBERATE. Most of
this population is NOT per-attempt: catalogs, caches, dev-tool probes, prefetch
ledgers and a character roster all hold collections and must survive a death.
"Not `AttemptScoped`" is therefore not evidence of anything, exactly as
`room_replay_reader_slots.py` reports slots without policing membership. Whether a
given resource is per-attempt is a content judgement; what this file removes is
the excuse that nobody enumerated them.

⚠ THE DENOMINATOR IS `game/`, AND THE SCOPE PHRASE IS LOAD-BEARING. The same
sweep over `crates/` returns 217, dominated by registries, catalogs, views and
indexes -- engine per-attempt state is entity-shaped or lives in the rollback
registry. Running the content question over that population would bury the three
answers in two hundred non-answers.

⭐ THE FLOOR IS A PRESENCE: the three known per-attempt resources must still be
found AND still implement the trait. A rename, a move, or a silently dropped impl
reddens instead of quietly certifying an empty set.
"""
from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
TRAIT = "AttemptScoped"
COLLECTION = re.compile(r"\b(Vec|HashSet|BTreeSet|HashMap|BTreeMap|VecDeque)\s*<")
#: ⭐ The three that ARE per-attempt, each verified by a test that a death
#: re-arms it. Named here so a rename or a dropped impl is loud.
KNOWN_PER_ATTEMPT = {"BrokenBricks", "SpentPowerBlocks", "SpentMonitors"}


def struct_body(src: list[str], start: int) -> str:
    """The declaration at `start` and NOTHING AFTER IT.

    ⛔⛔ THE FIRST CUT TOOK A FIXED 40-LINE WINDOW AND CUT IT AT THE FIRST `\n}`.
    A TUPLE struct has no closing brace on its own line, so its "body" ran on
    through everything below it -- and a plain `struct Flag(pub u32);` counted as
    a collection because the NEXT struct in the file held a `Vec`. Found by the
    unit test, not by reading: the population was over-counted by whatever
    happened to sit underneath.
    """
    if src[start].rstrip().endswith(";"):       # tuple struct, one line
        return src[start]
    body: list[str] = []
    for line in src[start : start + 60]:
        body.append(line)
        if line.startswith("}"):                # brace struct closes at column 0
            break
    return "\n".join(body)


def collection_resources() -> list[tuple[str, int, str]]:
    """Every `#[derive(.., Resource, ..)]` struct in `game/` with a collection field."""
    found: list[tuple[str, int, str]] = []
    for path in sorted(REPO.glob("game/*/src/**/*.rs")):
        src = path.read_text(encoding="utf-8", errors="replace").split("\n")
        for i, line in enumerate(src):
            if "derive(" not in line or "Resource" not in line:
                continue
            # The struct name is on one of the next few lines: other derives and
            # attributes may sit between the derive and the item.
            for j in range(i + 1, min(i + 6, len(src))):
                name = re.search(r"(?:pub )?struct (\w+)", src[j])
                if not name:
                    continue
                if COLLECTION.search(struct_body(src, j)):
                    found.append((str(path.relative_to(REPO)), j + 1, name.group(1)))
                break
    return found


def implementors() -> set[str]:
    """Types with an `impl .. AttemptScoped for T` anywhere in the tree."""
    names: set[str] = set()
    for path in sorted(REPO.glob("game/*/src/**/*.rs")):
        for line in path.read_text(encoding="utf-8", errors="replace").split("\n"):
            if line.lstrip().startswith("//"):
                continue
            hit = re.search(rf"impl .*{TRAIT} for (\w+)", line)
            if hit:
                names.add(hit.group(1))
    return names


def main() -> int:
    population = collection_resources()
    impls = implementors()
    scoped = [row for row in population if row[2] in impls]
    plain = [row for row in population if row[2] not in impls]

    print(f"collection-holding `Resource` types in game/: {len(population)}")
    print(f"  retracted through `{TRAIT}`: {len(scoped)}")
    for path, line, name in scoped:
        print(f"    {path}:{line}  {name}")
    print(f"  everything else: {len(plain)}  (catalogs, caches, probes, rosters —")
    print("    not per-attempt, and NOT a finding; see this file's docstring)")
    for path, line, name in plain:
        print(f"    {path}:{line}  {name}")

    missing = KNOWN_PER_ATTEMPT - {row[2] for row in scoped}
    if missing:
        print(
            f"\nFAIL: {sorted(missing)} should be retracted through `{TRAIT}` and "
            "is not.\n  Either the impl was dropped, or the type was renamed and "
            "this census now\n  certifies a population that no longer contains it.",
            file=sys.stderr,
        )
        return 1
    if not population:
        print(
            "\nFAIL: no collection-holding content resources found at all — the "
            "sweep is broken,\n  not the tree.",
            file=sys.stderr,
        )
        return 1
    print(f"\nok: all {len(KNOWN_PER_ATTEMPT)} known per-attempt resources retract "
          f"through `{TRAIT}`.")
    print("⇒ Reported, not enforced: 'not AttemptScoped' is not a finding.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
