"""`classify()` decides what counts as production code in the legacy census.

`measure_archetype_legacy.py` reports how much of the archetype-era surface is
left, and every number in its table is `classify()` summed over some files. The
script is honest about the one thing it cannot know — *"the baseline's own unit
is unrecorded … compare the trend, not the difference"* — but the CODE column it
computes today is its own, and nothing checked it.

⛔⛔ THE FAILURE THAT MATTERS IS TEST CODE COUNTED AS PRODUCTION. A `#[cfg(test)]
module inside a production file is exactly what makes a migration look
unfinished when the remaining lines are its own tests — the same trap
`check_rollback_mutators_run_in_sim` records paying for ("skipping `tests.rs`
and `tests/` is not enough"). These pin the three shapes it must get right.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/measure_archetype_legacy.py"


def load():
    spec = importlib.util.spec_from_file_location("archetype_legacy", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def write(tmp_path: Path, name: str, body: str) -> str:
    path = tmp_path / name
    path.write_text(body)
    return str(path)


def test_code_and_comments_are_separated_and_blanks_count_as_neither(tmp_path):
    module = load()
    code, comment, test = module.classify(write(tmp_path, "a.rs", """\
// a comment
pub fn real() {}

// another
pub struct Thing;
"""))
    assert (code, comment, test) == (2, 2, 0)


def test_an_inline_cfg_test_module_is_test_not_production(tmp_path):
    """⭐ THE ONE THAT DECIDES THE HEADLINE. Counted as production, a file whose
    remaining lines are its own tests reads as unfinished migration."""
    module = load()
    code, _comment, test = module.classify(write(tmp_path, "b.rs", """\
pub fn real() {}

#[cfg(test)]
mod tests {
    fn inner() {
        if true { let _ = 1; }
    }
}
"""))
    assert code == 1, f"only `pub fn real() {{}}` is production, got {code}"
    assert test >= 6, "the whole cfg(test) block, braces and all, is test"


def test_a_nested_brace_does_not_end_the_test_module_early(tmp_path):
    """Depth counting, not the first `}`. Ending early would leak the tail of a
    test module back into the production count."""
    module = load()
    code, _c, _t = module.classify(write(tmp_path, "c.rs", """\
#[cfg(test)]
mod tests {
    fn a() { { } }
    fn b() {}
}
pub fn after_the_module() {}
"""))
    assert code == 1, (
        f"only the line after the module is production, got {code} — a naive "
        "first-brace match would count the module's tail as code"
    )


def test_a_whole_tests_file_is_test(tmp_path):
    module = load()
    code, _c, test = module.classify(write(tmp_path, "thing_tests.rs", """\
pub fn looks_like_production() {}
"""))
    assert code == 0 and test > 0, "a `_tests.rs` file is test in its entirety"


def test_a_ron_file_has_no_test_bucket(tmp_path):
    """RON is data: it has comments and content and no notion of a test module.
    Reporting a test count for one would be inventing a distinction."""
    module = load()
    code, comment, test = module.classify(write(tmp_path, "d.ron", """\
// a note
(field: 1)
"""))
    assert (code, comment, test) == (1, 1, 0)


def test_the_live_table_still_finds_something_to_measure():
    """⛔ POSITIVE CONTROL. Every row whose path no longer matches prints
    "GONE — component deleted", which is a legitimate outcome — but if ALL of
    them were gone the script would report a table of zeroes and read as a
    completed migration rather than a stale path list."""
    module = load()
    present = 0
    for _label, pattern, _baseline in module.COMPONENTS:
        import glob

        if glob.glob(str(Path(module.REPO) / pattern)):
            present += 1
    assert present, (
        "no COMPONENTS pattern matches any file, so every row reads GONE and the "
        "census measures nothing — the paths moved rather than the code leaving"
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
