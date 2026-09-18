"""Nothing may read cargo's diagnostics with an anchor a colour escape can break.

⛔⛤ **THE DEFECT THIS RATCHETS WAS FOUND IN THREE PLACES AT ONCE ON 2026-09-18,
AND THE LANE THAT HOSTS THEM IS WHAT CAUSES IT.** `scripts/run_tests.py` exports
`CARGO_TERM_COLOR=always` to every child job, so rustc and rustdoc write
`ESC[1m ESC[33m warning ESC[0m: ...` and `ESC[1m ESC[91m error[E0432] ESC[0m:
...`. Every pattern below was anchored on the bare word:

- `check_doc_link_ratchet.py` scored **0 broken links for all 13 crates against
  a banked baseline of 141**, marked each row *"repaired"*, and advised
  `--update`, which would have written an empty baseline;
- `check_no_warnings.py` — the workspace warning gate, whose ONLY caller is that
  runner — matched nothing, so it reported clean whatever the build said;
- `run_tests.py` itself: `FailureEvidence.OTHER` and `UNRUNNABLE_SIGNATURES`
  both anchor, so a red job recorded no evidence and an UNRUNNABLE job was
  reported as a genuine failure of the code.

⚠ **THE POPULATION IS MEASURED, NOT LISTED.** A new script that greps cargo's
output for `warning:` or `error:` at a line anchor joins this test by existing,
and fails it until it reads through `scripts/lib/cargo_output.py`. That is the
point: the three above were written months apart by people who each had no
reason to think about terminal colour.
"""

from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
SCRIPTS = REPO / "scripts"

#: A cargo/rustc DIAGNOSTIC word in a position an SGR escape would precede.
#: Narrow on purpose — `^\s*(?:...)` in some unrelated log parser is not this
#: class, and a pattern broad enough to catch it would make this test noise.
ANCHORED_DIAGNOSTIC = re.compile(
    r"""startswith\(\s*["'](?:warning|error)[:\[]"""   # line.startswith("warning:")
    r"""|\^(?:warning|error)[:\[]"""                   # re.compile(r"^warning: ...")
    r"""|:\s(?:warning|error):\s"""                    # "path:line:col: warning: "
)

#: A cargo invocation. `cargo tree` and `cargo metadata` are in here too, and
#: that is fine: a file only enters the population if it ALSO anchors.
RUNS_CARGO = re.compile(r"""cargo_binary\(\)|["']cargo["']|\bCARGO\b""")

#: The owner. It states the patterns in its own docstring, so it matches its own
#: detector; everything else must go THROUGH it.
OWNER = SCRIPTS / "lib" / "cargo_output.py"

#: ⛔ AN IMPORT STATEMENT, NOT THE SUBSTRING `cargo_output`. The first version of
#: this test asked whether the module's NAME appeared anywhere in the file, and a
#: file that merely MENTIONS it in a comment — which every file this test would
#: send somebody to edit soon will — satisfied that and was excused. Caught by a
#: poison that passed.
IMPORTS_OWNER = re.compile(r"^\s*(?:from cargo_output import|import cargo_output)", re.M)


def population() -> dict[pathlib.Path, str]:
    """Every non-test script that runs cargo AND anchors on a diagnostic word."""
    found = {}
    for path in sorted(SCRIPTS.rglob("*.py")):
        if "tests" in path.relative_to(SCRIPTS).parts or path == OWNER:
            continue
        text = path.read_text(errors="replace")
        if RUNS_CARGO.search(text) and ANCHORED_DIAGNOSTIC.search(text):
            found[path] = text
    return found


def test_every_anchored_cargo_reader_goes_through_the_one_owner():
    offenders = [
        str(path.relative_to(REPO))
        for path, text in population().items()
        if not IMPORTS_OWNER.search(text)
    ]
    assert not offenders, (
        "these read cargo's diagnostics with a line anchor and do not import "
        "`scripts/lib/cargo_output.py`, so `CARGO_TERM_COLOR=always` — which "
        f"`scripts/run_tests.py` exports to every child — silences them: {offenders}"
    )


def test_the_detector_still_finds_the_three_it_was_written_for():
    """⛔ ANTI-VACUITY, and it is not decoration: the arm above passes over an
    EMPTY population, and this detector is two regexes that a refactor can
    quietly stop matching. These three are the files the defect was measured
    in — if the detector loses them it has stopped being an instrument."""
    found = {str(p.relative_to(REPO)) for p in population()}
    for name in (
        "scripts/check_doc_link_ratchet.py",
        "scripts/check_no_warnings.py",
        "scripts/run_tests.py",
    ):
        assert name in found, f"the detector no longer sees {name}: {sorted(found)}"


def test_the_owner_is_what_the_offenders_would_have_to_import():
    """⚠ A guard naming a module that does not exist is a guard that passes."""
    text = OWNER.read_text()
    for name in ("def strip_ansi", "def plain_env", "COLOR_NEVER"):
        assert name in text, f"{OWNER} no longer provides {name}"
