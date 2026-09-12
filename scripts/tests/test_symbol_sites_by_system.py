"""The refusal branches of `measure_symbol_sites_by_system.py`, plus its verdicts.

⛔⛤ THE REFUSALS ARE THE POINT, NOT THE HAPPY PATH. An attribution tool that
prints a table for a file it could not read, or for a symbol that appears nowhere,
reports a clean bill of health for a measurement it never made — the exact defect
this repository has been bitten by (a `git grep` pathspec before the pattern,
scanning 0 files and printing `ok`). Each arm below drives one branch and asserts
the EXIT CODE, because that is the half a caller in a pipeline reads.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "measure_symbol_sites_by_system.py"

FIXTURE = '''\
pub struct Thing {
    pub active_tab: usize,
}

pub(crate) fn already_has_it(
    mut pages: ResMut<ActiveMenuPages<A, B>>,
    mut state: ResMut<Thing>,
) {
    state.active_tab = 1;
    pages.active = None;
}

fn takes_it_as_a_param(active_tab: usize) -> usize {
    active_tab + 1
}

pub(crate) fn would_need_it(mut state: ResMut<Thing>) {
    // active_tab in a COMMENT must not be counted as a site
    state.active_tab = 2;
}
'''


def run(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        capture_output=True,
        text=True,
        check=False,
    )


def test_the_script_is_where_the_test_thinks_it_is() -> None:
    # ⛔ THE PREMISE. Every arm below shells out, and a moved script would make
    # them all fail with an unhelpful "No such file", which reads like a defect in
    # the subject rather than in the test.
    assert SCRIPT.is_file(), SCRIPT


def test_a_missing_file_is_refused_rather_than_reported_as_clean(tmp_path: Path) -> None:
    done = run("active_tab", str(tmp_path / "nope.rs"))
    assert done.returncode == 2, done
    assert "not a file" in done.stderr


def test_an_empty_file_is_refused(tmp_path: Path) -> None:
    empty = tmp_path / "empty.rs"
    empty.write_text("")
    done = run("active_tab", str(empty))
    assert done.returncode == 2, done
    assert "nothing was measured" in done.stderr


def test_a_symbol_that_appears_nowhere_is_refused_and_says_why(tmp_path: Path) -> None:
    src = tmp_path / "x.rs"
    src.write_text(FIXTURE)
    done = run("no_such_symbol", str(src))
    assert done.returncode == 1, done
    # ⭐ The message must push the reader at the QUERY, not at the code.
    assert "claim about the query" in done.stderr


def test_a_comment_only_symbol_is_refused_and_counted_separately(tmp_path: Path) -> None:
    src = tmp_path / "c.rs"
    src.write_text("// active_tab is only mentioned here\nfn f() {}\n")
    done = run("active_tab", str(src))
    assert done.returncode == 1, done
    assert "1 comment mentions" in done.stderr


def test_the_three_verdicts_are_distinguished(tmp_path: Path) -> None:
    src = tmp_path / "g.rs"
    src.write_text(FIXTURE)
    done = run("active_tab", str(src))
    assert done.returncode == 0, done
    out = done.stdout
    assert "already_has_it" in out and "YES" in out
    assert "takes_it_as_a_param" in out and "takes-it-as-a-param" in out
    assert "would_need_it" in out and "NO" in out
    # ⛔ AND THE COMMENT LINE IS NOT A SITE. Without this the tool would price a
    # doc mention as work, which is how a sizing gets inflated.
    assert "1 in comments" in out
    assert "1 site(s) in functions that already have it" in out
    assert "1 site(s) in functions that would need it" in out


def test_the_needs_pattern_is_configurable(tmp_path: Path) -> None:
    src = tmp_path / "n.rs"
    src.write_text(FIXTURE)
    # ⚠ A pattern nothing matches must move every row to NO, which is what proves
    # the verdict is computed from the pattern rather than hard-coded.
    done = run("active_tab", str(src), "--needs", r"never_appears_anywhere")
    assert done.returncode == 0, done
    assert "YES" not in done.stdout


def test_a_field_above_the_first_fn_is_attributed_not_dropped(tmp_path: Path) -> None:
    src = tmp_path / "f.rs"
    src.write_text("pub struct T {\n    pub active_tab: usize,\n}\n\nfn later() {}\n")
    done = run("active_tab", str(src))
    assert done.returncode == 0, done
    # A field declaration is exactly what a collapse deletes; losing it would
    # under-price the job by one.
    assert "<module>" in done.stdout
