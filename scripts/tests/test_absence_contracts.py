"""The absence guard has to FIRE, and has to stay quiet on prose.

Every contract in `ABSENCE_CONTRACTS` is green against the live tree — that is
the point of it — so a test that only ran the script would prove nothing. A
guard that is green at minute zero guards nothing (learned three times on this
repo's goal checks). These tests do the two things the live run cannot:

* feed each contract a line that VIOLATES it and require a hit, so the pattern
  is known to match the thing it forbids; and
* feed each contract that same text as a DOC COMMENT and require silence, which
  is the specific recurrence this mechanism exists to survive — three separate
  absence checks went red because somebody documented the removal.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from check_absence_contracts import (  # noqa: E402
    ABSENCE_CONTRACTS,
    strip_comments_for,
    violations,
)

# One line that each contract must reject, written the way real code would.
VIOLATING_LINE = {
    "registration-does-not-demand-art": "    CharacterLoadDemand::request(&mut demand, id);",
    "no-string-keyed-sheet-row-lookup": "    let row = sheet.row_index_of(name)?;",
    "rollback-exit-oracle-is-not-quarantined": "#[ignore]",
    "fight-tests-do-not-hand-roll-damage": "    mary_hp -= 12;",
}


def confirm_patterns(contract: dict) -> list[re.Pattern]:
    compiled = []
    for pattern in contract["patterns"]:
        expression = pattern if isinstance(pattern, str) else pattern["match"]
        compiled.append(re.compile(expression))
    return compiled


@pytest.mark.parametrize("contract", ABSENCE_CONTRACTS, ids=lambda c: c["id"])
def test_each_contract_matches_a_real_violation(contract):
    line = VIOLATING_LINE[contract["id"]]
    stripped = strip_comments_for("some/file.rs", line)
    assert any(pattern.search(stripped) for pattern in confirm_patterns(contract)), (
        f"{contract['id']} would not notice {line!r} — a contract that cannot "
        "match its own violation is decoration"
    )


@pytest.mark.parametrize("contract", ABSENCE_CONTRACTS, ids=lambda c: c["id"])
def test_no_contract_fires_on_prose_describing_the_removal(contract):
    """Documenting a removal must not break the guard that verified it."""
    line = VIOLATING_LINE[contract["id"]]
    for comment in (f"/// This used to be `{line.strip()}` and no longer is.",
                    f"//! {line.strip()}",
                    f"    // {line.strip()}"):
        stripped = strip_comments_for("some/file.rs", comment)
        assert not any(
            pattern.search(stripped) for pattern in confirm_patterns(contract)
        ), (
            f"{contract['id']} fires on the doc comment {comment!r}; that is the "
            "exact failure this mechanism replaced"
        )


def test_a_diagnostic_ignore_is_tooling_and_a_bare_one_is_a_disabled_guard():
    """The oracle contract's whole subtlety, pinned.

    Two `#[ignore]`s in that file are bisection tools you run WHEN the oracle is
    red. Forbidding them outright is the noise that gets a guard waived; the
    reason string is what distinguishes opt-in tooling from a switched-off guard.
    """
    contract = next(
        c for c in ABSENCE_CONTRACTS if c["id"] == "rollback-exit-oracle-is-not-quarantined"
    )
    patterns = confirm_patterns(contract)

    tooling = '#[ignore = "diagnostic bisection: five sim boots"]'
    assert not any(p.search(tooling) for p in patterns), "opt-in tooling is allowed"

    quarantine = '#[ignore = "flaky, re-enable later"]'
    assert any(p.search(quarantine) for p in patterns), "a switched-off guard is not"


@pytest.mark.parametrize("contract", ABSENCE_CONTRACTS, ids=lambda c: c["id"])
def test_every_contract_holds_against_the_live_tree(contract):
    root = Path(__file__).resolve().parents[2]
    found = violations(contract, root)
    assert not found, (
        f"{contract['id']} is violated:\n"
        + "\n".join(f"  {path}:{number}: {text}" for path, number, text in found)
        + f"\n{contract['reason']}"
    )
