"""Regression tests for the git-well-backed Ambition source archiver."""
from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path
from types import SimpleNamespace

import pytest

pytestmark = pytest.mark.detached_tool

_ARCHIVER_PATH = Path(__file__).resolve().parent.parent / "archive_agent_source.py"


def _load_archiver():
    spec = importlib.util.spec_from_file_location("archive_agent_source", _ARCHIVER_PATH)
    mod = importlib.util.module_from_spec(spec)
    sys.modules["archive_agent_source"] = mod
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


archiver = _load_archiver()


def _fake_context(tmp_path: Path, **overrides):
    archive_root = tmp_path / "stage" / "ambition-agent-source-test"
    archive_root.mkdir(parents=True)
    defaults = {
        "repo_root": tmp_path / "repo",
        "archive_root": archive_root,
        "archive_root_name": archive_root.name,
        "archive_format": "tar.gz",
        "head_sha": "a" * 40,
        "short_sha": "a" * 12,
        "depth": 100,
        "include_git_history": True,
        "all_branches": False,
        "branch_refs": None,
        "submodule_decisions": (),
        "source_display_path": lambda: str(tmp_path / "repo"),
        "add_generated_excludes": lambda paths: None,
    }
    defaults.update(overrides)
    return SimpleNamespace(**defaults)


def test_build_archive_delegates_to_git_well(monkeypatch, tmp_path: Path):
    captured = {}
    output = tmp_path / "out.zip"

    def fake_git(_root, *args, **_kwargs):
        if args == ("rev-parse", "HEAD"):
            return "a" * 40
        if args == ("rev-parse", "--short=12", "HEAD"):
            return "a" * 12
        if args == ("status", "--short"):
            return ""
        raise AssertionError(args)

    def fake_archive_source(**kwargs):
        captured.update(kwargs)
        return output

    monkeypatch.setattr(archiver, "coerce_repo_root", lambda path: tmp_path)
    monkeypatch.setattr(archiver, "git", fake_git)
    monkeypatch.setattr(
        archiver,
        "_load_git_well_archive_api",
        lambda: (fake_archive_source, object),
    )

    result = archiver.build_archive(
        tmp_path,
        output,
        "custom-root",
        keep_stage=True,
        verbose=0,
        depth="25",
        all_branches=True,
        submodule_depth='{"*": 0, tools/ambition_sfx_renderer: 50}',
        exclude_submodule=["tools/experimental/*"],
        no_submodules=False,
        archive_format="zip",
        redact_local_paths=True,
    )

    assert result == output
    assert captured["repo_dpath"] == tmp_path
    assert captured["output"] == output
    assert captured["depth"] == "25"
    assert captured["all_branches"] is True
    assert captured["submodule_depth"] == '{"*": 0, tools/ambition_sfx_renderer: 50}'
    assert captured["exclude_submodule"] == ["tools/experimental/*"]
    assert captured["format"] == "zip"
    assert captured["redact_local_paths"] is True
    assert captured["archive_root_name"] == "custom-root"
    assert captured["keep_stage"] is True
    assert callable(captured["prepare"])
    assert callable(captured["validate"])


def test_build_archive_uses_repo_depth_defaults(monkeypatch, tmp_path: Path):
    captured = {}

    def fake_git(_root, *args, **_kwargs):
        values = {
            ("rev-parse", "HEAD"): "b" * 40,
            ("rev-parse", "--short=12", "HEAD"): "b" * 12,
            ("status", "--short"): "",
        }
        return values[args]

    def fake_archive_source(**kwargs):
        captured.update(kwargs)
        return tmp_path / "out.tar.gz"

    monkeypatch.setattr(archiver, "coerce_repo_root", lambda path: tmp_path)
    monkeypatch.setattr(archiver, "git", fake_git)
    monkeypatch.setattr(
        archiver,
        "_load_git_well_archive_api",
        lambda: (fake_archive_source, object),
    )

    archiver.build_archive(
        tmp_path,
        None,
        None,
        keep_stage=False,
        verbose=0,
    )

    assert captured["depth"] == archiver.CONFIG["super_depth"]
    assert captured["submodule_depth"] == archiver.CONFIG["submodule_depths"]
    assert captured["format"] == "tar.gz"
    assert str(captured["archive_root_name"]).startswith("ambition-agent-source-")


def test_main_forwards_git_well_cli_options(monkeypatch, tmp_path: Path):
    captured = {}
    output = tmp_path / "agent.zip"

    def fake_build(*args, **kwargs):
        captured["args"] = args
        captured["kwargs"] = kwargs
        captured["toggles"] = {
            key: archiver.CONFIG[key]
            for key in (
                "run_agent_index",
                "run_ecs_inventory",
                "run_agent_navigation",
                "run_dirstats",
                "run_live_disk_inventory",
            )
        }
        return output

    monkeypatch.setattr(archiver, "build_archive", fake_build)
    monkeypatch.setattr(archiver, "print_output_location", lambda path: None)

    result = archiver.main(
        [
            str(tmp_path),
            "--output",
            str(output),
            "--depth",
            "25",
            "--all-branches",
            "--submodule-depth",
            "full",
            "--exclude-submodule",
            "tools/a",
            "--exclude-submodule",
            "tools/b*",
            "--no-submodules",
            "--format",
            "zip",
            "--redact-local-paths",
            "--slim",
            "--keep-stage",
            "--quiet",
        ]
    )

    assert result == 0
    kwargs = captured["kwargs"]
    assert kwargs["depth"] == "25"
    assert kwargs["all_branches"] is True
    assert kwargs["submodule_depth"] == "full"
    assert kwargs["exclude_submodule"] == ["tools/a", "tools/b*"]
    assert kwargs["no_submodules"] is True
    assert kwargs["archive_format"] == "zip"
    assert kwargs["redact_local_paths"] is True
    assert kwargs["verbose"] == 0
    assert kwargs["full_reports"] is False
    assert all(value is False for value in captured["toggles"].values())


def test_prepare_registers_excludes_and_refreshes_stamp(monkeypatch, tmp_path: Path):
    events = []
    excludes = []
    context = _fake_context(
        tmp_path,
        add_generated_excludes=lambda paths: excludes.extend(paths),
        source_display_path=lambda: "(redacted)",
    )
    extension = archiver.AmbitionArchiveExtension(
        repo_root=tmp_path / "repo",
        dirty_status="",
        allow_forbidden=False,
        full_reports=False,
        log=archiver.Log(0),
    )

    monkeypatch.setattr(
        archiver,
        "enforce_forbidden_path_policy",
        lambda *args: events.append("forbidden"),
    )
    monkeypatch.setattr(
        archiver,
        "run_agent_index",
        lambda *args: events.append("index"),
    )
    monkeypatch.setattr(
        archiver,
        "refresh_generation_stamp",
        lambda *args: events.append("stamp"),
    )
    monkeypatch.setattr(
        archiver,
        "run_ecs_inventory",
        lambda *args: events.append("ecs"),
    )
    monkeypatch.setattr(
        archiver,
        "run_agent_navigation",
        lambda *args: events.append("navigation"),
    )
    monkeypatch.setattr(
        archiver,
        "run_dirstats",
        lambda *args: events.append("dirstats"),
    )
    monkeypatch.setattr(
        archiver,
        "run_live_disk_inventory",
        lambda *args: events.append("live"),
    )
    monkeypatch.setattr(
        archiver,
        "write_manifests",
        lambda **kwargs: events.append("manifest"),
    )

    extension.prepare(context)

    assert excludes == ["/.agent/", "/SOURCE_ARCHIVE_MANIFEST.txt"]
    assert events == [
        "forbidden",
        "index",
        "stamp",
        "ecs",
        "navigation",
        "dirstats",
        "live",
        "manifest",
    ]


def test_refresh_generation_stamp_works_without_staged_git(monkeypatch, tmp_path: Path):
    context = _fake_context(tmp_path, include_git_history=False)
    stamp = context.archive_root / ".agent" / "index" / "generation_stamp.json"
    stamp.parent.mkdir(parents=True)
    stamp.write_text('{"generated_from_commit": "unknown"}\n')
    monkeypatch.setattr(
        archiver,
        "git",
        lambda *args, **kwargs: "2026-08-01T12:00:00-04:00",
    )

    archiver.refresh_generation_stamp(
        context,
        "2026-08-01T16:00:00Z",
        archiver.Log(0),
    )

    payload = json.loads(stamp.read_text())
    assert payload["generated_from_commit"] == context.short_sha
    assert payload["source_commit_time"] == "2026-08-01T12:00:00-04:00"
    assert payload["generated_at"] == "2026-08-01T16:00:00Z"


def test_forbidden_scan_uses_all_archived_branch_refs(monkeypatch, tmp_path: Path):
    refs = SimpleNamespace(
        local_branches=("main", "feature"),
        remote_tracking_branches=("origin/old",),
    )
    context = _fake_context(
        tmp_path,
        all_branches=True,
        branch_refs=refs,
    )
    calls = []

    def fake_git(_root, *args, **_kwargs):
        calls.append(args)
        if args[:4] == ("ls-tree", "-r", "--name-only", "HEAD"):
            return "README.md"
        if args[:3] == ("log", "--format=", "--name-only"):
            return "docs/planning/plugin_refactor/snapshots/secret.txt"
        raise AssertionError(args)

    monkeypatch.setattr(archiver, "git", fake_git)
    found = archiver.find_forbidden_paths(context)

    assert found["history"] == [
        "docs/planning/plugin_refactor/snapshots/secret.txt"
    ]
    log_call = calls[1]
    assert "refs/heads/main" in log_call
    assert "refs/heads/feature" in log_call
    assert "refs/remotes/origin/old" in log_call


def test_clean_validation_rejects_tracked_mutation(monkeypatch, tmp_path: Path):
    context = _fake_context(tmp_path)
    monkeypatch.setattr(
        archiver,
        "git",
        lambda *args, **kwargs: " M .agent/manifest.yaml",
    )
    with pytest.raises(RuntimeError, match="modified tracked source"):
        archiver.validate_staged_checkout_clean(context)


def test_clean_validation_allows_omitted_submodule(monkeypatch, tmp_path: Path):
    decision = SimpleNamespace(
        omitted=True,
        info=SimpleNamespace(path="tools/omitted"),
    )
    context = _fake_context(tmp_path, submodule_decisions=(decision,))
    monkeypatch.setattr(
        archiver,
        "git",
        lambda *args, **kwargs: " D tools/omitted",
    )
    archiver.validate_staged_checkout_clean(context)


def test_manifests_honor_redaction(tmp_path: Path):
    context = _fake_context(
        tmp_path,
        source_display_path=lambda: "(redacted by --redact-local-paths)",
    )
    archiver.write_manifests(
        context=context,
        generated_at="2026-08-01T16:00:00Z",
        dirty_status="",
        full_reports=False,
    )
    yaml_text = (
        context.archive_root / ".agent" / "source_archive_manifest.yaml"
    ).read_text()
    txt_text = (context.archive_root / "SOURCE_ARCHIVE_MANIFEST.txt").read_text()
    assert str(tmp_path) not in yaml_text
    assert str(tmp_path) not in txt_text
    assert "redacted by --redact-local-paths" in yaml_text
    assert "redacted by --redact-local-paths" in txt_text


def test_slim_manifest_omits_skipped_payload_outputs(monkeypatch, tmp_path: Path):
    context = _fake_context(tmp_path)
    for key in (
        "run_agent_index",
        "run_ecs_inventory",
        "run_agent_navigation",
        "run_dirstats",
        "run_live_disk_inventory",
    ):
        monkeypatch.setitem(archiver.CONFIG, key, False)

    archiver.write_manifests(
        context=context,
        generated_at="2026-08-01T18:25:20Z",
        dirty_status="",
        full_reports=False,
    )

    yaml_text = (
        context.archive_root / ".agent" / "source_archive_manifest.yaml"
    ).read_text()
    txt_text = (context.archive_root / "SOURCE_ARCHIVE_MANIFEST.txt").read_text()

    assert "schema_version: 3" in yaml_text
    for key in (
        "agent_index_enabled",
        "ecs_inventory_enabled",
        "agent_navigation_enabled",
        "full_reports_enabled",
        "dirstats_enabled",
        "live_disk_inventory_enabled",
    ):
        assert f"  {key}: false" in yaml_text

    for skipped_output in (
        ".agent/index/generation_stamp.json",
        ".agent/ecs_inventory/project.md",
        ".agent/index/catalog.json",
        ".agent/reports/cargo-check-warnings.md",
        ".agent/dirstats-repo-summary.txt",
        ".agent/live-disk-inventory-summary.txt",
        ".agent/live-git-status-ignored.txt",
    ):
        assert skipped_output not in yaml_text

    for label in (
        "Agent index",
        "ECS inventory",
        "Agent navigation",
        "Full reports",
        "Dirstats",
        "Live disk inventory",
    ):
        assert f"- {label}: skipped" in txt_text


def test_manifest_lists_outputs_for_enabled_payloads(monkeypatch, tmp_path: Path):
    context = _fake_context(tmp_path)
    for key in (
        "run_agent_index",
        "run_ecs_inventory",
        "run_agent_navigation",
        "run_dirstats",
        "run_live_disk_inventory",
    ):
        monkeypatch.setitem(archiver.CONFIG, key, True)

    archiver.write_manifests(
        context=context,
        generated_at="2026-08-01T18:25:20Z",
        dirty_status="",
        full_reports=True,
    )

    yaml_text = (
        context.archive_root / ".agent" / "source_archive_manifest.yaml"
    ).read_text()
    txt_text = (context.archive_root / "SOURCE_ARCHIVE_MANIFEST.txt").read_text()

    for key in (
        "agent_index_enabled",
        "ecs_inventory_enabled",
        "agent_navigation_enabled",
        "full_reports_enabled",
        "dirstats_enabled",
        "live_disk_inventory_enabled",
    ):
        assert f"  {key}: true" in yaml_text

    for generated_output in (
        ".agent/index/generation_stamp.json",
        ".agent/ecs_inventory/project.md",
        ".agent/index/catalog.json",
        ".agent/reports/cargo-check-warnings.md",
        ".agent/dirstats-repo-summary.txt",
        ".agent/live-disk-inventory-summary.txt",
        ".agent/live-git-status-ignored.txt",
    ):
        assert generated_output in yaml_text

    for label in (
        "Agent index",
        "ECS inventory",
        "Agent navigation",
        "Full reports",
        "Dirstats",
        "Live disk inventory",
    ):
        assert f"- {label}: generated" in txt_text


def _run_main_capturing_toggles(monkeypatch, tmp_path: Path, argv: list[str]) -> dict:
    captured: dict = {}

    def fake_build(*args, **kwargs):
        captured["run_dependency_graph"] = archiver.CONFIG["run_dependency_graph"]
        return tmp_path / "agent.tar.gz"

    monkeypatch.setattr(archiver, "build_archive", fake_build)
    monkeypatch.setattr(archiver, "print_output_location", lambda path: None)
    assert archiver.main([str(tmp_path), *argv]) == 0
    return captured


def test_dependency_graph_is_off_unless_asked_for(monkeypatch, tmp_path: Path):
    """Graphviz is the one tool this repo's payload does not otherwise need."""
    assert archiver.CONFIG["run_dependency_graph"] is False
    off = _run_main_capturing_toggles(monkeypatch, tmp_path, [])
    on = _run_main_capturing_toggles(monkeypatch, tmp_path, ["--dependency-graph"])
    assert off["run_dependency_graph"] is False
    assert on["run_dependency_graph"] is True
    # and the flag is restored afterwards, so one run cannot leak into the next.
    assert archiver.CONFIG["run_dependency_graph"] is False


def test_dependency_graph_cannot_outlive_its_prerequisite(monkeypatch, tmp_path: Path):
    """The drawings read graphs the navigation step writes.

    Asking for a picture of a graph that was never built is a request that
    cannot be honored; it must not turn into a confusing failure deep in the
    staged tree, nor into an empty drawing.
    """
    for skip in (["--skip-agent-navigation"], ["--skip-index"], ["--slim"]):
        captured = _run_main_capturing_toggles(
            monkeypatch, tmp_path, ["--dependency-graph", *skip]
        )
        assert captured["run_dependency_graph"] is False, skip


def test_dependency_graph_paths_required_only_when_the_step_runs(tmp_path: Path):
    archive_root = tmp_path / "root"
    (archive_root / ".agent/index/crates").mkdir(parents=True)
    for name in archiver.CONFIG["required_archive_paths"]:
        target = archive_root / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text("x")
    saved = {
        key: archiver.CONFIG[key]
        for key in (
            "run_agent_index",
            "run_ecs_inventory",
            "run_agent_navigation",
            "run_dirstats",
            "run_live_disk_inventory",
            "run_dependency_graph",
        )
    }
    try:
        for key in saved:
            archiver.CONFIG[key] = False
        archiver.validate_archive_root(archive_root)  # nothing else enabled: passes
        archiver.CONFIG["run_dependency_graph"] = True
        with pytest.raises(RuntimeError, match="graph-declared.dot"):
            archiver.validate_archive_root(archive_root)
    finally:
        archiver.CONFIG.update(saved)


def test_resolved_graph_is_drawn_only_when_it_was_resolved(tmp_path: Path):
    """⚠ absence stays explained.

    A resolved graph that cargo could not build ships as `available: false`
    with a reason, so skipping its drawing leaves a file in the same directory
    saying why. Drawing it anyway would produce a picture of nothing.
    """
    crates = tmp_path / ".agent/index/crates"
    crates.mkdir(parents=True)
    resolved = crates / "graph-resolved.json"

    assert archiver.dependency_graphs_to_draw(tmp_path) == ["declared"]

    resolved.write_text(json.dumps({"available": False, "reason": "cargo not runnable"}))
    assert archiver.dependency_graphs_to_draw(tmp_path) == ["declared"]

    resolved.write_text(json.dumps({"available": True, "edges": {}}))
    assert archiver.dependency_graphs_to_draw(tmp_path) == ["declared", "resolved"]


def test_dependency_graph_step_fails_loudly_without_graphviz(monkeypatch, tmp_path: Path):
    """⛔ opt-in means it does NOT degrade: you asked for a picture."""
    monkeypatch.setattr(archiver.shutil, "which", lambda name: None)
    monkeypatch.setitem(archiver.CONFIG, "run_dependency_graph", True)
    with pytest.raises(RuntimeError, match="graphviz"):
        archiver.run_dependency_graph(tmp_path, lambda *a, **k: None)

    # ...and it is silent, not fatal, when the step was never requested.
    monkeypatch.setitem(archiver.CONFIG, "run_dependency_graph", False)
    archiver.run_dependency_graph(tmp_path, lambda *a, **k: None)


def test_ecs_inventory_command_honors_inline_dependency_metadata():
    assert archiver.CONFIG["ecs_inventory_command"][:4] == [
        "uv",
        "run",
        "--script",
        "scripts/ecs_inventory.py",
    ]
    assert sys.executable not in archiver.CONFIG["ecs_inventory_command"]


def test_run_ecs_inventory_resolves_uv_and_runs_script(monkeypatch, tmp_path: Path):
    captured = {}

    monkeypatch.setattr(
        archiver.shutil,
        "which",
        lambda name: "/opt/bin/uv" if name == "uv" else None,
    )

    def fake_run(args, **kwargs):
        captured["args"] = list(args)
        captured["kwargs"] = kwargs
        return SimpleNamespace(returncode=0, stdout="", stderr="")

    monkeypatch.setattr(archiver, "run", fake_run)
    archiver.run_ecs_inventory(tmp_path, archiver.Log(0))

    assert captured["args"][:4] == [
        "/opt/bin/uv",
        "run",
        "--script",
        "scripts/ecs_inventory.py",
    ]
    assert captured["kwargs"]["cwd"] == tmp_path
    assert captured["kwargs"]["check"] is True


def test_run_ecs_inventory_fails_cleanly_without_uv(monkeypatch, tmp_path: Path):
    monkeypatch.setattr(archiver.shutil, "which", lambda _name: None)

    with pytest.raises(archiver.CommandError, match="requires `uv`"):
        archiver.run_ecs_inventory(tmp_path, archiver.Log(0))

