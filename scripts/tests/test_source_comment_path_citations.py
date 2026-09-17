"""`--comment-paths`: a path named in a Rust comment must exist.

⛔⛤ **THE SURFACE THIS COVERS HAD NO READER AT ALL UNTIL 2026-09-17.**
`check_planning_citations.py` resolved paths in `docs/`, and its `--comments`
mode resolved SYMBOLS in `.rs` files — so a doc comment naming a directory was
checked by nobody. Censused that day: 234 such citations, seven of them live
claims pointing at nothing, two telling a reader the simulation phase order is
*"configured by `app/schedule.rs`"*, a file that does not exist.
`docs/planning/triage/a-prose-path-inside-a-doc-comment-is-not-checked.md` is the
row.

⚠ **AND IT IS A SEPARATE FLAG FROM `--comments` BECAUSE THE TWO CAN BE WRONG IN
DIFFERENT WAYS.** A path either exists or it does not. A symbol citation can name
something this checker cannot see — a macro declaration, or an upstream method
reached through a type name the repo also defines. Both halves happen to be clean
at HEAD (2026-09-17), so the split is about the class of mistake, not a backlog.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
CHECKER = REPO / "scripts" / "check_planning_citations.py"


def _repo_with(tmp_path: Path, comment: str) -> Path:
    """A throwaway git repo whose one source file carries `comment`.

    Built rather than mocked for the reason `_tiny_repo` gives in
    `test_planning_citations.py`: the tracked set comes from the working
    directory's repository, so a citation scenario can be real.
    """
    def git(*a: str) -> None:
        subprocess.run(["git", *a], cwd=tmp_path, check=True,
                       capture_output=True, text=True)

    git("init", "-q")
    git("config", "user.email", "t@t")
    git("config", "user.name", "t")
    (tmp_path / "src").mkdir()
    (tmp_path / "src" / "stays.rs").write_text(f"{comment}\npub fn still_here() {{}}\n")
    (tmp_path / "note.md").write_text("a corpus, so the run has a document\n")
    git("add", "-A")
    git("commit", "-qm", "seed")
    return tmp_path


def _run(repo: Path, *args: str) -> tuple[int, str]:
    proc = subprocess.run(
        [sys.executable, str(CHECKER), "--comment-paths", *args,
         str(repo / "note.md")],
        cwd=repo, capture_output=True, text=True,
    )
    return proc.returncode, proc.stdout + proc.stderr


def test_a_comment_naming_a_missing_file_is_reported(tmp_path: Path) -> None:
    repo = _repo_with(tmp_path, "// see `src/gone_forever.rs` for the shape")
    code, out = _run(repo, "--strict")
    assert "src/gone_forever.rs" in out, out
    assert "no file at this path" in out, out
    assert code == 1, out


def test_a_comment_naming_a_real_file_is_not_reported(tmp_path: Path) -> None:
    """⛔ THE CONTROL. A checker that reports every path is not a checker."""
    repo = _repo_with(tmp_path, "// see `src/stays.rs` for the shape")
    code, out = _run(repo, "--strict")
    assert "src/stays.rs" not in out, out
    assert code == 0, out


def test_cite_ok_silences_a_deliberately_dead_path(tmp_path: Path) -> None:
    """A path named BECAUSE it is gone is the commonest legitimate case.

    Six of the fourteen annotations the first sweep needed were this: a comment
    saying *"that path is GONE"*, *"This was … in"*, *"used to contrast itself
    with"*. The marker is the same one `docs/` already uses.
    """
    repo = _repo_with(
        tmp_path,
        "// this WAS `src/gone_forever.rs` <!-- cite-ok: it is gone, that is the point -->",
    )
    code, out = _run(repo, "--strict")
    assert "src/gone_forever.rs" not in out, out
    assert code == 0, out


def test_the_symbol_half_stays_out_of_this_mode(tmp_path: Path) -> None:
    """⛔ THE SPLIT, ASSERTED RATHER THAN DESCRIBED.

    `--comment-paths` must not drag in the symbol sweep, or the lane job that
    runs it inherits 21 advisory findings and stops being able to gate.
    """
    repo = _repo_with(
        tmp_path,
        "// `NoSuchType::no_such_method` and `src/stays.rs`",
    )
    code, out = _run(repo, "--strict")
    assert "no_such_method" not in out, out
    assert code == 0, out


def test_a_code_line_is_not_a_comment(tmp_path: Path) -> None:
    """A string literal holding a path is data, not a citation."""
    repo = _repo_with(
        tmp_path,
        'pub const P: &str = "src/gone_forever.rs";',
    )
    code, out = _run(repo, "--strict")
    assert "src/gone_forever.rs" not in out, out
    assert code == 0, out
