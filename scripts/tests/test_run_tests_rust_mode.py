"""The Rust-only runner lane stays free of Python tooling jobs."""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import run_tests  # noqa: E402


def test_rust_lane_omits_python_tooling_and_keeps_rust_backbone():
    rust_jobs = run_tests.build_jobs(
        [], False, [], everything=False, include_python_tooling=False
    )
    full_jobs = run_tests.build_jobs(
        [], False, [], everything=False, include_python_tooling=True
    )

    assert any(job.name == "workspace (default features)" for job in rust_jobs)
    assert any("pytest" in job.argv for job in full_jobs), (
        "the full backbone no longer contains the Python tooling lane, so this "
        "test cannot prove --rust removes it"
    )
    assert not any("pytest" in job.argv for job in rust_jobs)
    assert not any(
        Path(job.argv[0]).name.startswith("python") for job in rust_jobs
    ), rust_jobs
