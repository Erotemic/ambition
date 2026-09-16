"""One rule for "is this Rust file test-only", shared by the source-scanning gates.

Four gates each carried a copy and the copies had drifted: one missed the 51
`*_tests.rs` files, one missed `test_support.rs`, and only one asked the
question that actually decides it.

⛔ A NAME IS A PROXY; THE `#[cfg(test)]` ON THE DECLARATION IS THE FACT, and the
two halves below only approximate it. A file is compiled out because its `mod`
line carries `#[cfg(test)]` — which lives in the PARENT — or because the file
itself opens with `#![cfg(test)]`. `features/ecs/fighter_harness.rs` is why the
second half exists: a harness beside the code it exercises, declared with a bare
`mod` line, which every name rule admits.

⚠ The proxy was measured against the fact on 2026-09-16: of 284 files excluded
by name, 281 were genuinely test-gated, one was declared by nothing at all, and
two shipped — `enemy_projectile/tests.rs` and its `test_support.rs`, whose own
doc comment called the module "test-only" while both `mod` lines were ungated.
`scripts/tests/test_rust_sources.py` re-runs that comparison, so a name may be
trusted here only for as long as it stays true.

⚠ Widening an exclusion makes a gate see LESS, and a gate that sees less reports
CLEANER. Give a caller a population floor before you change this rule.
"""

from __future__ import annotations

import re
from pathlib import Path

#: Whole-file test modules by name. `test_support.rs` is a fixture crate-side;
#: `test.rs` and `tests.rs` are the two spellings of the inline convention.
TEST_FILE_NAMES = frozenset({"tests.rs", "test.rs", "test_support.rs"})

#: The repo's other convention. `foo_tests.rs` is always declared as
#: `#[cfg(test)] mod foo_tests;`, sometimes through a `#[path]` attribute, which
#: is why looking at the parent directory does not find them.
TEST_FILE_SUFFIX = "_tests.rs"

_INNER_CFG_TEST = re.compile(r"^[ \t]*#!\[cfg\(test\)\]", re.M)


def is_test_path(path: Path) -> bool:
    """Does this path NAME a test file? A proxy — see `file_is_test_only`."""
    return (
        "tests" in path.parts
        or path.name in TEST_FILE_NAMES
        or path.name.endswith(TEST_FILE_SUFFIX)
    )


def file_is_test_only(text: str) -> bool:
    """Does `#![cfg(test)]` compile away this WHOLE file?

    The attribute must sit at brace depth 0. The same spelling inside a `mod`
    block compiles out only that block, and reading it as the file would drop
    production code out of the corpus with no sign that it happened.
    """
    for match in _INNER_CFG_TEST.finditer(text):
        before = text[: match.start()]
        if before.count("{") == before.count("}"):
            return True
    return False
