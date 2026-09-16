"""The ledger-resolution guard, shown finding and shown declining.

⛔⛤ **THIS FILE IS WHY THE GUARD RUNS AT ALL.** `scripts/run_tests.py` gates
`python -m pytest scripts/tests` as "repo tooling"; a `check_*.py` with no test
beside it is invoked by NOTHING. MEASURED 2026-09-16: of 31 check scripts, two
had no invoker anywhere in the repository, and one of them was this guard —
written, poisoned by hand, committed, and then run by no lane.
⇒ A gate lane you did not run is a guard that does not exist, and the same is
true one level up: a guard no lane runs is a script.

⚠ The DECLINE cases carry the weight here. This guard resolves CamelCase names
against the tracked Rust sources, and its first real run reported four
unresolved names of which ALL FOUR were false — types owned by Bevy, std, and
two sibling crates. A guard whose findings are mostly noise gets waived into
uselessness, so the `EXTERNAL` escape and the anti-vacuity floor are pinned
below as their own cases.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import check_consolidation_ledger_still_resolves as guard  # noqa: E402


def test_the_real_ledger_resolves() -> None:
    """The shipped corpus, which is the only case that can rot on its own."""
    assert guard.main() == 0


def test_a_cited_path_that_does_not_exist_is_found(monkeypatch, tmp_path, capsys) -> None:
    ledger = {
        "items": [
            {
                "id": "PLANTED",
                "current_truth": "",
                "source_paths": ["crates/does_not_exist/src/lib.rs"],
            }
        ]
    }
    planted = tmp_path / "ledger.json"
    planted.write_text(json.dumps(ledger), encoding="utf-8")
    monkeypatch.setattr(guard, "LEDGER", planted)
    assert guard.main() == 1
    assert "no longer exist" in capsys.readouterr().out


def test_a_name_with_no_definition_is_found(monkeypatch, tmp_path, capsys) -> None:
    ledger = {
        "items": [
            {
                "id": "PLANTED",
                "current_truth": "TheVanishedOwner replaced it.",
                "source_paths": [],
            }
        ]
    }
    planted = tmp_path / "ledger.json"
    planted.write_text(json.dumps(ledger), encoding="utf-8")
    monkeypatch.setattr(guard, "LEDGER", planted)
    assert guard.main() == 1
    out = capsys.readouterr().out
    assert "TheVanishedOwner" in out and "PLANTED" in out


def test_a_name_owned_by_another_crate_is_declined(monkeypatch, tmp_path) -> None:
    """⛔ THE DECLINE THAT MATTERS. `ResMut` is Bevy's; a guard reading only this
    workspace can never find its definition, and reporting it is noise."""
    ledger = {
        "items": [
            {"id": "PLANTED", "current_truth": "ResMut and TypeId are used here.", "source_paths": []}
        ]
    }
    planted = tmp_path / "ledger.json"
    planted.write_text(json.dumps(ledger), encoding="utf-8")
    monkeypatch.setattr(guard, "LEDGER", planted)
    assert guard.main() == 0


def test_an_empty_ledger_is_refused(monkeypatch, tmp_path, capsys) -> None:
    planted = tmp_path / "ledger.json"
    planted.write_text(json.dumps({"items": []}), encoding="utf-8")
    monkeypatch.setattr(guard, "LEDGER", planted)
    assert guard.main() == 1
    assert "empty corpus" in capsys.readouterr().out


def test_a_shrunken_corpus_is_refused(monkeypatch, capsys) -> None:
    """⚠ The floor is on the SCAN, not on the ledger: a source tree a move
    silently emptied looks exactly like a repository with no defects."""
    monkeypatch.setattr(guard, "MIN_CORPUS_KIB", 99_000_000)
    assert guard.main() == 1
    assert "floor is" in capsys.readouterr().out
