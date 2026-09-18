#!/usr/bin/env python3
"""THE one answer to *"is this Rust file test-only?"*.

⛔⛤ **THIS EXISTS BECAUSE THERE WERE FIVE, AND THEY GAVE FIVE DIFFERENT
ANSWERS.** MEASURED 2026-09-16, before this module existed:

    script                                        tests/  tests.rs  test.rs  *_tests.rs  test_support.rs
    check_rollback_mutators_run_in_sim.py           yes      yes      yes        yes           no
    ecs_inventory.py                                yes      yes      yes        yes           no
    check_set_pins_have_engine_members.py           yes      yes      yes        NO            no
    check_capability_ships.py                       yes      yes      NO         NO           yes
    test_every_smash_technique_has_a_translator.py  yes      yes      NO         NO            no

The `*_tests.rs` rule reached TWO of five, and one of those docstrings records
that it *"used to miss all 51 of them"* — so a known defect was found and fixed
once while three copies kept it. ⚠ The population of copies was never
enumerated, only the one somebody was looking at, which is why the fix stopped
where it did.

⛔⛔ **AND THE NAME IS A PROXY FOR A FACT NONE OF THE FIVE CHECKED.** A file
whose first attribute is an inner `#![cfg(test)]` is compiled out ENTIRELY,
whatever it is called. Four files carry it —
`features/ecs/fighter_harness.rs`, `demo_smash/src/capture.rs`,
`content/src/moves_are_content.rs`, `content/src/moveset_artifact.rs` — and
every name rule above passes all four as production.

⚠ **TWO QUESTIONS LIVE HERE NOW, AND THEY ARE DIFFERENT FACTS.**
[`is_test_path`] answers *"skip this file"*; [`strip_test_modules`] answers
*"strip part of this source"*, for an inline `#[cfg(test)] mod tests` inside a
production file. ⭐ The second MOVED here from
`check_rollback_mutators_run_in_sim.py` on 2026-09-17, because a THIRD consumer
appeared and copied it: `multi_writer_resource_census.py` wrote its own, and
`architecture_census.py` had a weaker variant that discarded the whole file TAIL
— which in this tree is usually production code, since a module declares its
tests near the top. Splitting one question across two owners is how the five
spellings above happened.

⛔ **WIDENING THIS MAKES EVERY CONSUMER SEE LESS, AND A CHECK THAT SEES LESS
REPORTS CLEANER.** Every consumer carries a `POPULATION_FLOOR` for exactly that
reason. Do not add a rule here without running each consumer and reading what
its counts did.
"""

from __future__ import annotations

import re
from pathlib import Path

#: A file compiled out in its entirety. Anchored to the line start so a
#: `#![cfg(test)]` quoted inside a doc comment or a string is not matched.
INNER_CFG_TEST = re.compile(r"^[ \t]*#!\[cfg\(test\)\]", re.MULTILINE)


def file_is_test_only(source: str) -> bool:
    """Does `#![cfg(test)]` compile away this WHOLE file?

    ⚠ The attribute must sit at brace depth 0. The same spelling inside a `mod`
    block compiles out only that block, and reading it as the file drops
    production code out of a consumer's corpus with nothing to show for it.
    """
    for match in INNER_CFG_TEST.finditer(source):
        before = source[: match.start()]
        if before.count("{") == before.count("}"):
            return True
    return False

#: Names that mean "this whole file is tests", unioned across the five copies.
TEST_FILE_NAMES = frozenset({"tests.rs", "test.rs", "test_support.rs"})

#: Directory component that means the same thing.
TEST_DIR_PARTS = frozenset({"tests"})


def is_test_path(path: Path, source: str | None = None) -> bool:
    """Is `path` test-only?

    `source` is optional; pass it when the caller has already read the file, so
    the `#![cfg(test)]` half is checked without a second read. ⚠ Omitting it
    falls back to reading the file, and an unreadable file answers False —
    a scanner that swallows a read error reports its own finding, so callers
    that care should pass the text they already hold.
    """
    if TEST_DIR_PARTS & set(path.parts):
        return True
    if path.name in TEST_FILE_NAMES or path.name.endswith("_tests.rs"):
        return True
    if source is None:
        try:
            source = path.read_text(encoding="utf-8")
        except OSError:
            return False
    return file_is_test_only(source)


#: An inline test module's opening brace. ⚠ Deliberately NOT bare
#: `#[cfg(test)]`: this function removes a BLOCK by brace balance, and an
#: attribute on an item with no block (`#[cfg(test)] mod tests;`,
#: `#[cfg(test)] use ...;`) has no braces to balance.
_CFG_TEST_MOD = re.compile(r"#\[cfg\(test\)\]\s*mod\s+[A-Za-z_][A-Za-z_0-9]*\s*\{")


def strip_test_modules(source: str) -> str:
    """Remove inline `#[cfg(test)] mod … { … }` blocks by brace balance.

    These modules legitimately do things production code may not — register
    rollback mutators into `Update`, build a resource by hand — and they sit
    inside production files, so path-based test filtering never sees them.

    ⛔⛤ **THE VARIANT THIS REPLACES DISCARDED THE FILE TAIL**
    (`text.split("#[cfg(test)]", 1)[0]`), and in this tree the tail is usually
    production code: a module declares its tests near the TOP —
    `#[cfg(test)] mod tests;` at `platformer2d_runtime/src/lib.rs:25` — so
    everything below that line was invisible. MEASURED 2026-09-17 over
    `architecture_census.py`'s own corpus: the tail cut reports **731 optional
    `Res`/`ResMut` occurrences over 197 types** and per-item stripping reports
    **820 over 206**, while removing the strip entirely adds only three more. ⇒
    The 12% the campaign was missing was production code, not fixtures.

    ⚠ The brace match is naive about braces inside string literals inside a test
    body. Over-cutting loses production code, which is the direction the tail cut
    already erred in; under-cutting counts a fixture. Both move a consumer's
    population, which is why every consumer of this module carries a floor.
    """
    while (match := _CFG_TEST_MOD.search(source)) is not None:
        depth = 1
        index = match.end()
        while index < len(source) and depth:
            if source[index] == "{":
                depth += 1
            elif source[index] == "}":
                depth -= 1
            index += 1
        source = source[: match.start()] + source[index:]
    return source
