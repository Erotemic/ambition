"""The `Consumed` deferral guard must fire on the case it exists for.

It is green against the tree — no live producer exists today — so a test that
only ran it would prove nothing. These drive each arm from its own branch.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "check_consumed_whereabouts_owes_a_rewind_answer.py"


def _module():
    spec = importlib.util.spec_from_file_location("check_consumed", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_it_is_green_against_the_tree() -> None:
    proc = subprocess.run([sys.executable, str(SCRIPT)], capture_output=True, text=True)
    assert proc.returncode == 0, proc.stdout + proc.stderr


def test_a_new_mention_outside_the_census_fails(monkeypatch) -> None:
    """⭐ THE CASE IT EXISTS FOR. A file that starts naming the variant has to
    answer whether it PRODUCES one, because the ledger's derived rollback
    registration is only sound while nothing does."""
    module = _module()
    monkeypatch.setattr(module, "CENSUS", {})
    assert module.main() == 1


def test_a_stale_census_entry_fails(monkeypatch) -> None:
    """The other direction, so the census cannot rot into a list of files that
    no longer say anything — which would make the net look tighter than it is."""
    module = _module()
    census = dict(module.CENSUS)
    census["crates/ambition_time/src/lib.rs"] = "mentions nothing"
    monkeypatch.setattr(module, "CENSUS", census)
    assert module.main() == 1


def test_an_emptied_corpus_fails(monkeypatch) -> None:
    """⛔⛔ A CHECK THAT CANNOT FAIL. Rename the variant and the grep finds
    nothing, which reads exactly like compliance."""
    module = _module()
    monkeypatch.setattr(module, "VARIANT", "OccurrenceWhereabouts::NoSuchVariant")
    assert module.main() == 1


def test_a_discharged_obligation_fails(monkeypatch) -> None:
    """⭐ If the ledger stops being registered DERIVED, the premise this guard
    defends is gone — either the obligation was discharged (delete the guard) or
    something else changed (say what). Passing quietly is the one wrong answer."""
    module = _module()
    monkeypatch.setattr(module, "DERIVED_REGISTRATION", "declare_rollback_no_such_thing")
    assert module.main() == 1
