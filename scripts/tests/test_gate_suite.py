"""Tests for `scripts/gate_suite.py` change classification.

Documentation/measurement-only edits may select the smoke gate. Any source,
asset, configuration, or unknown path must fail toward the full suite so a
classification bug cannot silently skip behavioral coverage."""

from __future__ import annotations

import importlib.util
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("gate_suite", REPO / "scripts" / "gate_suite.py")
gate = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gate)


def test_prose_only_is_the_case_the_ruling_exists_for():
    assert gate.is_skippable_only(["docs/planning/queue.md", "docs/README.md"])


def test_one_non_prose_path_forces_the_full_suite():
    """The whitelist's entire job: a single source file outvotes any amount of prose."""
    assert not gate.is_skippable_only(
        ["docs/planning/queue.md"] * 50 + ["crates/ambition_combat/src/lib.rs"]
    )


def test_an_asset_is_not_prose():
    """⛔ kills the BLACKLIST draft ("skip when no .rs changed").

    A `.ron` the tests read is not `.rs` and does change behaviour, so a
    blacklist would call this prose-only. The whitelist must not.
    """
    assert not gate.is_skippable_only(["assets/ambition/platformer_defaults.ron"])
    assert not gate.is_skippable_only(["crates/x/assets/sheet.ron"])


def test_a_generated_file_and_a_submodule_pointer_are_not_skippable():
    """Generated gate inputs and submodule pointers are not prose-only changes.

    The compile-ratchet baseline changes gate behavior and therefore must run the
    gate suite.
    """
    assert not gate.is_skippable_only(["dev/compile_ratchet_baseline.json"])
    assert not gate.is_skippable_only([".agent/index/catalog.json"])
    assert not gate.is_skippable_only(["tools/ambition_sprite2d_renderer"])
    assert not gate.is_skippable_only(["Cargo.toml"])


def test_the_measurements_submodule_is_skippable():
    """⭐ the second whitelist entry, and the reason it is not a slippery slope.

    `dev/ambition_dev_measurements` is a write-only record of what past runs
    cost; nothing in the build or test graph reads a row of it. It is exempt
    because `run_tests.py` appends to it on EVERY run, so without the exemption
    the submodule pointer would be dirty on the very next turn and the
    prose-only case this file exists to protect would never fire again.
    """
    assert gate.is_skippable_only(["dev/ambition_dev_measurements"])
    assert gate.is_skippable_only(
        ["docs/planning/queue.md", "dev/ambition_dev_measurements/run_tests_cost.jsonl"]
    )
    # and it does not extend to the rest of `dev/`, which is prose and memory
    # the agents read but also holds the ratchet's baseline.
    assert not gate.is_skippable_only(["dev/journals/lessons_learned.md"])


def test_the_gate_script_itself_is_not_prose():
    """A change to the decider must be gated by the full suite, not by itself."""
    assert not gate.is_skippable_only(["scripts/gate_suite.py"])


def test_an_empty_change_set_is_NOT_skippable():
    """⛔ kills the `all([])` draft, which is vacuously True in Python.

    "Nothing changed" and "only docs changed" are different states. Answering the
    first with a smoke suite means a turn whose diff failed to compute silently
    skips the gate — the failure mode this whole file is written to avoid.
    """
    assert not gate.is_skippable_only([])


def test_a_path_that_merely_starts_with_the_letters_docs_is_not_prose():
    """⛔ kills the `startswith("docs")` draft — no separator, so it matches
    `docs_generator/`, `docsite/src/main.rs`, and anything else beginning `docs`."""
    assert not gate.is_skippable_only(["docsite/src/main.rs"])
    assert not gate.is_skippable_only(["docs_generator/lib.rs"])


def test_the_smoke_modules_exist_and_each_carries_a_reason():
    """⛔ a subset that names a module which no longer exists silently runs
    NOTHING — cargo's filter matches zero tests and reports success."""
    tests_dir = REPO / "game" / "ambition_app" / "tests"
    for name, reason in gate.SMOKE_MODULES.items():
        assert (tests_dir / f"{name}.rs").is_file(), f"smoke module {name} does not exist"
        assert len(reason) > 20, f"smoke module {name} has no stated reason"


def test_the_smoke_modules_are_registered_in_the_aggregate_binary():
    """`app_it` is one binary of `mod` declarations; a file on disk that is not
    `mod`-ed is not in the binary, and filtering for it matches nothing."""
    aggregate = (REPO / "game" / "ambition_app" / "tests" / "app_it.rs").read_text()
    for name in gate.SMOKE_MODULES:
        assert f"mod {name};" in aggregate, f"{name} is not a module of app_it"
