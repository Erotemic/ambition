"""Every world object the smash ruleset spawns says what ends it.

⛔⛔ THE DEFECT THIS PREVENTS SHIPPED ONCE ALREADY, and a player found it. Jon,
2026-09-05: *"a mine laid in a match still persists into the next match… Ending a
match should be cleaning everything up."* Five spawn sites, none of them saying
what ended the object, and each ending only by its own rule — a fuse, a trigger, a
lifetime. A match ending was not among them.

⭐ THE FIX WAS ONE OWNER (`MatchScoped`, stamped at spawn and swept by whoever
owns the match), AND THIS IS WHAT KEEPS IT ONE. The failure mode of that fix is
not that it breaks; it is that the SIXTH technique somebody authors spawns
something and nobody remembers to stamp it, which reproduces the original bug for
one object while every existing test stays green.

⚠ A SOURCE CHECK, AND HONEST ABOUT WHY. Whether an entity is match-scoped cannot
be asked of a running world without knowing which entities were supposed to be:
that is the very question the marker exists to answer. So this asks the call
sites instead — the same place the lifecycle vocabulary already lives, per
`SpawnScopedExt`'s "lifecycle policy part of the CALL SITE".

⇒ EXEMPTIONS ARE NAMED, not inferred. A spawn that genuinely outlives a match —
the select screen's UI, say — says so here once.
"""

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
RULESET = REPO / "game/ambition_demo_smash/src"

SPAWN = re.compile(r"\.spawn\w*\(")
STAMP = "match_scope::stamp"

# file -> why its spawns are not match-scoped. One line each, and each must say
# what DOES end the object.
EXEMPT = {
    "select_screen.rs": (
        "the character-select UI, which exists BETWEEN matches and is torn down "
        "with the screen; a match-scoped select screen would delete itself"
    ),
    "match_scope.rs": "the sweep itself spawns nothing; it only despawns",
}


def _is_test_module(path):
    """A file that IS a test module, included with `mod tests;`.

    ⛔ The `#[cfg(test)]` scan below cannot see these: the attribute sits on the
    `mod` line in the PARENT file, so the whole of `tests.rs` reads as production
    to a per-file scanner. Caught on this guard's first run, which reported six
    fixture spawns as unstamped ruleset objects.
    """
    return path.name == "tests.rs" or path.stem.endswith("_tests")


def _production_spawn_lines(path):
    """Spawn sites before the file's test module. A fixture spawning a bare
    component is not a ruleset creating a world object."""
    lines = path.read_text().splitlines()
    out = []
    for number, line in enumerate(lines, start=1):
        if line.startswith("#[cfg(test)]"):
            break
        code = line.split("//", 1)[0]
        if SPAWN.search(code):
            out.append((number, line.strip()))
    return out


def test_every_spawn_in_the_ruleset_says_what_ends_it():
    unstamped = []
    checked = 0
    stamped_files = 0
    for path in sorted(RULESET.glob("*.rs")):
        if _is_test_module(path):
            continue
        sites = _production_spawn_lines(path)
        if not sites:
            continue
        checked += 1
        if path.name in EXEMPT:
            continue
        body = path.read_text()
        if STAMP in body:
            stamped_files += 1
            continue
        for number, text in sites:
            unstamped.append(f"{path.name}:{number}: {text}")

    # ⛔⛔ ANTI-VACUITY FIRST, AND THIS ORDER WAS EARNED. Placed after the main
    # assertion, this arm was UNREACHABLE: renaming the stamp helper makes every
    # file fail the `STAMP in body` test, so the list below fills up and fires
    # before anything asks whether the helper still exists. The generic failure
    # then hides the specific one — "five sites are unstamped" when the truth is
    # "the thing this guard looks for was renamed". Found by poisoning, not by
    # reading, which is how the same defect turned up twice in one day.
    assert checked >= 5, (
        f"only {checked} file(s) in the ruleset spawn anything; this guard has "
        "lost its corpus"
    )
    assert stamped_files >= 1, (
        f"no file calls `{STAMP}`. Five sites were stamped when the match-scoped "
        "lifetime landed (bomb, bolt, mine, portal, spring), so ZERO means the "
        "helper was renamed or removed and this guard is now checking for a "
        "string nothing uses — every spawn below is reported unstamped for that "
        "reason rather than for its own."
    )

    assert not unstamped, (
        "these spawns do not say what ends them:\n  "
        + "\n  ".join(unstamped)
        + "\n⇒ A world object a match creates must be stamped with "
        "`crate::match_scope::stamp(...)`, so the match owns its end. Otherwise "
        "it outlives the match exactly as the mine did — and it will be found by "
        "a player rather than by this suite. If the object genuinely outlives a "
        "match, add it to EXEMPT in this file with the reason and say what DOES "
        "end it."
    )
