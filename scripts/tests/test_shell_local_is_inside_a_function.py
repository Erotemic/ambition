"""`local` outside a function is fatal at RUNTIME, and `bash -n` does not see it.

⛔⛔ **IT FIRED IN JON'S FACE AT THE END OF A CAPTURE.** `profile_desktop.sh`
copies the bundle's `summary.md` into the tracked `summaries/` directory in a
tail that runs at TOP LEVEL — `main` is invoked above it — and that copy declared
`local summaries=...`:

    ./scripts/profile_desktop.sh: line 1410: local: can only be used in a function

Bash parses it happily, so `bash -n` is green and the script ships. It only dies
when that line is reached, which is after the capture, so the profiling run
itself succeeded and only the summary copy was lost — the quietest possible
failure for the most easily-missed artifact.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPTS = sorted(
    [*REPO.glob("scripts/**/*.sh"), *REPO.glob("*.sh")],
    key=lambda p: str(p),
)

FN_START = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*\(\)\s*\{")
LOCAL = re.compile(r"^\s*local\s")


def locals_outside_functions(text: str) -> list[tuple[int, str]]:
    """Every `local` reached with no enclosing function, by brace depth."""
    depth = 0
    inside = False
    out: list[tuple[int, str]] = []
    for number, line in enumerate(text.split("\n"), 1):
        if FN_START.match(line):
            inside, depth = True, 1
            continue
        if inside:
            depth += line.count("{") - line.count("}")
            if depth <= 0:
                inside = False
        if not inside and LOCAL.match(line):
            out.append((number, line.strip()[:80]))
    return out


def test_no_shell_script_declares_local_outside_a_function():
    assert SCRIPTS, "found no shell scripts; this guard stopped guarding"
    offences = {
        script.relative_to(REPO).as_posix(): found
        for script in SCRIPTS
        if (found := locals_outside_functions(script.read_text(errors="replace")))
    }
    assert not offences, (
        "`local` outside a function is a runtime error bash -n cannot see:\n"
        + "\n".join(
            f"  {path}:{n}: {line}" for path, hits in offences.items() for n, line in hits
        )
    )


def test_the_scan_actually_finds_one():
    """⛔ PREMISE GUARD. A brace-counter that silently thinks it is always inside
    a function reports zero offences forever, which looks exactly like success."""
    planted = "main() {\n  local ok=1\n}\nmain\nlocal bad=2\n"
    found = locals_outside_functions(planted)
    assert [n for n, _ in found] == [5], f"expected the top-level local, got {found}"
