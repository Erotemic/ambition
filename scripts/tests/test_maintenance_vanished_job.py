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
was extended to stop. See `docs/recipes/checks-that-did-not-run.md`, member 14.

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
    # ⛔ THE WINDOW IS THE JOB'S OWN ARGUMENT LIST, not a character count. This
    # read `PLAN[i : i + 400]` until 2026-09-03 and broke the moment the job
    # grew a comment and four more corpus paths — a guard that fails because
    # the thing it guards got LONGER is measuring the wrong span.
    i = PLAN.index('"--vanished",')
    end = PLAN.index("],", i)
    window = PLAN[i:end]
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


def test_the_vanished_baseline_is_a_commit_git_can_actually_reach():
    """The pinned SHA must be an ancestor of HEAD, not merely well-formed.

    ⛔⛤ THE SIBLING GUARD ABOVE CHECKED THE SHAPE AND NOT THE REACHABILITY, and
    the baseline spent five weeks as a PRE-EPOCH commit that is an ancestor of
    no ref — present in the checkout that wrote it as a dangling object, absent
    from every clone. `check_planning_citations.py` exits 1 when it cannot
    resolve the ref, so the job was red for everyone but one machine while
    reading green here.

    ⚠ `--is-ancestor`, not `cat-file -e`. Existence is exactly the test that
    passed on the machine that had the object; the question is whether git can
    get there from a ref a clone would fetch. This is the same distinction
    `test_no_unresolvable_citation_that_the_epoch_did_not_grandfather` draws for
    prose citations — a commit can exist locally and be reachable from nothing.
    """
    import subprocess

    line = next(
        l for l in PLAN.splitlines() if l.startswith("PLANNING_VANISHED_BASELINE")
    )
    ref = line.split("=", 1)[1].strip().strip('"')
    reachable = subprocess.run(
        ["git", "merge-base", "--is-ancestor", ref, "HEAD"],
        cwd=REPO,
        capture_output=True,
    )
    assert reachable.returncode == 0, (
        f"the vanished baseline {ref} is not an ancestor of HEAD, so a fresh "
        "clone cannot resolve it and the maintenance job fails there while "
        "passing on any checkout that happens to hold the object. Pin a commit "
        "on the mainline — the epoch root is the oldest one a clone can read."
    )
