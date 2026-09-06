"""The derived registry verdict has to be able to go wrong.

It reports rather than gates, so what is left to protect is the derivation and
the two ways it silently produced nothing useful:

* a table shape change must FAIL, not report zero rows;
* the staleness test must not be a SUBSTRING scan. The first version flagged the
  one row that had already been FIXED, because that row's correction quotes its
  own error text -- "would SILENTLY change what every existing row expands to".
  A rule that scans for a word matches the prose discussing the word.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "registry_register_returns.py"


def _module():
    spec = importlib.util.spec_from_file_location("registry_register_returns", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_it_reads_a_return_type_through_a_multi_line_signature() -> None:
    """⚠ Real signatures wrap. Reading to the first `)` would stop inside the
    parameter list and miss the arrow entirely."""
    module = _module()
    src = """
    pub fn register(
        &mut self,
        key: impl Into<String>,
        builder: MovePrefabBuilder,
    ) -> Result<(), String> {
    """
    assert module.return_type(src, "register") == "Result<(), String>"


def test_a_function_with_no_arrow_reads_as_unit() -> None:
    module = _module()
    src = "pub fn insert_prepared(&mut self, prepared: Def) {\n"
    assert module.return_type(src, "insert_prepared") == "()"


def test_a_missing_function_is_none_not_a_guess() -> None:
    """⛔ `None` and `()` are different answers: one is "this registry has no such
    entry point", the other is "it has one and it refuses nothing"."""
    module = _module()
    assert module.return_type("pub fn other() {}", "register") is None


def test_an_empty_table_fails_rather_than_reporting_zero(monkeypatch, tmp_path) -> None:
    """⛔⛔ A CHECK THAT CANNOT FAIL. If the table's row shape changes, the regex
    matches nothing and every count reads as a calm zero."""
    module = _module()
    empty = tmp_path / "page.md"
    empty.write_text("# no table here\n", encoding="utf-8")
    monkeypatch.setattr(module, "PAGE", empty)
    assert module.main() == 1


def test_it_runs_against_the_live_page(capsys) -> None:
    module = _module()
    assert module.main() == 0
    out = capsys.readouterr().out
    assert "registry rows:" in out
    # The four deliberate replacements must still read as silent, or the page's
    # rulings about them have quietly stopped matching the source.
    assert "silent: 4" in out, out
