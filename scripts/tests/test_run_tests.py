"""Tests for scripts/run_tests.py job planning.

Regression coverage for the package-filter bugs: a `-p <crate>` filter used to be
able to plan ZERO jobs and exit 0 (making a typo look green), and `--fast` ignored
`-p` entirely. Run with `pytest scripts/tests/`.
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
    assert "--workspace" in jobs[0].argv


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


def test_fast_honors_package_filter():
    pkg = KNOWN[0]
    jobs = rt.build_jobs([pkg], heavy=False, libtest_args=[], fast=True)
    # --fast -p drops feature jobs, so exactly one default job for the package.
    assert len(jobs) == 1
    assert jobs[0].argv == [rt.CARGO, "test", "-p", pkg]


def test_fast_unfiltered_is_workspace_backbone():
    jobs = rt.build_jobs([], heavy=False, libtest_args=[], fast=True)
    assert len(jobs) == 1
    assert "--workspace" in jobs[0].argv


def test_multiple_packages_each_get_a_job():
    if len(KNOWN) < 2:
        pytest.skip("need >=2 workspace members")
    pair = KNOWN[:2]
    jobs = rt.build_jobs(pair, heavy=False, libtest_args=[], fast=True)
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
    jobs = rt.build_jobs([KNOWN[0]], heavy=False, libtest_args=["--nocapture"], fast=True)
    assert jobs[0].argv[-2:] == ["--", "--nocapture"]


def test_heavy_pass_is_whole_suite_only():
    # A package filter must not drag in the heavy acceptance cycles.
    jobs = rt.build_jobs([KNOWN[0]], heavy=True, libtest_args=[])
    assert all("run_game.sh" not in " ".join(a) for a in _argvs(jobs))
