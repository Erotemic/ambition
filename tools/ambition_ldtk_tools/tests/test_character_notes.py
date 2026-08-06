"""The sprite-target -> catalog notes join.

These pin the two properties that make the join safe to run unattended: it
normalizes shapes the targets genuinely disagree about, and it never turns a
missing field into a silent empty one.
"""

from ambition_ldtk_tools.character_notes import (
    CharacterNotes,
    fallback_pool,
    flatten_prose,
    notes_from_actor_metadata,
    render_notes_ron,
)


def test_prose_flattens_from_a_string_a_dict_or_nothing():
    assert flatten_prose("Already prose") == "Already prose."
    assert flatten_prose(None) == ""
    # Ordered keys lead, unrecognized keys still make it through rather than
    # being dropped on the floor.
    text = flatten_prose(
        {
            "invented_key": "Carried anyway",
            "core_joke": "The joke",
            "parody_of": "Someone Real",
        }
    )
    assert text.startswith("Parodies: Someone Real. The joke.")
    assert "Carried anyway." in text


def test_prose_flattening_is_deterministic():
    metadata = {"design_notes": ["a", "b"], "role": "caster", "z_extra": "tail"}
    assert flatten_prose(metadata) == flatten_prose(dict(reversed(list(metadata.items()))))


def test_suggested_barks_lead_the_fallback_pool():
    pool = fallback_pool(
        {
            "fallback_dialogue": ["A longer conversational line.", "Careful."],
            "suggested_barks": ["Careful.", "Let it simmer."],
        }
    )
    # Short barks first (rotation 0 is the line a player hears first), and a
    # line authored in both places appears once.
    assert pool == ("Careful.", "Let it simmer.", "A longer conversational line.")


def test_an_unrecognized_dialogue_key_is_carried_not_dropped():
    """⛔ **the regression that shipped mute lines.** `fallback_pool` read three
    hard-coded spellings while `flatten_prose`, one function above it, already
    carried unknown keys through. `patent_clerk.py` and `python_goras.py` spell
    theirs `fallback_lines`, so six authored lines each were read as nothing —
    and because both targets also author `barks`, the pool came back non-empty
    and `missing_fields` reported no gap. Silence, not a failure.

    The probe is a spelling nobody has used, so it cannot pass by being added to
    a list.
    """
    pool = fallback_pool(
        {
            "barks": ["Short."],
            "fallback_lines": ["The spelling two targets actually use."],
            "utterly_novel_key": ["Carried anyway."],
        }
    )
    assert pool == (
        "Short.",
        "The spelling two targets actually use.",
        "Carried anyway.",
    )


def test_the_dialogue_pool_does_not_depend_on_key_insertion_order():
    hints = {"z_late": ["z"], "barks": ["b"], "a_early": ["a"]}
    assert fallback_pool(hints) == fallback_pool(dict(reversed(list(hints.items()))))


def test_missing_fields_are_named_not_defaulted():
    notes = notes_from_actor_metadata(
        {"actor": {"character_id": "npc_quiet", "display_name": "Quiet"}}
    )
    assert notes.character_id == "npc_quiet"
    assert notes.missing_fields() == (
        "authoring_description",
        "gameplay_description",
        "fallback_dialogue",
    )
    # A character with nothing to say emits no fields at all, so splicing it
    # cannot overwrite hand-written catalog prose with blanks.
    assert render_notes_ron(notes) == ""


def test_rendered_ron_escapes_quotes():
    notes = CharacterNotes(
        character_id="npc_x",
        display_name="X",
        fallback_dialogue=('He said "no".',),
    )
    assert '\\"no\\"' in render_notes_ron(notes)


def test_splice_is_idempotent_and_never_rewrites_an_existing_row(tmp_path):
    from ambition_ldtk_tools.character_notes import SPLICE_ANCHOR, splice_rows

    catalog = tmp_path / "character_catalog.ron"
    catalog.write_text(
        '        "npc_already": (\n'
        '            display_name: "Hand Edited",\n'
        "        ),\n" + SPLICE_ANCHOR + "\n"
    )
    rows = {
        "npc_already": '        "npc_already": (\n            display_name: "Generated",\n        ),\n',
        "npc_new": '        "npc_new": (\n            display_name: "New",\n        ),\n',
    }
    assert splice_rows(catalog, rows) == ["npc_new"]
    text = catalog.read_text()
    # The hand edit survives; the generator does not get to overwrite authoring.
    assert "Hand Edited" in text and "Generated" not in text
    assert text.count('"npc_new": (') == 1

    # Re-running adds nothing.
    assert splice_rows(catalog, rows) == []
    assert catalog.read_text() == text


def test_splice_refuses_without_a_unique_anchor(tmp_path):
    import pytest

    from ambition_ldtk_tools.character_notes import splice_rows

    catalog = tmp_path / "character_catalog.ron"
    catalog.write_text("no anchor here\n")
    with pytest.raises(ValueError):
        splice_rows(catalog, {"npc_new": "row\n"})
