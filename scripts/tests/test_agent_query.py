"""Focused tests for scripts/agent_query.py argument and ranking behavior."""
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

_QUERY_PATH = Path(__file__).resolve().parent.parent / "agent_query.py"


def _load_query():
    spec = importlib.util.spec_from_file_location("agent_query", _QUERY_PATH)
    module = importlib.util.module_from_spec(spec)
    sys.modules["agent_query"] = module
    spec.loader.exec_module(module)
    return module


query = _load_query()


def test_implicit_task_preserves_global_limit():
    assert query.normalize_argv(["--limit", "5", "room transition"]) == [
        "--limit",
        "5",
        "task",
        "room transition",
    ]


def test_explicit_subcommand_is_not_rewritten():
    assert query.normalize_argv(["symbol", "GroundContactTransition"]) == [
        "symbol",
        "GroundContactTransition",
    ]


def test_exact_primary_name_scores_above_path_only_match():
    exact = query.score(
        "GroundContactTransition",
        ["GroundContactTransition", "crates/example.rs"],
        primary="GroundContactTransition",
    )
    path_only = query.score(
        "GroundContactTransition",
        ["OtherType", "crates/ground_contact_transition.rs"],
        primary="OtherType",
    )
    assert exact > path_only


def test_owner_for_path_uses_longest_matching_root():
    crates = [
        query.CrateInfo("outer", "crates/foo", "crates/foo/Cargo.toml", None),
        query.CrateInfo(
            "inner",
            "crates/foo/nested",
            "crates/foo/nested/Cargo.toml",
            None,
        ),
    ]
    owner = query.owner_for_path("crates/foo/nested/src/lib.rs", crates)
    assert owner is not None
    assert owner.name == "inner"

def test_limit_after_subcommand_is_promoted_to_global_option():
    assert query.normalize_argv(["symbol", "GroundContactTransition", "--limit", "3"]) == [
        "--limit",
        "3",
        "symbol",
        "GroundContactTransition",
    ]

