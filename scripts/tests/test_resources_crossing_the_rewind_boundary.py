"""The boundary census must find the defects that are already measured.

Three resources are known to be written outside the rewinding schedule and
consumed inside it, each held by a witness with an in-sim control arm in
`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`:

    NewGameResetRequested    menu press -> 0 commits
    OwnedItems               Update grant -> never lands
    CutsceneAdvanceRequest   host dismiss -> beat stays

⛔ THOSE THREE ARE THIS CENSUS'S KNOWN ANSWERS, and they are the only defence
against the failure mode a sweep like this actually has: reporting FEWER rows
looks like a tidier codebase and is indistinguishable from a parser that stopped
matching. A census that cannot see a defect somebody already reproduced is
broken, however clean its output reads.

⚠ The three do not all land in the same bucket, which is the point of asserting
membership rather than a count: two are rollback-REGISTERED (so the sibling
mutator guard can see them) and one carries no registration at all (so it cannot).
"""

from __future__ import annotations

import functools
import importlib.util
import subprocess
import sys
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/resources_crossing_the_rewind_boundary.py"

# Resource -> the system whose `Update`-side write makes it a crossing. Naming
# the SITE and not just the type is what makes a regression readable: if the
# type is still found but through a different writer, the row moved rather than
# vanished.
MEASURED_DEFECTS = {
    "NewGameResetRequested": "dispatch_menu_action",
    "OwnedItems": "dispatch_menu_action",
    "CutsceneAdvanceRequest": "apply_menu_frame_to_cutscene_request",
}


# ⭐ ONE MODULE INSTANCE FOR THE WHOLE FILE, and it is HALF of a 2.3× the other
# half of which lives in the script. `crossings()` re-reads every production
# source and is `lru_cache`d there; a fresh `importlib` module per arm threw that
# cache away every time. Measured: neither cache 271 s, either one alone 260-271 s,
# both 117 s.
#
# ⚠ SAFE because the two arms that mutate the module's tables use
# `monkeypatch.setitem`, which restores them at teardown — a plain assignment
# here would leak into the next arm.
@functools.lru_cache(maxsize=1)
def load():
    spec = importlib.util.spec_from_file_location("boundary", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules["boundary"] = module
    spec.loader.exec_module(module)
    return module


def test_every_measured_defect_is_still_visible_to_the_census():
    found = load().crossings()
    missing = sorted(name for name in MEASURED_DEFECTS if name not in found)
    assert not missing, (
        f"the census no longer sees {missing}, each of which has a witness that "
        "reproduces the defect. Either the sweep's parsing broke or the defect "
        "was fixed -- check WHICH, because the two look identical here"
    )


def test_the_population_did_not_collapse():
    """A sweep that matches nothing prints a clean bill of health."""
    module = load()
    found = module.crossings()
    assert len(found) >= 30, (
        f"only {len(found)} resources cross the boundary. It was 52 when this "
        "census was written; a collapse is the empty-corpus signature, not a "
        "tidier codebase"
    )
    assert len(module.resource_types()) >= 300, (
        "the `Resource` derive scan collapsed, so the filter that removes `App`, "
        "`Commands` and `NextState` is now removing everything"
    )


def test_a_stale_harmless_entry_fails(monkeypatch, capsys):
    """`CROSSING_IS_HARMLESS` must not outlive its subject.

    ⚠ `sys.argv` IS PYTEST'S HERE, so `main()`'s parser exits on pytest's own
    flags. Replacing argv is the whole reason this is not a subprocess test: the
    stale entry has to be injected into the module the assertion reads.
    """
    module = load()
    monkeypatch.setattr(sys, "argv", ["resources_crossing_the_rewind_boundary.py"])
    monkeypatch.setitem(
        module.CROSSING_IS_HARMLESS, "AResourceThatNeverCrossed", "invented by a test"
    )
    assert module.main() == 1
    assert "AResourceThatNeverCrossed" in capsys.readouterr().out


def test_a_stale_filed_entry_fails(monkeypatch, capsys):
    """`FILED` must not outlive its subject either, and for a sharper reason.

    A stale `CROSSING_IS_HARMLESS` row absorbs the next type to take its name. A
    stale `FILED` row does that AND keeps a question alive that the code has
    already answered — so the ruling's own subject count is wrong in the
    direction of asking a maintainer for a decision nobody needs.
    """
    module = load()
    monkeypatch.setattr(sys, "argv", ["resources_crossing_the_rewind_boundary.py"])
    monkeypatch.setitem(module.FILED, "AResourceThatNeverCrossed", "Q000 — invented")
    assert module.main() == 1
    assert "AResourceThatNeverCrossed" in capsys.readouterr().out


def test_a_filed_row_is_reported_apart_from_an_unexamined_one(capsys):
    """⛔ THE DISTINCTION IS THE POINT. `CutsceneAdvanceRequest` crosses in the
    defect's own shape and has `Q136` in front of it; a row nobody has looked at
    does not. Reporting both as UNCLASSIFIED made the census unable to answer the
    question its own docstring says it exists for — how many resources a ruling
    is responsible for."""
    module = load()
    found = module.crossings()
    for name, why in module.FILED.items():
        assert name in found, f"{name} is FILED and no longer crosses the boundary"
        assert "Q" in why, f"{name}'s FILED entry names no question: {why!r}"
    out = subprocess.run(
        [sys.executable, str(SCRIPT)], capture_output=True, text=True
    ).stdout
    assert "FILED, awaiting a ruling: " in out
    assert "UNCLASSIFIED with a per-frame `Update` writer: " in out


def test_it_is_green_against_the_tree():
    proc = subprocess.run(
        [sys.executable, str(SCRIPT)], capture_output=True, text=True
    )
    assert proc.returncode == 0, proc.stdout + proc.stderr
    assert "written on BOTH sides of the rewind boundary" in proc.stdout
