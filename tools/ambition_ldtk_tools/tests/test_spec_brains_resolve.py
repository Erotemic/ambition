"""Generated room specs must not retain unresolved brain references.

A spec `brain` may name a built-in controller or surviving roster entry. If it
names neither, that placement must also identify its character so regeneration
cannot restore a pre-migration archetype-only placement. Valid brain metadata may
remain alongside `character_id`."""

from __future__ import annotations

import re
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[3]
SPECS = REPO / "tools/ambition_ldtk_tools/specs"
# ⛔⛔ THE ARCHETYPE ROSTER WAS DELETED — `74bd5e9ae Delete the enemy-archetype
# ontology: a body is what its character says` — and this guard pointed at the
# file for long enough to go dark: every test here died on `FileNotFoundError`
# rather than reporting anything, in a suite the project's cargo gate never runs.
#
# ⭐ THE SURVIVING ROSTER IS THE CATALOG'S TWO BRAIN SECTIONS. A spec `brain:`
# names a built-in controller, or a key resolved through `autonomous_profiles` /
# `brain_presets`, or nothing — and the rule below is unchanged: naming nothing
# is allowed only when the placement also identifies its character.
ROSTER = REPO / "game/ambition_content/assets/data/character_catalog.ron"
ROSTER_SECTIONS = ("autonomous_profiles", "brain_presets")

# : `parse_enemy_brain` and `parse_boss_brain` :
# (crates/ambition_platformer2d_ldtk/src/fields.rs) resolve these themselves; : everything else
# becomes `CharacterBrain::Custom(key)` and is looked up in the : roster. : : `PhaseScript:`
# belongs to `BossSpawn`, a population the placement census : never counted — 11 bosses carry no
# `character_id` field at all and resolve : through boss profiles instead. A first draft of this
# list omitted the boss : vocabulary and flagged nine bosses as orphans. : : `Patrol:` is GONE.
BUILT_IN_PREFIXES = ("Guard:", "PhaseScript:")
BUILT_IN_EXACT = frozenset({"Passive", "Dormant"})

_BRAIN = re.compile(r'brain"?\s*:\s*"([^"]+)"')


def _roster_rows() -> frozenset[str]:
    """Keys a `brain:` can name — the catalog's brain sections, not characters.

    ⚠ the CHARACTERS section is deliberately excluded. A placement identifies its
    character with `character_id`; `brain:` names a CONTROLLER, and folding 130
    character ids into this set would let a placement that names a character in
    the brain field pass as resolved.
    """
    text = ROSTER.read_text()
    rows: set[str] = set()
    for section in ROSTER_SECTIONS:
        opened = re.search(r"^\s{4}" + section + r":\s*\{", text, re.MULTILINE)
        assert opened, f"{ROSTER} has no `{section}:` section"
        index, depth = opened.end(), 1
        while index < len(text) and depth:
            if text[index] == "{":
                depth += 1
            elif text[index] == "}":
                depth -= 1
            index += 1
        rows |= set(
            re.findall(
                r'^\s{8}"([a-zA-Z_][a-zA-Z0-9_]*)"\s*:', text[opened.end() : index], re.MULTILINE
            )
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
