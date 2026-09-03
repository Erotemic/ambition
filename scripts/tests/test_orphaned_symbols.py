"""The orphan census must not call a live function dead.

⛔ A FALSE ORPHAN IS WORSE THAN A MISSED ONE. The census exists to aim a doc
sweep after a carve, so a name it reports wrongly sends someone to rewrite
sentences that were true. Every fire path below is a way it could report a
comfortable-looking name that is not actually orphaned:

* a DOC COMMENT mentioning the function -- `/// See [`Self::foo`]` is prose, and
  counting it as a call hides a genuinely dead `foo`. This one bites in both
  directions: `demote_stale_realizations` is referenced twice by its own
  neighbours' doc blocks and IS orphaned;
* the DEFINITION itself, which is one production mention of the name and must
  not make every function look live;
* a name that appears only inside a `tests.rs` / `_tests.rs` / `tests/` path.

⭐ And the overload mode's own fire path: a `_with_...` sibling only counts as
"production took over" if PRODUCTION calls it -- a sibling that only tests call
is two orphans, not a split.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

import orphaned_symbols as orph  # noqa: E402


def _repo(tmp_path: Path, files: dict[str, str]) -> Path:
    """A real git repo, because the census enumerates with `git ls-files`."""
    for name, text in files.items():
        path = tmp_path / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf8")
    subprocess.run(["git", "init", "-q"], cwd=tmp_path, check=True)
    subprocess.run(["git", "add", "-A"], cwd=tmp_path, check=True)
    return tmp_path


def _orphans(root: Path) -> set[str]:
    definitions, production, tests = orph.census(root)
    return {n for n in definitions if production[n] <= 1 and tests[n] >= 1}


def test_a_function_only_tests_call_is_an_orphan(tmp_path):
    root = _repo(
        tmp_path,
        {
            "crates/c/src/lib.rs": "pub fn only_tested() {}\npub fn live() {}\n",
            "crates/c/src/other.rs": "fn caller() { live(); }\n",
            "crates/c/src/tests.rs": "fn t() { only_tested(); live(); }\n",
        },
    )
    assert _orphans(root) == {"only_tested"}


def test_a_doc_comment_is_not_a_call(tmp_path):
    """The bug this pins: `/// See [`orphan`]` made a dead function look live,
    and the doc block that mentions it is usually its own neighbour's."""
    root = _repo(
        tmp_path,
        {
            "crates/c/src/lib.rs": (
                "/// Prose naming orphan, twice: orphan.\n"
                "pub fn orphan() {}\n"
                "//! orphan orphan orphan\n"
            ),
            "crates/c/src/tests.rs": "fn t() { orphan(); }\n",
        },
    )
    assert _orphans(root) == {"orphan"}


def test_a_definition_alone_does_not_make_a_function_live(tmp_path):
    """`pub fn f` is one production mention of `f`; the threshold is <= 1, and a
    stricter `== 0` would report nothing at all."""
    root = _repo(
        tmp_path,
        {
            "crates/c/src/lib.rs": "pub fn f() {}\n",
            "crates/c/src/tests.rs": "fn t() { f(); }\n",
        },
    )
    assert _orphans(root) == {"f"}


def test_a_function_nothing_mentions_is_not_reported(tmp_path):
    """No test mention either -- that is unused code, a different question, and
    reporting it would bury the carve delta this tool exists for."""
    root = _repo(tmp_path, {"crates/c/src/lib.rs": "pub fn nobody() {}\n"})
    assert _orphans(root) == set()


def test_an_overload_split_needs_a_sibling_production_actually_calls(tmp_path):
    """Two orphans that happen to share a prefix are not a split."""
    root = _repo(
        tmp_path,
        {
            "crates/c/src/lib.rs": (
                "pub fn rows() {}\n"
                "pub fn rows_with_prompt() {}\n"
                "pub fn other() {}\n"
                "pub fn other_with_x() {}\n"
            ),
            "crates/c/src/use.rs": "fn caller() { rows_with_prompt(); }\n",
            "crates/c/src/tests.rs": "fn t() { rows(); other(); other_with_x(); }\n",
        },
    )
    definitions, production, _ = orph.census(root)
    orphans = _orphans(root)
    splits = {
        name
        for name in orphans
        if any(
            m != name and m.startswith(name + "_") and production[m] > 1
            for m in definitions
        )
    }
    # `rows` splits (production calls `rows_with_prompt`); `other` does not,
    # because `other_with_x` is itself orphaned.
    assert splits == {"rows"}
    assert {"rows", "other", "other_with_x"} <= orphans
