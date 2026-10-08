"""C07's None-arm guard must run in a lane and must be able to FAIL.

⛔ A guard with no test beside it runs in no lane (it happened on 2026-09-16 to a
checker that was committed, correct, and executed by nothing).
"""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_session_authority_none_arms as guard  # noqa: E402


def _tree(tmp_path: pathlib.Path, body: str, name: str = "crates/ambition_x/src/lib.rs") -> pathlib.Path:
    path = tmp_path / name
    path.parent.mkdir(parents=True)
    path.write_text(body)
    return tmp_path


def test_the_table_matches_source_today():
    assert guard.main() == 0


def test_an_optional_read_of_an_authority_is_found_with_its_enclosing_item(tmp_path):
    root = _tree(
        tmp_path,
        "pub fn reader(a: Option<Res<ControlledSubject>>, b: Res<Other>) {}\n"
        "pub fn world_reader(world: &World) { world.get_resource::<AuthoredOccurrences>(); }\n"
        "pub fn unrelated(c: Option<Res<NotAnAuthority>>) {}\n",
    )
    found = guard.scan(root)
    assert found == {
        ("crates/ambition_x/src/lib.rs", "reader"): {"ControlledSubject"},
        ("crates/ambition_x/src/lib.rs", "world_reader"): {"AuthoredOccurrences"},
    }


def test_a_required_read_and_a_test_module_are_not_sites(tmp_path):
    root = _tree(
        tmp_path,
        "pub fn reader(a: Res<ControlledSubject>) {}\n"
        "#[cfg(test)]\nmod tests {\n    fn t(a: Option<Res<ControlledSubject>>) {}\n}\n",
    )
    assert guard.scan(root) == {}


def test_a_new_optional_read_with_no_row_is_red():
    found = {("crates/a/src/lib.rs", "f"): {"QuestRegistry"}} | {
        (f"crates/a/src/m{i}.rs", "g"): {"QuestRegistry"} for i in range(guard.MIN_SITES)
    }
    rows = {k: ("reduced-composition", "x" * guard.MIN_REASON) for k in found if k[1] == "g"}
    problems = guard.check(found, rows)
    assert len(problems) == 1 and "no `none-arm:` row" in problems[0], problems


def test_a_stale_row_a_bad_class_and_a_non_sentence_are_each_red():
    found = {(f"crates/a/src/m{i}.rs", "g"): {"QuestRegistry"} for i in range(guard.MIN_SITES)}
    rows = {k: ("reduced-composition", "x" * guard.MIN_REASON) for k in found}
    rows[("crates/a/src/gone.rs", "h")] = ("reduced-composition", "x" * guard.MIN_REASON)
    rows[("crates/a/src/m0.rs", "g")] = ("because", "x" * guard.MIN_REASON)
    rows[("crates/a/src/m1.rs", "g")] = ("refuses", "too short")
    text = "\n".join(guard.check(found, rows))
    assert "STALE" in text and "is not one of" in text and "not a sentence" in text, text


def test_a_scan_that_lost_the_tree_cannot_pass_by_being_empty():
    assert any("lost the tree" in problem for problem in guard.check({}, {}))


def test_the_presentation_prefix_needs_no_row_but_nothing_else_does_not():
    render = {("crates/ambition_render/src/x.rs", "f"): {"ActiveSessionScope"}}
    sim = {("crates/ambition_combat/src/x.rs", "f"): {"ActiveSessionScope"}}
    assert not [p for p in guard.check(render, {}) if "no `none-arm:` row" in p]
    assert [p for p in guard.check(sim, {}) if "no `none-arm:` row" in p]


def test_a_row_must_parse():
    rows, problems = guard.parse_table("none-arm: crates/a.rs::f | refuses\nnone-arm: crates/a.rs::g | refuses | " + "r" * 40)
    assert list(rows) == [("crates/a.rs", "g")] and len(problems) == 1
