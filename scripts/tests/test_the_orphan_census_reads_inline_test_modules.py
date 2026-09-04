"""The orphan census must not read an in-file test module as production.

⛔⛔ IT DID, AND THE NUMBER IT PRINTED WAS A FLOOR. `is_test_path` keys on the
PATH (`tests.rs`, `_tests.rs`, `/tests/`), so a `pub fn` whose only caller lived
in an in-file `#[cfg(test)] mod tests` counted as having a PRODUCTION caller.
Measured 2026-09-04: of the 1,240 source files the census treats as production,
**584 (47%)** carry such a module, and fixing it moved the census from
**156 to 278** orphans of 5,318 `pub fn`s.

⚠ THE OBVIOUS FIX IS WRONG AND MEASURING IS WHAT SAID SO. Truncating each file
at its test module is right for 535 of the 584 and DELETES REAL PRODUCTION CODE
in the other 49 — `ambition_input/src/lib.rs`,
`ambition_platformer2d_host/src/portal.rs`, `ambition_input/src/local_seats.rs`
and more put items AFTER the module. ⇒ That direction turns live functions into
FALSE ORPHANS, which is the loud failure: somebody deletes something that is
used. So the module is brace-matched and skipped, and whatever follows it stays
production.

⭐ THIS TEST PINS BOTH DIRECTIONS, because a scoping fix can fail either way:
too little and the census keeps under-reporting; too much and it invents
orphans. The second is the one worth a test even though it is rarer.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent


def _census():
    path = REPO / "scripts/orphaned_symbols.py"
    spec = importlib.util.spec_from_file_location("orphaned_symbols", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules["orphaned_symbols"] = module
    spec.loader.exec_module(module)
    return module


SOURCE = """\
pub fn before_the_module() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_test() {
        before_the_module();
        let nested = || { let _inner = 1; };
        after_the_module();
    }
}

pub fn after_the_module() {}

pub fn also_after() {
    after_the_module();
}
"""


def test_an_inline_test_modules_calls_are_not_production() -> None:
    module = _census()
    production, inline = module.split_inline_test_module(SOURCE)
    assert "before_the_module();" not in production, (
        "the call inside `mod tests` must not land in production — that is the "
        "under-report this fix exists for"
    )
    assert "before_the_module();" in inline


def test_code_after_the_test_module_is_still_production() -> None:
    """⛔ THE DANGEROUS DIRECTION: 49 real files put items after their test module."""
    module = _census()
    production, _inline = module.split_inline_test_module(SOURCE)
    assert "pub fn after_the_module" in production, (
        "an item declared AFTER the test module is production; truncating the "
        "file at the module would invent a false orphan out of every one of them"
    )
    assert "after_the_module();" in production, (
        "and so is a CALL made after it — `also_after` calls `after_the_module`, "
        "which is exactly what stops it being reported as uncalled"
    )


def test_the_brace_walk_survives_a_nested_block() -> None:
    """The module ends at its OWN closing brace, not at the first one it meets."""
    module = _census()
    production, inline = module.split_inline_test_module(SOURCE)
    assert "let _inner = 1;" in inline, "a nested block inside the module belongs to it"
    assert "let _inner = 1;" not in production


def test_a_file_with_no_test_module_is_entirely_production() -> None:
    module = _census()
    plain = "pub fn only_this() {}\n"
    production, inline = module.split_inline_test_module(plain)
    assert production == plain
    assert inline == ""
