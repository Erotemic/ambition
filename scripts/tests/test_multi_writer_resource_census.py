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


def test_the_two_test_predicates_are_the_SHARED_ones():
    """⛔⛤ NOT A RESPELLING, AND THIS CENSUS BRIEFLY HAD ONE OF EACH.

    `scripts/lib/test_paths.py` exists because five scripts spelled *"is this
    file test-only?"* five ways and gave five answers; the inline-module half has
    its own keeper in the rollback-mutator guard. A copy here would be a sixth
    and a second, and it missed `test.rs`, `test_support.rs` and the four files
    whose first attribute is an inner `#![cfg(test)]`.

    ⇒ This arm pins the IDENTITY of the functions rather than their behaviour,
    because behaviour is what drifts while both copies keep passing.
    """
    import sys

    scripts = pathlib.Path(mod.__file__).resolve().parent
    sys.path.insert(0, str(scripts / "lib"))
    sys.path.insert(0, str(scripts))
    import check_rollback_mutators_run_in_sim as mutator_guard
    import test_paths

    # ⚠ THE CACHED MODULE, NOT A FRESH `exec_module`. A second execution of the
    # same file defines new function objects, so an identity check against it
    # fails whether or not the census respelled anything — the arm would be
    # measuring `importlib`.
    assert mod.is_test_path is test_paths.is_test_path
    assert mod.strip_test_modules is mutator_guard.strip_test_modules


def test_a_whole_test_file_is_not_a_second_authority():
    """⛔⛤ THE HALF THE `#[cfg(test)]` CUT CANNOT DO.

    An integration test under `tests/` and a `mod tests` carried in its own
    `tests.rs` have no `#[cfg(test)]` line to cut at — the parent module carries
    it — so every `ResMut<T>` in them counted as a writer while the module
    docstring said the census reads non-test code. MEASURED 2026-09-17: 1,866
    tracked files became 1,294 production ones and eight types left the
    shortlist.
    """
    assert mod.is_test_path(pathlib.Path("game/ambition_app/tests/a_thing.rs"))
    assert mod.is_test_path(pathlib.Path("crates/x/src/y/tests.rs"))
    assert mod.is_test_path(pathlib.Path("crates/x/src/y/hurtbox_damage_tests.rs"))
    assert mod.is_test_path(pathlib.Path("crates/x/src/projectile/tests/collision.rs"))
    # ⚠ AND THE SHARED PREDICATE DISAGREES WITH WHAT I WOULD HAVE WRITTEN:
    # `test_support.rs` IS test-only here, unioned across the five copies. That
    # is the keeper's call to make, and a second opinion in this file would be
    # the whole defect again.
    assert mod.is_test_path(pathlib.Path("crates/x/src/test_support.rs"))
    assert not mod.is_test_path(
        pathlib.Path("game/ambition_app/src/headless.rs"),
        source="fn main() {}\n",
    )


def test_production_code_after_a_test_module_is_still_read():
    """⛔⛔ THE UNDERCOUNT, AND IT IS THE ONE A POISON FOUND BY NOT FIRING.

    The cut was `src.split("#[cfg(test)]")[0]`, and a module declares its tests
    near the TOP: `#[cfg(test)] mod tests;` at `platformer2d_runtime/src/lib.rs:25`.
    MEASURED 2026-09-17: 777 of 1,302 production files carry the attribute and 40
    of them held 77 `ResMut<T>` occurrences below the first one. A poison appended
    to a file's end proved it by changing nothing.
    """
    src = (
        "fn a(mut r: ResMut<Before>) {}\n"
        "#[cfg(test)]\nmod tests;\n"
        "fn b(mut r: ResMut<After>) {}\n"
        "#[cfg(test)]\nmod inline {\n    fn t(mut r: ResMut<Fixture>) {}\n"
        "    mod deeper { fn u(mut r: ResMut<AlsoFixture>) {} }\n}\n"
        "fn c(mut r: ResMut<Last>) {}\n"
        "fn d(mut r: ResMut<Final>) {}\n"
    )
    kept = set(mod.RESMUT.findall(mod.strip_test_modules(src)))
    assert kept == {"Before", "After", "Last", "Final"}, kept


def test_the_stripper_leaves_no_inline_test_module_behind():
    src = "fn a(mut r: ResMut<X>) {}\n#[cfg(test)]\nmod t { fn f() {} }\n"
    assert "#[cfg(test)]" not in mod.strip_test_modules(src)


def test_a_cfg_test_fn_in_a_production_file_is_a_KNOWN_residual():
    """⚠ THE COST OF USING THE KEEPER, STATED RATHER THAN DISCOVERED.

    The shared stripper removes inline `#[cfg(test)] mod X { }` blocks only, so a
    `#[cfg(test)] fn` in a production file still reads as a writer. MEASURED
    2026-09-17: there are none in this tree — every combination of the two test
    rules gives 333 types and 85 multi-writer — and widening that function
    reaches all of its consumers, which its own docstring forbids doing without
    reading each one's counts.
    """
    src = "#[cfg(test)]\nfn helper(mut r: ResMut<HelperOnly>) {}\n"
    assert mod.RESMUT.findall(mod.strip_test_modules(src)) == ["HelperOnly"]
