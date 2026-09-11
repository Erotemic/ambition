"""The extents census refuses a file that would double-count, rather than summing it.

⛔⛤ **THIS EXISTS BECAUSE THE CENSUS ALREADY DID SUM ONE.**
`cellular_automaton.ron` is the only shipped move table carrying TWO entities
over one contract, and the reader keyed its volume and clock lists by MOVE ID
alone — so all 26 of its moves accumulated twice, and a reader extending the
script reported that fighter TYING the roster's outlier at 2 x 0.080 s. The
`n` column added to make multihits honest had been printing the duplicate the
whole time; two readers looked straight at it with different priors.

⇒ Keyed by `(entity, move)` now, and a repeat of that pair is a REFUSAL. This
test is what makes the refusal a fact rather than a line of code nothing runs:
an unexercised `raise` is a claim about a branch nobody has taken.
"""
from __future__ import annotations

import pathlib
import sys
import tempfile

import pytest

REPO = pathlib.Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO / "scripts"))

import measure_authored_strike_extents as census  # noqa: E402

ONE_ENTITY = """(
    schema_version: 1,
    entities: [
        (
            id: "probe",
            contracts: (
                moveset: Some((
                    verbs: {
                        "attack": "swat",
                        "attack_up": "swat",
                        "attack_down": "swat",
                        "attack_air": "swat",
                        "attack_air_up": "swat",
                        "special": "swat",
                    },
                    moves: [
                        (
                            id: "swat",
                            windows: [
                                (
                                    start_s: 0.1,
                                    end_s: 0.2,
                                    tag: Active,
                                    volumes: [
                                        (
                                            half_extents: (12.0, 10.0),
                                        ),
                                    ],
                                ),
                            ],
                        ),
                    ],
                )),
            ),
        ),
    ],
)
"""


def write(text: str) -> pathlib.Path:
    handle = tempfile.NamedTemporaryFile(
        "w", suffix=".ron", delete=False, encoding="utf-8"
    )
    handle.write(text)
    handle.close()
    return pathlib.Path(handle.name)


def test_a_well_formed_table_reads():
    """⛔ THE PREMISE. Without it, "the duplicate is refused" is satisfied by a
    reader that refuses everything."""
    verbs, volumes, clocks = census.read(write(ONE_ENTITY))
    assert verbs["attack"] == "swat"
    assert volumes[("probe", "swat")] == [(12.0, 10.0)]
    assert clocks[("probe", "swat")] == [(0.1, 0.2)]


def test_one_move_declared_twice_under_one_entity_is_refused():
    """⛔⛔ A REPEAT IS A BUG, NOT A MERGE — the failure that produced a doubled
    census, made unrepresentable at the place it would enter."""
    doubled = ONE_ENTITY.replace(
        """                        (
                            id: "swat",""",
        """                        (
                            id: "swat",
                            windows: [],
                        ),
                        (
                            id: "swat",""",
        1,
    )
    assert doubled != ONE_ENTITY, "the edit did not apply to the fixture"
    with pytest.raises(SystemExit) as refusal:
        census.read(write(doubled))
    assert "twice" in str(refusal.value), str(refusal.value)
