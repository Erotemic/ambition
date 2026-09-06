#!/usr/bin/env python3
"""What does each registry's `register` actually RETURN?

⛔⛔ WHY THIS EXISTS. `docs/planning/triage/ambition-registry-core.md` carries a
hand-typed verdict column — "silent", "Result", "bool/Option" — and on
2026-09-05 THREE of three rows re-read were stale: `RoomContentStagingRegistry`,
`PreparedCharacterRegistry` and `MovePrefabRegistry` had all been fixed in source
while the table still described the old shape. The same page's other columns
verified (31/31 citations, 25/26 determinable key types), and the difference is
the point: a key type rarely changes, whereas REFUSAL BEHAVIOUR is exactly what a
fix changes. A hand-kept copy of a signature goes stale the first time somebody
improves the signature.

⭐ So the verdict is derived here instead. The authority for "what does
`register` return" is `register`.

⚠ WHAT THIS CANNOT SEE, said plainly: a `Result` that is always `Ok`, a refusal
expressed by a panic, or a registry that replaces DELIBERATELY (which
`PreparedCharacterRegistry` does, argued in place). A return type tells you the
SHAPE of the answer, not whether the shape is the right one — so this reports and
never gates.
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
PAGE = ROOT / "docs/planning/triage/ambition-registry-core.md"
#: `| `Name` | `path:line` | `Key` | `fn` | verdict |`
ROW = re.compile(r"\|\s*`(\w+)`\s*\|\s*`([^`]+)`\s*\|\s*`([^`]+)`\s*\|\s*`?([^`|]+?)`?\s*\|(.*)\|")


def return_type(source: str, fn: str) -> str | None:
    """The declared return of `fn`, or `None` when it is not found."""
    m = re.search(rf"fn\s+{re.escape(fn)}\s*(?:<[^>]*>)?\s*\(", source)
    if not m:
        return None
    # Walk the parameter list to its matching paren, then read to the brace.
    i, depth = m.end() - 1, 0
    while i < len(source):
        if source[i] == "(":
            depth += 1
        elif source[i] == ")":
            depth -= 1
            if depth == 0:
                break
        i += 1
    tail = source[i + 1 : i + 200]
    arrow = re.match(r"\s*->\s*([^{]+)\{", tail)
    return arrow.group(1).strip() if arrow else "()"


def main() -> int:
    if not PAGE.exists():
        print(f"skip: {PAGE} is absent", file=sys.stderr)
        return 3
    rows = [m.groups() for m in (ROW.match(l) for l in PAGE.read_text(encoding="utf-8").splitlines()) if m]
    if not rows:
        print("FAIL: parsed ZERO rows — the table shape changed and every claim "
              "below would be vacuous.", file=sys.stderr)
        return 1

    print(f"registry rows: {len(rows)}")
    refusing = silent = absent = 0
    for name, loc, _key, fn, verdict in rows:
        fn = fn.strip()
        if fn in {"-", ""}:
            absent += 1
            continue
        path = ROOT / loc.split(":")[0]
        if not path.exists():
            print(f"  ⚠ {name}: cited file missing ({loc})")
            continue
        ret = return_type(path.read_text(encoding="utf-8"), fn)
        if ret is None:
            print(f"  ⚠ {name}: no `fn {fn}` in {loc.split(':')[0]}")
            continue
        refuses = "Result" in ret or "Option" in ret or ret.strip() == "bool"
        refusing += refuses
        silent += not refuses
        # ⛔ NOT a substring test. The corrected `MovePrefabRegistry` row quotes
        # its own error text -- "would SILENTLY change what every existing row
        # expands to" -- and a `"silent" in verdict` check flagged the very row
        # that had already been fixed. A rule that scans for a word matches the
        # prose that DISCUSSES the word; compare the cell's verdict, which is its
        # first token before any correction note.
        first = verdict.strip().lstrip("|").strip().split()[:1]
        stale = bool(first) and first[0].lower() == "silent" and refuses
        mark = "⛔ ROW STALE" if stale else "  "
        print(f"  {mark} {name:34} fn {fn:22} -> {ret}")
    print(f"\nrefusing (Result/Option/bool): {refusing}   silent: {silent}   "
          f"no register fn: {absent}")
    print("⇒ Reported, never gated: a return type is the SHAPE of the answer, "
          "not whether the shape is right.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
