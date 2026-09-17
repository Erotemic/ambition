"""The `path:N` drift checker, held against a fixture it can actually fail on.

⛔ **EVERY ARM HERE BUILDS ITS OWN TWO-COMMIT REPOSITORY.** Run against this
repository the checker can only assert today's corpus back at itself — a stub
returning `[]` would pass anything phrased as "no unexpected drift". A fixture
lets the test MOVE a line and watch the verdict flip, which is the only
observation that distinguishes a working checker from a silent one.
"""

from __future__ import annotations

import importlib.util
import pathlib
import subprocess

import pytest

REPO = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "check_planning_line_citations.py"


@pytest.fixture(scope="module")
def tool():
    spec = importlib.util.spec_from_file_location("cplc", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _git(root, *args):
    done = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True)
    assert done.returncode == 0, done.stderr
    return done


@pytest.fixture
def fixture_repo(tmp_path):
    """A source file, a doc citing one of its lines, and a commit holding both."""
    root = tmp_path / "repo"
    (root / "src").mkdir(parents=True)
    (root / "docs" / "planning").mkdir(parents=True)
    _git(root.parent, "init", "-q", str(root))
    _git(root, "config", "user.email", "fixture@example.invalid")
    _git(root, "config", "user.name", "fixture")
    (root / "src" / "thing.rs").write_text(
        "// one\n// two\nfn the_subject() {}\n// four\n"
    )
    (root / "docs" / "planning" / "page.md").write_text(
        "The subject is at `src/thing.rs:3`.\n"
    )
    _git(root, "add", "-A")
    _git(root, "commit", "-qm", "fixture")
    return root


def _verdicts(tool, root):
    corpus = tool.Corpus(tool._load_citation_checker(), root)
    docs = sorted((root / "docs" / "planning").rglob("*.md"))
    return {(f.path, f.lineno): f for f in tool.examine(docs, corpus)}


def test_an_unmoved_line_reads_as_same(tool, fixture_repo):
    found = _verdicts(tool, fixture_repo)
    assert found[("src/thing.rs", 3)].verdict == "same"


def test_a_line_pushed_down_reads_as_moved_and_names_where_it_went(tool, fixture_repo):
    """The defect this whole script exists for: the file grows ABOVE the citation."""
    src = fixture_repo / "src" / "thing.rs"
    src.write_text("// inserted\n" + src.read_text())

    found = _verdicts(tool, fixture_repo)
    row = found[("src/thing.rs", 3)]
    assert row.verdict == "moved"
    assert row.suggestion == 4, "the subject moved down exactly one line"
    assert row.was == "fn the_subject() {}"


def test_fix_repoints_the_citation_and_the_second_run_is_clean(tool, fixture_repo):
    src = fixture_repo / "src" / "thing.rs"
    src.write_text("// inserted\n" + src.read_text())
    doc = fixture_repo / "docs" / "planning" / "page.md"

    corpus = tool.Corpus(tool._load_citation_checker(), fixture_repo)
    assert tool.repoint(tool.examine([doc], corpus)) == 1
    assert "`src/thing.rs:4`" in doc.read_text()

    # ⚠ The repair is UNCOMMITTED now, so its own reference point is gone. That
    # is a real verdict and not a drift — the checker says so in its own word.
    corpus = tool.Corpus(tool._load_citation_checker(), fixture_repo)
    again = tool.examine([doc], corpus)
    assert [f.verdict for f in again] == ["uncommitted"]

    _git(fixture_repo, "add", "-A")
    _git(fixture_repo, "commit", "-qm", "repointed")
    corpus = tool.Corpus(tool._load_citation_checker(), fixture_repo)
    assert [f.verdict for f in tool.examine([doc], corpus)] == ["same"]


def test_a_line_that_was_deleted_reads_as_gone_rather_than_moved(tool, fixture_repo):
    """⛔ `gone` IS NOT REPAIRABLE AND MUST NOT BE REPOINTED.

    The cited statement no longer exists, so any line this tool picked would be
    a different claim wearing the citation's coordinates.
    """
    src = fixture_repo / "src" / "thing.rs"
    src.write_text("// one\n// two\n// gone\n// four\n")

    row = _verdicts(tool, fixture_repo)[("src/thing.rs", 3)]
    assert row.verdict == "gone"
    assert row.suggestion is None

    doc = fixture_repo / "docs" / "planning" / "page.md"
    before = doc.read_text()
    corpus = tool.Corpus(tool._load_citation_checker(), fixture_repo)
    assert tool.repoint(tool.examine([doc], corpus)) == 0
    assert doc.read_text() == before


def test_two_identical_lines_read_as_ambiguous_rather_than_guessed(tool, fixture_repo):
    src = fixture_repo / "src" / "thing.rs"
    src.write_text("fn the_subject() {}\n// two\nfn the_subject() {}\n// four\n")

    row = _verdicts(tool, fixture_repo)[("src/thing.rs", 3)]
    # The cited line still HOLDS its text, so nothing drifted at all.
    assert row.verdict == "same"

    src.write_text(
        "// inserted\nfn the_subject() {}\n// two\nfn the_subject() {}\n// four\n"
    )
    row = _verdicts(tool, fixture_repo)[("src/thing.rs", 3)]
    assert row.verdict == "ambiguous", "two candidates is a reader's call, not a guess"
    assert row.suggestion is None


def test_the_floor_fires_when_nothing_was_examined(tool, capsys):
    """⛔ THE EMPTY-CORPUS SIGNATURE IS NOT A CLEAN RUN.

    A broken resolver, an unreadable blame or an empty document list all produce
    zero findings, and "0 drifted" reads exactly like a repaired corpus.
    """
    assert tool.main(["--quiet", str(REPO / "docs" / "planning" / "README.md")]) in (0, 2)
    code = tool.main(["--quiet", "--root", str(REPO), "does-not-exist.txt"])
    assert code == 2
    assert "reporting on ITSELF" in capsys.readouterr().err


def test_a_cite_ok_marker_silences_a_deliberately_historical_coordinate(
    tool, fixture_repo
):
    """⛔ ONE KEEPER FOR "WRONG ON PURPOSE", AND USING THE TOOL IS WHAT FOUND IT.

    `pickup-carve-checklist.md` cites the PRE-CUT path on purpose and says so
    with `cite-ok`. Without this the coordinate is a permanent `gone`, and a
    report with permanent entries is a report nobody re-reads.
    """
    doc = fixture_repo / "docs" / "planning" / "page.md"
    src = fixture_repo / "src" / "thing.rs"
    src.write_text("// one\n// two\n// gone\n// four\n")

    before = _verdicts(tool, fixture_repo)[("src/thing.rs", 3)].verdict
    assert before == "gone", "the fixture must be drifted for this arm to mean anything"

    doc.write_text(
        "The subject was at `src/thing.rs:3`.\n"
        "<!-- cite-ok: the pre-cut path, kept as the record -->\n"
    )
    _git(fixture_repo, "add", "-A")
    _git(fixture_repo, "commit", "-qm", "marker")

    corpus = tool.Corpus(tool._load_citation_checker(), fixture_repo)
    rows = tool.examine([doc], corpus)
    assert [f.verdict for f in rows] == ["cite-ok"]

    # ⛔ AND IT MUST NOT BE REPOINTED EITHER: a marker that only silenced the
    # report while `--fix` rewrote the line would be worse than no marker.
    assert tool.repoint(rows) == 0
    assert "`src/thing.rs:3`" in doc.read_text()
