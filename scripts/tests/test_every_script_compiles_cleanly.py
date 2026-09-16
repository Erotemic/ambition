"""Every script must COMPILE with no warning, not merely parse.

⛔⛤ **WRITTEN BECAUSE I INTRODUCED ONE AND THE GATE DID NOT CARE.** On
2026-09-16 a docstring edit put `\\`` inside a non-raw string in
`check_planning_citations.py` — an invalid escape sequence — so every
`import check_planning_citations` printed a `SyntaxWarning`. `--maintenance` was
11/11 with it present, and `ast.parse` returned normally: the check I had run
answered a NARROWER question ("does it parse") than the one I cared about ("is
this file clean").

⭐ **INSTALLED WHILE THE ANSWER IS STILL ZERO.** 342 scripts, 0 warnings at the
moment this arm was written — which is exactly when a comparison is worth
installing, because a regression here is silent: a `SyntaxWarning` goes to stderr
beside a green verdict, and in a lane that captures output nobody sees it at all.

⚠ It compiles rather than imports: importing runs module-level code, and a test
that executes 342 scripts is a different and much worse test.
"""

from __future__ import annotations

import pathlib
import warnings

SCRIPTS = pathlib.Path(__file__).resolve().parents[1]


def compile_warnings() -> list[str]:
    found: list[str] = []
    for path in sorted(SCRIPTS.rglob("*.py")):
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            try:
                compile(path.read_text(encoding="utf-8"), str(path), "exec")
            except SyntaxError as error:
                found.append(f"{path.name}: SyntaxError {error}")
                continue
            for entry in caught:
                found.append(
                    f"{path.name}:{entry.lineno} "
                    f"{entry.category.__name__}: {entry.message}"
                )
    return found


def test_no_script_compiles_with_a_warning():
    found = compile_warnings()
    assert not found, "scripts compile with warnings:\n  " + "\n  ".join(found)


def test_the_corpus_is_there():
    """⛔ ANTI-VACUITY: an empty sweep reports clean, same as a healthy tree."""
    assert len(list(SCRIPTS.rglob("*.py"))) > 200


def test_the_sweep_can_see_an_invalid_escape():
    """⛔ THE KNOWN-ANSWER CONTROL, in the exact shape that got through.

    A regex for the string would not prove the COMPILER reports it; this asks
    the compiler.
    """
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        compile('def f():\n    """a \\` b"""\n', "<control>", "exec")
    assert any(w.category is SyntaxWarning for w in caught), [
        w.category.__name__ for w in caught
    ]
