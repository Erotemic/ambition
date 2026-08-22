"""Tests for the warning parser against representative Cargo short-format output.

The parser must return the actual diagnostics rather than Cargo's per-crate
warning summary lines, and planted warning output must both fail the guard and
name the underlying diagnostic."""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from check_no_warnings import warnings_from  # noqa: E402

# Captured from `cargo check -p ambition_geometry --all-targets
# --message-format=short` with one unused import planted.
REAL_STDERR = """\
    Checking ambition_geometry v0.1.0 (/home/joncrall/code/ambition/crates/ambition_geometry)
crates/ambition_geometry/src/lib.rs:53:5: warning: unused import: `std::collections::BTreeSet as _ProbeUnused`
warning: `ambition_geometry` (lib) generated 1 warning (run `cargo fix --lib -p ambition_geometry` to apply 1 suggestion)
warning: `ambition_geometry` (lib test) generated 1 warning (1 duplicate)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.53s
"""


def test_the_real_diagnostic_is_reported_once_with_its_location():
    found = warnings_from(REAL_STDERR)
    assert len(found) == 1, f"expected exactly the one diagnostic, got {found}"
    assert "crates/ambition_geometry/src/lib.rs:53:5" in found[0]
    assert "unused import" in found[0]


def test_cargos_per_crate_SUMMARY_lines_are_not_warnings():
    """⛔ the inverted-parser regression, pinned.

    Both summary lines describe the SAME diagnostic — one of them says
    "(1 duplicate)" outright — so counting them reports two problems where there
    is one, and neither row says where to look.
    """
    for line in REAL_STDERR.splitlines():
        if line.startswith("warning: "):
            assert warnings_from(line) == [], f"summary counted as a warning: {line}"


def test_a_clean_build_reports_nothing():
    clean = (
        "    Checking ambition_geometry v0.1.0 (/repo/crates/ambition_geometry)\n"
        "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.61s\n"
    )
    assert warnings_from(clean) == []


def test_several_diagnostics_are_each_reported():
    stderr = (
        "a/src/lib.rs:1:5: warning: unused import: `Foo`\n"
        "b/src/lib.rs:9:1: warning: method `bar` is never used\n"
        "warning: `a` (lib) generated 1 warning\n"
    )
    found = warnings_from(stderr)
    assert len(found) == 2
    assert any("unused import" in f for f in found)
    assert any("never used" in f for f in found)
