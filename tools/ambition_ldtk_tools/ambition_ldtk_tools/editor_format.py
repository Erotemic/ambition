#!/usr/bin/env python3
"""Editor-shaped JSON serializer for Ambition LDtk files.

The LDtk 1.5 editor writes JSON with a width-aware mixed layout:

- Empty arrays / objects inline as ``[]`` / ``{}``.
- Containers whose inline form fits in a width budget inline
  (``[1, 2]``, ``{ "value": 1, "identifier": "Solid", ... }``).
- Containers whose inline form would exceed the budget wrap to
  multi-line; their children recurse with the budget rule applied
  at the new indent depth.
- Asymmetric brace spacing: dict braces have a space inside
  (``{ "a": 1 }``) but list brackets do not (``[1, 2]``).
- When a wrapping dict's first value is itself a wrapping container,
  the first key flows onto the opener line:
  ``"defs": { "layers": [`` instead of putting ``{`` on its own
  line.

Stock ``json.dumps(indent=...)`` produces a fully-expanded output
that diffs against an editor-saved file as ~30k lines of pure
formatting noise. Stock single-line ``json.dumps`` collapses every
container onto one line. This module bridges that gap.

Tools that mutate the LDtk file should call :func:`dump_editor_style`
instead of ``json.dumps`` so future mutations diff cleanly against
files produced by the LDtk GUI.
"""
from __future__ import annotations

import json

# Width budget (characters per line, counting tabs as one char).
# The LDtk 1.5 editor inlines lines up to ~240 chars; longer lines
# are pushed to multi-line. We mirror that.
MAX_WIDTH = 240

INDENT = "\t"


def _inline(value) -> str:
    """Render a JSON value as a compact single-line string in the
    LDtk editor's idiosyncratic spacing convention:

    - Dicts gain an extra space inside braces: ``{ "a": 1 }``.
    - Arrays of scalars use NO space between elements: ``[1,2,3]``.
    - Arrays containing any object element gain spaces: ``[ {...},
      {...} ]`` (rare; LDtk usually wraps these to multi-line).

    The asymmetry shows up in things like ``"__grid": [58,51]`` and
    ``"px": [936,828]`` (no spaces) versus ``"params": [96]`` inside
    a dict (space after colon, but the array is scalar so no
    internal space).
    """
    if isinstance(value, dict):
        if not value:
            return "{}"
        parts = [
            f"{json.dumps(k, ensure_ascii=False)}: {_inline(v)}"
            for k, v in value.items()
        ]
        return "{ " + ", ".join(parts) + " }"
    if isinstance(value, list):
        if not value:
            return "[]"
        has_object = any(isinstance(item, dict) for item in value)
        if has_object:
            return "[ " + ", ".join(_inline(item) for item in value) + " ]"
        return "[" + ",".join(_inline(item) for item in value) + "]"
    return json.dumps(value, ensure_ascii=False)


def _render(
    value,
    depth: int,
    first_line_prefix_len: int,
    *,
    force_wrap: bool = False,
) -> str:
    """Render ``value`` with ``first_line_prefix_len`` chars already
    used on its first line by the caller (parent indent + key + `: `).
    Returns text starting at column ``first_line_prefix_len``.

    ``force_wrap`` is used for LDtk editor fields whose existing on-disk
    layout is stable even when the nested object would fit inline. The
    important case is field-definition ``defaultOverride`` values: LDtk
    leaves long-lived definitions in a multiline form, and compacting those
    objects causes large, unrelated schema diffs when a tool merely appends
    a new field.
    """
    if not isinstance(value, (dict, list)) or not value:
        # Scalars and empty containers always inline.
        return _inline(value)
    inline = _inline(value)
    width_ok = first_line_prefix_len + len(inline) <= MAX_WIDTH
    # The LDtk editor always wraps arrays-of-multiple-objects, even
    # when they would fit on a line (`intGridValues` with 2 entries
    # at ~230 chars wraps in the canonical despite fitting under
    # the width budget). Mirror that here.
    if (
        width_ok
        and not force_wrap
        and not (isinstance(value, list) and _is_multi_object_array(value))
        and not _contains_forced_wrap_child(value)
    ):
        return inline
    if isinstance(value, list):
        return _render_list(value, depth)
    return _render_dict(value, depth, first_line_prefix_len)


def _contains_forced_wrap_child(value) -> bool:
    """Return true when inline rendering would compact a child that
    should stay multiline for LDtk diff stability.

    The important case is ``defaultOverride`` in field definitions. The
    check is recursive so ancestors do not inline and bypass the special
    child renderer. LDtk files are shallow enough that this small traversal is
    cheap, and it only affects layout choices, not parsed values.
    """
    if isinstance(value, dict):
        if isinstance(value.get("defaultOverride"), dict):
            return True
        return any(_contains_forced_wrap_child(child) for child in value.values())
    if isinstance(value, list):
        return any(_contains_forced_wrap_child(item) for item in value)
    return False


def _is_multi_object_array(value: list) -> bool:
    """True if ``value`` contains two or more JSON objects (dicts).

    Single-object arrays inline freely (``[{ ... }]``); two-or-more
    object arrays always wrap, with each object on its own line.
    """
    return sum(1 for item in value if isinstance(item, dict)) >= 2


def _render_list(value, depth: int) -> str:
    """Wrap a list multi-line.

    For all-scalar arrays (the common case for ``intGridCsv``,
    ``__neighbours`` lists, level field arrays, etc.), pack many
    items per line up to ~80 chars; this matches the LDtk editor's
    "35 ones per line" intGridCsv layout closely enough to keep the
    diff small even on the long collision arrays.

    For object / nested-array elements, each item lands on its own
    line.
    """
    pad = INDENT * (depth + 1)
    if _all_scalar(value):
        return _render_scalar_list(value, depth, pad)
    last = len(value) - 1
    lines = ["["]
    for i, item in enumerate(value):
        suffix = "" if i == last else ","
        item_text = _render(item, depth + 1, depth + 1)
        lines.append(f"{pad}{item_text}{suffix}")
    lines.append(f"{INDENT * depth}]")
    return "\n".join(lines)


def _all_scalar(value) -> bool:
    """True if every element is a JSON scalar (number, bool, null,
    string). Lists/dicts among elements force per-item line layout.
    """
    return all(not isinstance(item, (dict, list)) for item in value)


# Soft target line width for chunked scalar arrays. The LDtk editor
# wraps `intGridCsv` data at ~76 chars per line (6 tabs + 35 `1,`).
# Matching that target keeps the on-disk diff against editor-saved
# files small even on the long collision arrays.
SCALAR_CHUNK_TARGET_WIDTH = 76


def _render_scalar_list(value, depth: int, pad: str) -> str:
    """Render a long scalar list with multiple items per line, packed
    up to ``SCALAR_CHUNK_TARGET_WIDTH`` chars per line.
    """
    items = [_inline(item) for item in value]
    if not items:
        return "[]"
    lines = ["["]
    current = pad
    is_first_on_line = True
    for i, item_text in enumerate(items):
        suffix = "" if i == len(items) - 1 else ","
        token = item_text + suffix
        # Will adding this token push current past target width?
        candidate = current + token if is_first_on_line else current + token
        sep = "" if is_first_on_line else ""
        # Try appending after a separator (we put no space after `,`
        # to match LDtk's CSV-style packing).
        prospective = current + token if is_first_on_line else current + token
        if len(prospective) <= SCALAR_CHUNK_TARGET_WIDTH or is_first_on_line:
            current += token
            is_first_on_line = False
        else:
            lines.append(current)
            current = pad + token
            is_first_on_line = False
    lines.append(current)
    lines.append(f"{INDENT * depth}]")
    return "\n".join(lines)


def _render_dict(value, depth: int, first_line_prefix_len: int) -> str:
    """Wrap a dict multi-line.

    Heuristic: when the dict's first value is itself a wrapping
    container, flow the first key onto the dict opener line:

        { "first_key": <first_value_first_line>
          ...first_value body...
        ],
        "next_key": ...,
        ...
        }

    This matches the editor's ``"defs": { "layers": [`` opening
    pattern. When the first value is a scalar (or fits inline), the
    dict is fully expanded with each key on its own line.
    """
    pad = INDENT * (depth + 1)
    items = list(value.items())
    last = len(items) - 1

    first_key, first_value = items[0]
    first_key_str = json.dumps(first_key, ensure_ascii=False)
    # Try rendering the first value with a flow prefix `{ "key": `.
    flow_prefix = f'{{ {first_key_str}: '
    first_text = _render(
        first_value,
        depth,
        first_line_prefix_len + len(flow_prefix),
        force_wrap=first_key == "defaultOverride" and isinstance(first_value, dict),
    )
    first_wraps = "\n" in first_text

    # The editor only flow-packs nested dicts (depth >= 1). The
    # top-level project dict's `{` lands on its own line and every
    # key (including the first) is on its own line at depth 1.
    if first_wraps and depth >= 1:
        # Flow first key onto the dict opener line. `first_text`
        # starts with the child's opener (`[` or `{ "k":`), spans
        # multiple lines, and ends with the child's closing bracket.
        first_lines = first_text.split("\n")
        # Glue the flow prefix to the first child line.
        first_lines[0] = flow_prefix + first_lines[0]
        # Append `,` to the child's closing bracket if more keys
        # follow.
        if len(items) > 1:
            first_lines[-1] = first_lines[-1] + ","
        lines = list(first_lines)
        # Remaining keys: each tries to flow-pack onto the previous
        # multi-line child's closing line; otherwise each lands on
        # its own line at depth+1.
        for i in range(1, len(items)):
            key, child = items[i]
            key_str = json.dumps(key, ensure_ascii=False)
            suffix = "" if i == last else ","
            # First try flow-pack: append `, "key": <opener>` onto
            # the closing-bracket line of the previous child.
            flow_attempt = None
            if not (key == "defaultOverride" and isinstance(child, dict)):
                flow_attempt = _try_flow_pack_next_key(
                    lines, key_str, child, depth, suffix
                )
            if flow_attempt is not None:
                lines = flow_attempt
                continue
            child_text = _render(
                child,
                depth + 1,
                len(pad) + len(key_str) + 2,
                force_wrap=key == "defaultOverride" and isinstance(child, dict),
            )
            lines.append(f"{pad}{key_str}: {child_text}{suffix}")
        # Close on its own line at parent depth.
        lines.append(f"{INDENT * depth}}}")
        return "\n".join(lines)

    # Full multi-line, each key on its own line at depth+1.
    lines = ["{"]
    for i, (key, child) in enumerate(items):
        key_str = json.dumps(key, ensure_ascii=False)
        suffix = "" if i == last else ","
        child_text = _render(
            child,
            depth + 1,
            len(pad) + len(key_str) + 2,
            force_wrap=key == "defaultOverride" and isinstance(child, dict),
        )
        lines.append(f"{pad}{key_str}: {child_text}{suffix}")
    lines.append(f"{INDENT * depth}}}")
    return "\n".join(lines)


def _try_flow_pack_next_key(
    lines: list[str],
    key_str: str,
    child,
    parent_depth: int,
    suffix: str,
) -> list[str] | None:
    """Attempt to pack `, "key": <child_opener>` onto the trailing
    line of ``lines`` (which should end with ``]`` / ``}``).

    Returns the new lines list on success, ``None`` on failure (caller
    falls back to the standard new-line-per-key emit).

    The flow only fires when:
    - The trailing line ends with ``]`` or ``}`` (or those + ``,``).
    - Rendering the child with `, "key": ` prepended at the trailing
      line's column produces a first-line width <= MAX_WIDTH.
    """
    if not lines:
        return None
    closing = lines[-1].rstrip(",")
    if not (closing.endswith("]") or closing.endswith("}")):
        return None
    # Render the child with the flow prefix's column. The child
    # itself sits at the parent dict's depth in flow mode.
    flow_prefix = f', {key_str}: '
    child_first_col = len(closing) + len(flow_prefix)
    child_text = _render(child, parent_depth, child_first_col)
    child_lines = child_text.split("\n")
    candidate_first = closing + flow_prefix + child_lines[0]
    if len(candidate_first) > MAX_WIDTH:
        return None
    # Apply: replace the last line (without its trailing `,`) and
    # splice the child's body lines after.
    new_lines = list(lines)
    new_lines[-1] = candidate_first
    if len(child_lines) > 1:
        new_lines.extend(child_lines[1:])
    # Append the suffix to the (possibly new) closing-bracket line.
    new_lines[-1] = new_lines[-1] + suffix
    return new_lines


def dump_editor_style(value) -> str:
    """Render ``value`` (the parsed LDtk project) in editor-shaped
    JSON. Returns a string with a trailing newline.
    """
    text = _render(value, 0, 0)
    return text + "\n"


def main(argv: list[str] | None = None) -> int:
    """CLI shim: read a JSON file, re-emit in editor style.

    Useful for canonicalizing an existing file or testing the
    formatter against editor-saved baselines.
    """
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("path", help="JSON file to reformat in place")
    parser.add_argument(
        "--output",
        default=None,
        help="Write to this path instead of editing in place",
    )
    args = parser.parse_args(argv)

    with open(args.path, "r", encoding="utf-8") as f:
        value = json.load(f)
    text = dump_editor_style(value)
    target = args.output or args.path
    with open(target, "w", encoding="utf-8") as f:
        f.write(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
