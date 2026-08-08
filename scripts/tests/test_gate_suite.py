"""`scripts/gate_suite.py` decides which suite a turn needs. Pin the decision.

⛔ **the ruling this implements is a deliberate loosening**, so the tests here
guard the SAFE direction rather than the fast one. Every case below is written so
that a bug makes the gate run MORE than it needs, never less — except the one
case that is the whole point (prose-only), which is pinned exactly.

Jon, 2026-08-08: *"I want to bias towards running less tests to balance out the
agent urge to run more … We will catch regressions eventually."*
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("gate_suite", REPO / "scripts" / "gate_suite.py")
gate = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gate)


def test_prose_only_is_the_case_the_ruling_exists_for():
    assert gate.is_prose_only(["docs/planning/queue.md", "docs/README.md"])


def test_one_non_prose_path_forces_the_full_suite():
    """The whitelist's entire job: a single source file outvotes any amount of prose."""
    assert not gate.is_prose_only(
        ["docs/planning/queue.md"] * 50 + ["crates/ambition_combat/src/lib.rs"]
    )


def test_an_asset_is_not_prose():
    """⛔ kills the BLACKLIST draft ("skip when no .rs changed").

    A `.ron` the tests read is not `.rs` and does change behaviour, so a
    blacklist would call this prose-only. The whitelist must not.
    """
    assert not gate.is_prose_only(["assets/ambition/platformer_defaults.ron"])
    assert not gate.is_prose_only(["crates/x/assets/sheet.ron"])


def test_a_generated_file_and_a_submodule_pointer_are_not_prose():
    assert not gate.is_prose_only(["dev/compile_units.jsonl"])
    assert not gate.is_prose_only(["tools/ambition_sprite2d_renderer"])
    assert not gate.is_prose_only(["Cargo.toml"])


def test_the_gate_script_itself_is_not_prose():
    """A change to the decider must be gated by the full suite, not by itself."""
    assert not gate.is_prose_only(["scripts/gate_suite.py"])


def test_an_empty_change_set_is_NOT_prose_only():
    """⛔ kills the `all([])` draft, which is vacuously True in Python.

    "Nothing changed" and "only docs changed" are different states. Answering the
    first with a smoke suite means a turn whose diff failed to compute silently
    skips the gate — the failure mode this whole file is written to avoid.
    """
    assert not gate.is_prose_only([])


def test_a_path_that_merely_starts_with_the_letters_docs_is_not_prose():
    """⛔ kills the `startswith("docs")` draft — no separator, so it matches
    `docs_generator/`, `docsite/src/main.rs`, and anything else beginning `docs`."""
    assert not gate.is_prose_only(["docsite/src/main.rs"])
    assert not gate.is_prose_only(["docs_generator/lib.rs"])


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
