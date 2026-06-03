#!/usr/bin/env python3
"""Editor-shaped JSON serializer for Ambition LDtk files.

This is a faithful Python port of Deepnight's ``dn.data.JsonPretty.stringify``
(the serializer LDtk itself uses to write project files), level ``Compact`` —
the level LDtk saves with. Upstream source:
``deepnightLibs/src/dn/data/JsonPretty.hx`` (github.com/deepnight/deepnightLibs);
LDtk's save entrypoint is ``ProjectSaver.jsonStringify`` in the ldtk repo.

Porting the real algorithm (rather than approximating it) is what makes
tool-written files diff cleanly against GUI-saved files: a round-trip
(``dump_editor_style(json.load(f))``) reproduces an editor-saved file
byte-for-byte, so a tool that merely appends an entity produces a diff of only
that entity instead of tens of thousands of reformatting lines.

The two rules that an approximation always gets wrong, and that the real
algorithm encodes:

- Inline-vs-multiline is decided by a cheap *heuristic length*
  (:func:`_evaluate_length`) compared against ``APPROXIMATE_MAX_LINE_LENGTH``
  (85), NOT the rendered string width. The heuristic charges 99 for "deep or
  long" arrays, which is why e.g. a ``defaultOverride`` whose ``params`` is a
  string array (``["Solid"]``) wraps while a numeric one (``[0]``) inlines.
- Array bracket spacing is value-type dependent: int/float arrays get no inner
  space (``[1,2]``), bool arrays of <=5 and object/string arrays of length>1
  get one (``[ true ]`` / ``[ {..}, {..} ]``).

Tools that mutate the LDtk file call :func:`dump_editor_style` so their output
matches the editor's.
"""
from __future__ import annotations

import json
import re

# dn.data.JsonPretty.APPROXIMATE_MAX_LINE_LENGTH
APPROXIMATE_MAX_LINE_LENGTH = 85
HEADER_VALUE_NAME = "__header__"
INDENT = "\t"

# dn.data.JsonPretty.floatReg — numbers stored as "<num>f" strings are emitted
# as floats (keeps int/float distinction in JSON).
_FLOAT_HINT_RE = re.compile(r"^[-0-9.]+f$")


def _typeof(v) -> str:
    # bool must be checked before int (Python bool is an int subclass).
    if v is None:
        return "null"
    if isinstance(v, bool):
        return "bool"
    if isinstance(v, int):
        return "int"
    if isinstance(v, float):
        return "float"
    if isinstance(v, str):
        return "string"
    if isinstance(v, list):
        return "array"
    if isinstance(v, dict):
        return "object"
    raise ValueError(f"Unsupported JSON value type: {type(v)!r}")


def _float_str(v: float) -> str:
    """Mirror JsonPretty's TFloat handling: integer-valued floats render as
    ``N.0``; others use the shortest round-trip repr (matches Haxe Std.string
    for the simple floats LDtk stores)."""
    if v == int(v):
        return f"{int(v)}.0"
    return repr(v)


def _evaluate_length(v, depth: int = 0) -> int:
    """Port of ``JsonPretty.evaluateLength`` — a cheap heuristic 'how long would
    this be inline' used only to pick single- vs multi-line; NOT the real width.
    """
    t = _typeof(v)
    if t == "null":
        return 4
    if t == "int":
        return 4
    if t == "float":
        return 5
    if t == "bool":
        return 4 if v else 5
    if t == "string":
        return len(v) + 2
    if t == "object":
        keys = list(v.keys())
        if depth <= 0 and len(keys) <= 5:
            return sum(_evaluate_length(v[k], depth + 1) for k in keys)
        return len(keys) * 10
    if t == "array":
        if len(v) == 0:
            return 2
        if 0 < len(v) < 50 and _typeof(v[0]) in ("int", "float"):
            return len(v)
        if len(v) > 5 or depth > 0:
            return 99
        return sum(_evaluate_length(e, depth + 1) for e in v)
    return 1


class _Printer:
    """Port of the stateful JsonPretty serializer (Compact level)."""

    def __init__(self) -> None:
        self.buf: list[str] = []
        self.indent = 0

    def _add_indent(self) -> None:
        if self.indent:
            self.buf.append(INDENT * self.indent)

    def add_value(self, name, v, force_multilines: bool = False) -> None:
        if name == HEADER_VALUE_NAME:
            force_multilines = True

        t = _typeof(v)
        if t in ("null", "int", "bool"):
            token = "null" if v is None else ("true" if v is True else ("false" if v is False else str(v)))
            self.buf.append(token if name is None else f'"{name}": {token}')
        elif t == "float":
            sf = _float_str(v)
            self.buf.append(sf if name is None else f'"{name}": {sf}')
        elif t == "string":
            if _FLOAT_HINT_RE.match(v):
                sf = _float_str(float(v[:-1]))
                self.buf.append(sf if name is None else f'"{name}": {sf}')
            else:
                s = (
                    v.replace("\n", " ")
                    .replace("\r", "")
                    .replace("\t", " ")
                    .replace('"', '\\"')
                )
                self.buf.append(f'"{s}"' if name is None else f'"{name}": "{s}"')
        elif t == "array":
            self.add_array(name, v, force_multilines)
        else:
            self.add_object(name, v, force_multilines)

    def add_object(self, name, o: dict, force_multilines: bool = False) -> None:
        keys = list(o.keys())
        if not keys:
            # JsonPretty's empty-object branch uses a space BEFORE the colon
            # (an asymmetry vs the empty-array branch); reproduced exactly.
            self.buf.append("{}" if name is None else f'"{name}" : {{}}')
            return

        length = _evaluate_length(o)
        if name is not None:
            self.buf.append(f'"{name}": ')

        if length <= APPROXIMATE_MAX_LINE_LENGTH and not force_multilines:
            self.buf.append("{ ")
            for i, k in enumerate(keys):
                self.add_value(k, o[k])
                if i < len(keys) - 1:
                    self.buf.append(", ")
            self.buf.append(" }")
        else:
            self.buf.append("{\n")
            self.indent += 1
            for i, k in enumerate(keys):
                self._add_indent()
                self.add_value(k, o[k])
                if i < len(keys) - 1:
                    self.buf.append(",\n")
            self.indent -= 1
            self.buf.append("\n")
            self._add_indent()
            self.buf.append("}")

    def add_array(self, name, arr: list, force_multilines: bool = False) -> None:
        if not arr:
            self.buf.append("[]" if name is None else f'"{name}": []')
            return

        length = _evaluate_length(arr)
        if name is not None:
            self.buf.append(f'"{name}": ')

        first_type = _typeof(arr[0])

        if (
            first_type in ("int", "float")
            and not force_multilines
            and len(arr) > APPROXIMATE_MAX_LINE_LENGTH
        ):
            # Number grid: 35 values per line, no space after comma (CSV-style).
            self.buf.append("[\n")
            self.indent += 1
            self._add_indent()
            line_limit = 0
            for i, item in enumerate(arr):
                self.add_value(None, item)
                if i < len(arr) - 1:
                    self.buf.append(",")
                line_limit += 1
                if line_limit >= 35:
                    self.buf.append("\n")
                    self._add_indent()
                    line_limit = 0
            self.indent -= 1
            self.buf.append("\n")
            self._add_indent()
            self.buf.append("]")
        elif length <= APPROXIMATE_MAX_LINE_LENGTH and not force_multilines:
            if first_type in ("int", "float"):
                sp = ""
            elif first_type == "bool":
                sp = "" if len(arr) > 5 else " "
            else:
                sp = "" if len(arr) == 1 else " "
            self.buf.append(f"[{sp}")
            for i, item in enumerate(arr):
                self.add_value(None, item)
                if i < len(arr) - 1:
                    self.buf.append(f",{sp}")
            self.buf.append(f"{sp}]")
        else:
            self.buf.append("[\n")
            self.indent += 1
            for i, item in enumerate(arr):
                self._add_indent()
                self.add_value(None, item)
                if i < len(arr) - 1:
                    self.buf.append(",\n")
            self.indent -= 1
            self.buf.append("\n")
            self._add_indent()
            self.buf.append("]")


def dump_editor_style(value) -> str:
    """Render ``value`` (the parsed LDtk project) in editor-shaped JSON, matching
    ``dn.data.JsonPretty.stringify(value, Compact)``. LDtk writes no trailing
    newline, so neither do we (``JsonPretty.stringify`` returns ``buf.toString()``
    verbatim and ``ProjectSaver`` writes it as-is)."""
    p = _Printer()
    p.add_value(None, value)
    return "".join(p.buf)


def main(argv: list[str] | None = None) -> int:
    """CLI shim: read a JSON file, re-emit in editor style (in place or to
    ``--output``). Useful for canonicalizing a file against the editor format."""
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("path", help="JSON file to reformat in place")
    parser.add_argument("--output", default=None, help="Write here instead of in place")
    args = parser.parse_args(argv)

    with open(args.path, "r", encoding="utf-8") as f:
        value = json.load(f)
    text = dump_editor_style(value)
    with open(args.output or args.path, "w", encoding="utf-8") as f:
        f.write(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
