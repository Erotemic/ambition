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

import pytest

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


@pytest.fixture(scope="module")
def checked(tmp_path_factory) -> tuple[int, str]:
    """One run of the checker, shared by every assertion below.

    ⛔ EIGHT TESTS USED TO SPAWN EIGHT IDENTICAL SUBPROCESSES over the same
    fixture, each re-indexing the whole tree: 12-18 s apiece, about two minutes
    of every gate run, for one result read eight ways. The checker is a pure
    read of the repository plus one document, so a second run cannot say
    anything the first did not.

    ⚠ Module-scoped ON PURPOSE, and it is safe only because `run` WRITES the
    fixture and never mutates the tree. A test that needed a different document
    must call `run` itself with its own tmp_path rather than widen this.
    """
    return run(tmp_path_factory.mktemp("citations"))


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


def test_a_line_number_past_the_end_of_a_real_file_is_reported(checked) -> None:
    _, out = checked
    assert "999999" in out, out
    assert "has" in out and "lines" in out, out


def test_a_name_nothing_defines_is_reported(checked) -> None:
    _, out = checked
    assert "this_function_was_never_written" in out, out


def test_a_cite_ok_line_is_not_reported(checked) -> None:
    """The same fabricated name appears twice; only the unmarked one reports.

    ⛔ This is the arm that keeps the checker from destroying the record. A row
    that QUOTES a wrong name in order to record a mistake is doing its job, and
    an over-eager checker would push the next reader to "correct" it.
    """
    _, out = checked
    findings = [ln for ln in out.splitlines() if FABRICATED.split("::")[-1] in ln]
    assert len(findings) == 1, f"expected exactly one, got {findings}"


def test_upstream_and_real_names_are_not_reported(checked) -> None:
    """Silence is load-bearing: noise trains the reader to skim past a finding.

    ⛔ AND SILENCE MUST BE EARNED. The first version of this asserted only the
    two absences and PASSED while the checker was crashing on the fixture path —
    a test whose subject never ran. It now requires the run to have reported the
    findings it should before believing the ones it should not.
    """
    _, out = checked
    assert "Traceback" not in out, out
    assert "this_function_was_never_written" in out, "the checker did not run"
    assert "RollbackId" not in out, out
    assert "hold_timer" not in out, out


def test_it_reports_without_gating_unless_asked(checked) -> None:
    """Exit 0 with findings — it is wired into the test lane non-strict."""
    code, out = checked
    assert code == 0, out


# ── the bare-path class ─────────────────────────────────────────────────────
#
# ⛔ THESE ARMS EXIST BECAUSE THE CLASS WAS INVISIBLE, not because it was wrong.
# `FILE_LINE` only judges a path carrying `:123`, and planning prose almost
# never writes one — so a sweep could report "all 353 citations resolve" with a
# dead path in the file it had just read. The path check judges the other ~460.


def test_a_bare_path_that_names_nothing_is_reported(checked) -> None:
    _, out = checked
    assert DEAD_PATH in out, out
    assert "no file at this path" in out, out


def test_the_repositorys_own_abbreviations_do_not_report(checked) -> None:
    """Silence is the hard half, and it is what makes this check usable.

    ⭐ Planning prose drops the vendor prefix and as much of the crate path as
    still reads: `platformer2d_core/src/abilities.rs` IS
    `crates/ambition_platformer2d_core/src/abilities.rs`, and
    `actor_monolith/src/action_scheme.rs` IS a crate directory named
    `ambition_platformer2d_actor_monolith`. Both are correct, both are common,
    and a checker that flagged them would be the "teaches its reader to skim"
    failure the checker's own docstring warns about — worse than no check.
    """
    _, out = checked
    assert DEAD_PATH in out, "the checker did not run"
    assert "abilities.rs" not in out, out
    assert "action_scheme.rs" not in out, out


def test_a_build_output_and_an_elided_path_do_not_report(checked) -> None:
    """Neither names a file that a checkout is supposed to contain.

    A row citing `target/run_tests_status.json` is naming a build output the
    correct way; it is absent because nothing has been built, not because the
    row is wrong. A path written with `...` names a SHAPE.
    """
    _, out = checked
    assert DEAD_PATH in out, "the checker did not run"
    assert "run_tests_status.json" not in out, out
    assert "some_area.ron" not in out, out


def _tiny_repo(tmp_path: Path) -> Path:
    """A throwaway git repo, because `REPO` is `git rev-parse` on the CWD.

    ⭐ THIS IS WHY THE CHECKER IS TESTABLE AT ALL for this class: the tracked
    set comes from the working directory's repository, not from the script's
    own location, so a deleted-file scenario can be built for real instead of
    mocked. Deleting a tracked file out of Ambition to test this would be a
    much worse idea.
    """
    def git(*a: str) -> None:
        subprocess.run(["git", *a], cwd=tmp_path, check=True,
                       capture_output=True, text=True)

    git("init", "-q")
    git("config", "user.email", "t@t")
    git("config", "user.name", "t")
    (tmp_path / "src").mkdir()
    (tmp_path / "src" / "gone.rs").write_text("pub fn kept_by_nobody() {}\n")
    (tmp_path / "src" / "stays.rs").write_text("pub fn still_here() {}\n")
    git("add", "-A")
    git("commit", "-qm", "seed")
    (tmp_path / "src" / "gone.rs").unlink()  # tracked, deleted, NOT committed
    return tmp_path


def test_a_tracked_file_deleted_in_the_worktree_does_not_crash_the_checker(
    tmp_path: Path,
) -> None:
    """⛔ REGRESSION: `git ls-files` lists it and the checker opened it.

    Reported 2026-09-03 by the review session: a mid-rename tree made the whole
    run die with `FileNotFoundError` and report nothing — the worst failure a
    checker has, because a green-by-absence looks like a pass to anyone reading
    only the exit code.
    """
    repo = _tiny_repo(tmp_path)
    doc = repo / "note.md"
    doc.write_text("cites `src/stays.rs` and `still_here`\n")

    proc = subprocess.run(
        [sys.executable, str(CHECKER), str(doc)],
        cwd=repo, capture_output=True, text=True,
    )
    out = proc.stdout + proc.stderr
    assert "Traceback" not in out, out
    assert "FileNotFoundError" not in out, out
    # ⚠ And it must SAY it skipped one — a silent skip is how a half-finished
    # rename turns into a checker that quietly stops covering a directory.
    assert "src/gone.rs" in out, out
    assert "missing from the worktree" in out, out


def test_a_citation_to_a_deleted_tracked_file_is_reported_dead(
    tmp_path: Path,
) -> None:
    """⭐ THE QUIET HALF, and the reason the fix is a filter rather than a
    `try/except` at the read.

    A tracked-but-deleted path stayed in the name index, so a citation to a file
    someone had just removed still RESOLVED. Catching the crash alone would have
    left this passing — which is precisely the thing this checker exists to
    catch.
    """
    repo = _tiny_repo(tmp_path)
    doc = repo / "note.md"
    doc.write_text("this cites `src/gone.rs`, which is not there any more\n")

    proc = subprocess.run(
        [sys.executable, str(CHECKER), str(doc)],
        cwd=repo, capture_output=True, text=True,
    )
    out = proc.stdout + proc.stderr
    assert "src/gone.rs" in out, out
    assert "no file at this path" in out, out
