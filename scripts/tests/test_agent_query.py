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



def _write_manifest(tmp_path, body):
    manifest = tmp_path / "Cargo.toml"
    manifest.write_text(body, encoding="utf-8")
    return manifest


def test_weak_feature_sigil_is_not_read_as_an_enabler(tmp_path):
    """`dep?/feat` does NOT turn `dep` on — that is the point of the `?`.

    Reading it as an enabler would report an optional edge as reachable when
    nothing in the manifest turns it on, which is the exact over-report the
    declared/resolved split exists to prevent.
    """
    manifest = _write_manifest(
        tmp_path,
        """
[package]
name = "probe"

[dependencies]
strong = { path = "../strong", optional = true }
weak = { path = "../weak", optional = true }

[features]
on = ["strong/inner"]
passthrough = ["weak?/inner"]
""",
    )
    deps = {dep.name: dep for dep in query.declared_deps(manifest, {"strong", "weak"})}
    assert deps["strong"].enabled_by == ("on",)
    assert deps["weak"].enabled_by == ()


def test_declared_deps_reads_every_table_and_marks_internal(tmp_path):
    manifest = _write_manifest(
        tmp_path,
        """
[package]
name = "probe"

[dependencies]
inside = { path = "../inside" }
outside = "1.0"

[dev-dependencies]
only_for_tests = { path = "../only_for_tests" }

[target.'cfg(unix)'.dependencies]
unixy = { path = "../unixy" }
""",
    )
    deps = {dep.name: dep for dep in query.declared_deps(manifest, {"inside", "only_for_tests", "unixy"})}
    assert deps["inside"].kind == "normal" and deps["inside"].internal
    assert deps["outside"].internal is False
    assert deps["only_for_tests"].kind == "dev"
    assert deps["unixy"].target == "cfg(unix)"


def test_renamed_dependency_records_the_real_package(tmp_path):
    manifest = _write_manifest(
        tmp_path,
        """
[package]
name = "probe"

[dependencies]
alias = { package = "ambition_real", path = "../real" }
""",
    )
    deps = query.declared_deps(manifest, {"ambition_real"})
    assert [dep.name for dep in deps] == ["ambition_real"]
    assert deps[0].internal


def test_graph_files_survive_the_packet_prune():
    """⛔ build-catalog DELETES every `crates/*.json` it does not name.

    Both graph files live in that directory and neither is a per-crate packet;
    the resolved one is written by a DIFFERENT generator, so a prune that
    forgot it would delete it on every run and the only symptom would be a
    resolved graph that is always mysteriously missing.
    """
    crates = [query.CrateInfo("only_crate", "crates/only", "crates/only/Cargo.toml", None)]
    allowed = query.packet_prune_allowlist(crates)
    assert "graph-declared.json" in allowed
    assert "graph-resolved.json" in allowed
    assert "only_crate.json" in allowed
    # and it still prunes what it should — a packet for a crate that is gone.
    assert "a_crate_that_no_longer_exists.json" not in allowed
