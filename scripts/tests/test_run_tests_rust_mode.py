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
    """`--rust`: the Rust lane PLUS the cheap repo-coupled guard set."""
    return run_tests.build_jobs(
        [], False, [], everything=False, include_python_tooling=True,
        include_slow_python_checkers=False,
    )


def _rust_alone_jobs():
    """`--rust-alone`: nothing but Rust/Cargo."""
    return run_tests.build_jobs(
        [], False, [], everything=False, include_python_tooling=False,
        include_slow_python_checkers=False,
    )


def _pytest_marker_expression(job):
    marker_indices = [i for i, arg in enumerate(job.argv) if arg == "-m"]
    if len(marker_indices) < 2:
        return None
    return job.argv[marker_indices[1] + 1]


def test_rust_lane_keeps_the_cheap_guard_set_and_drops_the_slow_checkers():
    """⛔⛔ `--rust` USED TO DROP THE GUARD SET, and it cost a day.

    On 2026-09-02 a gate run reported 4/4 green over a tree where the rollback
    stable-name ratchet, the rollback codec-shape baseline and a crate's
    MODULES.md were all red. Nothing was swallowed and nothing lied: the run was
    `--rust`, which omitted the lane those checks live in, and said so in a
    notice among other notices.

    ⇒ The repo-coupled pytest lane RIDES ALONG with `--rust` now. MEASURED at
    43.6s against a Rust lane of ~894s -- 4.9%, which is noise against the
    thing it guards. The slower checkers (a whole `cargo check --all-targets`,
    doc links, planning citations) still go, because those are the ones `--rust`
    exists to skip.
    """
    rust_jobs = _rust_jobs()

    assert any(job.name == "workspace (default features)" for job in rust_jobs)
    assert any("pytest" in job.argv for job in rust_jobs), (
        "--rust dropped the repo-coupled guard set again; that is the exact "
        "hole that let three schema/rollback checks sit red behind a green gate"
    )
    names = [job.name for job in rust_jobs]
    assert "no warnings (cargo check --all-targets, fresh)" not in names
    assert "doc links (active KB)" not in names
    assert "planning citations (reports, does not gate)" not in names


def test_the_rust_FLAG_itself_plans_the_guard_set(tmp_path):
    """⛔⛔ THE KWARG TEST ABOVE CANNOT SEE THE WIRING, and I proved it.

    `test_rust_lane_keeps_the_cheap_guard_set...` calls `build_jobs` with
    explicit keywords, so it pins the FUNCTION. Poison-verified by restoring the
    old behaviour at the CALL SITE -- `include_python_tooling=not (args.rust or
    args.rust_alone)` -- and all twelve tests stayed green. The regression this
    whole change exists to prevent would have walked straight back in.

    ⇒ This one goes through argument parsing, which is where the mapping lives.
    `--list` plans without running anything, so it costs a process and no cargo.
    """
    import subprocess

    def planned(*flags):
        """The planned ARGV lines only.

        ⛔ NOT the whole stdout. Asserting `"pytest" in stdout` matched the
        runner's own NOTICE text ("...the repo-coupled pytest guard set...") and
        so could not fail: poisoning the wiring dropped the job and the test
        stayed green. A test that reads the prose is testing the prose.
        """
        out = subprocess.run(
            [sys.executable, str(REPO / "scripts" / "run_tests.py"), "--list", *flags],
            capture_output=True, text=True, cwd=REPO,
        )
        assert out.returncode == 0, out.stderr
        return [
            line.strip() for line in out.stdout.splitlines()
            if line.startswith("      ")
        ]

    with_rust = planned("--rust")
    assert any("pytest scripts/tests" in argv for argv in with_rust), (
        "`--rust` no longer plans the repo-coupled guard set. That is the hole "
        "that let the rollback ratchet and the codec-shape baseline sit red "
        "behind a gate reporting 4/4 green."
    )
    assert not any("check_no_warnings" in argv for argv in with_rust), (
        "`--rust` is planning the slow checkers it exists to skip"
    )

    alone = planned("--rust-alone")
    assert not any("pytest" in argv for argv in alone), (
        f"--rust-alone planned a Python job: {alone}"
    )


def test_rust_alone_is_the_shape_that_guards_nothing():
    """The escape hatch keeps existing -- under a name that says what it costs.

    ⚠ Asserted against `_rust_jobs()` as well as against the default, so this
    cannot pass by the two shapes silently becoming the same thing.
    """
    alone = _rust_alone_jobs()
    assert any(job.name == "workspace (default features)" for job in alone)
    assert not any("pytest" in job.argv for job in alone)
    assert not any(
        Path(job.argv[0]).name.startswith("python") for job in alone
    ), alone
    assert len(alone) < len(_rust_jobs()), (
        "--rust-alone and --rust plan the same jobs, so one of them is not "
        "doing what its name says"
    )


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
    warnings = names.index("no warnings (cargo check --all-targets, fresh)")
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


def test_the_maintenance_lane_holds_only_periodic_hygiene():
    """The lane exists so a housekeeping failure cannot delay a Rust edit, so
    what must not appear in it is ROUTINE VALIDATION — a cargo build, a test
    binary, anything on the path of an ordinary change.

    ⚠ THIS ASSERTED THE EXACT CONTENTS UNTIL 2026-09-03 (`len(jobs) == 1`, and
    the one job's name), which made it a test of a LIST rather than of the
    property. It reddened when the two CI-only ratchets moved into the lane —
    a change the lane is precisely for. A guard that fails on the thing it
    should permit is pinning the fix, not the gap.

    ⛔ AND IT HAPPENED AGAIN THE SAME DAY, which is why the check now reads the
    whole argv. The first relaxation replaced the list with `argv[-1]` — a
    script path OR the literal `--check`, because those were the two shapes then
    in the lane. That is still the LIST, spelled as a suffix rule: the vanished
    job ends in `--strict` and was rejected for running "not a repository
    script" while its second argument was `scripts/check_planning_citations.py`.
    ⇒ The property is "this job runs a repository script and not cargo", and it
    does not care where in the argv the script appears. A proxy that enumerates
    today's shapes will keep failing on tomorrow's, one relaxation at a time.
    """
    jobs = run_tests.build_maintenance_jobs()
    assert jobs, "the lane must not be empty, or every assertion here is vacuous"
    for job in jobs:
        assert any(str(a).startswith("scripts/") for a in job.argv), (
            f"{job.name!r} does not run a repository script: {job.argv}"
        )
        assert "cargo" not in " ".join(job.argv).lower(), (
            f"{job.name!r} runs cargo. Routine validation belongs in the default "
            "plan; this lane exists so housekeeping cannot delay a Rust edit."
        )
    names = {job.argv[1] if len(job.argv) > 1 else "" for job in jobs}
    assert "scripts/check_agent_kb.py" in names, (
        "the agent-KB audit is the lane's founding member; if it moved, say where"
    )


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


def test_an_unmeasured_job_s_wall_time_is_unclassified_not_build_time():
    """⛔⛔ THE PER-JOB NULL WAS NOT ENOUGH, and the aggregates undid it.

    Making `JobResult.executed_seconds` nullable fixed the per-job payload and
    left three aggregate roads summing `r.executed_seconds or 0.0` — the ledger
    row, the status payload and the human report. So a nextest run whose split is
    unknown was still PERSISTED as "0s executing, 100s building", which is the
    number this telemetry exists to inform. A mixed run was worse: a plausible
    partial figure with nothing saying it was partial.

    ⇒ the split is three numbers now. Build time is derived only from jobs whose
    runner reported, and everything else is named as unsplit rather than
    attributed to the build.
    """
    nextest = run_tests.JobResult("nextest job", ["cargo", "nextest"], True, 100.0)
    libtest = run_tests.JobResult("libtest job", ["cargo", "test"], True, 40.0, 30.0)

    only_unknown = run_tests.wall_time_split([nextest])
    assert only_unknown["executed_seconds"] == 0.0
    assert only_unknown["build_seconds"] == 0.0, (
        "a job that reported nothing gave its whole wall clock to the build column"
    )
    assert only_unknown["unclassified_seconds"] == 100.0
    assert only_unknown["unclassified_jobs"] == 1

    mixed = run_tests.wall_time_split([nextest, libtest])
    assert mixed["executed_seconds"] == 30.0
    assert mixed["build_seconds"] == 10.0, "build is the measured jobs' remainder"
    assert mixed["unclassified_seconds"] == 100.0

    # ⛔ AND THE REPORT SAYS SO IN WORDS. A reader must be able to see that 100s
    # of a 140s run is unaccounted for rather than infer it from a ratio.
    report = run_tests.timing_report([nextest, libtest])
    assert "NEITHER" in report and "100s" in report


def test_the_report_calls_unclassified_time_neither_build_nor_run():
    """The reader of the persisted ledger gets the same three numbers."""
    import compile_report

    rows = [
        {"seconds": 100.0, "executed_seconds": 0.0, "unclassified_seconds": 100.0},
        {"seconds": 40.0, "executed_seconds": 30.0, "unclassified_seconds": 0.0},
    ]
    totals = compile_report.suite_totals(rows)
    assert totals.executed_seconds == 30.0
    assert totals.build_seconds == 10.0, (
        "the unmeasured run's 100s was counted as build time"
    )
    assert totals.unclassified_seconds == 100.0
    # The share is against what the report can account for, not the wall clock.
    assert abs(totals.build_share - 0.25) < 1e-9
