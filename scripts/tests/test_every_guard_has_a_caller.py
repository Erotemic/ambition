"""A `check_*.py` that nothing runs is indistinguishable from one that passes.

⚠ **and one that IS run, without the flag that lets it fail, is the same thing
one level down.** That is the second test in this file, added 2026-08-02 after
`test_doc_and_roadmap_guards_run.py` was found invoking
`check_roadmap_evidence.py` WITHOUT `--check` — proved by injecting a problem and
watching the test PASS. The defect had already happened once, to
`check_absence_contracts.py` in the goal, and is recorded in the paragraph below;
it happened again anyway, in the file that records it.

Four of them were found this way on 2026-08-01: `modules_md.py` had a check mode
nobody called, the goal invoked `check_absence_contracts.py` WITHOUT `--check`
(the flag that lets it fail), and `check_doc_links.py` and
`check_roadmap_evidence.py` appeared in no runner at all.

⚠ **`check_doc_links.py` is why this is a test and not a note.** It was green for
the whole run *because a person was running it by hand after every doc edit*. The
greenness was evidence about the person, not about the repository, and it would
have stayed green right up until they stopped.

## Why the `check_` prefix is the rule

The first draft looked for "any script with a non-zero exit path" and had to
carve out `ecs_inventory.py`, `regen_music_registry.py`, `render_line_profiles.py`
and friends — GENERATORS, which legitimately exit non-zero and legitimately have
no caller in a test suite. An allowlist of those would need maintaining and would
be waived the first time it got in the way.

The naming convention already draws the line the allowlist was trying to draw, so
this uses it. A guard is called `check_*`; if a new one is not, it is not covered,
and that is a naming problem with an obvious fix rather than a silent hole.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# Anywhere a guard can legitimately be invoked from.
CALLER_PATHS = (
    REPO / "scripts" / "run_tests.py",
    REPO / "scripts" / "tests",
    REPO / ".github" / "workflows",
    REPO / ".goal" / "active.json",
)


def _caller_text() -> str:
    chunks: list[str] = []
    for path in CALLER_PATHS:
        if path.is_dir():
            for child in sorted(path.rglob("*")):
                if child.is_file():
                    try:
                        chunks.append(child.read_text(encoding="utf-8"))
                    except (UnicodeDecodeError, OSError):
                        continue
        elif path.is_file():
            try:
                chunks.append(path.read_text(encoding="utf-8"))
            except (UnicodeDecodeError, OSError):
                pass
    return "\n".join(chunks)


def test_every_check_script_is_invoked_by_something():
    guards = sorted(p.stem for p in (REPO / "scripts").glob("check_*.py"))
    assert guards, "the glob found no guards at all, which means it is broken"

    callers = _caller_text()
    orphaned = [name for name in guards if name not in callers]
    assert not orphaned, (
        "these guards are never invoked, so they cannot fail and cannot be "
        f"trusted: {orphaned}. Add a `scripts/tests` case that runs the guard "
        "against the live tree — that suite runs first and cheaply in the "
        "backbone — or a `run_tests.py` job, or a CI step. A check with no "
        "caller is indistinguishable from one that passes."
    )


# A guard whose exit code is gated on a flag: `return 1 if (problems and
# args.check) else 0`. Without the flag it prints its findings and exits 0, which
# every caller that reads an exit code will read as a pass.
def _executable_lines(text: str) -> list[str]:
    """Caller text with comments and docstrings blanked out.

    ⛔ **the prose that documents a defect mentions the defect.** This test read
    its own explanatory comments — which contain `--check` — as evidence that a
    caller passed the flag, and passed while the caller did not. Blanked rather
    than dropped so line numbers and windows still line up.
    """
    out: list[str] = []
    fence: str | None = None
    for line in text.splitlines():
        stripped = line.strip()
        if fence is None:
            for quote in ('"""', "\'\'\'"):
                if stripped.startswith(quote):
                    rest = stripped[3:]
                    fence = None if quote in rest else quote
                    line = ""
                    break
            else:
                line = line.split("#", 1)[0]
        else:
            if fence in line:
                fence = None
            line = ""
        out.append(line)
    return out


EXIT_IS_FLAG_GATED = re.compile(r"return\s+1\s+if\s+[^\n]*args\.(check|strict)")
TAKES_FLAG = re.compile(r'add_argument\(\s*"--(check|strict)"')


def test_every_flag_gated_guard_is_called_with_the_flag():
    """A guard that CAN fail must be invoked in the mode where it can.

    ⛔ this is the shallower test's blind spot. `test_every_guard_has_a_caller`
    asks whether anything runs the script; this asks whether that run can report
    a failure. `check_roadmap_evidence.py` passed the first and failed the second
    for as long as both existed.
    """
    callers = _caller_text()
    offenders: list[str] = []
    for script in sorted((REPO / "scripts").glob("check_*.py")):
        body = script.read_text(encoding="utf-8")
        if not (EXIT_IS_FLAG_GATED.search(body) and TAKES_FLAG.search(body)):
            continue
        name = script.name
        lines = _executable_lines(callers)
        sites = [i for i, line in enumerate(lines) if name in line]
        if not sites:
            continue  # the other test owns "nothing calls it at all"
        # ⛔ **a WINDOW over EXECUTABLE lines**, and both halves were learned the
        # hard way. Draft 1 matched the flag on the same line as the script name
        # and missed a parametrize entry that puts args on the next line. Draft 2
        # added the window and STILL passed — because the comments explaining this
        # very defect contain the string `--check`, so the prose describing the
        # hole convinced the checker the hole was closed. Both were found by
        # removing the flag and watching this test stay green.
        window = 6
        contexts = []
        for i in sites:
            chunk = lines[max(0, i - window) : i + window + 1]
            contexts.append("\n".join(chunk))
        if not any("--check" in c or "--strict" in c for c in contexts):
            offenders.append(
                f"{name}: exit code is gated on --check/--strict, and no caller "
                f"passes it. Sites:\n      "
                + "\n      ".join(lines[i].strip() for i in sites)
            )

    assert not offenders, (
        "a guard is run in a mode where it CANNOT FAIL:\n  "
        + "\n  ".join(offenders)
        + "\n\nWithout the flag the script prints its findings and exits 0, and a "
        "caller that reads the exit code sees a pass. Add the flag, or make the "
        "script fail by default and delete the flag."
    )
