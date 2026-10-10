"""`run_tests.sh -p <package>` adds a doctest job only for a package with a
library target.

`cargo test -p ambition_app_tools --doc` exits 101 ("no library targets found
in package"): the package has only binaries. The doctest job failed every
`-p ambition_app_tools` run whose tests all passed (reported by a peer session,
2026-10-10), so `required_checks` could not certify that package.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import run_tests  # noqa: E402


def _job_names(monkeypatch, package: str) -> list[str]:
    monkeypatch.setattr(run_tests, "NEXTEST", True)
    return [job.name for job in run_tests.build_jobs([package], False, [])]


def test_a_package_with_only_binaries_gets_no_doctest_job_and_says_so(monkeypatch, capsys):
    names = _job_names(monkeypatch, "ambition_app_tools")
    assert "ambition_app_tools (default features)" in names, names
    assert "ambition_app_tools doctests" not in names, names
    assert "no library target" in capsys.readouterr().err


def test_a_package_with_a_library_keeps_its_doctest_job(monkeypatch):
    """The control: the doctest job stays where there are doctests."""
    names = _job_names(monkeypatch, "ambition_extension_host")
    assert "ambition_extension_host doctests" in names, names


def test_the_rule_reads_the_manifest_of_each_kind(tmp_path):
    bins = tmp_path / "bins"
    (bins / "src" / "bin").mkdir(parents=True)
    (bins / "Cargo.toml").write_text('[package]\nname = "bins"\n\n[[bin]]\nname = "x"\npath = "src/bin/x.rs"\n')
    named = tmp_path / "named"
    (named / "src").mkdir(parents=True)
    (named / "Cargo.toml").write_text('[package]\nname = "named"\n\n[lib]\npath = "src/other.rs"\n')
    auto = tmp_path / "auto"
    (auto / "src").mkdir(parents=True)
    (auto / "Cargo.toml").write_text('[package]\nname = "auto"\n')
    (auto / "src" / "lib.rs").write_text("")
    assert not run_tests.has_lib_target(bins)
    assert run_tests.has_lib_target(named)
    assert run_tests.has_lib_target(auto)
