"""The unpriced-share report must not turn a missing baseline into a clean zero.

It is a REPORT, not a guard, so the only ways it can lie are by finding nothing
and saying so quietly. Both are pinned here.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "unpriced_baseline_crates.py"


def _module():
    spec = importlib.util.spec_from_file_location("unpriced_baseline", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_it_runs_against_the_committed_baseline() -> None:
    proc = subprocess.run([sys.executable, str(SCRIPT)], capture_output=True, text=True)
    assert proc.returncode == 0, proc.stdout + proc.stderr
    # ⭐ The anti-vacuity floor: the baseline must actually hold crates, or the
    # share is a division dressed as a fact.
    assert "crates:" in proc.stdout


def test_a_missing_baseline_fails_rather_than_reporting_zero(monkeypatch, tmp_path) -> None:
    """⛔ The file is the whole input. Absent, "0 placeholders" would read exactly
    like a fully measured baseline."""
    module = _module()
    monkeypatch.setattr(module, "BASELINE", tmp_path / "nope.json")
    assert module.main() == 1


def test_an_empty_crate_table_fails(monkeypatch, tmp_path) -> None:
    """A baseline with no crates would divide by zero to compute the share, or
    silently report 0% — both worse than saying the input is empty."""
    module = _module()
    empty = tmp_path / "empty.json"
    empty.write_text(json.dumps({"commit": "x", "crates": {}}), encoding="utf-8")
    monkeypatch.setattr(module, "BASELINE", empty)
    assert module.main() == 1


def test_a_crate_with_no_source_counts_as_placeholder(monkeypatch, tmp_path) -> None:
    """⚠ ABSENT is not MEASURED. A crate whose row predates `seconds_source`
    must count as unpriced, or an older baseline reads as fully measured."""
    module = _module()
    doc = tmp_path / "b.json"
    doc.write_text(json.dumps({
        "commit": "x",
        "crates": {
            "a": {"seconds_source": "measured"},
            "b": {"seconds_source": "estimated"},
            "c": {},
        },
    }), encoding="utf-8")
    monkeypatch.setattr(module, "BASELINE", doc)
    import io
    import contextlib

    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        assert module.main() == 0
    assert "2 crate(s) priced by PLACEHOLDER" in out.getvalue()
