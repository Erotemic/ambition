"""The no-warnings guard's parser, pinned against real cargo output.

⛔ **the first version of this parser was inverted, and it FIRED on the probe
anyway.** It matched `^warning:` — which in `--message-format=short` is cargo's
per-crate SUMMARY line, not a diagnostic — so it reported

    `ambition_geometry` (lib) generated 1 warning (run `cargo fix` …)
    `ambition_geometry` (lib test) generated 1 warning (1 duplicate)

and dropped the actual `unused import` it was standing on. Exit code 1, two rows,
zero information, and both rows describing the same single warning.

⭐ that is the lesson this file exists for: **a red guard is not a working
guard.** "I planted a defect and it went red" is the standard probe, and it was
satisfied here by an instrument that could not name the defect. The probe has to
check WHAT came back, not just that something did.

The samples below are real stderr from this workspace, not invented shapes.
"""

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
