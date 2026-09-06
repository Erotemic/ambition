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
