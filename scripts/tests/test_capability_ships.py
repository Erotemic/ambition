"""Tests that shipping compositions install resources required by live capabilities.

A capability must not depend on a resource that is only provided by developer or
optional scaffolding. The test exercises the guard against that missing-provider
shape."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


def test_every_optionally_read_capability_has_a_shipping_writer():
    result = subprocess.run(
        [sys.executable, "scripts/check_capability_ships.py"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, (
        "a capability is installed only behind a dev feature, so it is absent "
        "from every build a player runs:\n"
        f"{result.stdout}{result.stderr}"
    )


def _floor_module():
    import importlib

    return importlib.import_module("check_capability_ships")


# ── the population floor (added 2026-09-16) ────────────────────────────────


def test_the_scan_still_reaches_its_own_population() -> None:
    """⛔ A FINDING HERE IS AN INTERSECTION, SO EITHER HALF CAN EMPTY IT.

    The check needs BOTH an `Option<Res<T>>` reader and a writer to be found.
    Lose either pattern and the intersection empties silently — which reads as
    "every capability ships", the exact answer this guard exists to doubt.

    ⚠ This arm runs against the REAL tree deliberately. A synthetic workspace
    cannot answer "did the scan stop reaching the repository".
    """
    sizes = _floor_module().population_sizes()
    assert not _floor_module().population_shortfalls(), (
        f"the scan lost reach: {sizes} against {_floor_module().POPULATION_FLOOR}"
    )


def test_the_floor_can_actually_fail() -> None:
    """A floor that cannot fire guards nothing and reads identically to one
    that can."""
    module = _floor_module()
    original = dict(module.POPULATION_FLOOR)
    try:
        module.POPULATION_FLOOR["optional read types"] = (
            module.population_sizes()["optional read types"] + 1
        )
        assert module.population_shortfalls(), "the floor cannot fail, so it guards nothing"
    finally:
        module.POPULATION_FLOOR.clear()
        module.POPULATION_FLOOR.update(original)
