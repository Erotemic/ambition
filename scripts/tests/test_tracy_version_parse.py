"""The Tracy version check must read a VERSION, not the word "Major".

⛔⛔ **A CHECK THAT COULD ONLY EVER FAIL.** `profile_deps.sh` compares the Tracy
the game links against the Tracy that was built, and parsed the first out of the
vendored `TracyVersion.hpp`. The field split took the wrong column:

    constexpr int Major = 0;   ->  "Major"

so the check printed *"the game links Major.Minor.Patch"* and compared that to a
real version. It never matched. **A warning that fires on every correctly
installed machine is one a reader learns to skip** — and that is how a REAL
mismatch (client 0.14.0, server 0.13.1) survived to cost a capture on real
hardware its per-system zones, on 2026-09-01.
"""

from __future__ import annotations

import re
import subprocess
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts/setup/profile_deps.sh"

HEADER = """#ifndef __TRACYVERSION_HPP__
namespace tracy { namespace Version {
constexpr int Major = 0;
constexpr int Minor = 14;
constexpr int Patch = 0;
}}
#endif
"""


def awk_from_script() -> str:
    """The awk program the checker actually runs, lifted from the script."""
    text = SCRIPT.read_text()
    match = re.search(r"client_version=\"\$\(awk '(.*?)' \"\$header\"\)\"", text, re.S)
    assert match, "could not find the version parse in profile_deps.sh"
    return match.group(1)


def test_the_parse_yields_a_real_version():
    with tempfile.NamedTemporaryFile("w", suffix=".hpp", delete=False) as handle:
        handle.write(HEADER)
        path = handle.name
    out = subprocess.run(
        ["awk", awk_from_script(), path], capture_output=True, text=True, check=True
    ).stdout.strip()
    assert out == "0.14.0", (
        f"the check compares this against the built Tracy; it parsed {out!r}. "
        "A value that is not a version can never equal one, so the check would "
        "report MISMATCH on every machine and mean nothing."
    )


def test_the_parse_result_looks_like_a_version_at_all():
    """⛔ PREMISE GUARD: the real defect produced 'Major.Minor.Patch', which is
    three dot-separated tokens and would pass a naive shape check."""
    with tempfile.NamedTemporaryFile("w", suffix=".hpp", delete=False) as handle:
        handle.write(HEADER)
        path = handle.name
    out = subprocess.run(
        ["awk", awk_from_script(), path], capture_output=True, text=True, check=True
    ).stdout.strip()
    assert re.fullmatch(r"\d+\.\d+\.\d+", out), f"{out!r} is not a semver"
