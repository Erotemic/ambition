"""The agent-KB job must generate the index before checking it.

`check_agent_kb.py` requires `.agent/index/*.json`. Those files are GENERATED
and git-ignored, so a clean checkout does not have them and the check fails on
their absence before it can check anything — the job could never pass as
written. Found by review 2026-09-02 and reproduced by deleting `.agent/index`:

    Agent KB check failed:
    - .agent/index/ is generated, ignored by Git, and currently missing or
      incomplete. Run `python scripts/generate_agent_index.py` ...

⚠ The workflow is `workflow_dispatch` only right now — push and pull_request are
commented out — so this is a manually invoked job, not an always-running gate.
That is exactly why it went unnoticed, and it is not a reason to leave it
broken: the next person to press the button is owed a job that can pass.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
WORKFLOW = REPO / ".github/workflows/test.yml"

GENERATE = "scripts/generate_agent_index.py"
CHECK = "scripts/check_agent_kb.py"


def workflow_text() -> str:
    if not WORKFLOW.exists():
        pytest.skip(f"{WORKFLOW} is absent")
    return WORKFLOW.read_text()


def run_lines() -> list[str]:
    """Every `run:` line, in file order."""
    return [
        line.strip()
        for line in workflow_text().splitlines()
        if re.match(r"\s*run:\s*\S", line)
    ]


def test_the_index_is_generated_before_it_is_checked():
    runs = run_lines()
    checks = [i for i, line in enumerate(runs) if CHECK in line]
    if not checks:
        pytest.skip("the agent-KB check is not in this workflow")
    generates = [i for i, line in enumerate(runs) if GENERATE in line]
    assert generates, (
        f"{CHECK} requires .agent/index/*.json, which are generated and "
        f"git-ignored — a clean checkout has none, so the job cannot pass "
        f"without a `{GENERATE}` step before it"
    )
    assert min(generates) < min(checks), (
        "the index must be generated BEFORE the check runs, not after"
    )


def test_the_checker_really_does_require_the_generated_index():
    """⛔ POSITIVE CONTROL. The test above is about ordering; if the checker ever
    stopped requiring the index, the ordering rule would be pinning nothing.
    """
    checker = (REPO / "scripts/check_agent_kb.py").read_text()
    assert "generate_agent_index.py" in checker, (
        "the checker no longer points at the generator; re-derive whether the "
        "workflow ordering rule above is still the real constraint"
    )
    assert ".agent/index" in checker


def test_the_workflow_trigger_is_described_honestly():
    """The job is workflow_dispatch only. A test asserting it runs on push
    would be asserting a policy nobody chose — this only pins that the file
    parses and names its trigger, so a reader is not misled about coverage.
    """
    text = workflow_text()
    assert "workflow_dispatch" in text, (
        "the workflow declares no manual trigger; if push/pull_request were "
        "re-enabled that is a real change and this test should be updated "
        "deliberately rather than deleted"
    )


def test_the_maintenance_lane_generates_before_it_checks_too():
    """⛔ THE SAME DEFECT LIVED IN THE LOCAL LANE, and this file's own docstring
    describes it for CI only.

    `./run_tests.sh --maintenance` ran `check_agent_kb.py` with no generate step
    until 2026-09-03, so it passed only on a machine that had already generated
    `.agent/index/` by hand — which every developer who has ever run the
    generator has, and a clean checkout has not. CI learned this on 2026-09-02
    and the lane did not hear about it.

    ⚠ Asserts ORDER, not mere presence: generating after the check is the same
    as not generating at all.
    """
    plan = (REPO / "scripts" / "run_tests.py").read_text(encoding="utf-8")
    gen = plan.find("scripts/generate_agent_index.py")
    chk = plan.find("scripts/check_agent_kb.py")
    assert gen != -1, (
        "run_tests.py no longer plans scripts/generate_agent_index.py; "
        "--maintenance cannot pass on a clean checkout without it"
    )
    assert chk != -1, "run_tests.py no longer plans scripts/check_agent_kb.py"
    assert gen < chk, (
        "the maintenance lane checks the agent KB before generating its index; "
        "generating afterwards is the same as not generating at all"
    )
