"""The zone-name ratchet's population floor, and its ratchet behaviour.

`check_zone_name_ratchet.py` scores player-visible loading-zone names that still
look like authoring ids. It sits at 0 of 151 today — a PERFECT score, which is
the reason to test it: a check whose best possible answer is zero cannot tell
"every zone is authored" from "I parsed nothing".

The script already guards that, and the guard is the thing nothing checked:

    if total == 0: FAIL: no LoadingZone names were observed at all
                   — the check is broken, not the content

⚠ It was also the last of the 16 `scripts/check_*.py` with no test naming it,
alongside `check_doc_links` and `check_doc_link_ratchet`, both now covered.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/check_zone_name_ratchet.py"


def load():
    spec = importlib.util.spec_from_file_location("zones", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def run(module, argv):
    saved = sys.argv
    sys.argv = ["check_zone_name_ratchet.py", *argv]
    try:
        return module.main()
    finally:
        sys.argv = saved


@pytest.fixture
def rig(tmp_path, monkeypatch):
    module = load()
    baseline = tmp_path / "zones.json"
    monkeypatch.setattr(module, "BASELINE", str(baseline))

    def configure(counts, total, id_shaped, recorded=None):
        monkeypatch.setattr(module, "measure", lambda: (counts, total, id_shaped))
        if recorded is not None:
            baseline.write_text(json.dumps({"worlds": recorded}))
    return module, configure


def test_observing_nothing_fails_instead_of_scoring_perfect(rig, capsys):
    """⭐ THE ARM THAT MATTERS. A moved directory or a parse error yields zero
    rows, and zero id-shaped names out of zero is 'perfect' to any ratio."""
    module, configure = rig
    configure({}, 0, 0)
    assert run(module, ["--check"]) == 1
    assert "no LoadingZone names were observed" in capsys.readouterr().out


def test_a_real_clean_run_passes(rig):
    """⛔ THE PREMISE: the floor must not be the only way to pass or fail."""
    module, configure = rig
    configure({"w.ldtk": 0}, 151, 0, recorded={"w.ldtk": 0})
    assert run(module, ["--check"]) == 0


def test_a_rise_fails_under_check(rig):
    module, configure = rig
    configure({"w.ldtk": 3}, 151, 3, recorded={"w.ldtk": 1})
    assert run(module, ["--check"]) == 1


def test_a_world_absent_from_the_baseline_is_treated_as_zero(rig):
    """A NEW world with id-shaped names must fail rather than be ignored —
    `baseline.get(world, 0)` is what makes an unlisted world strict."""
    module, configure = rig
    configure({"new.ldtk": 2}, 151, 2, recorded={})
    assert run(module, ["--check"]) == 1


def test_a_missing_baseline_fails_rather_than_passing_vacuously(rig, capsys):
    module, configure = rig
    configure({"w.ldtk": 0}, 151, 0)
    assert run(module, ["--check"]) == 1
    assert "no baseline" in capsys.readouterr().out


def test_the_live_tree_still_has_zones_to_score():
    """The population, asserted rather than assumed: if this ever reads 0 the
    guard above fires, but a slow drift downward is worth seeing too."""
    module = load()
    _counts, total, _bad = module.measure()
    assert total > 100, f"only {total} named loading zones found; was 151"


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
