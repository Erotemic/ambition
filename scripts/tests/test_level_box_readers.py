"""The ratchet of the readers that ask a body's level box.

`check_level_box_readers.py` runs one search and compares each line with a
classified baseline. A check of this shape has three ways to be green and say
nothing: the search finds nothing, the baseline hides a new line behind an old
one, or the search counts lines that are not production code and so its count
is not the count the queue row states.

Each arm below builds a scratch tree, so that the result does not depend on
the level-box reads the repository has today. The last arm runs the check on
the repository.
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
SCRIPT = REPO / "scripts/check_level_box_readers.py"

#: One production line that reads the level box.
READ = "    let reach = kinematics.aabb();"
CONVERTED = "    let reach = kinematics.collision_box(last_step);"


def load():
    spec = importlib.util.spec_from_file_location("level_box_readers", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture
def rig(tmp_path, capsys):
    module = load()
    baseline_path = tmp_path / "level-box-readers.json"

    def write(relative: str, text: str):
        path = tmp_path / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def check(reads=(), classes=None):
        baseline = {
            "classes": {"a shot": "a shot has no frame"} if classes is None else classes,
            "reads": list(reads),
        }
        baseline_path.write_text(json.dumps(baseline), encoding="utf-8")
        capsys.readouterr()
        code = module.main([], repo=tmp_path, baseline_path=baseline_path)
        return code, capsys.readouterr().out

    return module, write, check


def entry(file: str, code: str = READ.strip(), cls: str = "a shot") -> dict:
    return {"file": file, "code": code, "class": cls}


def test_a_new_level_box_read_is_red_and_names_its_line(rig):
    """The poison of the guard: one level-box read in a scratch file."""

    _, write, check = rig
    write("crates/scratch/src/reach.rs", f"fn reach() {{\n    let a = 1;\n{READ}\n}}\n")
    code, out = check()
    assert code == 1
    assert "crates/scratch/src/reach.rs:3: let reach = kinematics.aabb();" in out
    assert "does not classify" in out


def test_the_same_tree_is_green_when_the_read_is_classified_or_converted(rig):
    """The control of the poison: it is the read that is red, not the tree."""

    _, write, check = rig
    write("crates/scratch/src/reach.rs", f"fn reach() {{\n{READ}\n}}\n")
    code, out = check([entry("crates/scratch/src/reach.rs")])
    assert code == 0, out
    write("crates/scratch/src/reach.rs", f"fn reach() {{\n{CONVERTED}\n}}\n")
    code, out = check()
    assert code == 0, out


def test_each_spelling_of_the_search_is_found(rig):
    module, write, _ = rig
    spellings = [
        "let a = kinematics.aabb();",
        "let a = kin.aabb();",
        "let a = body.aabb();",
        "let half = kinematics.size * 0.5;",
        "let half = kin.size * 0.5;",
        "let half = kinematics.size / 2.0;",
        "let half = kin.size / 2.0;",
    ]
    write("game/scratch/src/lib.rs", "fn f() {\n" + "\n".join(spellings) + "\n}\n")
    found = module.find(_root(write))
    assert [line["code"] for line in found] == spellings


def _root(write) -> Path:
    """The scratch root the `write` helper of the rig writes under."""

    for cell in write.__closure__:
        if isinstance(cell.cell_contents, Path):
            return cell.cell_contents
    raise AssertionError("the rig has no scratch root")


def test_a_second_read_cannot_hide_behind_a_classified_one(rig):
    """The identity of a line is its file and its text, and a count of each.

    A copy of a classified line in the same file is one read more than the
    baseline has, so the count of a file may only fall.
    """

    _, write, check = rig
    write("crates/scratch/src/reach.rs", f"fn a() {{\n{READ}\n}}\nfn b() {{\n{READ}\n}}\n")
    code, out = check([entry("crates/scratch/src/reach.rs")])
    assert code == 1
    assert "crates/scratch/src/reach.rs:5:" in out
    assert "crates/scratch/src/reach.rs:2:" not in out
    code, out = check([entry("crates/scratch/src/reach.rs")] * 2)
    assert code == 0, out


def test_a_classified_line_in_one_file_does_not_cover_another_file(rig):
    _, write, check = rig
    write("crates/scratch/src/shot.rs", f"fn a() {{\n{READ}\n}}\n")
    write("crates/scratch/src/interact.rs", f"fn a() {{\n{READ}\n}}\n")
    code, out = check([entry("crates/scratch/src/shot.rs")])
    assert code == 1
    assert "crates/scratch/src/interact.rs:2:" in out


def test_a_baseline_line_the_search_no_longer_finds_is_red(rig):
    _, write, check = rig
    write("crates/scratch/src/shot.rs", f"fn shot() {{\n{READ}\n}}\n")
    write("crates/scratch/src/reach.rs", f"fn reach() {{\n{CONVERTED}\n}}\n")
    code, out = check([entry("crates/scratch/src/shot.rs"), entry("crates/scratch/src/reach.rs")])
    assert code == 1
    assert "no longer finds" in out
    assert "crates/scratch/src/reach.rs: let reach = kinematics.aabb();" in out
    assert "crates/scratch/src/shot.rs" not in out.split("no longer finds")[1]


def test_a_class_the_baseline_does_not_define_is_red(rig):
    _, write, check = rig
    write("crates/scratch/src/reach.rs", f"fn reach() {{\n{READ}\n}}\n")
    code, out = check([entry("crates/scratch/src/reach.rs", cls="it is fine")])
    assert code == 1
    assert "does not define" in out


def test_test_code_and_comments_are_not_counted(rig):
    """A path filter is not enough: the tests of a file can be in the file.

    The first census of this population counted an inline test literal and two
    comment lines as production reads.
    """

    module, write, check = rig
    write("crates/scratch/src/tests.rs", f"fn t() {{\n{READ}\n}}\n")
    write("crates/scratch/src/reach/tests/arm.rs", f"fn t() {{\n{READ}\n}}\n")
    write("crates/scratch/src/reach_tests.rs", f"fn t() {{\n{READ}\n}}\n")
    write("crates/scratch/tests/arm.rs", f"fn t() {{\n{READ}\n}}\n")
    write(
        "crates/scratch/src/inline.rs",
        "fn production() {}\n"
        "// A shot asks kinematics.aabb() and that is right.\n"
        "#[cfg(test)]\n"
        "mod tests {\n"
        "    fn t() {\n"
        f"    {READ}\n"
        "    }\n"
        "}\n"
        "fn more_production() {}\n",
    )
    assert module.find(_root(write)) == []
    code, out = check()
    assert code == 0, out


def test_production_code_after_an_inline_test_module_is_counted(rig):
    """The test module is not always the last item of its file."""

    module, write, _ = rig
    write(
        "crates/scratch/src/inline.rs",
        "#[cfg(test)]\n"
        "mod tests {\n"
        "    fn t() {\n"
        f"    {READ}\n"
        "    }\n"
        "}\n"
        "fn production() {\n"
        f"{READ}\n"
        "}\n",
    )
    assert [line["line"] for line in module.find(_root(write))] == [8]


def test_a_search_that_finds_nothing_cannot_pass_a_baseline_that_is_not_empty(rig):
    _, write, check = rig
    write("crates/scratch/src/reach.rs", "fn reach() {}\n")
    code, out = check([entry("crates/scratch/src/reach.rs")])
    assert code == 1
    assert "the search is broken" in out


def test_the_check_prints_the_search_it_ran(rig):
    module, write, check = rig
    write("crates/scratch/src/reach.rs", "fn reach() {}\n")
    _, out = check()
    assert module.SEARCH in out


def test_the_repository_is_classified_and_the_search_is_not_empty():
    module = load()
    found = module.find()
    assert found, "the search finds no level-box read: the search is broken, or the baseline must be deleted"
    baseline = module.load_baseline()
    new, stale, errors = module.compare(found, baseline)
    assert not errors, errors
    assert not new, "\n".join(f"{line['file']}:{line['line']}: {line['code']}" for line in new)
    assert not stale, stale
    for name, reason in baseline["classes"].items():
        assert reason.strip(), f"the class `{name}` has no reason"
