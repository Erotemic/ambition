"""A phase split taken while rendering must not read as CPU attribution.

⛔⛔ **THE GAME ALREADY SAID SO AND THE SUMMARY PRINTED THE TABLE ANYWAY.**
Every rendering capture emits, to stderr:

    [census] phases_warning untrustworthy=render_blocking — `[census] phases`
    attributes wall time between markers, so GPU blocking lands in whichever
    phase brackets it. Trust phase splits only from a run with no rendering.

The summary's "Which phase of the frame owned the time" table carried no trace
of that. On an RTX 3090 capture it printed `PreUpdate 3.15 ms 32.7%`, and a
reader with the summary but not the stderr — which is the normal case, the
summary is the artifact people read — concludes the phase was BUSY. It may have
been BLOCKED. Nothing in the numbers tells the two apart.

Measured 2026-09-01: that is exactly the wrong conclusion I drew from Jon's
capture before re-reading his stderr.
"""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts" / "lib"))

import profile_bundle_summary as summary  # noqa: E402

WARNING = "untrustworthy=render_blocking"
CAVEAT = "THIS SPLIT IS NOT CPU WORK"

PHASES_CSV = (
    "wall_s,t,frames,PreUpdate,Update,outside\n"
    "1.0,0.5,100,3.154,2.936,0.628\n"
)


def bundle_with(stderr: str):
    tmp = Path(tempfile.mkdtemp())
    (tmp / "schedule_phases.csv").write_text(PHASES_CSV)
    (tmp / "game-stderr-stamped.txt").write_text(stderr)
    return summary.Bundle(str(tmp))


def rendered_summary(stderr: str) -> str:
    return summary.build_summary(bundle_with(stderr))


def test_a_rendering_capture_warns_that_the_split_may_be_a_wait():
    text = rendered_summary(f"[census] phases_warning t=1.0 {WARNING} world_rendering=1\n")
    assert "3.15" in text, "premise: the table still prints its numbers"
    assert CAVEAT in text, (
        "a phase split taken while rendering must say so where the numbers are, "
        "not only in a stderr file the reader does not have"
    )
    assert "no rendering" in text, "and it must say what a trustworthy split needs"


def test_a_headless_capture_keeps_the_split_uncaveated():
    """⛔ PREMISE GUARD. Warning on every capture teaches the reader to skip it —
    which is how the stderr warning came to be ignored in the first place."""
    text = rendered_summary(
        "[census] phases_trust t=1.0 trustworthy=no_render_backend world_rendering=0\n"
    )
    assert "3.15" in text
    assert CAVEAT not in text, (
        "a run with no render backend IS a trustworthy phase split; caveating it "
        "too would make the caveat noise"
    )
