#!/usr/bin/env python3
"""Regression tests for the LDtk editor-shaped JSON serializer.

The formatter aims to reproduce the LDtk 1.5 editor's idiosyncratic
mixed inline/multi-line JSON layout closely enough that tool-edited
files diff cleanly against editor-saved files. These tests pin the
format-decision rules (inline thresholds, scalar-array chunking,
``{ ... }``-vs-``[...]`` spacing asymmetry, first-key flow,
closing-line key flow) so future formatter changes can be reviewed
against concrete examples instead of staring at a 5000-line diff.

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


def _check(name: str, value, expected_substring: str) -> None:
    text = dump_editor_style(value)
    if expected_substring not in text:
        print(f"FAIL {name}: expected substring not found", file=sys.stderr)
        print(f"  expected: {expected_substring!r}", file=sys.stderr)
        print(f"  got:\n{text}", file=sys.stderr)
        sys.exit(1)
    print(f"  ok: {name}")


def _check_exact(name: str, value, expected: str) -> None:
    text = dump_editor_style(value)
    if text != expected:
        print(f"FAIL {name}", file=sys.stderr)
        print(f"  expected:\n{expected}", file=sys.stderr)
        print(f"  got:\n{text}", file=sys.stderr)
        sys.exit(1)
    print(f"  ok: {name}")


def main() -> int:
    print("== inline-spacing rules ==")
    # Numbers in arrays: NO space after comma (matches `__grid: [58,51]`).
    _check("scalar array no inner space", [1, 2, 3], "[1,2,3]")
    # Object keys: SPACE after `,` and inside `{ }` (matches
    # `{ "value": 1, "identifier": "Solid" }`).
    _check(
        "object inline has spaces inside braces",
        {"value": 1, "identifier": "Solid"},
        '{ "value": 1, "identifier": "Solid" }',
    )
    # Empty containers always inline.
    _check("empty array", [], "[]\n")
    _check("empty object", {}, "{}\n")

    print("== width-driven wrap ==")
    # Short scalar array stays inline.
    _check_exact("short scalar array stays inline", [1, 2, 3], "[1,2,3]\n")
    # Long scalar array wraps, packing items at ~76-char width budget.
    long_array = [0] * 1000
    text = dump_editor_style(long_array)
    assert "\n" in text, "long array should wrap"
    chunk_lines = [
        line for line in text.split("\n") if line and not line.strip() in {"[", "]"}
    ]
    assert chunk_lines, "long array should produce chunk lines"
    # Each chunk line should be reasonably packed (more than 30 chars
    # of payload). One-int-per-line would be a regression.
    short_chunks = [line for line in chunk_lines if len(line) < 30]
    assert not short_chunks, (
        f"long scalar array should chunk densely, found short lines: {short_chunks[:3]}"
    )
    print("  ok: long scalar array packs densely")

    print("== first-key flow ==")
    # When a wrapping dict's first value is itself a wrapping
    # container, the first key flows onto the dict opener line.
    nested = {
        "outer": {
            "layers": [{"name": "a"}, {"name": "b"}, {"name": "c"}, {"name": "d"}],
            "tilesets": [],
        }
    }
    text = dump_editor_style(nested)
    # Expect `"outer": { "layers": [` on the same line, NOT
    # `"outer": {` and `"layers": [` on separate lines.
    if '"outer": { "layers": [' not in text:
        print(f"FAIL first-key flow: missing flow opener.\nGot:\n{text}", file=sys.stderr)
        return 1
    print("  ok: first-key flow on `{ \"key\": [`")

    print("== closing-line key flow ==")
    # After a multi-line child closes, the next key may flow onto
    # the closing line if it fits.
    flow_target = {
        "outer": {
            "layers": [{"name": "a"}, {"name": "b"}, {"name": "c"}],
            "entities": [{"id": "x"}, {"id": "y"}, {"id": "z"}],
        }
    }
    text = dump_editor_style(flow_target)
    # Expect `], "entities": [` on a single line somewhere.
    if '], "entities": [' not in text:
        print(f"FAIL closing-line flow.\nGot:\n{text}", file=sys.stderr)
        return 1
    print("  ok: `], \"entities\": [` flow on closing line")

    print("== top-level dict ==")
    # Top-level dict opens `{` on its own line (no flow-pack at
    # depth 0), when its inline form would exceed the width budget.
    # A dict whose first child is itself big enough to wrap.
    big_first_child = {f"key_{i}": "x" * 30 for i in range(20)}
    tl = {"first_big": big_first_child, "second": "tail"}
    text = dump_editor_style(tl)
    first_line = text.split("\n", 1)[0]
    if first_line != "{":
        print(
            f"FAIL top-level open: expected `{{` alone, got `{first_line}`",
            file=sys.stderr,
        )
        return 1
    print("  ok: top-level `{` on its own line")


    print("== LDtk defaultOverride stability ==")
    # Existing LDtk field definitions keep defaultOverride as a
    # multiline object. The schema helpers rewrite the whole file, so
    # compacting these values would cause large unrelated diffs such as:
    #   "defaultOverride": { "id": "V_String", "params": ["Solid"] }
    # instead of the editor-stable multiline form below.
    field_def = {
        "fieldDefs": [
            {
                "identifier": "kind",
                "doc": "",
                "defaultOverride": {"id": "V_String", "params": ["Solid"]},
                "textLanguageMode": None,
            }
        ]
    }
    text = dump_editor_style(field_def)
    if '"defaultOverride": { "id": "V_String"' in text:
        print(f"FAIL defaultOverride compacted.\nGot:\n{text}", file=sys.stderr)
        return 1
    expected = '\t\t\t"defaultOverride": {\n\t\t\t\t"id": "V_String",\n\t\t\t\t"params": ["Solid"]\n\t\t\t}'
    if expected not in text:
        print(f"FAIL defaultOverride multiline.\nExpected substring:\n{expected}\nGot:\n{text}", file=sys.stderr)
        return 1
    print("  ok: defaultOverride stays multiline")

    print("== round-trip equivalence ==")
    # Anything we serialize should round-trip back to the same Python
    # value via json.loads.
    for sample in [
        [1, 2, 3],
        {"a": 1, "b": [True, False, None]},
        {"nested": {"deep": [{"x": 1}, {"y": 2}]}},
        {"empty_list": [], "empty_dict": {}, "string": "hello world"},
    ]:
        text = dump_editor_style(sample)
        restored = json.loads(text)
        if restored != sample:
            print(
                f"FAIL round-trip:\n  sent: {sample}\n  got:  {restored}",
                file=sys.stderr,
            )
            return 1
    print("  ok: round-trip identity")

    print("\nall editor-format tests passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
