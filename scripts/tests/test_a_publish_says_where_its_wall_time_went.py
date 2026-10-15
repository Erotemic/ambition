"""`scripts/regen/sprites.sh` prints the wall time of each top-level phase.

The render cost counts only the renderer subprocesses. The phases between
them (the entity sprites, the atlases, the LDtk art, the variants) were not
timed, so a publish could not say where a third of its time went. The test
runs the script's own `phase` helpers with a stubbed clock.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
REGEN = REPO / "scripts/regen/sprites.sh"


def _helpers() -> str:
    text = REGEN.read_text()
    start = text.index("phase_names=()")
    end = text.index("\n}\n", text.index("print_phase_seconds() {")) + 3
    return text[start:end]


def test_each_phase_is_timed_and_the_slowest_is_first() -> None:
    script = (
        "date() { echo \"$now\"; }\n"
        + _helpers()
        + "now=100; phase 'entity sprites → /some/dir'\n"
        + "now=103; phase 'LDtk editor art:'\n"
        + "now=110; print_phase_seconds\n"
    )
    out = subprocess.run(["bash", "-c", script], capture_output=True, text=True, check=True).stdout
    lines = out.splitlines()
    assert lines[:2] == ["==> entity sprites → /some/dir", "==> LDtk editor art:"], out
    table = [line.split() for line in lines if line.startswith("    ")]
    assert "==> phase wall time: 2 phase(s), 10s total" in out, out
    assert table == [["7s", "LDtk", "editor", "art"], ["3s", "entity", "sprites", "→", "/some/dir"]], out
