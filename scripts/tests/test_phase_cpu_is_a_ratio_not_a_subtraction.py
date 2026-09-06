"""`phases_cpu` is a PROCESS clock, so wall minus CPU is not a stall.

⛔⛔ **THE SUMMARY PRINTED A COLUMN HEADED `stall`, COMPUTED AS `wall - cpu`,**
and described it as "time the frame spent in that phase with nothing running".
`process_cpu_ms` in `runtime_census.rs` reads `CLOCK_PROCESS_CPUTIME_ID`, which
SUMS EVERY THREAD — its own doc says so, and says a busy phase can report more
CPU than wall. A phase that keeps four cores busy for 2 ms reports 8 ms of CPU
against 2 ms of wall, and the column printed `-6.00` as a "stall".

The definition site had it right from the start; three comments, a planning-doc
sentence and this table propagated the thread-clock reading it had rejected.
Corrected 2026-09-02 to a `cpu/wall` RATIO: near zero is a stall, around one is
serial work, above one is parallel work.

⚠ This is not a hypothetical arm. `PostUpdate` on a rendering capture is exactly
the parallel case, and the withdrawn "the frame is CPU-bound" reading in
`performance-and-iteration.md` came off this road.
"""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts" / "lib"))

import profile_bundle_summary as summary  # noqa: E402

# `Update` keeps ~4 cores busy: 8.0 ms of process CPU against 2.0 ms of wall.
# `outside` is the opposite — a real stall, 0.05 ms of CPU against 4.0 ms wall.
PHASES_CSV = "wall_s,t,frames,PreUpdate,Update,outside\n1.0,0.5,100,3.000,2.000,4.000\n"
PHASES_CPU_CSV = "wall_s,t,frames,PreUpdate,Update,outside\n1.0,0.5,100,3.000,8.000,0.050\n"


def rendered_summary() -> str:
    tmp = Path(tempfile.mkdtemp())
    (tmp / "schedule_phases.csv").write_text(PHASES_CSV)
    (tmp / "schedule_phases_cpu.csv").write_text(PHASES_CPU_CSV)
    (tmp / "game-stderr-stamped.txt").write_text("")
    return summary.build_summary(summary.Bundle(str(tmp)))


def test_a_parallel_phase_is_never_reported_as_a_negative_stall():
    text = rendered_summary()
    # Premise: the CPU table rendered at all. Without this the absence checks
    # below would pass on a summary that never reached the phase section.
    assert "cpu/wall" in text, f"premise: the wall-against-CPU table is printed\n{text}"
    assert "8.00" in text, "premise: the parallel phase's CPU number is in the table"

    assert "-6.00" not in text, (
        "a phase keeping four cores busy printed a stall of -6.00 ms; the CPU "
        "clock sums every thread, so wall minus CPU is not a stall"
    )
    assert "stall   phase" not in text, (
        "the column header still claims the subtraction is a stall"
    )


def test_the_ratio_separates_a_parallel_phase_from_a_real_stall():
    text = rendered_summary()
    rows = {
        line.split()[-1]: line.split()
        for line in text.splitlines()
        if line.strip().endswith(("PreUpdate", "Update", "outside")) and line.startswith(" ")
    }
    assert set(rows) == {"PreUpdate", "Update", "outside"}, f"three phase rows: {rows}"

    # cpu/wall is the third column.
    assert float(rows["Update"][2]) == 4.0, "8 ms of CPU over 2 ms of wall is four cores"
    assert float(rows["PreUpdate"][2]) == 1.0, "3 over 3 is serial work"
    assert float(rows["outside"][2]) < 0.1, "0.05 ms of CPU over 4 ms of wall is a stall"

    # ⭐ AND THE ORDERING THE OLD COLUMN GOT BACKWARDS. Ranked by `wall - cpu`,
    # the four-core phase (-6.00) sorted BELOW the genuine stall (+3.95) as if it
    # were the more idle of the two. The ratio puts them the right way round.
    assert float(rows["outside"][2]) < float(rows["Update"][2]), (
        "the genuine stall must read as more idle than the parallel phase"
    )


def test_the_table_says_what_the_clock_is():
    text = rendered_summary()
    assert "CLOCK_PROCESS_CPUTIME_ID" in text, (
        "a reader with the summary but not the source must be told the clock "
        "sums every thread — that fact is the whole reason the ratio is right "
        "and the subtraction is wrong"
    )
