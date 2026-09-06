"""The coverage footer's gated-test COUNT must not rot.

`run_tests.py`'s footer has always said the default plan does not cover "tests
behind `#[cfg(feature = ...)]`". That is true and gives no magnitude, and the
magnitude is the whole point: measured 2026-09-03 it is 783 tests across 29
crates, which makes it the largest omission the footer names. A reader who meets
the qualitative caveat reads past it; one who meets a number does not.

⛔ SO THE NUMBER IS NOW A RATCHET, because a stated figure is exactly the thing
that goes stale — this repository has caught that in `closure_size`, in
`MODULES.md`'s crate count and in the capability footprint's sub-lists, all in
one week. When this test goes red it is usually not a defect: somebody added a
feature-gated test. ⇒ That is worth one red, because a gated test does NOT run
in the default plan, and the person adding it should learn that from the gate
rather than from a bug six weeks later.

⚠ The scanner's own count is approximate and says so (it over-counts default-on
features and under-counts transitively gated modules). Pinning it is still
right: what must not drift is the footer AGREEING with the tool the footer sends
you to. Both move together or the footer is lying about the tool's answer.
"""

from __future__ import annotations

import importlib.util
import re
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
GATE = REPO / "scripts/run_tests.py"
SCANNER = REPO / "scripts/feature_gated_tests.py"


def load(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def footer() -> str:
    gate = load(GATE, "gate_under_test")
    return gate.coverage_notice(exhaustive=False, filtered=False)


@pytest.fixture(scope="module")
def measured() -> tuple[int, int]:
    """`(tests, crates)` from the scanner the footer names."""
    scanner = load(SCANNER, "gated_scanner")
    tests = crates = 0
    for crate in scanner.crate_dirs():
        _, gated, _ = scanner.scan_crate(crate)
        if gated:
            tests += gated
            crates += 1
    return tests, crates


def test_the_footer_states_a_number_at_all(footer):
    """⛔ THE PREMISE. If the footer stops naming a count, the assertions below
    would have nothing to compare and could pass by matching nothing."""
    assert re.search(r"\b\d{2,}\s+tests\s+across\s+\d+\s+crates", footer), (
        "the footer no longer states the gated-test magnitude:\n" + footer
    )


def test_the_stated_count_matches_the_scanner_it_names(footer, measured):
    tests, crates = measured
    m = re.search(r"\b(\d{2,})\s+tests\s+across\s+(\d+)\s+crates", footer)
    stated = (int(m.group(1)), int(m.group(2)))
    assert stated == (tests, crates), (
        f"the footer says {stated[0]} tests across {stated[1]} crates; "
        f"`scripts/feature_gated_tests.py` now measures {tests} across {crates}.\n"
        "⇒ If you ADDED a feature-gated test, that test does NOT run in the "
        "default gate plan — only under --run-everything-you-probably-dont-need-this. "
        "Update the number in `coverage_notice` and say so in your commit."
    )


def test_the_footer_says_where_the_union_job_lives(footer):
    """Naming the count without naming WHY nothing runs it leaves the reader to
    guess that some other job might cover it."""
    assert "everything" in footer and "union" in footer.lower()


def test_the_exhaustive_plan_does_not_carry_the_warning(footer):
    """The omission is real only for the non-exhaustive plan; printing it after
    a run that DID execute the union job would be false."""
    gate = load(GATE, "gate_under_test")
    assert "tests across" not in gate.coverage_notice(exhaustive=True, filtered=False)


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
