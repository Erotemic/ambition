"""The id-prefix sweep has to be able to go wrong.

It REPORTS rather than enforces — a reported pair is a question about ownership,
and one of today's rows is a genuine false pair (`npc_` collides between the save
flags and a character id). So what is left to protect is the measurement:

* an empty corpus must FAIL, not print a calm zero;
* a prefix built by ONE side only must not be reported, or every `format!` in the
  tree becomes a finding; and
* COMMENT LINES must stay excluded, because the first version of this sweep
  reported `room_visited_` out of the doc comment that RECORDS the repair.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "measure_id_prefixes_spelled_twice.py"


def _module():
    spec = importlib.util.spec_from_file_location("id_prefix_sweep", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_it_is_green_against_the_tree(capsys) -> None:
    module = _module()
    assert module.main() == 0
    out = capsys.readouterr().out
    assert "spelled in BOTH a producer and a consumer" in out


def test_a_prefix_only_a_producer_spells_is_not_a_finding(tmp_path, monkeypatch) -> None:
    """⛔ Otherwise every `format!` in the tree is a row and the report is noise."""
    module = _module()
    crate = tmp_path / "crates" / "solo" / "src"
    crate.mkdir(parents=True)
    (crate / "lib.rs").write_text('fn a() { let _ = format!("only_written_{id}"); }\n')
    monkeypatch.setattr(module, "REPO", tmp_path)
    sites = module.scan()
    assert "only_written_" in sites, "the sweep must still SEE it"
    assert {row.kind for row in sites["only_written_"]} == {"WRITE"}


def test_a_comment_quoting_both_literals_is_not_a_finding(tmp_path, monkeypatch) -> None:
    """⛔⛔ THE FIRST VERSION OF THIS SWEEP REPORTED ITS OWN REPAIR NOTE.

    The doc comment recording the `room_visited_` fix quotes the `format!` and the
    `strip_prefix` it removed. A sweep that reads prose finds the sentence about
    the defect and calls it the defect.
    """
    module = _module()
    crate = tmp_path / "crates" / "solo" / "src"
    crate.mkdir(parents=True)
    (crate / "lib.rs").write_text(
        '/// It was two: `format!("prose_only_{id}")` and `strip_prefix("prose_only_")`.\n'
        "// and the same again in a line comment\n"
        "fn a() {}\n"
    )
    monkeypatch.setattr(module, "REPO", tmp_path)
    assert module.scan() == {}


def test_an_empty_corpus_fails_rather_than_reporting_zero(tmp_path, monkeypatch, capsys) -> None:
    """⛔⛔ A grep-shaped sweep over a moved directory prints a calm zero and every
    claim built on it becomes trivially true."""
    module = _module()
    (tmp_path / "crates").mkdir()
    (tmp_path / "game").mkdir()
    monkeypatch.setattr(module, "REPO", tmp_path)
    assert module.main() == 1
    err = capsys.readouterr().err
    assert "broken" in err


def test_the_floor_is_on_the_corpus_not_the_findings(tmp_path, monkeypatch) -> None:
    """⚠ Zero FINDINGS is the goal state, so a floor on findings would fail forever
    exactly when the work succeeded. The floor counts SPELLINGS."""
    module = _module()
    crate = tmp_path / "crates" / "solo" / "src"
    crate.mkdir(parents=True)
    body = "\n".join(f'fn f{n}() {{ let _ = format!("p{n}_{{id}}"); }}' for n in range(25))
    (crate / "lib.rs").write_text(body + "\n")
    monkeypatch.setattr(module, "REPO", tmp_path)
    assert module.main() == 0, "25 spellings and no pairs is a PASS, not a vacuous one"
