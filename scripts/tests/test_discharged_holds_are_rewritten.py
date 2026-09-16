"""A planning row never announces a hold is discharged while still stating it.

⛔⛔ THREE INSTANCES ON 2026-09-11, BY TWO AGENTS, AND ONE COST A PACKET BRIEF. A
row adds a `DELIVERED` banner at the top when its hold is discharged and leaves
the `**HOLD:**` sentence twenty lines below saying wait. A7 was in that state
while an agent was briefed from it to redo work that had landed the day before;
A4 was too, and a THIRD banner was then added above A4's still-live hold by the
agent who had just been told about A7.

⚠ THE HEALTHY ANSWER IS ZERO FINDINGS, which is also what a scanner that parsed
nothing reports, and also what a scanner whose pattern rotted reports. So this
asserts all three: the corpus is SEEN, a known-bad row is CAUGHT, and a row whose
hold is discharged-and-merely-quoted is NOT caught — the last one matters because
every corrected row on this convention quotes its own old hold text.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts" / "check_discharged_holds_are_rewritten.py"

# ⚠ The arms above run the script as a SUBPROCESS, which is the right shape for
# its exit codes. Rule 3's arms call the predicate DIRECTLY, because what they
# pin is which sentences it recognises, and routing a three-line fixture through
# a whole-corpus scan would test the scan instead.
sys.path.insert(0, str(REPO / "scripts"))
import check_discharged_holds_are_rewritten as guard  # noqa: E402

FLAGGED = [
    "## A7. Separate item custody",
    "**THE ENUMERATION THIS HOLD ASKS FOR IS DELIVERED:** page.md",
    "",
    "**HOLD:** after A1, enumerate item occurrence, holder and inventory writers.",
]
# The shape every CORRECTED row on this convention has: a banner, and the old
# hold text quoted in prose beneath it. It must not be flagged.
CLEAN = [
    "## A4. Co-locate accepted control",
    "**THE MAP THIS HOLD ASKS FOR IS DELIVERED:** page.md",
    "",
    '**HOLD DISCHARGED 2026-09-10** by the map above. ⚠ This line read *"HOLD on',
    'extraction: first map writers and select production fixtures"* until 2026-09-11.',
]


def _load():
    import importlib.util

    spec = importlib.util.spec_from_file_location("dh", SCRIPT)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def test_the_shipped_corpus_has_no_row_that_contradicts_itself():
    out = subprocess.run(
        [sys.executable, str(SCRIPT)], cwd=REPO, capture_output=True, text=True
    )
    assert out.returncode == 0, f"{out.stdout}{out.stderr}"
    # ⛔ The population, not just the verdict: a clean report over an empty scan
    # root is the failure this assertion exists for, and the script prints both.
    assert "rows," in out.stdout and "bold hold line(s) scanned" in out.stdout, out.stdout


def test_it_catches_the_shape_that_cost_a_packet_brief():
    banners, live = _load().offenders(FLAGGED, 0)
    assert banners and live, "the A7 shape must be caught, or a clean run means nothing"


def test_it_does_not_catch_a_row_that_merely_quotes_its_old_hold():
    banners, live = _load().offenders(CLEAN, 0)
    assert not live, (
        "a corrected row quotes the hold it discharged; flagging that would make "
        f"the fix fail the check: {live}"
    )


def test_it_refuses_an_empty_scan_root_instead_of_passing(tmp_path):
    out = subprocess.run(
        [sys.executable, str(SCRIPT), str(tmp_path)],
        cwd=REPO,
        capture_output=True,
        text=True,
    )
    assert out.returncode == 1, "an empty corpus must refuse, not report clean"
    assert "POPULATION IS NOT THERE" in out.stdout, out.stdout


def test_rule_three_catches_a_prose_gate_on_a_finished_campaign():
    """⛔ THE KNOWN-ANSWER CONTROL FOR THE THIRD SPELLING, and it is verbatim.

    `consolidation/architecture-census.md` carried exactly this sentence while
    A10 was closed and its demolition finished. Both other rules were blind to
    it: it is not a bold `**HOLD:**` marker, and its file announces no discharge
    of its own. A widening that is not poisoned against the defect it was built
    for is a wider POPULATION, not a wider REACH.
    """
    stale = guard.stale_gates(
        "The current A10 implementation should finish before another agent "
        "changes the room publication model.\n",
        {"A10"},
    )
    assert [(name) for _, name, _ in stale] == ["A10"], stale


def test_rule_three_leaves_ordinary_history_alone():
    """⚠ The corpus is full of legitimate mentions of finished campaigns."""
    assert guard.stale_gates(
        "A10's room scope landed first, and a publication authority followed.\n",
        {"A10"},
    ) == []


def test_rule_three_accepts_a_struck_through_rewrite():
    """⚠ Striking the id through IS the rewrite this check asks for."""
    assert guard.stale_gates(
        "**DO NOT START BEFORE:** ~~A10~~ and the identity checkpoint.\n",
        {"A10"},
    ) == []
