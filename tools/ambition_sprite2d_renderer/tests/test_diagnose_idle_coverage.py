"""Unit tests for the publish-time Idle-row check in
`tackon_sheet.diagnose_idle_coverage`.

The check warns when a sheet looks like a character sheet (≥1 row maps
to `CharacterAnim`) but no row is an Idle alias. The runtime's
`try_load_spec_for_character_id` would silently render it as a
colored-rectangle placeholder; this surfaces it at publish time."""

from __future__ import annotations

from ambition_sprite2d_renderer.authoring.tackon_sheet import diagnose_idle_coverage


def test_idle_row_present_returns_none():
    """Standard character sheet with an `idle` row is fine."""
    assert diagnose_idle_coverage("foo", ["idle", "walk", "death"]) is None


def test_idle_alias_rest_returns_none():
    """`rest` is one of the runtime's Idle aliases."""
    assert diagnose_idle_coverage("foo", ["rest", "walk", "hurt"]) is None


def test_idle_alias_front_idle_returns_none():
    """`front_idle` (girdle's facing-split sheet) is also an Idle alias."""
    assert diagnose_idle_coverage("foo", ["front_idle", "side_idle"]) is None


def test_no_character_rows_returns_none():
    """A sheet that doesn't look like a character (e.g. a prop sheet
    with custom row names) is not flagged."""
    assert (
        diagnose_idle_coverage("prop", ["opening", "closing"]) is None
    )  # opening IS idle alias, but matches our "any idle alias" criterion
    # Pure props with no recognized rows: no warning either.
    assert diagnose_idle_coverage("prop", ["spin_left", "spin_right"]) is None


def test_character_rows_without_idle_warns():
    """The actual case the check exists for: walk + death but no idle.
    The runtime would render this as a placeholder."""
    msg = diagnose_idle_coverage("foo", ["walk", "death", "hurt"])
    assert msg is not None
    assert "foo" in msg
    assert "Idle" in msg or "idle" in msg


def test_character_rows_with_taunt_only_warns():
    """taunt is a CharacterAnim but not an Idle alias — still triggers."""
    msg = diagnose_idle_coverage("npc_loner", ["taunt"])
    assert msg is not None
    assert "npc_loner" in msg


def test_galwah_pre_fix_would_have_warned():
    """Regression marker: galwah's "turn" row (pre-rename to "rest")
    was not a recognized CharacterAnim alias, so the sheet had zero
    character rows and the warning didn't fire — but every OTHER row
    (`walk`, `attack`, `cast`) was a character row. This test pins
    the diagnosis for the historical galwah row-set so a future
    misnaming trips the warning before runtime placeholder fallback."""
    # Pre-fix galwah rows (approximate; pre-fix sheet had walk + attack rows).
    pre_fix_rows = ["turn", "walk", "attack", "cast"]
    msg = diagnose_idle_coverage("galwah", pre_fix_rows)
    assert msg is not None
    # Post-fix: "turn" → "rest" (Idle alias) — no warning.
    post_fix_rows = ["rest", "walk", "attack", "cast"]
    assert diagnose_idle_coverage("galwah", post_fix_rows) is None
