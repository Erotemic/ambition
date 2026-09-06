"""The repeat scanner must FIND a duplicate and must not invent one.

It is a report whose whole value is the hits, so the failure modes are finding
nothing (a broken parser reads as a clean tree) and finding everything.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "repeated_statements_in_one_function.py"

DUPLICATED = """
pub fn end_two_ways(active: &mut Active, save: &mut Save) {
    if skip {
        save.data_mut().set_flag(seen.clone(), true);
        active.runtime = None;
        return;
    }
    if completed {
        save.data_mut().set_flag(seen.clone(), true);
        active.runtime = None;
    }
}
"""

DISTINCT = """
pub fn two_different_statements(a: &mut A) {
    a.set_first_value(compute_the_first(), true);
    a.set_second_value(compute_the_second(), false);
}
"""


def _module():
    spec = importlib.util.spec_from_file_location("repeated_statements", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_it_finds_a_statement_written_twice_in_one_function(tmp_path) -> None:
    """⭐ THE POSITIVE CONTROL. This is the real shape it was written for: one
    ending spelled at two exits, which is what `advance_active_cutscene` had."""
    path = tmp_path / "sample.rs"
    path.write_text(DUPLICATED, encoding="utf-8")
    found = {statement for _, statement, _ in _module().repeats_in(path)}
    assert any("set_flag" in s for s in found), found
    assert any("active.runtime = None" in s for s in found), found


def test_two_different_statements_are_not_a_repeat(tmp_path) -> None:
    """Otherwise every function with two setters would be a hit and the report
    would be unreadable, which is the same as reporting nothing."""
    path = tmp_path / "sample.rs"
    path.write_text(DISTINCT, encoding="utf-8")
    assert list(_module().repeats_in(path)) == []


IN_FILE_TEST_BLOCK = """
pub fn only_one_real_statement(a: &mut A) {
    a.do_the_only_production_thing(1);
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_table_driven_check() {
        assert_eq!(compute_the_value(1), 2);
        assert_eq!(compute_the_value(1), 2);
    }
}
"""


def test_in_file_cfg_test_blocks_are_excluded(tmp_path) -> None:
    """⛔⛔ THE DEFECT THIS FILE'S FIRST VERSION SHIPPED WITH. This repo puts
    `#[cfg(test)] mod tests` INSIDE ordinary source files, so filtering by PATH
    leaves every in-file test block in the numerator — the first run over the item
    and persistence crates returned mostly duplicated `assert!` lines, which are
    true repeats in test code and not the question being asked."""
    path = tmp_path / "sample.rs"
    path.write_text(IN_FILE_TEST_BLOCK, encoding="utf-8")
    assert list(_module().repeats_in(path)) == []


def test_stripping_keeps_line_numbers_honest(tmp_path) -> None:
    """⚠ Blanked, not deleted: a reported line number must still match the file a
    reader opens."""
    module = _module()
    lines = IN_FILE_TEST_BLOCK.splitlines()
    assert len(module.strip_cfg_test_blocks(lines)) == len(lines)


def test_an_empty_corpus_fails(tmp_path, monkeypatch) -> None:
    """⛔ A parser that matches no function prints a clean report, which reads
    exactly like a tree with no duplication."""
    module = _module()
    monkeypatch.setattr(sys, "argv", ["x", str(tmp_path.relative_to(tmp_path))])
    monkeypatch.setattr(module, "rust_files", lambda paths: [])
    assert module.main() == 1


def test_it_runs_against_the_tree() -> None:
    proc = subprocess.run(
        [sys.executable, str(SCRIPT), "crates/ambition_portal2d_presentation/src/"],
        cwd=REPO, capture_output=True, text=True,
    )
    assert proc.returncode == 0, proc.stdout + proc.stderr
    assert "repeated statement(s)" in proc.stdout
