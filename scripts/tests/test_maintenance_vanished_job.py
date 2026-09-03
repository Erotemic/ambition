"""`--maintenance` must run the vanished-citation check, and `--strict` with it.

⛔ WHY THIS GUARD EXISTS, and it is not "the job might get deleted". The check it
guards spent its whole life as a *correct instrument nobody aimed*: its own unit
tests exercised `vanished_report` six ways and passed, while no gate, lane or CI
job ever pointed it at `docs/planning` with a real ref. A green suite covered the
FUNCTION and not the corpus. The first real run returned 45 findings.

⛔ AND `--strict` IS THE LOAD-BEARING HALF. Without it the check PRINTS its
findings and exits 0 — verified 2026-09-03 at this baseline: 13 findings, exit 0
bare, exit 1 with `--strict`. A maintenance job that lists real problems and
reports success is the same failure one level down, and it is the shape this lane
was extended to stop. See `docs/recipes/checks-that-did-not-run.md`, member 13.

⚠ These read the plan as TEXT rather than importing it, the way
`test_probe_tests_are_named_probe.py` does: `run_tests.py` resolves sibling
modules at import time and is not importable from a test's working directory.
"""

from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
PLAN = (REPO / "scripts" / "run_tests.py").read_text(encoding="utf-8")


def test_the_maintenance_lane_runs_the_vanished_check():
    assert '"--vanished",' in PLAN, (
        "run_tests.py no longer plans the vanished-citation job. It was added to "
        "--maintenance because nothing else in the repository ever ran it; "
        "removing it returns the check to being tested but never aimed."
    )
    assert "PLANNING_VANISHED_BASELINE" in PLAN, (
        "the vanished job no longer names its baseline constant"
    )


def test_the_vanished_job_can_actually_fail():
    """The one assertion that stops this becoming a decorative job."""
    i = PLAN.index('"--vanished",')
    window = PLAN[i : i + 400]
    assert '"--strict",' in window, (
        "the vanished job lost `--strict`, so it exits 0 with findings on "
        "screen and can never fail. That is the exact defect this lane exists "
        "to catch, reintroduced in the lane itself."
    )


def test_the_baseline_is_a_pinned_ref_not_a_range_or_a_symbol():
    """⛔ FIXED REF, NEVER A ROLLING WINDOW.

    A rolling baseline lets a finding stop being a finding because the window
    slid past the rename — nobody decides that, and the lane quietly means
    something new. `HEAD~N`, a branch name or an `A..B` range would each do it.
    """
    line = next(
        l for l in PLAN.splitlines() if l.startswith("PLANNING_VANISHED_BASELINE")
    )
    ref = line.split("=", 1)[1].strip().strip('"')
    assert len(ref) == 40 and all(c in "0123456789abcdef" for c in ref), (
        f"the vanished baseline must be a full pinned SHA, got {ref!r}. A "
        "rolling or symbolic baseline changes what the lane means without "
        "anyone choosing to."
    )
