"""The Tracy version must be READ from the header, not guessed at.

⛔⛔ **TWO COPIES OF ONE PARSE, BOTH WRONG THE SAME WAY.** `run_developer_setup.sh`
and `profile_deps.sh` each read the version the game's `tracy-client-sys`
vendors, and each split the field wrongly:

    constexpr int Major = 0;   ->  "Major"

Both produced the version string `"Major.Minor.Patch"`, which is non-empty, so
every emptiness guard passed.

* the INSTALLER handed it to `git clone --branch vMajor.Minor.Patch`, which
  cannot resolve — so `--profile` could not install the right Tracy at all and
  quietly left whatever was already there;
* the CHECKER compared it against a real version, so it reported MISMATCH on
  every correctly-installed machine, and a warning that always fires is one a
  reader learns to skip.

Between them, a real 0.14.0-client / 0.13.1-server mismatch survived long enough
to cost a capture on an RTX 3090 its per-system zones (2026-09-01). There is now
ONE implementation, in `install_profiling_tools.sh`, and both callers ask it.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts/setup/install_profiling_tools.sh"

HEADER = """#ifndef __TRACYVERSION_HPP__
namespace tracy { namespace Version {
constexpr int Major = 0;
constexpr int Minor = 14;
constexpr int Patch = 3;
}}
#endif
"""


def print_version_with(header: str, tmp_path: Path) -> subprocess.CompletedProcess:
    """Run the real script against a fixture cargo registry."""
    vendored = (
        tmp_path
        / "registry/src/index.crates.io-abc/tracy-client-sys-0.29.0/tracy/common"
    )
    vendored.mkdir(parents=True)
    (vendored / "TracyVersion.hpp").write_text(header)
    return subprocess.run(
        ["bash", str(SCRIPT), "--print-version"],
        capture_output=True,
        text=True,
        env={"PATH": "/usr/bin:/bin", "HOME": str(tmp_path), "CARGO_HOME": str(tmp_path)},
    )


def test_the_version_comes_out_of_the_header(tmp_path):
    proc = print_version_with(HEADER, tmp_path)
    assert proc.returncode == 0, proc.stderr
    assert proc.stdout.strip() == "0.14.3", (
        f"the installer clones `v<this>` and the checker compares against it; "
        f"it produced {proc.stdout.strip()!r}"
    )


def test_a_header_it_cannot_parse_fails_loudly_instead_of_inventing_a_version(tmp_path):
    """⛔ PREMISE GUARD. The old code's failure mode was a CONFIDENT wrong answer.

    `"Major.Minor.Patch"` is three dot-separated tokens and passes a naive shape
    check, so the only safe behaviour on an unreadable header is to refuse.
    """
    proc = print_version_with("nothing resembling a version here\n", tmp_path)
    assert proc.returncode != 0, (
        f"an unparseable header must refuse, not emit a version; got {proc.stdout!r}"
    )
    assert "0.14" not in proc.stdout


def test_no_second_copy_of_the_parse_survives():
    """Both callers must ask the one script rather than re-reading the header."""
    for caller in ("run_developer_setup.sh", "scripts/setup/profile_deps.sh"):
        text = (REPO / caller).read_text()
        assert "TracyVersion.hpp" not in text or "install_profiling_tools.sh" in text, (
            f"{caller} reads the header itself again; that is how this broke twice"
        )
