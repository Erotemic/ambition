"""The cast size quoted in prose must match the room.

⛔⛔ **FOUR PLACES SAID THE HALL AUTHORS 144 NPCs. IT AUTHORS 129.** The number
appeared in a test's failure message, in a doc comment justifying a pacing
constant, and in two probe headers — every one of them written confidently, none
of them checked by anything. A reader debugging a staging failure would have been
told to expect fifteen characters that do not exist.

Nothing asserted it because the number is PROSE. `hall_transition_cover` asserts
`>= MINIMUM_HALL_CAST` (100), which 129 and 144 both satisfy, so the count could
drift by any amount without a single test noticing.

⭐ SO THE PROSE IS THE ASSERTION NOW. This reads the count out of the room and
compares it against every sentence that quotes one. Changing the room's cast
fails here until the sentences are updated with it.

⚠ A DATED MEASUREMENT IS NOT A STALE CLAIM. `every_character_says_something`
records "4 of 144 pedestals" as a historical finding — that was true when it was
measured and it must not be rewritten to today's number. This checks only
PRESENT-TENSE claims, matched by their phrasing.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
HALL = REPO / "game/ambition_content/assets/worlds/hall_of_characters.ldtk"

# Files whose prose states the CURRENT cast size, and the pattern that quotes it.
QUOTING = [
    ("game/ambition_app/tests/hall_transition_cover.rs", r"authors (\d+) NpcSpawn placements"),
    (
        "crates/ambition_platformer2d_actor_monolith/src/character_runtime/mod.rs",
        r"authors (\d+) NpcSpawn placements",
    ),
]


def npc_spawns(world: Path) -> int:
    data = json.loads(world.read_text())
    return sum(
        1
        for level in data.get("levels", [])
        for layer in (level.get("layerInstances") or [])
        for entity in (layer.get("entityInstances") or [])
        if entity.get("__identifier") == "NpcSpawn"
    )


def test_the_hall_still_has_a_cast_to_count():
    """Premise guard: a room that parsed to zero would satisfy every check below."""
    assert HALL.exists(), f"the hall world file moved: {HALL}"
    count = npc_spawns(HALL)
    assert count > 50, (
        f"only {count} NpcSpawn placements parsed out of the hall; either the room "
        f"changed shape or this test is reading the wrong thing, and in both cases "
        f"the comparison below means nothing"
    )


def test_every_sentence_quoting_the_cast_size_quotes_the_right_one():
    actual = npc_spawns(HALL)
    checked = 0
    for relative, pattern in QUOTING:
        path = REPO / relative
        assert path.exists(), f"{relative} moved; update this test with it"
        text = path.read_text()
        matches = re.findall(pattern, text)
        assert matches, (
            f"{relative} no longer quotes the cast size in the expected phrasing "
            f"({pattern!r}). Either the sentence was reworded — update this test — "
            f"or it was deleted, in which case drop its row."
        )
        for quoted in matches:
            checked += 1
            assert int(quoted) == actual, (
                f"{relative} says the hall authors {quoted} NpcSpawn placements; "
                f"the room has {actual}. Update the sentence, not this test."
            )
    assert checked >= len(QUOTING), "every listed file must have contributed a claim"
