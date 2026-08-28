"""The test runner keeps repo, Rust, detached-tool, and maintenance scopes honest."""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import run_tests  # noqa: E402


def _default_jobs():
    return run_tests.build_jobs(
        [], False, [], everything=False, include_python_tooling=True
    )


def _rust_jobs():
    return run_tests.build_jobs(
        [], False, [], everything=False, include_python_tooling=False
    )


def _pytest_marker_expression(job):
    marker_indices = [i for i, arg in enumerate(job.argv) if arg == "-m"]
    if len(marker_indices) < 2:
        return None
    return job.argv[marker_indices[1] + 1]


def test_rust_lane_omits_python_tooling_and_keeps_rust_backbone():
    rust_jobs = _rust_jobs()
    full_jobs = _default_jobs()

    assert any(job.name == "workspace (default features)" for job in rust_jobs)
    assert any("pytest" in job.argv for job in full_jobs), (
        "the full backbone no longer contains the Python tooling lane, so this "
        "test cannot prove --rust removes it"
    )
    assert not any("pytest" in job.argv for job in rust_jobs)
    assert not any(
        Path(job.argv[0]).name.startswith("python") for job in rust_jobs
    ), rust_jobs


def test_default_starts_rust_immediately_after_repo_coupled_pytest():
    jobs = _default_jobs()
    assert [job.name for job in jobs[:2]] == [
        "repo tooling (scripts/tests; repo-coupled)",
        "workspace (default features)",
    ]

    names = [job.name for job in jobs]
    rust = names.index("workspace (default features)")
    render = names.index("acceptance: the render composition draws a frame")
    consumer = names.index("external consumer: outlander COMPILES against the umbrella")
    warnings = names.index("no warnings (cargo check --all-targets)")
    docs = names.index("doc links (active KB)")
    ratchet = names.index("compile-cost ratchet (frozen weights, not a stopwatch)")
    assert rust < render < consumer < warnings < docs < ratchet


def test_default_omits_periodic_and_detached_tool_suites():
    jobs = _default_jobs()
    names = [job.name for job in jobs]
    assert not any("agent KB" in name for name in names)
    assert not any("ldtk authoring tool" in name for name in names)


def test_repo_backbone_excludes_detached_tool_tests():
    tooling = next(job for job in _default_jobs() if job.name.startswith("repo tooling"))
    assert _pytest_marker_expression(tooling) == "not detached_tool"


def test_detached_tool_lane_contains_marker_suite_and_ldtk_suite():
    jobs = run_tests.build_detached_tool_jobs()
    assert [job.name for job in jobs] == [
        "detached repo developer tools",
        "ldtk authoring tool tests",
    ]
    assert _pytest_marker_expression(jobs[0]) == "detached_tool"
    assert jobs[1].cwd == str(REPO / "tools" / "ambition_ldtk_tools")
    assert "pytest" in jobs[1].argv


def test_focused_detached_tool_lane_does_not_run_unrelated_ldtk_suite():
    jobs = run_tests.build_detached_tool_jobs("timeout")
    assert len(jobs) == 1
    job = jobs[0]
    assert _pytest_marker_expression(job) == "detached_tool"
    assert job.argv[job.argv.index("-k") + 1] == "timeout"


def test_maintenance_lane_is_only_periodic_agent_kb_hygiene():
    jobs = run_tests.build_maintenance_jobs()
    assert len(jobs) == 1
    assert jobs[0].name == "agent KB (periodic doc/index hygiene)"
    assert jobs[0].argv[-1] == "scripts/check_agent_kb.py"


def _doctest_jobs(jobs):
    return [j for j in jobs if "--doc" in j.argv]


def test_a_package_filtered_run_under_nextest_still_runs_that_package_s_doctests(
    monkeypatch,
):
    """⛔⛔ `./run_tests.sh -p X` covered LESS than the `cargo test -p X` it replaced.

    nextest does not execute doctests, so the runner adds a separate Cargo
    doctest job — but only on the unfiltered workspace branch. A package-scoped
    run got none, on a road whose own reason for existing is that
    `ambition_sim_harness` had a doctest that never compiled.

    ⛔ AND ONLY UNDER NEXTEST, which is the other half: plain `cargo test`
    already runs doctests, so adding the job unconditionally is a second pass.
    Both arms are asserted because a fix for either one alone reads correct.
    """
    monkeypatch.setattr(run_tests, "NEXTEST", True)
    scoped = run_tests.build_jobs(
        ["ambition_sim_harness"], False, [], everything=False,
        include_python_tooling=False,
    )
    assert [j.argv for j in _doctest_jobs(scoped)] == [
        [run_tests.CARGO, "test", "-p", "ambition_sim_harness", "--doc"]
    ], "a package-scoped nextest run skips that package's doctests"

    monkeypatch.setattr(run_tests, "NEXTEST", False)
    plain = run_tests.build_jobs(
        ["ambition_sim_harness"], False, [], everything=False,
        include_python_tooling=False,
    )
    assert not _doctest_jobs(plain), (
        "plain `cargo test -p X` already runs doctests; a separate job is a "
        "second pass over the same code"
    )


def test_an_unreported_execution_time_serializes_as_absent_not_as_zero():
    """⛔⛔ A zero is a MEASUREMENT: "this job spent no time running tests".

    nextest prints its `Summary [ Xs ]` on stderr, which the runner leaves
    attached so cargo's progress bar renders — so a nextest job has no duration
    to parse. Defaulting the field to `0.0` wrote that non-answer into the
    timing payload and the compile-cost ledger as a real number, and the derived
    build-vs-execution split then attributed the entire wall clock to the build.
    """
    unmeasured = run_tests.JobResult("nextest job", ["cargo", "nextest"], True, 12.5)
    measured = run_tests.JobResult("libtest job", ["cargo", "test"], True, 12.5, 9.25)

    rows = run_tests.timings_payload([unmeasured, measured])
    assert rows[0]["executed_seconds"] is None
    assert rows[1]["executed_seconds"] == 9.25

    # And the human report says so in words rather than printing `0.0s`.
    assert "not reported" in run_tests.timing_report([unmeasured])
    assert "9.2s" in run_tests.timing_report([measured])
