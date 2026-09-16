"""The canonical-mint census, against synthetic corpora with known answers.

⛔⛤ **A POSITIVE CONTROL PINNED TO A LIVE DEFECT DIES WHEN THE DEFECT IS FIXED.**
The defect this checker exists for -- `SimId::singleton("session", "root")` minted
in two production places -- was repaired in the same sitting the checker was
written, so an arm asserting "the checker finds it" would have been red on
arrival and green forever after for the wrong reason. Every arm here builds its
own corpus instead.
"""

from __future__ import annotations

import importlib.util
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]


def _load():
    name = "one_owner_per_canonical_identity"
    spec = importlib.util.spec_from_file_location(name, REPO / "scripts" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


CHECK = _load()


def _corpus(*files: tuple[str, str]):
    return [(pathlib.Path(name), text) for name, text in files]


def test_two_sites_spelling_one_constant_identity_are_reported():
    mints = CHECK.constant_mints(
        _corpus(
            ("a.rs", 'let root = SimId::singleton("session", "root");'),
            ("b.rs", 'let also = SimId::singleton("session", "root");'),
        )
    )
    assert mints[("singleton", '"session", "root"')] == ["a.rs:1", "b.rs:1"]


def test_a_wrapped_call_is_found_because_the_scan_is_not_line_by_line():
    """⛔ THE FIRST VERSION SCANNED LINE BY LINE AND REPORTED ONE MINT WORKSPACE-WIDE.

    Nearly every mint in this repository is wrapped across lines by the
    formatter, including the one the checker was written for, so a line scan was
    a census of the few short ones. The number -- 1 -- is what exposed it.
    """
    mints = CHECK.constant_mints(
        _corpus(
            (
                "wrapped.rs",
                'SimId::singleton(\n    "session",\n    "root",\n);',
            )
        )
    )
    assert list(mints) == [("singleton", '"session", "root"')]
    # The reported line is where the CALL starts, not where the argument sits.
    assert mints[("singleton", '"session", "root"')] == ["wrapped.rs:1"]


def test_a_variable_argument_is_not_a_constant_identity():
    """Two `SimId::placement(id)` sites mint two DIFFERENT identities."""
    mints = CHECK.constant_mints(
        _corpus(
            ("a.rs", "SimId::placement(&feature.0)"),
            ("b.rs", "SimId::placement(&spawn.id)"),
            ("c.rs", "SimId::child(parent.as_str(), counter.next())"),
        )
    )
    assert mints == {}


def test_a_snapshot_reconstruction_is_not_a_mint():
    assert CHECK.constant_mints(_corpus(("a.rs", 'SimId::from_snapshot("session:root")'))) == {}


def test_one_site_for_one_identity_is_the_healthy_answer():
    mints = CHECK.constant_mints(
        _corpus(("a.rs", 'SimId::encounter("goblin_encounter")'))
    )
    assert len(mints) == 1
    assert all(len(sites) == 1 for sites in mints.values())


def test_the_repository_has_no_unadjudicated_duplicate():
    mints = CHECK.constant_mints()
    # ⛔ THE FLOOR FIRST. Every filter this census inherits removes files, and a
    # scan root that stopped matching prints the same "no duplicates" as a
    # healthy tree.
    assert mints, "the production corpus yielded no constant mint at all"
    duplicates = {
        key: sites
        for key, sites in mints.items()
        if len(sites) > 1 and key not in CHECK.ADJUDICATED
    }
    assert not duplicates, f"constant identities with two production mints: {duplicates}"


def test_every_adjudicated_row_still_names_two_sites():
    """⚠ AN ADJUDICATION IS A DECISION ABOUT A MEASUREMENT, AND MEASUREMENTS MOVE.

    A row whose second site has since been deleted is a stale decision that would
    silently excuse a NEW duplicate of the same identity.
    """
    mints = CHECK.constant_mints()
    for key, reason in CHECK.ADJUDICATED.items():
        assert key in mints, f"adjudicated {key} is no longer minted anywhere: {reason}"
        assert len(mints[key]) > 1, (
            f"adjudicated {key} now has ONE site, so the row excuses nothing and "
            f"should be deleted: {mints[key]}"
        )
