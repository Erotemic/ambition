"""The planning-citation checker reports what it should and stays quiet elsewhere.

⛔ THE POINT OF THIS TEST IS THE THREE-WAY SPLIT, not the count. The checker
exists because a fabricated citation and a moved one look identical in prose, and
a deliberately-quoted wrong name must not be "fixed". A checker that reported all
three the same way, or none of them, would be worse than none at all — so each
arm here pins one of the three outcomes, on a fixture rather than on the live
docs (which are corrected as they are read, and would make this test a moving
target).
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CHECKER = REPO / "scripts" / "check_planning_citations.py"

#: A name no source in this repository defines. `ControlHold::Sequence` and
#: `ids::SHOCKWAVE` were both reported as missing by an earlier indexer and both
#: EXIST, so the fixture uses a name chosen to exist nowhere rather than a real
#: one that might quietly come back.
FABRICATED = "grid_backend::this_function_was_never_written"

FIXTURE = f"""# Fixture

A moved file:line citation: `{{moved_file}}:999999`.

A fabricated symbol: `{FABRICATED}`.

A deliberately quoted wrong name: `{FABRICATED}` <!-- cite-ok -->

An upstream qualifier that must NOT report: `bevy_ggrs::RollbackId`.

A real symbol that must NOT report: `PossessionState::hold_timer`.

A bare path that names nothing: `{{dead_path}}`.

An abbreviated crate path that must NOT report: `platformer2d_core/src/abilities.rs`.

A deeper abbreviation that must NOT report: `actor_monolith/src/action_scheme.rs`.

A build output that must NOT report: `target/run_tests_status.json`.

An elided path that must NOT report: `tools/.../specs/some_area.ron`.
"""

#: A path shaped exactly like the repository's own, naming a file that is not
#: there. `features/ecs/footstool.rs` was the real one this class was written
#: for -- `00030e603` moved it to `crates/ambition_combat/src/` -- and the
#: fixture uses a name chosen to stay missing rather than a real one that might
#: come back.
DEAD_PATH = "features/ecs/this_file_was_never_there.rs"


def run(tmp_path: Path) -> tuple[int, str]:
    doc = tmp_path / "fixture.md"
    # A file that certainly exists and certainly has fewer than 999999 lines.
    doc.write_text(FIXTURE.format(
        moved_file="scripts/check_planning_citations.py", dead_path=DEAD_PATH,
    ))
    proc = subprocess.run(
        [sys.executable, str(CHECKER), str(doc)],
        cwd=REPO, capture_output=True, text=True,
    )
    return proc.returncode, proc.stdout + proc.stderr


def test_a_line_number_past_the_end_of_a_real_file_is_reported(tmp_path: Path) -> None:
    _, out = run(tmp_path)
    assert "999999" in out, out
    assert "has" in out and "lines" in out, out


def test_a_name_nothing_defines_is_reported(tmp_path: Path) -> None:
    _, out = run(tmp_path)
    assert "this_function_was_never_written" in out, out


def test_a_cite_ok_line_is_not_reported(tmp_path: Path) -> None:
    """The same fabricated name appears twice; only the unmarked one reports.

    ⛔ This is the arm that keeps the checker from destroying the record. A row
    that QUOTES a wrong name in order to record a mistake is doing its job, and
    an over-eager checker would push the next reader to "correct" it.
    """
    _, out = run(tmp_path)
    findings = [ln for ln in out.splitlines() if FABRICATED.split("::")[-1] in ln]
    assert len(findings) == 1, f"expected exactly one, got {findings}"


def test_upstream_and_real_names_are_not_reported(tmp_path: Path) -> None:
    """Silence is load-bearing: noise trains the reader to skim past a finding.

    ⛔ AND SILENCE MUST BE EARNED. The first version of this asserted only the
    two absences and PASSED while the checker was crashing on the fixture path —
    a test whose subject never ran. It now requires the run to have reported the
    findings it should before believing the ones it should not.
    """
    _, out = run(tmp_path)
    assert "Traceback" not in out, out
    assert "this_function_was_never_written" in out, "the checker did not run"
    assert "RollbackId" not in out, out
    assert "hold_timer" not in out, out


def test_it_reports_without_gating_unless_asked(tmp_path: Path) -> None:
    """Exit 0 with findings — it is wired into the test lane non-strict."""
    code, out = run(tmp_path)
    assert code == 0, out


# ── the bare-path class ─────────────────────────────────────────────────────
#
# ⛔ THESE ARMS EXIST BECAUSE THE CLASS WAS INVISIBLE, not because it was wrong.
# `FILE_LINE` only judges a path carrying `:123`, and planning prose almost
# never writes one — so a sweep could report "all 353 citations resolve" with a
# dead path in the file it had just read. The path check judges the other ~460.


def test_a_bare_path_that_names_nothing_is_reported(tmp_path: Path) -> None:
    _, out = run(tmp_path)
    assert DEAD_PATH in out, out
    assert "no file at this path" in out, out


def test_the_repositorys_own_abbreviations_do_not_report(tmp_path: Path) -> None:
    """Silence is the hard half, and it is what makes this check usable.

    ⭐ Planning prose drops the vendor prefix and as much of the crate path as
    still reads: `platformer2d_core/src/abilities.rs` IS
    `crates/ambition_platformer2d_core/src/abilities.rs`, and
    `actor_monolith/src/action_scheme.rs` IS a crate directory named
    `ambition_platformer2d_actor_monolith`. Both are correct, both are common,
    and a checker that flagged them would be the "teaches its reader to skim"
    failure the checker's own docstring warns about — worse than no check.
    """
    _, out = run(tmp_path)
    assert DEAD_PATH in out, "the checker did not run"
    assert "abilities.rs" not in out, out
    assert "action_scheme.rs" not in out, out


def test_a_build_output_and_an_elided_path_do_not_report(tmp_path: Path) -> None:
    """Neither names a file that a checkout is supposed to contain.

    A row citing `target/run_tests_status.json` is naming a build output the
    correct way; it is absent because nothing has been built, not because the
    row is wrong. A path written with `...` names a SHAPE.
    """
    _, out = run(tmp_path)
    assert DEAD_PATH in out, "the checker did not run"
    assert "run_tests_status.json" not in out, out
    assert "some_area.ron" not in out, out
