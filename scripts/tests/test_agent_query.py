"""Focused tests for scripts/agent_query.py argument and ranking behavior."""
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

pytestmark = pytest.mark.detached_tool

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


# --- Drawing -----------------------------------------------------------------
#
# The fixture is deliberately tiny and hand-checkable: `app` links `engine`,
# `engine` optionally links `audio` behind a feature and `dead` behind nothing,
# and `serde` is outside the workspace.

_DRAW_GRAPH = {
    "graph": "declared",
    "means": "what manifests declare",
    "members": ["app", "engine", "audio", "dead"],
    "edges": {
        "app": [{"name": "engine", "internal": True, "kind": "normal"}],
        "engine": [
            {
                "name": "audio",
                "internal": True,
                "kind": "normal",
                "optional": True,
                "enabled_by": ["sound"],
            },
            {"name": "dead", "internal": True, "kind": "normal", "optional": True},
            {"name": "serde", "internal": False, "kind": "normal"},
        ],
        "audio": [],
        "dead": [],
    },
}


def test_dot_says_which_graph_it_is():
    """A picture of declared edges and one of resolved edges look identical.

    A PNG in a chat window has lost every bit of context but its pixels, so the
    distinction has to survive inside the drawing itself.
    """
    declared = query.build_dot(_DRAW_GRAPH)
    resolved = query.build_dot(
        {**_DRAW_GRAPH, "graph": "resolved", "means": "what cargo resolves"}
    )
    assert "// graph: declared" in declared and "DECLARED" in declared
    assert "// graph: resolved" in resolved and "RESOLVED" in resolved
    assert "what cargo resolves" in resolved


def test_optional_edges_are_drawn_as_optional():
    dot = query.build_dot(_DRAW_GRAPH)
    enabled = next(line for line in dot.splitlines() if '"engine" -> "audio"' in line)
    orphan = next(line for line in dot.splitlines() if '"engine" -> "dead"' in line)
    plain = next(line for line in dot.splitlines() if '"app" -> "engine"' in line)
    assert "style=dashed" in enabled and 'label="sound"' in enabled
    assert "style=dotted" in orphan and "nothing enables" in orphan
    assert "style=" not in plain


def test_external_nodes_are_opt_in():
    assert '"serde"' not in query.build_dot(_DRAW_GRAPH)
    with_external = query.build_dot(_DRAW_GRAPH, external=True)
    assert '"serde"' in with_external
    # and it is drawn as a foreigner, not as a workspace member.
    assert 'shape=ellipse' in next(
        line for line in with_external.splitlines() if line.strip().startswith('"serde"')
    )


def test_focus_direction_selects_dependents_or_dependencies():
    """⚠ 'what breaks if I change this' is the rdeps question.

    It is not answerable by reversing a deps drawing by eye once the graph is
    wider than a few nodes, so the two directions are separate requests.
    """
    edges = query.normalized_graph_edges(_DRAW_GRAPH, external=False)
    assert query.graph_neighbourhood(edges, "engine", 1, "deps") == {"engine", "audio", "dead"}
    assert query.graph_neighbourhood(edges, "engine", 1, "rdeps") == {"engine", "app"}
    assert query.graph_neighbourhood(edges, "app", 1, "deps") == {"app", "engine"}
    assert query.graph_neighbourhood(edges, "app", 2, "deps") == {"app", "engine", "audio", "dead"}


def test_focus_depth_actually_bounds_the_drawing():
    near = query.build_dot(_DRAW_GRAPH, focus="app", depth=1, direction="deps")
    far = query.build_dot(_DRAW_GRAPH, focus="app", depth=2, direction="deps")
    assert '"audio"' not in near
    assert '"audio"' in far


def test_graph_title_keeps_graphvizs_own_line_break():
    """⛔ `_dot_quote` escapes backslashes, which is right for a crate name.

    Running the title through it turns graphviz's `\\n` into a literal
    backslash-n printed inside the picture — which is exactly what happened.
    """
    assert query._dot_label(r"one\ntwo") == r'"one\ntwo"'
    assert query._dot_quote(r"one\ntwo") == r'"one\\ntwo"'
    title = next(line for line in query.build_dot(_DRAW_GRAPH).splitlines() if "label=" in line)
    assert r"\nedges:" in title and r"\\nedges:" not in title


def test_render_shells_out_to_dot_and_reports_its_failure(tmp_path, monkeypatch):
    """The rendering plumbing, checked against a stand-in for `dot`.

    ⚠ this machine has no graphviz, and "the .dot file looks right" is not
    evidence that the render path passes the source on stdin, asks for the
    right format, or notices a non-zero exit.
    """
    stub = tmp_path / "bin" / "dot"
    stub.parent.mkdir()
    seen = tmp_path / "seen.txt"
    stub.write_text(
        # The stub is the only thing on PATH during this test, so its shebang
        # cannot go looking for an interpreter through PATH.
        f"#!{sys.executable}\n"
        "import sys, pathlib\n"
        f"pathlib.Path({str(seen)!r}).write_text(' '.join(sys.argv[1:]) + '\\n' + sys.stdin.read())\n"
        "if 'FAIL' in ' '.join(sys.argv):\n"
        "    sys.stderr.write('stub refused\\n'); sys.exit(3)\n"
        "out = sys.argv[sys.argv.index('-o') + 1]\n"
        "pathlib.Path(out).write_text('rendered')\n",
        encoding="utf-8",
    )
    stub.chmod(0o755)
    monkeypatch.setenv("PATH", str(stub.parent), prepend=False)

    out = tmp_path / "graph.svg"
    query.render_dot("digraph g { a -> b; }", out, "svg")
    assert out.read_text() == "rendered"
    recorded = seen.read_text()
    assert "-Tsvg" in recorded
    assert "digraph g { a -> b; }" in recorded  # source arrives on stdin, not as a file

    # The format is the caller's, not a constant: asking for one and getting
    # another is invisible until something downstream refuses to open the file.
    query.render_dot("digraph g {}", tmp_path / "graph.png", "png")
    assert "-Tpng" in seen.read_text()

    with pytest.raises(SystemExit, match="stub refused"):
        query.render_dot("digraph g {}", tmp_path / "FAIL.svg", "FAIL")


def test_missing_graphviz_is_an_explained_absence(tmp_path, monkeypatch):
    monkeypatch.setenv("PATH", str(tmp_path / "empty"), prepend=False)
    with pytest.raises(SystemExit, match="graphviz is not installed"):
        query.render_dot("digraph g {}", tmp_path / "x.svg", "svg")
