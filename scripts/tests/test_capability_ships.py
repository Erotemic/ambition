"""`check_capability_ships.py` holds against the live tree.

The guard finds a resource that every shipping build reads as `Option<Res<T>>`
and that only a DEV-gated module ever installs — S35's shape, where the frozen
seat topology existed solely inside `#[cfg(feature = "dev_tools")]` code and so
was absent from every build a player runs.

⚠ it lives here rather than as a `run_tests.py` job for the same reason as the
other guards added on 2026-08-01: this suite runs first and cheaply, and a guard
nobody executes is indistinguishable from one that passes.
"""

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
