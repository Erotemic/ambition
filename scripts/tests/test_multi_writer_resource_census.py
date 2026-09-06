"""Unit tests for `scripts/multi_writer_resource_census.py` — hand-built corpus."""

from __future__ import annotations

import importlib.util
import pathlib

_SCRIPT = pathlib.Path(__file__).resolve().parents[1] / "multi_writer_resource_census.py"
_spec = importlib.util.spec_from_file_location("multi_writer_resource_census", _SCRIPT)
mod = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(mod)


def _write(tmp_path, name: str, body: str) -> str:
    p = tmp_path / name
    p.write_text(body, encoding="utf-8")
    return str(p)


def test_two_files_writing_one_resource_are_reported(tmp_path):
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Foo>) {}")
    b = _write(tmp_path, "b.rs", "fn t(mut r: ResMut<Foo>) {}")
    found = mod.writers([a, b])
    assert found["Foo"] == {a, b}


def test_a_qualified_path_is_the_same_resource_as_its_short_name(tmp_path):
    """⚠ Otherwise one crate's `ResMut<crate::Foo>` and another's `ResMut<Foo>`
    read as two different types and the duplication hides."""
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<ambition_x::y::Foo>) {}")
    b = _write(tmp_path, "b.rs", "fn t(mut r: ResMut<Foo>) {}")
    assert len(mod.writers([a, b])["Foo"]) == 2


def test_a_test_module_is_not_a_second_authority(tmp_path):
    """⛔ A fixture building a resource by hand is not a writer. Counting them is
    how a census manufactures findings nobody can act on."""
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Foo>) {}")
    b = _write(
        tmp_path,
        "b.rs",
        "fn unrelated() {}\n#[cfg(test)]\nmod t { fn f(mut r: ResMut<Foo>) {} }\n",
    )
    found = mod.writers([a, b])
    assert found["Foo"] == {a}, "the test-module write must not count"


def test_a_single_writer_is_not_in_the_shortlist(tmp_path):
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Solo>) {}")
    found = mod.writers([a])
    assert {t: fs for t, fs in found.items() if len(fs) > 1} == {}


def test_a_file_with_no_resmut_contributes_nothing(tmp_path):
    """⚠ Anti-vacuity for the parser: if the regex matched loosely, every file
    would contribute and the shortlist would be the whole workspace."""
    a = _write(tmp_path, "a.rs", "fn s(r: Res<Foo>, q: Query<&Foo>) {}")
    assert mod.writers([a]) == {}
