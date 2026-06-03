#!/usr/bin/env python3
"""Regression tests for the LDtk editor-shaped JSON serializer.

``editor_format.dump_editor_style`` is a faithful port of Deepnight's
``dn.data.JsonPretty.stringify`` at level ``Compact`` — the serializer LDtk
itself uses to write ``.ldtk`` files. These tests pin the format-decision
rules straight from that algorithm (the ``evaluateLength`` heuristic vs the
85-char budget, the type-dependent array spacing, float ``.0`` suffix,
float-hint strings, string flattening, the ``"name" : {}`` empty-object
quirk, the >85 numeric grid, ``__header__`` multiline forcing) so a future
change to the formatter is reviewed against concrete examples rather than a
5000-line LDtk diff.

Run directly:
    PYTHONPATH=tools/ambition_ldtk_tools \\
      python tools/ambition_ldtk_tools/tests/test_editor_format.py
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.editor_format import dump_editor_style  # noqa: E402


def _eq(name: str, value, expected: str) -> None:
    text = dump_editor_style(value)
    if text != expected:
        print(f"FAIL {name}", file=sys.stderr)
        print(f"  expected: {expected!r}", file=sys.stderr)
        print(f"  got:      {text!r}", file=sys.stderr)
        sys.exit(1)
    print(f"  ok: {name}")


def _contains(name: str, value, needle: str) -> None:
    text = dump_editor_style(value)
    if needle not in text:
        print(f"FAIL {name}: substring not found", file=sys.stderr)
        print(f"  needle: {needle!r}", file=sys.stderr)
        print(f"  got:\n{text}", file=sys.stderr)
        sys.exit(1)
    print(f"  ok: {name}")


def _absent(name: str, value, needle: str) -> None:
    text = dump_editor_style(value)
    if needle in text:
        print(f"FAIL {name}: unexpected substring present", file=sys.stderr)
        print(f"  needle: {needle!r}", file=sys.stderr)
        print(f"  got:\n{text}", file=sys.stderr)
        sys.exit(1)
    print(f"  ok: {name}")


def main() -> int:
    print("== no trailing newline (LDtk writes buf verbatim) ==")
    _eq("scalar array, no trailing newline", [1, 2, 3], "[1,2,3]")
    _eq("empty array", [], "[]")
    _eq("empty object", {}, "{}")

    print("== array spacing is value-type dependent ==")
    # int/float arrays: NO inner space (matches `"px": [1080,874]`).
    _eq("int array no space", [1080, 874], "[1080,874]")
    _eq("float array no space + .0 suffix", [1.0, 2.5], "[1.0,2.5]")
    # bool arrays inline (spaced) only at length<=5; a >5 array trips the
    # evaluateLength `99` rule and goes multiline (so the inline ">5 -> no
    # space" branch in addArray is faithfully ported but unreachable).
    _eq("bool array <=5 spaced", [True, False], "[ true, false ]")
    _contains("bool array >5 multiline", [True] * 6, "[\n\ttrue,")
    # string/object arrays: no space at length 1, space when >1.
    _eq("string array length 1 unspaced", ["Solid"], '["Solid"]')
    _eq("string array length>1 spaced", ["a", "b"], '[ "a", "b" ]')

    print("== inline vs multiline objects (evaluateLength vs 85) ==")
    # Small object inlines with spaces inside the braces.
    _eq(
        "small object inline",
        {"value": 1, "identifier": "Solid"},
        '{ "value": 1, "identifier": "Solid" }',
    )
    # `realEditorValues: [{...}]` — a length-1 object array (no bracket
    # space) wrapping a multiline object, exactly as LDtk emits switch
    # field instances. The string-array `params` trips the 99 heuristic so
    # the inner object is multiline while the array stays inline.
    rev = {"realEditorValues": [{"id": "V_String", "params": ["FlipGravity"]}]}
    _contains("realEditorValues inline-array, multiline-object", rev,
              '"realEditorValues": [{\n')
    _absent("realEditorValues has no bracket space", rev, '[ {')

    print("== defaultOverride: numeric inline, string/bool multiline ==")
    # Numeric defaultOverride inlines (params [0] evaluates to length 1).
    _eq(
        "V_Float defaultOverride inline",
        {"id": "V_Float", "params": [0]},
        '{ "id": "V_Float", "params": [0] }',
    )
    # String defaultOverride goes multiline (string-array params -> 99).
    _contains(
        "V_String defaultOverride multiline",
        {"fieldDefs": [{"defaultOverride": {"id": "V_String", "params": ["Solid"]}}]},
        '"defaultOverride": {\n',
    )

    print("== float + float-hint formatting ==")
    _eq("whole float gets .0", {"a": 3.0}, '{ "a": 3.0 }')
    _eq("fractional float verbatim", {"a": 3.14}, '{ "a": 3.14 }')
    _eq("float-hint string -> number", {"a": "0f"}, '{ "a": 0.0 }')
    _eq("negative float-hint", {"a": "-65f"}, '{ "a": -65.0 }')

    print("== string flattening (newline/tab -> space, escape quote) ==")
    _eq("newline becomes space", {"d": "l1\nl2"}, '{ "d": "l1 l2" }')
    _eq("tab becomes space", {"d": "a\tb"}, '{ "d": "a b" }')
    _eq("carriage return removed", {"d": "a\r\nb"}, '{ "d": "a b" }')
    _eq("quote escaped", {"d": 'say "hi"'}, '{ "d": "say \\"hi\\"" }')

    print("== empty named object quirk ('name' : {}) ==")
    # JsonPretty's named-empty-object branch hardcodes spaces around the colon.
    _eq("named empty object spaced colon", {"x": {}}, '{ "x" : {} }')
    _eq("named empty array normal colon", {"x": []}, '{ "x": [] }')

    print("== numeric grid (>85 numbers, 35 per line) ==")
    grid = dump_editor_style([0] * 100)
    lines = grid.split("\n")
    assert lines[0] == "[", f"grid opens on its own line, got {lines[0]!r}"
    # First data row: 35 values, comma-separated, no spaces.
    first_row = lines[1].strip()
    assert first_row == ",".join(["0"] * 35) + ",", f"expected 35/row, got {first_row!r}"
    assert " " not in first_row, "numeric grid rows have no spaces"
    print("  ok: 35-per-line numeric grid, no inner spaces")

    print("== __header__ forces multiline even when short ==")
    header_doc = {"__header__": {"app": "LDtk"}, "iid": "x"}
    _contains("header value multiline", header_doc, '"__header__": {\n')

    print("== round-trip identity through json.loads ==")
    for sample in [
        [1, 2, 3],
        {"a": 1, "b": [True, False, None]},
        {"nested": {"deep": [{"x": 1}, {"y": 2}]}},
        {"empty_list": [], "empty_dict": {}, "string": "hello world"},
        {"px": [1080, 874], "realEditorValues": [{"id": "V_String", "params": ["x"]}]},
    ]:
        restored = json.loads(dump_editor_style(sample))
        if restored != sample:
            print(f"FAIL round-trip:\n  sent: {sample}\n  got:  {restored}", file=sys.stderr)
            return 1
    print("  ok: round-trip identity")

    print("\nall editor-format tests passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
