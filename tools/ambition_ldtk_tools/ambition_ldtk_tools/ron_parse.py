"""RON parse + dump helpers for Ambition's Python tooling.

Uses the upstream `python-ron` PyPI package (module: `pyron`) which
wraps the actual Rust `ron` crate — so anything the Rust side accepts
parses here without behavioural drift.

For *output*, we keep an in-house `dumps` that prefers idiomatic
struct syntax — `(field: value)` with unquoted field names — over
`pyron.to_string`'s map syntax — `{"field": value}`. The struct
style matches the rest of the project's hand-authored RON files
(`character_catalog.ron`, `sandbox.ron`).

## Install

```
pip install --user --break-system-packages python-ron
```

(The package's compiled module installs at `pyron/`; the older
Python-2 `pyron-0.2` package is incompatible and must be uninstalled
first if both ever land in the same site-packages.)
"""
from __future__ import annotations

from typing import Any

try:
    import pyron  # type: ignore
except ImportError as ex:  # pragma: no cover
    raise SystemExit(
        "ron_parse requires the python-ron PyPI package. Install with:\n"
        "  pip install --user --break-system-packages python-ron\n"
        f"original error: {ex}"
    )


class RonParseError(Exception):
    """Raised when a RON document fails to parse."""


def load(text: str) -> Any:
    """Parse a RON document. Returns plain Python dict / list /
    str / int / float / bool / None values — a drop-in replacement
    for `yaml.safe_load` on the same data tree."""
    try:
        return pyron.loads(text)
    except Exception as ex:
        raise RonParseError(str(ex)) from ex


# ---------- YAML/JSON → RON serializer ----------
#
# Output style: struct syntax `(field: value)` for dicts whose keys
# are all valid identifiers; map syntax `{"key": value}` otherwise.
# Lists go on multiple lines for readability. Strings use double-
# quotes with the usual escapes (\\, \", \n, \t, \r).


def _dump_value(value: Any, indent: int, indent_step: int) -> str:
    pad = " " * indent
    inner_pad = " " * (indent + indent_step)
    if isinstance(value, bool):
        return "true" if value else "false"
    if value is None:
        return "None"
    if isinstance(value, (int, float)):
        return repr(value)
    if isinstance(value, str):
        esc = (
            value.replace("\\", "\\\\")
            .replace('"', '\\"')
            .replace("\n", "\\n")
            .replace("\r", "\\r")
            .replace("\t", "\\t")
        )
        return f'"{esc}"'
    if isinstance(value, list):
        if not value:
            return "[]"
        items = ",\n".join(
            f"{inner_pad}{_dump_value(v, indent + indent_step, indent_step)}"
            for v in value
        )
        return "[\n" + items + f",\n{pad}]"
    if isinstance(value, dict):
        if not value:
            # Empty dict → `{}` (Map syntax). RON's `()` is the unit
            # type / empty struct, which a `HashMap` field can't
            # accept — that's how `synth_boss_manifest`'s empty
            # `anchors: {}` produced `anchors: ()` and broke the
            # gnu_ton_boss / mockingbird_boss sheet parse for weeks.
            # `{}` parses cleanly as an empty `HashMap` and also as
            # an empty struct-with-defaults.
            return "{}"
        all_idents = all(
            isinstance(k, str)
            and k
            and (k[0].isalpha() or k[0] == "_")
            and all(c.isalnum() or c == "_" for c in k)
            for k in value.keys()
        )
        if all_idents:
            items = ",\n".join(
                f"{inner_pad}{k}: {_dump_value(v, indent + indent_step, indent_step)}"
                for k, v in value.items()
            )
            return "(\n" + items + f",\n{pad})"
        items = ",\n".join(
            f"{inner_pad}{_dump_value(k, indent + indent_step, indent_step)}: "
            f"{_dump_value(v, indent + indent_step, indent_step)}"
            for k, v in value.items()
        )
        return "{\n" + items + f",\n{pad}}}"
    raise RonParseError(f"unsupported value type: {type(value).__name__}")


def dumps(value: Any, indent: int = 4) -> str:
    """Serialize a Python value to RON. Returns a single string with
    a trailing newline."""
    return _dump_value(value, 0, indent) + "\n"
