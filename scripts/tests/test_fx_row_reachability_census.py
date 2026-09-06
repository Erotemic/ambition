"""The fx-row census, which a maintainer decision rests on.

`awaiting-maintainer-decision.md` carries *"nothing in the repository names 34 of
those 35 rows"* for `npc_pirate_admiral` and `smash_george_booul`, and asks
whether to wire them or treat them as superseded. That row is only as good as
this script.

⛔ THE SCRIPT IS FLAT — no functions, so there is nothing to unit-test. The
honest guard is to RUN it and check the invariants its own output must satisfy,
plus the population control: a scan that found no sheets would report "0 named
by nothing", which reads as a clean tree rather than a broken glob.

⚠ It asserts SHAPE and CONSISTENCY, not the specific counts. Wiring an effect
should not turn this red; a scan that stops seeing the tree should.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/measure_fx_row_reachability.py"

TOTALS = re.compile(r"(\d+) sheets, (\d+) rows, (\d+) named, (\d+) named by nothing")
PER_SHEET = re.compile(r"^(\S+)\s+(\d+)\s+(\d+)\s+(\d+)\s*$", re.M)


@pytest.fixture(scope="module")
def output() -> str:
    done = subprocess.run(
        [sys.executable, str(SCRIPT)], capture_output=True, text=True, cwd=REPO
    )
    assert done.returncode == 0, f"the census exited {done.returncode}:\n{done.stderr}"
    return done.stdout


def test_it_found_a_tree_to_measure(output):
    """⛔ THE CONTROL. An empty scan reports 0 rows named by nothing, which reads
    as 'every effect is wired' — the most reassuring possible broken result."""
    hit = TOTALS.search(output)
    assert hit, f"no totals line in the output:\n{output[-400:]}"
    sheets, rows, _named, _unnamed = (int(g) for g in hit.groups())
    assert sheets >= 5, f"only {sheets} fx sheets found; the sheet scan is broken"
    assert rows >= 50, f"only {rows} rows found; the row scan is broken"


def test_named_and_unnamed_account_for_every_row(output):
    """The arithmetic the decision quotes. If these do not add up, a row is being
    counted twice or dropped, and '34 of 35' is arithmetic on sand."""
    sheets, rows, named, unnamed = (int(g) for g in TOTALS.search(output).groups())
    assert named + unnamed == rows, (
        f"{named} named + {unnamed} unnamed != {rows} rows — the buckets do not "
        "partition the population"
    )


def test_the_per_sheet_rows_sum_to_the_total(output):
    """A per-sheet table that disagrees with its own total means the table and
    the headline were computed over different populations."""
    sheets, rows, named, _unnamed = (int(g) for g in TOTALS.search(output).groups())
    per = [(m.group(1), int(m.group(2)), int(m.group(3))) for m in PER_SHEET.finditer(output)]
    per = [row for row in per if not row[0].startswith("sheets")]
    assert len(per) == sheets, f"{len(per)} per-sheet lines against {sheets} sheets"
    assert sum(r[1] for r in per) == rows
    assert sum(r[2] for r in per) == named


def test_a_sheet_with_no_named_row_is_called_out_separately(output):
    """`pirate_admiral_vfx` is the extreme case the decision row is about, and a
    sheet where NOTHING is named is a different fact from a sheet with a few
    unnamed rows. The census must keep them apart."""
    assert "sheets with NO row named by anything" in output, (
        "the all-unnamed sheets must be reported as their own class; folding "
        "them into the per-row total loses the fact the decision turns on"
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
