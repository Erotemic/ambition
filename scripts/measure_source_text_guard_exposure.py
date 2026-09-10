#!/usr/bin/env python3
"""Which SOURCE-TEXT guards fail SILENTLY when their spelling stops matching.

⛔⛔ **A GUARD THAT READS SOURCE TEXT HAS TWO INPUTS IT DOES NOT CONTROL: THE
FORMATTER, AND THE LANGUAGE'S OWN CONSTRUCTION RULES.** Three were found in one
night, 2026-09-10, and *none of them by looking*:

* `test_what_the_stage_declares_the_stage_gives_back` anchored on a contiguous
  `commands.insert_resource(`; `rustfmt` wrapped one call because its path was
  long, and the guard reported a leak that was not there.
* `the_death_drop_table_is_complete` recognised a drop by the literal
  `GroundItem {`; sealing the type with `#[non_exhaustive]` plus constructors
  made every drop in the file invisible.
* `measure_state_writers.py` recognised construction by a hand-kept list of
  blessed method names (`::new`, `::default`, `::from`); the same seal dropped
  OCCURRENCE from 25 to 18 with no code removed, and correcting it surfaced a
  minting site (`WorldItem::equipping`) that had never been visible at all.

⭐⭐ **THE AXIS IS DIRECTION, NOT LIKELIHOOD.** A guard that goes blind and
reports a PHANTOM is self-limiting -- somebody chases it, as happened with the
first of the three. A guard that goes blind and reports **"no offenders"** is the
one to find: it prints a clean bill of health forever and nobody is looking.

⇒ So this asks ONE mechanical question per test: **if the scan underneath it
returned NOTHING, would the test still pass?** An assertion shaped as a CEILING
(`len(found) <= N`) or an EMPTINESS check (`assert not violations`) passes on an
empty scan. An ANTI-VACUITY FLOOR (`assert len(found) >= N`, `assert corpus`) or
an EQUALITY against a hand-written expectation fails on one. A guard with a
ceiling and no floor is EXPOSED.

⚠ **THIS DOES NOT SAY THE EXPOSED ONES ARE WRONG.** It says nothing would tell
you if they were. A named, reproducible way to blind a specific guard is a
finding; a row on this list is a place to look.

⚠ **AND IT IS AST-SHAPED, NOT SEMANTIC.** It cannot tell that a floor asserted
on a DIFFERENT collection than the ceiling does not protect it, and it reports
tests it could not classify rather than assuming they are safe.

    python3 scripts/measure_source_text_guard_exposure.py
    python3 scripts/measure_source_text_guard_exposure.py --exposed-only
"""

from __future__ import annotations

import argparse
import ast
import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[1]
TEST_DIR = REPO / "scripts/tests"

# How a test gets its hands on source text. A guard that never reads a file is
# not this sweep's subject even if it asserts a ceiling.
READS_SOURCE = (
    "read_text",
    "rglob",
    "iterdir",
    "read_bytes",
    "check_output",
    "run(",
    "open(",
    "glob(",
)


def reads_source(src: str) -> bool:
    return any(marker in src for marker in READS_SOURCE)


class Verdict:
    FLOORED = "floored"
    EXPOSED = "exposed"
    EQUALITY = "equality"
    CROSS_CHECKED = "cross-checked"
    NOT_A_GUARD = "not-a-guard"
    NO_SCAN_ASSERT = "unclassified"


def _is_len_call(node: ast.AST) -> bool:
    return (
        isinstance(node, ast.Call)
        and isinstance(node.func, ast.Name)
        and node.func.id == "len"
    )


def classify_assert(node: ast.Assert) -> set[str]:
    """The SHAPES this one assertion has. A test may carry several."""
    shapes: set[str] = set()
    test = node.test

    # `assert not <thing>` / `assert <thing> == 0` / `assert x == []`
    if isinstance(test, ast.UnaryOp) and isinstance(test.op, ast.Not):
        shapes.add("emptiness")
    if isinstance(test, ast.Compare) and len(test.ops) == 1:
        op = test.ops[0]
        left, right = test.left, test.comparators[0]
        zero_right = isinstance(right, ast.Constant) and right.value in (0, [], (), "")
        zero_left = isinstance(left, ast.Constant) and left.value in (0, [], (), "")
        if isinstance(op, ast.Eq):
            # ⛔⛔ **A POSITIVE CONTROL IS OFTEN `assert checker.main() == 1` ON A
            # FIXTURE.** Five of the six files this sweep first shortlisted are
            # built that way — `test_an_emptied_corpus_fails` runs the real
            # checker against a corpus it constructed and requires a FAILING exit
            # code. That is anti-vacuity, and reading it as "an equality with no
            # floor" reported five guards as exposed that carry the strongest
            # control shape in the tree.
            if (
                isinstance(left, ast.Call)
                and isinstance(right, ast.Constant)
                and isinstance(right.value, int)
                and right.value != 0
            ):
                shapes.add("control")
            elif zero_right or zero_left:
                shapes.add("emptiness")
            else:
                shapes.add("equality")
        elif isinstance(op, (ast.LtE, ast.Lt)):
            # `len(found) <= CEILING` -- an empty scan satisfies it.
            if _is_len_call(left) or isinstance(left, ast.Name):
                shapes.add("ceiling")
            # `FLOOR <= len(found)` is a floor written the other way round.
            if _is_len_call(right) or isinstance(right, ast.Name):
                shapes.add("floor")
        elif isinstance(op, (ast.GtE, ast.Gt)):
            # ⛔⛔ **A FLOOR IS OFTEN ASSERTED ON A COUNT THAT IS ALREADY A NUMBER**
            # -- `count = len(...)` on the line above, then `assert count >= 10`.
            # The first cut of this sweep only recognised a floor spelled
            # `assert len(x) >= N` and put `test_actor_construction_inversion` on
            # the shortlist, a file whose LAST test is named "the checker has a
            # positive control on the intended edge" and asserts exactly that.
            if _is_len_call(left) or isinstance(left, ast.Name):
                shapes.add("floor")
            if _is_len_call(right) or isinstance(right, ast.Name):
                shapes.add("ceiling")
        elif isinstance(op, (ast.In, ast.NotIn)):
            shapes.add("membership")
    # `assert corpus` / `assert found` -- the bare anti-vacuity spelling.
    if isinstance(test, ast.Name) or _is_len_call(test):
        shapes.add("floor")
    if isinstance(test, ast.Attribute) or isinstance(test, ast.Subscript):
        shapes.add("floor")
    return shapes


# ⭐⭐ **THE SECOND DIMENSION: WHAT THE SCAN ANCHORS ON.** "Exposed" alone is a
# place to look, not a finding -- plenty of ceilings anchor on something the
# language makes canonical and cannot drift. What made all three of the
# 2026-09-10 cases real was a RIGID JOIN: two tokens spelled adjacent with no
# tolerance for what may legally sit between them.
#
#   `commands\.insert_resource\(`   -- rustfmt may put a newline before the `.`
#   `GroundItem \{`                 -- a seal may remove struct-literal syntax
#   `::new\b|::default\b`          -- a hand-kept list of blessed names
#
# ⇒ A pattern joining tokens with `.` or `::` or `(` and NO `\s*` between them
# is breakable by a formatter or a refactor that nobody would think to check.
RIGID_JOINS = (r"\.", r"::", r"\(", r"\{")

# `foo.py`, `Cargo.toml`, `baseline.json` -- a dot that names a FILE, not a call.
FILENAME_TAIL = re.compile(
    r"\.(?:py|rs|toml|json|md|ron|yml|yaml|txt|otf|ttf|png|svg|jsonl|sh|lock|html)$"
)


def rigid_literals(src: str) -> list[str]:
    """String/regex literals in `src` that join tokens with no whitespace slack."""
    found: list[str] = []
    try:
        tree = ast.parse(src)
    except SyntaxError:
        return found
    for node in ast.walk(tree):
        if not isinstance(node, ast.Constant) or not isinstance(node.value, str):
            continue
        text = node.value
        # A join has to have something on BOTH sides to be a join at all, and a
        # literal with no word characters is punctuation, not an anchor.
        if len(text) < 4 or not any(c.isalnum() for c in text):
            continue
        # ⛔⛔ PROSE IS NOT AN ANCHOR, AND THE FIRST RUN OF THIS SWEEP WAS
        # ENTIRELY PROSE. These guards carry long explanatory docstrings and
        # multi-paragraph failure messages, every one of which contains a `.`
        # between two word characters — so the shortlist came back as a wall of
        # documentation with the four real patterns buried in it. An anchor is
        # CODE-SHAPED: one line, short, and almost all identifier characters.
        if "\n" in text or len(text) > 90:
            continue
        if text.count(" ") > 2:
            continue
        # ⛔ A FILE PATH IS NOT A FRAGILE ANCHOR. `Cargo.toml`, `run_tests.py` and
        # `crates/x/src/lib.rs` all join word characters with a `.` or a `/`, and
        # a formatter cannot touch any of them — the second run of this sweep was
        # mostly filenames. The joins that BREAK are Rust's own: a path (`::`), a
        # method call (`.name(`), a struct literal (`{`), an attribute (`#[`).
        if "/" in text or FILENAME_TAIL.search(text):
            continue
        if not any(marker in text for marker in ("::", "(", "{", "#[", "<")):
            continue
        for join in RIGID_JOINS:
            for idx in _positions(text, join.replace("\\", "")):
                before = text[max(0, idx - 4) : idx]
                if r"\s" in before or "s*" in before:
                    continue
                if idx > 0 and (text[idx - 1].isalnum() or text[idx - 1] == "_"):
                    found.append(text)
                    break
            else:
                continue
            break
    return found


def _positions(text: str, needle: str) -> list[int]:
    out, at = [], text.find(needle)
    while at != -1:
        out.append(at)
        at = text.find(needle, at + 1)
    return out


def verdict_for(fn: ast.FunctionDef) -> tuple[str, set[str]]:
    shapes: set[str] = set()
    for node in ast.walk(fn):
        if isinstance(node, ast.Assert):
            shapes |= classify_assert(node)
    if "floor" in shapes or "control" in shapes:
        return Verdict.FLOORED, shapes
    if "equality" in shapes or "membership" in shapes:
        return Verdict.EQUALITY, shapes
    if "ceiling" in shapes or "emptiness" in shapes:
        return Verdict.EXPOSED, shapes
    return Verdict.NO_SCAN_ASSERT, shapes


# ---------------------------------------------------------------------------
# THE RUST HALF
#
# ⛔⛔ **THE PYTHON POPULATION IS NOT THE POPULATION.** One of the two guards
# found blind on 2026-09-10 was a RUST test reading `.rs` text
# (`the_death_drop_table_is_complete`), so a sweep that classifies by Python AST
# is answering the question for the half that happened not to contain it.
#
# ⚠ NO AST, AND THE REPORT SAYS SO. This reads `assert!`/`assert_eq!` invocations
# textually and classifies them by the same shapes. It is weaker than the Python
# side and errs toward `unclassified` rather than toward `floored`, because the
# expensive mistake here is calling an exposed guard safe.

RUST_SOURCE_READ = ("include_str!", "read_to_string", "fs::read")


def rust_guard_files() -> list[pathlib.Path]:
    """`.rs` files that read `.rs` SOURCE TEXT -- not data, not assets."""
    out: list[pathlib.Path] = []
    for root in ("crates", "game"):
        base = REPO / root
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            text = path.read_text(encoding="utf-8", errors="replace")
            if not any(marker in text for marker in RUST_SOURCE_READ):
                continue
            if '.rs"' not in text:
                continue
            out.append(path)
    return out


def _macro_args(text: str, start: int) -> str:
    """The balanced argument list of a macro whose `(` is at `start`."""
    depth, i = 0, start
    while i < len(text):
        if text[i] in "([{":
            depth += 1
        elif text[i] in ")]}":
            depth -= 1
            if depth == 0:
                return text[start + 1 : i]
        i += 1
    return text[start + 1 :]


FLOOR_SHAPES = (
    re.compile(r"\.len\(\)\s*>=?\s*\d"),
    re.compile(r"\d\s*<=?\s*\w+\.len\(\)"),
    re.compile(r"^\s*!\s*[\w.()\[\]:]+\.is_empty\(\)"),
    re.compile(r"^\s*[\w.()\[\]:]+\.contains\("),
    re.compile(r"^\s*\w+\s*>=\s*\d"),
)
EMPTY_SHAPES = (
    re.compile(r"^\s*!\s*[\w.()\[\]:]+\.contains\("),
    re.compile(r"^\s*[\w.()\[\]:]+\.is_empty\(\)"),
    re.compile(r"^\s*[\w.()\[\]:]+\.all\("),
    re.compile(r"\.len\(\)\s*<=?\s*\d"),
)


def classify_rust_file(text: str) -> tuple[str, list[str]]:
    shapes: list[str] = []
    for match in re.finditer(r"\bassert(_eq|_ne)?!\s*\(", text):
        args = _macro_args(text, match.end() - 1)
        first = args.split(",\n")[0]
        if match.group(1) in ("_eq", "_ne"):
            shapes.append("equality")
            continue
        if any(shape.search(first) for shape in FLOOR_SHAPES):
            shapes.append("floor")
        elif any(shape.search(first) for shape in EMPTY_SHAPES):
            shapes.append("emptiness")
        else:
            shapes.append("unclassified")
    if not shapes:
        # No assertions at all -- a baker or a helper, not a guard.
        return Verdict.NOT_A_GUARD, shapes
    if "floor" in shapes:
        return Verdict.FLOORED, shapes
    # ⛔⛔ **CROSS-EVIDENCE IS A FLOOR THE SHAPE CLASSIFIER CANNOT SEE, AND IT
    # REPORTED FOUR GUARDS AS EXPOSED FOR WANT OF IT.** The `*_it_sync` family
    # derives one set from SOURCE TEXT (`mod <name>;`) and one from a DIRECTORY
    # LISTING, then asserts each difference is empty. If the text side goes blind
    # the disk side is still full and `missing` reddens; if the disk side goes
    # blind the text side is still full and `orphaned` reddens. Neither can be
    # silently emptied by a spelling change, because the other is not made of
    # spellings. ⇒ Two `difference(` calls over independently-derived sets, each
    # asserted empty, is a stronger anti-vacuity than a count floor -- it is the
    # shape worth copying, not an exposure to fix.
    if text.count(".difference(") >= 2 and "emptiness" in shapes:
        return Verdict.CROSS_CHECKED, shapes
    if "equality" in shapes:
        return Verdict.EQUALITY, shapes
    if "emptiness" in shapes:
        return Verdict.EXPOSED, shapes
    return Verdict.NO_SCAN_ASSERT, shapes


def report_rust() -> int:
    files = rust_guard_files()
    # ⛔ ANTI-VACUITY, the same one this sweep asks of everything else.
    assert len(files) >= 8, (
        f"only {len(files)} Rust files read `.rs` source text; measured at 12 on "
        "2026-09-10. This sweep has lost its corpus rather than found a tidy tree"
    )
    print("⛔ RUST SOURCE-TEXT GUARDS — would a scan that matched NOTHING still pass?")
    print(f"   {len(files)} file(s) read `.rs` source text.\n")
    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace")
        verdict, shapes = classify_rust_file(text)
        counts = {s: shapes.count(s) for s in sorted(set(shapes))}
        print(f"   [{verdict:12}] {path.relative_to(REPO)}  {counts}")
    print()
    print("⚠ TEXTUAL, NOT AST. A macro argument spanning lines is read from its")
    print("  FIRST line, and a floor asserted through a helper reads as absent.")
    print("  This side errs toward `unclassified`, never toward `floored`.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exposed-only", action="store_true")
    parser.add_argument("--rust", action="store_true", help="the Rust half")
    args = parser.parse_args()
    if args.rust:
        return report_rust()

    rows: list[tuple[str, str, str, set[str]]] = []
    candidates: dict[str, list[str]] = {}
    by_file: dict[str, set[str]] = {}
    scanned_files = 0
    for path in sorted(TEST_DIR.glob("test_*.py")):
        src = path.read_text(encoding="utf-8", errors="replace")
        if not reads_source(src):
            continue
        scanned_files += 1
        try:
            tree = ast.parse(src)
        except SyntaxError:
            rows.append((path.name, "<unparseable>", "unparseable", set()))
            continue
        rigid = rigid_literals(src)
        for node in ast.walk(tree):
            if isinstance(node, ast.FunctionDef) and node.name.startswith("test_"):
                verdict, shapes = verdict_for(node)
                if rigid:
                    shapes = shapes | {"rigid-anchor"}
                rows.append((path.name, node.name, verdict, shapes))
                by_file.setdefault(path.name, set()).add(verdict)
                if verdict == Verdict.EXPOSED and rigid:
                    candidates.setdefault(path.name, sorted(set(rigid))[:4])

    # ⛔ ANTI-VACUITY ON THIS SWEEP ITSELF. An empty population prints a serene
    # "0 exposed", which is the exact failure this file is about.
    assert scanned_files >= 50, (
        f"only {scanned_files} source-reading guards found under {TEST_DIR}; this "
        "sweep has lost its own corpus rather than found a tidy tree"
    )

    counts: dict[str, int] = {}
    for _, _, verdict, _ in rows:
        counts[verdict] = counts.get(verdict, 0) + 1

    print(f"⛔ SOURCE-TEXT GUARDS — would a scan that matched NOTHING still pass?")
    print(f"   {scanned_files} guard files read source text; {len(rows)} test functions in them.\n")
    for verdict in (Verdict.EXPOSED, Verdict.EQUALITY, Verdict.FLOORED, Verdict.NO_SCAN_ASSERT):
        print(f"   {counts.get(verdict, 0):4}  {verdict}")
    print()
    print("   exposed      = a CEILING or an EMPTINESS check and no floor. An empty scan")
    print("                  is a clean bill of health, and nothing says otherwise.")
    print("   equality     = compares a derived set against a hand-written one; an empty")
    print("                  scan fails LOUDLY, which is the self-limiting direction.")
    print("   floored      = carries an anti-vacuity assertion, or a POSITIVE CONTROL")
    print("                  that runs the checker on a fixture and requires it to FAIL.")
    print("   unclassified = no assertion this sweep recognises. Read it.\n")

    # ⛔⛔ **A POSITIVE CONTROL IS OFTEN A SIBLING TEST, NOT A SECOND ASSERTION.**
    # The first cut of this sweep classified per FUNCTION and put
    # `test_scoped_ruleset_policy_restores_everything` on the shortlist — a file
    # whose `assert not offenders` is floored by a whole separate test named
    # "THE POSITIVE CONTROL" three functions down. Counting the function alone
    # reports a guard as exposed because its anti-vacuity lives next door.
    shortlist = {
        name: literals
        for name, literals in candidates.items()
        if Verdict.FLOORED not in by_file.get(name, set())
    }
    floored_by_sibling = sorted(set(candidates) - set(shortlist))

    print("⛔⛔ THE SHORTLIST — EXPOSED *and* anchored on a RIGID JOIN,")
    print("   in a file where NO sibling test carries an anti-vacuity floor.")
    print("   These are the ones where a formatter or a refactor can blind the guard")
    print("   and the result is a clean bill of health. Each row prints the literals")
    print("   the file joins with no whitespace slack.\n")
    for name in sorted(shortlist):
        print(f"   {name}")
        for literal in shortlist[name]:
            print(f"        {literal!r}")
    print(f"\n   {len(shortlist)} file(s) on the shortlist.")
    print(
        f"   {len(floored_by_sibling)} more had a rigid anchor and a ceiling but a "
        "FLOORED SIBLING test:"
    )
    for name in floored_by_sibling:
        print(f"        {name}")
    print()

    for name, fn, verdict, shapes in rows:
        if args.exposed_only and verdict != Verdict.EXPOSED:
            continue
        print(f"   [{verdict:12}] {name}::{fn}  {sorted(shapes)}")

    print()
    print("⛔ WHAT THIS CANNOT SEE — the number travels with these or not at all.")
    print("   * A FLOOR ON A DIFFERENT COLLECTION than the ceiling protects nothing,")
    print("     and this sweep cannot tell the two apart.")
    print("   * A FLOOR INSIDE THE HELPER rather than the test reads as absent here.")
    print("   * `equality` is only self-limiting when the EXPECTATION is hand-written.")
    print("     Two sides derived from the SAME scan agree when both are empty.")
    print("   * RUST guards that read source text are not in this population.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
