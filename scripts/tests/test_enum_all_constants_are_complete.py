"""`const ALL` is a hand-written copy of an enum's variant list, and nothing in
Rust holds the two together.

⛔⛤ **THE ARMS HERE ARE ABOUT THE PARSER, BECAUSE THE PARSER IS THE WHOLE
INSTRUMENT.** The guard's verdict on the real tree is one line
(`check_enum_all_constants_are_complete.py`, 41 of 41 agreeing on 2026-09-18);
what can rot is its ability to SEE the population, and every way it can stop
seeing looks exactly like a clean run.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/check_enum_all_constants_are_complete.py"


def load():
    spec = importlib.util.spec_from_file_location("enum_all", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_the_real_tree_agrees_and_the_population_is_the_measured_one():
    """⭐ THE CONTROL FOR EVERY ARM BELOW: on the tree as it is, the guard passes
    and it passes over a population, not over nothing."""
    module = load()
    found = module.rows()
    assert len(found) >= 30, len(found)
    assert module.faults(found) == [], module.faults(found)


def test_a_missing_variant_is_reported_by_name():
    module = load()
    row = {
        "file": "x.rs",
        "type": "Tab",
        "declared_len": "2",
        "listed": ["Items", "Map"],
        "variants": ["Items", "Map", "Quests"],
    }
    problems = module.faults([row])
    assert len(problems) == 1, problems
    assert "Quests" in problems[0], problems


def test_a_variant_the_enum_does_not_declare_is_reported():
    module = load()
    row = {
        "file": "x.rs",
        "type": "Tab",
        "declared_len": "3",
        "listed": ["Items", "Map", "Nope"],
        "variants": ["Items", "Map"],
    }
    problems = module.faults([row])
    assert any("Nope" in p for p in problems), problems


def test_the_declared_length_is_checked_against_what_is_listed():
    """⚠ NOT REDUNDANT WITH THE COMPILER. `[Self; N]` makes rustc count the
    ARRAY; nothing makes it count the enum, and `N` is the number a reader
    quotes."""
    module = load()
    row = {
        "file": "x.rs",
        "type": "Tab",
        "declared_len": "9",
        "listed": ["Items", "Map", "Quests"],
        "variants": ["Items", "Map", "Quests"],
    }
    problems = module.faults([row])
    assert any("[Self; 9]" in p for p in problems), problems


def test_an_enum_the_guard_cannot_find_is_a_refusal_not_a_pass():
    """⛔ THE SILENT-ZERO SHAPE. An unresolvable enum compared against nothing
    would agree with everything."""
    module = load()
    row = {
        "file": "x.rs",
        "type": "Tab",
        "declared_len": "3",
        "listed": ["Items"],
        "variants": None,
    }
    problems = module.faults([row])
    assert len(problems) == 1 and "not declared in this file" in problems[0], problems


def test_a_tuple_variants_own_commas_are_not_read_as_more_variants():
    """⛔⛤ **THE PAYLOAD THAT SEPARATES THE TWO PARSERS IS A TUPLE VARIANT WITH
    AN UPPERCASE SECOND ELEMENT, AND THE FIRST VERSION OF THIS ARM DID NOT HAVE
    ONE.** It used `Cycle { index, count }` and `Slider { value, min, max, step
    }` — real shapes from `SettingsOptionKind` — and passed with the depth
    tracking REMOVED, because Rust field names are lowercase and the
    `^[A-Z]` filter was already dropping them. A poison that passes is a finding
    about the arm.

    `Bound(Lower, Upper)` is what actually distinguishes them: split on every
    comma and ` Upper)` becomes a variant the enum never declared, which this
    guard would then report as a fault ON A CORRECT FILE.

    ⚠ NO ENUM IN THE COMPARED POPULATION CARRIES THAT SHAPE TODAY — measured
    2026-09-18, zero tuple variants with an inner comma across all 41. So the
    depth tracking is a CONTRACT of this parser rather than a repair of a live
    misreading, and this arm says so instead of implying a catch.
    """
    module = load()
    body = """
        Toggle(bool),
        Bound(Lower, Upper),
        Cycle { index: usize, count: usize },
        Action,
    """
    assert module.variants(body) == ["Toggle", "Bound", "Cycle", "Action"]


def test_an_attribute_does_not_hide_the_variant_it_decorates():
    """A `#[serde(rename = "x")]` or a `#[cfg(feature = "audio")]` sits between
    the comma and the name. A variant the parser loses is a variant `ALL` may
    omit for free."""
    module = load()
    body = """
        #[serde(rename = "lo")]
        Low,
        #[cfg(feature = "audio")]
        Loud,
        Off,
    """
    assert module.variants(body) == ["Low", "Loud", "Off"]


def test_the_population_floor_refuses_rather_than_reporting_clean():
    """⛔ Two regexes and a brace walker stand between this guard and an empty
    population. Run against a tree it cannot read, it must REFUSE."""
    module = load()
    module.ROOTS = ("does_not_exist_anywhere",)
    module.production_files = lambda: []
    assert module.main() == 2


def test_a_partial_by_design_entry_must_carry_a_reason():
    """⛔ AN ENTRY IN THAT TABLE IS AN ARGUMENT, NOT A SILENCER. A blank reason
    is a variant excused with nothing written down, which is the state this
    whole guard exists to prevent."""
    module = load()
    blank = [name for name, why in module.PARTIAL_BY_DESIGN.items() if not why.strip()]
    assert not blank, blank
