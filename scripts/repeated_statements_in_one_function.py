#!/usr/bin/env python3
"""Statements written more than once inside a SINGLE function.

⭐ THE SHAPE THIS FINDS is one fact with several authorities standing side by
side, which is the cheapest kind to fix and the easiest to miss in review because
each copy reads correctly. Two landed from this on 2026-09-05:

  * `advance_active_cutscene` ended a cutscene TWICE -- skip and completed each
    read `script.seen_flag`, set it, nulled the runtime and reset the
    presentation. A third ending would have had to remember all four steps.
  * `sync_portal_view_cones` retired a rig FOUR times -- despawning the root and
    `rig.cone` at each of four exit conditions. A rig growing a third entity
    would have leaked it at whichever site was forgotten.

⭐⭐ WHAT A FIXED HIT LOOKS LIKE, so the report can be read quickly.
`sync_authored_gated_lock_walls` calls `retract_gated_lock_wall_verdicts(world)`
at FOUR early exits and is entirely correct: the retraction is a NAMED FUNCTION,
so the fact has one authority and only the CALL repeats -- which is unavoidable
for early returns. That is what both fixes above were turned INTO.
⇒ The distinction is whether the repeated line is a CALL to the one owner, or the
owner's body inlined again. Four calls to `retire_rig` is right; four copies of
its two lines was the bug.

⭐ OBSERVED PRECISION, so nobody expects a defect list. Every production hit
across one full lane was read on 2026-09-05 — engine crates (`world/`,
`platformer2d_world`, `items`, `persistence`) and content (`ambition_content`,
`demo_mary_o`, `demo_sanic`): **THIRTEEN hits, FOUR real**, all four fixed. The
nine others each read correctly:

  * a named helper called at four early exits (the FIXED shape);
  * `fs::remove_file(&backup)` used once to clear a stale backup and once to
    clean up after a successful rename — one line, two purposes;
  * a codec writing two different fields with identical `put_f32` lines;
  * rollback probes each opening with a `DefaultHasher` over its OWN fields;
  * two folds that share an opening line and then hash DIFFERENT things, the
    second deliberately folding a count the first has no business seeing;
  * `.in_set(..)` registrations for different systems.

`ambition_render` adds two more read-and-left: two early returns of the same
value, and a grid-line colour rule computed once per AXIS in `debug_viz` (a real
duplication, including a `0.01` tolerance, but of a dev gizmo — desynchronising
two axes of a debug overlay is not worth a change made in a hurry). ⇒ FIFTEEN
hits read, four real.

⇒ Expect to read most hits and act on roughly a quarter. That ratio is the tool
working: a scanner that flagged only certain defects would have to understand
intent, and one pretending to would be worse than this.
⚠ AND SEVERITY IS NOT PART OF THE REPORT. The grid-colour hit is genuine
duplication that I chose not to fix; "real" above means "one fact with two
owners", not "worth changing today". Keep those two judgements separate when
reading a hit, or the count stops meaning anything.

⛔ IT IS A REPORT, NOT A GUARD, and most hits are FINE. Two `continue`s, two
identical asserts in a table-driven check, or the same setter called for two
different keys are not duplication of an AUTHORITY. The question a hit asks is
*"do these lines together mean ONE thing that now has two owners?"* -- a human
question, which is why this prints instead of failing.

⚠ WHAT IT CANNOT SEE. Function boundaries are found by regex and brace depth, so
a macro body or an unusual formatting can merge or split a function. It compares
statements LITERALLY, so the same fact spelled two ways (`a.despawn()` vs
`commands.entity(x).despawn()`) reads as two different lines and is missed --
which means a clean report is weak evidence, while a hit is worth reading.
"""
from __future__ import annotations

import collections
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
FN = re.compile(r"^(pub(\([\w:]+\))? )?(async )?fn \w+")
#: A statement worth comparing: long enough to be distinctive, and an actual
#: action rather than punctuation or a comment.
MIN_LEN = 18
NOISE = {"}", "};", "});", "});;"}


def rust_files(paths: list[str]) -> list[pathlib.Path]:
    out = subprocess.run(
        ["git", "ls-files", *paths], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.split()
    return [
        REPO / f for f in out
        if f.endswith(".rs") and "/tests/" not in f and not f.endswith("tests.rs")
    ]


def strip_cfg_test_blocks(lines: list[str]) -> list[str]:
    """Blank out `#[cfg(test)]` items, BY POSITION rather than by path.

    ⛔⛔ THE DEFECT THIS FIXES SHIPPED IN THIS FILE'S FIRST VERSION, and the trap
    was already written down in `durable_fact_writers.py`: this repo puts
    `#[cfg(test)] mod tests` INSIDE ordinary source files, so a path filter
    (`tests.rs`, `/tests/`) leaves every in-file test block in the numerator. The
    first run over the item and persistence crates returned mostly duplicated
    `assert!` lines -- true repeats, in test code, which is not the question.

    ⚠ Lines are BLANKED, not deleted, so reported line numbers still match the
    file a reader will open.
    """
    out = list(lines)
    i = 0
    while i < len(out):
        if out[i].strip().startswith("#[cfg(test)]"):
            depth, seen_brace = 0, False
            while i < len(out):
                depth += out[i].count("{") - out[i].count("}")
                seen_brace = seen_brace or "{" in out[i]
                out[i] = ""
                i += 1
                if seen_brace and depth <= 0:
                    break
            continue
        i += 1
    return out


def repeats_in(path: pathlib.Path):
    """Yield (fn signature, statement, [line numbers]) for repeats in one fn."""
    lines = strip_cfg_test_blocks(
        path.read_text(encoding="utf-8", errors="replace").splitlines()
    )
    start, depth, body = None, 0, []
    for i, raw in enumerate(lines):
        if start is None and FN.match(raw.strip()):
            start, depth, body = i, 0, []
        if start is None:
            continue
        body.append((i + 1, raw))
        depth += raw.count("{") - raw.count("}")
        if depth > 0 or len(body) <= 3:
            continue
        counts, where = collections.Counter(), collections.defaultdict(list)
        for no, b in body:
            s = b.strip()
            if len(s) < MIN_LEN or s.startswith(("//", "/*", "*")) or s in NOISE:
                continue
            if s.endswith(";") and ("=" in s or "(" in s):
                counts[s] += 1
                where[s].append(no)
        for s, n in counts.items():
            if n >= 2:
                yield (lines[start].strip()[:70], s[:80], where[s])
        start = None


def main() -> int:
    paths = sys.argv[1:] or ["crates/", "game/"]
    files = rust_files(paths)
    if not files:
        print(f"FAIL: no Rust files under {paths} — the scan has nothing to say.",
              file=sys.stderr)
        return 1
    total = 0
    for path in files:
        rel = path.relative_to(REPO)
        for signature, statement, at in repeats_in(path):
            total += 1
            print(f"{rel}\n   fn: {signature}\n   {len(at)}x  {statement}\n"
                  f"   at lines {at}\n")
    print(f"{total} repeated statement(s) inside a single function, "
          f"across {len(files)} file(s).")
    print("⚠ Most are fine. Ask of each: do these lines together mean ONE thing "
          "that now has two owners?")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
