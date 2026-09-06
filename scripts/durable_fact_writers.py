#!/usr/bin/env python3
"""Who WRITES each durable save family, and which families have more than one.

⭐ THE QUESTION: a durable fact with two writers gets its policy from whoever
remembered. `boss.cleared` is the worked example — WRITTEN by one generic
authority (`boss_encounter/src/systems.rs`, keyed by placement, on the death
edge) and RETRACTED by a content system naming itself, so ten of eleven shipped
placements have no retraction at all and nobody chose that. This census asks
which other families have the same shape.

⛔⛔ IT EXCLUDES `#[cfg(test)]` MODULES BY POSITION, NOT BY PATH. This repo puts
`#[cfg(test)] mod tests` INSIDE ordinary source files, so a path filter
(`tests.rs`, `/tests/`) leaves every in-file test block in the numerator. My
first pass at this counted 7 `set_flag` writers; most were test seeds inside
`save_data.rs` and `save.rs`.

⚠ It is a LEXICAL census and says so: it finds `.set_<family>(` calls, not
every road to a durable write. A helper that wraps one is counted at the helper.
Read it as "where does the tree name this family", not as a proof of arity.

⭐⭐ AND SINCE 2026-09-05 THAT CAVEAT IS MUCH SMALLER, because the structure
changed under it. `AmbitionGameSaveData`'s fourteen fact fields were `pub`, so
any crate could write a durable fact by assignment under any variable name —
which is what made this census best-effort rather than an answer. They are now
`pub(crate)` behind named setters, so FROM OUTSIDE `ambition_persistence` A
SETTER CALL IS THE ONLY ROAD, and the lexical scan is complete for that
population.

⛔ What is still outside the scan, stated precisely so nobody over-reads the
number: (1) writes INSIDE `ambition_persistence` itself, where the fields are
visible — including `snapshot_impls`-style codecs that rebuild a value from a
struct literal, which no grep for `.set_x(` or `x =` can ever see; (2) helpers
that wrap a setter, counted at the helper. ⇒ the census is now an answer about
CONSUMER crates and an approximation about the owning crate.
"""

import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
# ⛔⛔ THE RECEIVER IS NOT ALWAYS `data_mut()`. The first version of this pattern
# required `data_mut().set_x(` and therefore MISSED the one writer this census
# was written to find: `cut_rope/mod.rs:214` binds `let data = save.data_mut();`
# and then calls `data.set_boss(...)` in a loop. ⇒ A census that cannot find its
# own worked example is not a weak census, it is a false one — it would have
# reported `boss` as single-writer, which is the exact claim it exists to test.
# Match the VERB on any receiver and name the families explicitly instead.
FAMILIES = "boss|flag|switch|encounter|quest|item|occurrence|custody|checkpoint"
CALL = re.compile(rf"\.\s*(set|clear|remove)_({FAMILIES})\s*\(")
TEST_ATTR = re.compile(r"^\s*#\[cfg\(test\)\]")
# ⛔ BRACES INSIDE STRING LITERALS ARE NOT CODE. `collision.rs:365` asserts on
# `"not ron at all {{{"` — three unmatched `{` in a test string, which left the
# brace counter permanently open and made the whole file report as unclosed.
# Strip string and char literals before counting. A raw string with an embedded
# quote would defeat this; there is none in the corpus, and the ambiguity check
# below FAILS rather than guessing if one ever appears.
_LITERAL = re.compile(r'"(?:\\.|[^"\\])*"' + r"|'(?:\\.|[^'\\])'")


def _code(line: str) -> str:
    """The part of a line that is Rust code: no line comment, no literals."""
    return _LITERAL.sub("", line.split("//", 1)[0])


def production_lines(text: str) -> tuple[list[tuple[int, str]], list[int]]:
    """Every line NOT inside a `#[cfg(test)]` item, 1-based, plus unclosed spans.

    ⛔⛔ THE FIRST VERSION TRUNCATED THE FILE AT THE FIRST `#[cfg(test)]`, AND
    THAT IS NOT "EXCLUDE TEST CODE" — it is "discard everything after the first
    test module". Measured 2026-09-05:
    `crates/ambition_encounter/src/switches.rs` opens an inline
    `mod queue_checksum_tests` at :123 and then continues with real systems —
    `drain_switch_activations` at :384 holds the canonical persisted switch
    writes at :401, :406, :410. The census reported `switch` had TWO writers
    when it has FIVE, and the one it dropped is the system that describes itself
    as the single author of the toggle.

    ⚠ AND THE SCRIPT HAD SAID SO. It printed "files whose writes are ALL after a
    `#[cfg(test)]`: 4 — read them if non-zero" and exited 0. A diagnostic that
    does not fail is a diagnostic nobody reads; that is why ambiguity is now an
    ERROR rather than a note.

    ⭐ The rule is structural: on `#[cfg(test)]`, find the item it attaches to
    and skip it — to the matching `}` for a braced body, or to the `;` for a
    bodyless one (`#[cfg(test)] use …;`, `#[cfg(test)] mod tests;`). Nested
    braces are counted, so the scan cannot stop at the first inner `}`.
    """
    lines = text.splitlines()
    out: list[tuple[int, str]] = []
    unclosed: list[int] = []
    i = 0
    while i < len(lines):
        if not TEST_ATTR.match(_code(lines[i])):
            out.append((i + 1, lines[i]))
            i += 1
            continue

        start = i
        i += 1
        depth = 0
        opened = False
        closed = False
        while i < len(lines):
            code = _code(lines[i])
            if not opened and "{" not in code and ";" in code:
                # A bodyless item: `use …;` or `mod tests;`. It ends here.
                i += 1
                closed = True
                break
            depth += code.count("{") - code.count("}")
            if "{" in code:
                opened = True
            i += 1
            if opened and depth <= 0:
                closed = True
                break
        if not closed:
            unclosed.append(start + 1)
    return out, unclosed


def main() -> int:
    files = subprocess.run(
        ["git", "ls-files", "*.rs"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()

    writers: dict[str, list[str]] = defaultdict(list)
    suppressed = 0
    ambiguous: list[str] = []
    for rel in files:
        if "/tests/" in rel or rel.endswith("tests.rs"):
            continue
        text = (REPO / rel).read_text(encoding="utf-8", errors="replace")
        lines, unclosed = production_lines(text)
        if unclosed:
            ambiguous.append(rel)
        total_hits = sum(
            len(CALL.findall(l.split("//", 1)[0])) for l in text.splitlines()
        )
        prod_hits = 0
        for number, line in lines:
            # ⛔ A COMMENT IS NOT A WRITER. `yarn_vocabulary.rs:133` is a doc line
            # describing `world.set_flag(flag, on)` and the first version counted
            # it — the same class as the Yarn census counting a character SAYING
            # a call. Strip `//` before matching; a `//` inside a string literal
            # would over-strip, and no writer in this tree is written that way.
            code = line.split("//", 1)[0]
            for verb, family in CALL.findall(code):
                writers[family].append(f"{rel}:{number} ({verb})")
                prod_hits += 1
        suppressed += total_hits - prod_hits

    print(f"durable families named by production code: {len(writers)}")
    print(f"in-file `#[cfg(test)]` writes excluded: {suppressed}")
    print()
    for family in sorted(writers, key=lambda f: (-len(writers[f]), f)):
        sites = writers[family]
        mark = "  <-- MORE THAN ONE" if len(sites) > 1 else ""
        print(f"{len(sites):3}  {family}{mark}")
        for site in sites:
            print(f"       {site}")

    # ⛔⛔ AN AMBIGUOUS CLASSIFICATION MUST NOT COEXIST WITH AN AUTHORITATIVE
    # CENSUS. The earlier version printed a "possible misses" line, exited 0, and
    # then printed writer counts that read as fact — and the miss it was warning
    # about was real (`switches.rs` lost three production writes). A census that
    # reports its own doubt and passes anyway trains the reader to skip the doubt.
    if ambiguous:
        print()
        print(
            f"⛔ {len(ambiguous)} file(s) have a `#[cfg(test)]` item this scanner "
            "could not close, so the counts above are NOT authoritative:"
        )
        for rel in ambiguous:
            print(f"       {rel}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
