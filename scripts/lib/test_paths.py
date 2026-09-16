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

⚠ **THIS IS THE FILE-LEVEL QUESTION ONLY.** An inline `#[cfg(test)] mod tests`
inside a production file is a DIFFERENT fact with a different owner
(`strip_test_modules` in `check_rollback_mutators_run_in_sim.py`), because the
answer there is "strip part of the source", not "skip the file".

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
