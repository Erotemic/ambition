"""Round-trip tests for `ron_parse.{load,dumps}` — the in-house
RON helpers used across the python tooling.

Tests focus on the parse↔dump invariant: `load(dumps(x)) == x` for
the data shapes we actually serialize."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.ron_parse import dumps, load  # noqa: E402


def test_round_trip_primitive_values():
    samples = [
        True,
        False,
        None,
        0,
        1,
        -42,
        999999,
        0.0,
        1.5,
        -2.71,
        3.14159,
        "",
        "hello",
        "with space",
    ]
    for value in samples:
        # Single primitives serialize as bare values; wrap in a list
        # for top-level test consistency.
        wrapped = [value]
        assert load(dumps(wrapped)) == wrapped, f"round-trip failed for {value!r}"


def test_round_trip_string_with_escapes():
    samples = [
        'has "quotes"',
        "has \\backslash",
        "has\nnewline",
        "has\ttab",
        "has\rcr",
    ]
    for value in samples:
        wrapped = [value]
        assert load(dumps(wrapped)) == wrapped, (
            f"escape round-trip failed for {value!r}"
        )


def test_round_trip_dict_with_identifier_keys():
    data = {
        "id": "alice_relay",
        "level_id": "alice_relay",
        "world_x": 4928,
        "world_y": 1024,
        "px_wid": 1024,
        "px_hei": 768,
        "fill_collision": "empty",
        "bg_color": "#11161E",
    }
    out = dumps(data)
    # Identifier-key dicts use struct syntax `(key: value)`.
    assert out.lstrip().startswith("("), out[:40]
    assert load(out) == data


def test_round_trip_dict_with_quoted_keys():
    """Keys that aren't valid identifiers fall back to map syntax."""
    data = {
        "valid_ident": 1,
        "1starts-with-digit": 2,
    }
    # `dumps` falls back to map syntax when any key isn't an ident.
    out = dumps(data)
    assert out.lstrip().startswith("{"), out[:40]
    # Round-trip preserves contents.
    assert load(out) == data


def test_round_trip_nested_list_and_dict():
    data = {
        "entities": [
            {"type": "Solid", "px": [0, 0], "size": [100, 16]},
            {"type": "Solid", "px": [0, 100], "size": [100, 16]},
            {"type": "PlayerStart", "px": [50, 50], "fields": {"name": "spawn"}},
        ],
        "metadata": {"biome": "cave", "music": "ambient_1"},
    }
    out = dumps(data)
    assert load(out) == data


def test_load_handles_comments():
    text = """\
// Top-level comment
(
    // Inline comment before a field
    foo: 1,
    bar: 2,
    /* Block comment */
    baz: "hello",
)
"""
    data = load(text)
    assert data == {"foo": 1, "bar": 2, "baz": "hello"}


def test_load_handles_some_wrapper():
    text = """\
(
    melee: Some(Swipe(damage: 1, reach_px: 28.0)),
    ranged: None,
)
"""
    data = load(text)
    # pyron returns `Some(...)`'s payload as the dict directly (the
    # variant tag is lost — see [[feedback-pyron-unit-variants]]).
    assert data["ranged"] is None
    # The melee branch returns either the inner dict (variant tag
    # lost) or a single-item dict — both are acceptable for runs
    # where the variant name doesn't matter.
    assert data["melee"] is not None


def test_empty_dict_dumps_as_map_not_unit():
    """Regression: `dumps({})` used to emit `()` (the RON unit type),
    which the runtime's `ron::from_str` rejected as a value for any
    `HashMap` field. The `synth_boss_manifest` script emitted
    `anchors: {}` (empty per-frame anchors) → `anchors: ()` → boss
    subdir manifests silently failed to parse → Hall pedestals for
    gnu_ton + mockingbird rendered as colored-rectangle placeholders
    for weeks (caught 2026-05-24)."""
    out = dumps({})
    assert "{}" in out, f"empty dict should serialize as `{{}}`, got: {out!r}"
    assert "()" not in out, f"empty dict must not serialize as `()`, got: {out!r}"
    # And it round-trips back to an empty dict.
    # `load` on `{}` returns an empty dict.
    assert load(out) == {}


def test_dict_containing_empty_dict_field_dumps_correctly():
    """The actual shape that broke: a dict with an `anchors` field
    that is empty must serialize so the parent struct lands as
    `(anchors: {})` (HashMap value), not `(anchors: ())`."""
    out = dumps({"x": 0, "anchors": {}})
    # The inner empty dict must be `{}` not `()`.
    assert "anchors: {}" in out, f"empty anchors field should be {{}}, got: {out!r}"


def test_load_handles_trailing_commas():
    text = """\
(
    a: 1,
    b: 2,
    c: [
        "x",
        "y",
        "z",
    ],
)
"""
    data = load(text)
    assert data == {"a": 1, "b": 2, "c": ["x", "y", "z"]}
