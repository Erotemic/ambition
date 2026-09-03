"""`check_doc_links.py` is a gate guard and had no test of its own.

It is one of only three `check_*.py` the default gate runs directly, and since
2026-09-03 `check_agent_kb.py` IMPORTS its `_blank_code_spans` so the two
checkers cannot disagree about what a code span is. A helper with a second
consumer and no test is a change waiting to break something silently.

⛔ THE OFFSET RULE IS THE SUBTLE ONE and the module says so in its own comment:
blanking must PRESERVE OFFSETS, because `line_for_offset` counts newlines up to
a match. Deleting the spans instead of blanking them would report every later
finding on the wrong line -- a checker that points at the wrong line is worse
than one that stays quiet, and nothing else in the suite would notice.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/check_doc_links.py"


@pytest.fixture(scope="module")
def mod():
    spec = importlib.util.spec_from_file_location("doc_links", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_blanking_preserves_offsets(mod):
    """⛔ THE LOAD-BEARING PROPERTY. Same length in, same length out."""
    text = "a `[x](y.md)` b\n\n```\n[q](z.md)\n```\ntail\n"
    assert len(mod._blank_code_spans(text)) == len(text)


def test_a_link_in_a_code_span_is_not_collected(mod):
    """The real case: a page NAMING the `[text](other.md#anchor)` construct."""
    text = "explaining `[text](other.md#anchor)` as a form\n"
    assert list(mod.collect_links(text)) == []


def test_a_link_in_a_fenced_block_is_not_collected(mod):
    text = "before\n\n```\n[example](nowhere.md)\n```\n\nafter\n"
    assert list(mod.collect_links(text)) == []


def test_a_reference_definition_in_a_fence_is_not_collected(mod):
    """⛔ THE MODULE APPLIED ITS OWN RULE TO ONE MATCHER OF TWO. `REF_RE` scanned
    the RAW text, so `[label]: nowhere.md` inside a ``` block was collected and
    reported broken, while `[text](nowhere.md)` beside it was correctly ignored.
    Latent when found -- no doc in the tree demonstrated the syntax -- which is
    exactly when it is cheap to fix."""
    fenced = "before\n\n```\n[label]: nowhere_at_all.md\n```\n\nafter\n"
    assert list(mod.collect_links(fenced)) == []


def test_a_real_reference_definition_is_still_collected(mod):
    """⛔ THE PREMISE for the test above."""
    assert [t for _, t in mod.collect_links("[label]: real/path.md\n")] == [
        "real/path.md"
    ]


def test_a_real_link_outside_code_is_collected(mod):
    """⛔ THE PREMISE. Without it the three tests above could pass because
    `collect_links` returns nothing at all.

    ⚠ It yields `(offset, target)` in that order. Reading it the other way round
    makes every link look broken -- 928 of them, which is the whole corpus, and
    is how I first wrote this.
    """
    found = list(mod.collect_links("see [here](real.md)\n"))
    assert [t for _, t in found] == ["real.md"], found


def test_an_image_is_not_a_link(mod):
    """`LINK_RE` carries a `(?<!!)`: `![alt](x.png)` is an embed, and reporting
    a missing image as a broken doc link would be a different finding wearing
    this one's name."""
    assert list(mod.collect_links("![alt](missing.png)\n")) == []


def test_line_for_offset_counts_from_one(mod):
    text = "one\ntwo\nthree\n"
    assert mod.line_for_offset(text, 0) == 1
    assert mod.line_for_offset(text, text.index("three")) == 3


@pytest.mark.parametrize(
    "target,exists",
    [
        ("sibling.md", True),
        ("#anchor-only", True),
        ("https://example.com/x.md", True),
        ("mailto:someone@example.com", True),
        ("<sibling.md>", True),
        ("sibling.md#section", True),
        ("nope.md", False),
    ],
)
def test_local_target_resolution(mod, tmp_path, target, exists):
    (tmp_path / "sibling.md").write_text("x")
    source = tmp_path / "doc.md"
    source.write_text("x")
    assert mod.local_target_exists(tmp_path, source, target) is exists


def test_the_live_tree_has_no_broken_links(mod):
    """The population the gate actually checks, asserted rather than assumed."""
    broken = []
    for path in mod.iter_markdown(REPO):
        text = path.read_text(encoding="utf-8", errors="replace")
        for _offset, target in mod.collect_links(text):
            if not mod.local_target_exists(REPO, path, target):
                broken.append((str(path.relative_to(REPO)), target))
    assert not broken, f"{len(broken)} broken local link(s): {broken[:5]}"


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))


def test_a_double_backtick_code_span_hides_the_link_inside_it():
    """⛔ THE CLOSING RUN MUST MATCH THE OPENING RUN.

    `` `{1,3}[^`]*`{1,3} `` paired the OPENING two backticks with the inner ONE
    and the inner one with the closing two, so on CommonMark's `` `x` `` form it
    blanked both delimiter groups and left the content exposed.

    ⚠ Measured, not imagined: `` `[text](other.md#anchor)` `` inside double
    backticks in `dev/journals/blind-checks-2026-09-03.md` survived blanking and
    was reported as a broken link by `check_agent_kb.py`, which imports this
    helper — reddening `./run_tests.sh --maintenance` on exactly the false
    positive that journal row exists to describe.
    """
    import check_doc_links as cdl

    text = "was `` `[text](other.md#anchor)` `` in backticks"
    blanked = cdl._blank_code_spans(text)
    assert "other.md#anchor" not in blanked, (
        "a link inside a double-backtick code span is an EXAMPLE, not a link; "
        f"it survived blanking: {blanked!r}"
    )
    assert len(blanked) == len(text), (
        "offsets must be preserved so line numbers stay right"
    )
    # And the ordinary cases still work.
    assert "y.md" not in cdl._blank_code_spans("a `[x](y.md)` span")
    assert "keep.md" in cdl._blank_code_spans("a real [x](keep.md) link")
