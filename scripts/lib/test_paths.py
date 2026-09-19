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


#: `#[cfg(` — the start of an attribute whose predicate this module EVALUATES
#: rather than pattern-matches. See [`cfg_requires_test`].
_CFG_OPEN = "#[cfg("

#: Whitespace and comments, then an optional visibility, then `mod NAME {`.
#: ⚠ The visibility group is why `#[cfg(test)] pub(crate) mod tests {` used to
#: survive; the comment group is why a `///` between the attribute and the `mod`
#: used to.
_MOD_HEAD = re.compile(
    r"\s*(?://[^\n]*\n\s*|/\*.*?\*/\s*)*"
    r"(?:pub(?:\s*\([^)]*\))?\s+)?"
    r"mod\s+[A-Za-z_][A-Za-z_0-9]*\s*\{",
    re.S,
)


def _split_cfg_args(predicate: str) -> list[str]:
    """`test, feature = "x"` -> `['test', 'feature = "x"']`, paren-aware."""
    parts, depth, current = [], 0, []
    for ch in predicate:
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append("".join(current))
            current = []
        else:
            current.append(ch)
    if "".join(current).strip():
        parts.append("".join(current))
    return [part.strip() for part in parts]


def cfg_requires_test(predicate: str) -> bool:
    """Does this `cfg(..)` predicate mean *"compiled ONLY under `cfg(test)`"*?

    ⛔⛤ **THIS IS A PREDICATE EVALUATOR AND NOT A PATTERN, BECAUSE `any` AND
    `all` ANSWER OPPOSITELY AND BOTH APPEAR IN THIS TREE.**

        #[cfg(all(test, not(target_arch = "wasm32")))]   -> test-only    (8 sites)
        #[cfg(all(test, feature = "input"))]             -> test-only    (5 sites)
        #[cfg(test)]                                     -> test-only
        #[cfg(any(test, feature = "test-support"))]      -> SHIPS        (1 site)

    ⚠ **THE LAST ONE IS THE WHOLE REASON FOR THE CARE.** `any(test, feature =
    "test-support")` compiles whenever that feature is on, and this repository
    has a live guard for exactly that condition — see the arm asserting
    `test-support` is not enabled outside `[dev-dependencies]`. Stripping it here
    would delete code that SHIPS, which is the one direction a test filter must
    never err in: over-cutting hides production facts from a census that exists
    to find them.

    `not(...)` is never test-requiring: `not(test)` is the opposite claim.
    """
    predicate = predicate.strip()
    if predicate == "test":
        return True
    for form, decide in (("all(", any), ("any(", all)):
        if predicate.startswith(form) and predicate.endswith(")"):
            args = _split_cfg_args(predicate[len(form) : -1])
            return bool(args) and decide(cfg_requires_test(a) for a in args)
    return False


def _balanced(source: str, index: int, opener: str, closer: str) -> int:
    """Index just past the `closer` matching the `opener` at `index`."""
    depth = 1
    index += 1
    while index < len(source) and depth:
        if source[index] == opener:
            depth += 1
        elif source[index] == closer:
            depth -= 1
        index += 1
    return index


def strip_test_modules(source: str) -> str:
    r"""Remove inline test-only `mod … { … }` blocks by brace balance.

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

    ⛔⛔ **AND THE PATTERN THAT REPLACED IT WAS `#\[cfg\(test\)\]\s*mod NAME
    \{`, WHICH IS THREE SEPARATE UNDERCOUNTS.** Each was measured over the 1,294
    production files, 2026-09-17:

      - a COMMENT between the attribute and the `mod` (`\s*` cannot cross a
        `///`): **2 sites**, 13,215 characters — and one of them,
        `abilities/src/ranged/sentry.rs:681`, is where the multi-writer census
        got a second writer for `Captured`, a type its own docstring lists as
        multi-writer only because a fixture wrote it;
      - a VISIBILITY before the `mod` (`pub(crate) mod tests`): **1 site**,
        `persistence/src/store.rs:222`;
      - a cfg PREDICATE rather than the bare attribute — `all(test, …)`:
        **16 sites**, eight of them `all(test, not(target_arch = "wasm32"))`.

    ⇒ Which is why the attribute is now EVALUATED by [`cfg_requires_test`] rather
    than matched, and why `#[cfg(any(test, feature = "test-support"))]` is
    deliberately left standing: it ships.

    ⚠ The brace match is naive about braces inside string literals inside a test
    body. Over-cutting loses production code, which is the direction the tail cut
    already erred in; under-cutting counts a fixture. Both move a consumer's
    population, which is why every consumer of this module carries a floor.
    """
    spans = test_module_spans(source)
    kept: list[str] = []
    cursor = 0
    for start, end in spans:
        kept.append(source[cursor:start])
        cursor = end
    kept.append(source[cursor:])
    return "".join(kept)


def test_module_spans(source: str) -> list[tuple[int, int]]:
    """The `[start, end)` character spans [`strip_test_modules`] removes.

    ⛔⛤ **THE SPANS ARE THE FACT AND THE STRIPPED TEXT IS DERIVED FROM THEM, not
    the other way round — split out 2026-09-19 because a consumer needed to map
    a position in the stripped text back to a line in the file.** That consumer
    first tried to recover the alignment by walking the two strings and matching
    characters greedily. A greedy walk finds AN embedding of a subsequence, not
    the one the deletion actually produced: every character it needs exists
    earlier in the file too, so the cursor drifts into the removed region and
    the answer is confidently wrong. It reported a declaration at line 733 as
    line 595, and reported it in a format nobody would re-check.

    ⇒ A transformation that deletes should publish WHAT it deleted. Anything
    downstream that needs to point back at the original can then do so exactly,
    and nothing has to re-derive the strip's rules to follow it.
    """
    spans: list[tuple[int, int]] = []
    index = 0
    while (found := source.find(_CFG_OPEN, index)) != -1:
        predicate_start = found + len(_CFG_OPEN) - 1
        predicate_end = _balanced(source, predicate_start, "(", ")")
        predicate = source[predicate_start + 1 : predicate_end - 1]
        rest = source[predicate_end:]
        if not (rest.startswith("]") and cfg_requires_test(predicate)):
            index = found + len(_CFG_OPEN)
            continue
        head = _MOD_HEAD.match(source, predicate_end + 1)
        if head is None:
            # `#[cfg(test)] mod tests;`, `#[cfg(test)] use ..;`, `#[cfg(test)]
            # fn helper() {}` — no module BLOCK here to balance.
            index = found + len(_CFG_OPEN)
            continue
        spans.append((found, _balanced(source, head.end() - 1, "{", "}")))
        index = spans[-1][1]
    return spans
