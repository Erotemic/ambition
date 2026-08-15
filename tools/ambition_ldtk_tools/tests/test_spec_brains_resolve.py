"""A room's GENERATOR must not out-live the roster row it was written against.

⛔⛔ **THE THIRD COPY NOBODY COUNTED.** D73's migration moves a placement off the
archetype roster and onto a character: the row is deleted from
`character_archetypes.ron` and the placement in the shipped `.ldtk` grows a
`character_id`. Both halves were done and checked. But a room is not authored by
hand — it is generated from a spec in `specs/*_area.ron`, and that spec keeps its
own copy of every placement's fields.

Measured 2026-08-13: the worlds were fully migrated (every `EnemySpawn` /
`NpcSpawn` carried a `character_id` bar one open content decision) while **twelve
spec placements still named `brain:` keys whose rows had been deleted and named
no character at all** — `ranged_skirmisher`, `pirate_on_shark`, `exploding_mite`,
`dividing_mite`, `giant_gnu`, `small_skitter`, `large_brute`. Regenerating any of
those rooms would have written the pre-migration placement straight back over the
migrated one, and nothing in the repository would have said so.

⚠ **`brain` staying is fine; `brain` staying ALONE is not.**
`intro_raid_corridor_area.ron` is the convention this asserts — it carries
`brain: "medium_striker"` *and* `character_id: "npc_lab_raider"`, so the
controller policy is still named and the body no longer depends on a row.

⇒ so the condition is exactly: **a spec's `brain:` must name something that still
resolves — a built-in, or a surviving roster row — or else the placement must
name its character.** Not "specs mention no dead names" (a proxy that would go
green by deleting the field), and not "every placement has a `character_id`"
(which would demand casting decisions that are Jon's to make, ledger D96).
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[3]
SPECS = REPO / "tools/ambition_ldtk_tools/specs"
ROSTER = REPO / "game/ambition_content/assets/data/character_archetypes.ron"

#: `parse_enemy_brain` and `parse_boss_brain`
#: (crates/ambition_platformer2d_ldtk/src/fields.rs) resolve these themselves;
#: everything else becomes `CharacterBrain::Custom(key)` and is looked up in the
#: roster.
#:
#: ⚠ `PhaseScript:` belongs to `BossSpawn`, a population the placement census
#: never counted — 11 bosses carry no `character_id` field at all and resolve
#: through boss profiles instead. A first draft of this list omitted the boss
#: vocabulary and flagged nine bosses as orphans.
#:
#: ⛔ `Patrol:` is GONE. A patrol's path is a native `path_ref` EntityRef now,
#: and `convert_enemy_spawn` refuses the retired prefix out loud — so a spec
#: still spelling it must be reported here, not excused.
BUILT_IN_PREFIXES = ("Guard:", "PhaseScript:")
BUILT_IN_EXACT = frozenset({"Passive", "Dormant"})

_BRAIN = re.compile(r'brain"?\s*:\s*"([^"]+)"')


def _roster_rows() -> frozenset[str]:
    """Top-level keys of the archetype roster — the rows a `brain:` can name."""
    rows = re.findall(
        r'^\s{4}"([a-zA-Z_][a-zA-Z0-9_]*)"\s*:\s*\(', ROSTER.read_text(), re.MULTILINE
    )
    assert rows, (
        f"{ROSTER} parsed to zero rows, so every brain below would look dead and "
        "this guard would report the whole repository as broken. The row regex "
        "is what to fix, not the specs"
    )
    return frozenset(rows)


def _live_brain_fields() -> list[tuple[Path, int, str, bool]]:
    """`(spec, line, brain, names_a_character)` for every non-comment `brain:`.

    ⚠ comments are skipped deliberately and that is not a detail: the migration
    specs annotate their edits with lines like ``# was `brain: "puppy_slug"`,
    whose row is deleted``. A first pass at this measurement counted those and
    reported 42 stale placements where there were 12.
    """
    found = []
    for spec in sorted(SPECS.rglob("*")):
        if not spec.is_file() or spec.suffix not in (".ron", ".yaml", ".json"):
            continue
        lines = spec.read_text(errors="replace").splitlines()
        for index, line in enumerate(lines):
            if line.lstrip().startswith(("//", "#")):
                continue
            match = _BRAIN.search(line)
            if not match:
                continue
            window = lines[max(0, index - 12) : index + 12]
            names_character = any(
                "character_id" in text and not text.lstrip().startswith(("//", "#"))
                for text in window
            )
            found.append((spec, index + 1, match.group(1), names_character))
    return found


def test_every_spec_brain_still_resolves() -> None:
    rows = _roster_rows()
    fields = _live_brain_fields()
    assert fields, (
        "no `brain:` field was found in any spec, so this guard checked nothing. "
        "Either the specs moved or the field was renamed — an empty census is "
        "the one result that must never read as success"
    )

    orphans = [
        (spec, line, brain)
        for spec, line, brain, names_character in fields
        if not names_character
        and brain not in BUILT_IN_EXACT
        and not brain.startswith(BUILT_IN_PREFIXES)
        and brain not in rows
    ]
    assert not orphans, (
        "these generator specs name an archetype row that no longer exists and "
        "name no character either, so regenerating the room writes a placement "
        "that construction cannot resolve — and silently reverts a migration "
        "the shipped world already has:\n"
        + "\n".join(
            f"  {spec.relative_to(SPECS)}:{line}  brain: {brain!r}"
            for spec, line, brain in orphans
        )
        + "\n⇒ copy the `character_id` the .ldtk already carries for that "
        "placement into the spec beside its `brain`, as "
        "intro_raid_corridor_area.ron does."
    )


@pytest.mark.parametrize("dead_brain", ["ranged_skirmisher", "a_row_that_never_existed"])
def test_the_guard_notices_a_dead_brain_standing_alone(dead_brain: str) -> None:
    """⛔ the poison — without this the check above passes by finding nothing.

    Both halves matter: a name that WAS a row and was deleted (the real
    regression) and a name that never existed (a typo), because the roster
    lookup cannot tell them apart and neither can be built.
    """
    rows = _roster_rows()
    assert dead_brain not in rows, (
        f"{dead_brain!r} is a live roster row again, so it no longer poisons "
        "anything — pick a key that is genuinely absent"
    )
    assert dead_brain not in BUILT_IN_EXACT
    assert not dead_brain.startswith(BUILT_IN_PREFIXES)


def test_a_brain_beside_a_character_is_accepted() -> None:
    """The convention this guard is built around, asserted so it cannot drift.

    If `intro_raid_corridor_area.ron` ever stops pairing the two, the rule above
    is enforcing a pattern the repository no longer follows.
    """
    spec = SPECS / "intro_raid_corridor_area.ron"
    text = spec.read_text()
    assert 'brain: "medium_striker"' in text and 'character_id: "npc_lab_raider"' in text, (
        f"{spec.name} is this guard's reference example of a placement that "
        "keeps its controller policy AND names its body; it no longer does, so "
        "the rule and the repository disagree about what correct looks like"
    )
