"""Tests for scripts/run_tests.py job planning.

Regression coverage for the package-filter bugs: a `-p <crate>` filter used to be
able to plan ZERO jobs and exit 0 (making a typo look green), and the old
`--fast` ignored `-p` entirely. Run with `pytest scripts/tests/`.

⭐ 2026-08-02: the default INVERTED. The backbone (python suites + one
`cargo test --workspace`) is what an unflagged run plans; the exhaustive plan is
`everything=True`, reached by `--run-everything-you-probably-dont-need-this`.
The assertions below are the contract for that inversion, so a future change
that quietly restores the 33-job default fails here.
"""
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

_RT_PATH = Path(__file__).resolve().parent.parent / "run_tests.py"


def _load_rt():
    spec = importlib.util.spec_from_file_location("run_tests", _RT_PATH)
    mod = importlib.util.module_from_spec(spec)
    # Register before exec: the module's @dataclass needs sys.modules['run_tests']
    # to resolve its string annotations (from __future__ import annotations).
    sys.modules["run_tests"] = mod
    spec.loader.exec_module(mod)
    return mod


rt = _load_rt()
KNOWN = sorted(c.name for c in rt.workspace_members() if (c / "Cargo.toml").exists())


def _argvs(jobs):
    return [j.argv for j in jobs]


def _has_pkg(jobs, pkg):
    return any("-p" in a and pkg in a for a in _argvs(jobs))


def test_workspace_backbone_when_unfiltered():
    jobs = rt.build_jobs([], heavy=False, libtest_args=[])
    assert jobs, "unfiltered run must plan at least the workspace backbone"
    assert any("--workspace" in a for a in _argvs(jobs))


def test_repo_tooling_runs_first_and_only_unfiltered():
    """`scripts/tests/` guards the goal guard, this runner, and the absence
    contracts, and it was invisible to a cargo-only plan — one of its tests had
    been red for a day because nothing ran it (2026-07-28).

    FIRST, because if the thing that decides whether the suite is honest is
    broken, that is the answer rather than the forty minutes of cargo behind it.
    Not under a package filter: `-p some_crate` is a question about that crate."""
    jobs = rt.build_jobs([], heavy=False, libtest_args=[])
    assert jobs[0].argv[1:] == ["-m", "pytest", "scripts/tests", "-q"]

    filtered = rt.build_jobs([KNOWN[0]], heavy=False, libtest_args=[])
    assert all("pytest" not in a for a in _argvs(filtered))


def test_selected_package_always_gets_a_default_job():
    # The core regression: a package with no extra feature-gated tests must still
    # plan its default-feature `cargo test -p` job, never zero jobs.
    for pkg in ("ambition_sfx", KNOWN[0]):
        if pkg not in KNOWN:
            continue
        jobs = rt.build_jobs([pkg], heavy=False, libtest_args=[])
        assert jobs, f"-p {pkg} planned zero jobs"
        assert _has_pkg(jobs, pkg)
        # A package filter never runs the whole-workspace backbone.
        assert all("--workspace" not in a for a in _argvs(jobs))


def test_package_filter_plans_exactly_the_one_default_job():
    pkg = KNOWN[0]
    jobs = rt.build_jobs([pkg], heavy=False, libtest_args=[])
    # A filtered default run drops feature jobs: exactly one job for the package.
    assert len(jobs) == 1
    assert jobs[0].argv == [rt.CARGO, "test", "-p", pkg]


def test_the_default_plan_is_the_workspace_backbone():
    """⭐ The inversion, asserted. An unflagged run is the BACKBONE.

    Before 2026-08-02 this shape was only reachable with `--fast`, and the
    default was 33 jobs / ~63 minutes of which ~7% executed tests. Being the
    default is what made it the thing an agent ran instead of the focused test
    that would have answered the question, so the default is now this.
    """
    jobs = rt.build_jobs([], heavy=False, libtest_args=[])
    # The two Python tooling jobs plus the workspace backbone. Both tooling jobs
    # are in the backbone deliberately: together they cost ~6s, and they are what
    # the rest of the plan is trusted through — one guards the runner and the
    # architectural contracts, the other guards the LDtk authoring path every
    # room in the game is built with.
    #
    # ⚠ the WARNING gate joined on 2026-08-02, deliberately and not for free:
    # it is a `cargo check --all-targets`, so it is the only backbone job whose
    # cost is not a few seconds. It is here because CI sets
    # `RUSTFLAGS: -D warnings` and nothing local did — five warnings had
    # accumulated in the tree, so every local run was green while CI was red. A
    # gate that only exists after a push finds things late.
    #
    # Named, not counted. A bare `len(jobs) == N` passes when a job is swapped
    # for a different one, and its failure message says nothing about which job
    # is missing — this assertion caught the ldtk job being ADDED and reported
    # only "3 != 2" (2026-07-28), and it caught this one being added too.
    names = {j.name for j in jobs}
    assert names == {
        "repo tooling (scripts/tests)",
        "ldtk authoring tools (tools/ambition_ldtk_tools)",
        "no warnings (cargo check --all-targets)",
        "workspace (default features)",
    }, f"unexpected backbone plan: {sorted(names)}"
    assert any("--workspace" in a for a in _argvs(jobs))
    assert all("--features" not in a for a in _argvs(jobs)), (
        "the backbone plans no feature jobs")


def test_multiple_packages_each_get_a_job():
    if len(KNOWN) < 2:
        pytest.skip("need >=2 workspace members")
    pair = KNOWN[:2]
    jobs = rt.build_jobs(pair, heavy=False, libtest_args=[])
    for pkg in pair:
        assert _has_pkg(jobs, pkg)


def test_unknown_package_is_a_hard_error():
    with pytest.raises(SystemExit):
        rt.build_jobs(["definitely_not_a_real_crate_zzz"], heavy=False, libtest_args=[])


def test_unknown_package_reported_even_alongside_a_valid_one():
    with pytest.raises(SystemExit):
        rt.build_jobs([KNOWN[0], "definitely_not_a_real_crate_zzz"],
                      heavy=False, libtest_args=[])


def test_libtest_args_are_forwarded():
    jobs = rt.build_jobs([KNOWN[0]], heavy=False, libtest_args=["--nocapture"])
    assert jobs[0].argv[-2:] == ["--", "--nocapture"]


def test_heavy_pass_is_whole_suite_only():
    # A package filter must not drag in the heavy acceptance cycles.
    jobs = rt.build_jobs([KNOWN[0]], heavy=True, libtest_args=[])
    assert all("run_game.sh" not in " ".join(a) for a in _argvs(jobs))


# ── Per-job timing report ─────────────────────────────────────────────────────


def _results():
    return [
        rt.JobResult("fast-green", ["cargo", "test", "-p", "a"], True, 1.2),
        rt.JobResult("slow-green", ["cargo", "test", "--workspace"], True, 300.7),
        rt.JobResult("mid-red", ["cargo", "test", "-p", "b"], False, 42.0),
    ]


def test_timing_report_ranks_slowest_first():
    report = rt.timing_report(_results())
    lines = report.splitlines()
    assert "slowest first" in lines[0]
    order = [line.split()[-1] for line in lines[1:]]
    assert order == ["slow-green", "mid-red", "fast-green"]


def test_timing_report_tags_failures_without_hiding_their_time():
    report = rt.timing_report(_results())
    (red_line,) = [l for l in report.splitlines() if "mid-red" in l]
    assert "FAIL" in red_line
    assert "42.0s" in red_line
    (green_line,) = [l for l in report.splitlines() if "slow-green" in l]
    assert "FAIL" not in green_line


def test_timings_payload_shape():
    payload = rt.timings_payload(_results())
    assert [row["job"] for row in payload] == ["fast-green", "slow-green", "mid-red"]
    for row in payload:
        assert set(row) == {"job", "command", "ok", "seconds"}
        assert isinstance(row["ok"], bool)
        assert isinstance(row["seconds"], float)
    assert payload[1]["command"] == "cargo test --workspace"


def test_timings_payload_is_json_serializable(tmp_path):
    import json

    path = tmp_path / "timings.json"
    path.write_text(json.dumps(rt.timings_payload(_results())))
    assert json.loads(path.read_text())[2]["ok"] is False


def test_the_web_build_is_checked_in_the_whole_suite():
    """The web target sat broken for at least four days because nothing in the
    suite compiled it — every native job stayed green while `--features web` had
    four errors in it (docs/planning/repair_wasm.md).

    A CHECK rather than a test run: there is no wasm runner here, and a check is
    what the failure mode needs anyway. Exhaustive-only, because it builds a
    second target's dependency graph, and not under a package filter, because
    `-p some_crate` is a question about that crate.

    ⚠ 2026-08-02: this moved OUT of the default plan with the inversion, so the
    web build is unchecked by a default run. That is a real loss and it is
    accepted deliberately — `coverage_notice()` names the web check as one of
    the three things the backbone does not cover, on every single run, which is
    the difference between a stated trade and the silent gap that let the web
    target sit broken for four days in the first place.
    """
    if not rt.wasm_target_installed():
        pytest.skip("wasm32-unknown-unknown is not installed on this machine")

    jobs = rt.build_jobs([], heavy=False, libtest_args=[], everything=True)
    web = [j for j in jobs if "wasm32-unknown-unknown" in j.argv]
    assert len(web) == 2, f"expected both web personas, got {[j.name for j in web]}"
    personas = {a for j in web for a in j.argv} & {"web", "web_served_assets"}
    assert personas == {"web", "web_served_assets"}
    for job in web:
        assert "check" in job.argv, "the web job must be a check, not a test run"

    backbone = rt.build_jobs([], heavy=False, libtest_args=[])
    assert not [j for j in backbone if "wasm32-unknown-unknown" in j.argv], (
        "the default is the backbone; a second target's dependency graph is not"
    )


def test_a_non_exhaustive_run_says_what_it_did_not_cover():
    """⛔ A skipped coverage that says nothing reads exactly like coverage that
    passed. That is the defect that let the web target stay broken for four
    days, and making the backbone the DEFAULT would recreate it wholesale — so
    every non-exhaustive plan ends by naming its own blind spots.
    """
    notice = rt.coverage_notice(exhaustive=False, filtered=False)
    assert "does NOT cover" in notice
    for missing in ("cfg(feature", "external-consumer", "wasm"):
        assert missing in notice, f"the notice must name {missing}"
    assert "--run-everything-you-probably-dont-need-this" in notice, (
        "naming the gap without naming the flag that closes it is half a notice")

    # The exhaustive plan has nothing to disclaim.
    assert rt.coverage_notice(exhaustive=True, filtered=False) == ""

    # A package filter is narrower still, and says so in its own words.
    assert "package filter" in rt.coverage_notice(exhaustive=False, filtered=True)


def test_heavy_implies_the_exhaustive_plan():
    """`--heavy` is the MORE-than-exhaustive pass, so it cannot mean less.

    Before the inversion `heavy` rode on top of a default that already planned
    the feature jobs. It now has to request them itself, and this is the
    assertion that catches it if that wiring is ever dropped.
    """
    jobs = rt.build_jobs([], heavy=True, libtest_args=[])
    assert any("--features" in a for a in _argvs(jobs)), (
        "--heavy must plan the feature jobs, not just the ignored-test pass")
    assert any("--include-ignored" in a for a in _argvs(jobs))
