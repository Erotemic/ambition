"""Every world object the smash ruleset spawns says what ends it.

⛔⛔ THE DEFECT THIS PREVENTS SHIPPED ONCE ALREADY, and a player found it. Jon,
2026-09-05: *"a mine laid in a match still persists into the next match… Ending a
match should be cleaning everything up."* Five spawn sites, none of them saying
what ended the object, and each ending only by its own rule — a fuse, a trigger, a
lifetime. A match ending was not among them.

⭐ THE FIX WAS ONE OWNER (`MatchScoped`, stamped at spawn with
`ambition_match::stamp_match_object` and swept by the engine), AND THIS IS WHAT
KEEPS IT ONE. The failure mode of that fix is
not that it breaks; it is that the SIXTH technique somebody authors spawns
something and nobody remembers to stamp it, which reproduces the original bug for
one object while every existing test stays green.

⚠ A SOURCE CHECK, AND HONEST ABOUT WHY. Whether an entity is match-scoped cannot
be asked of a running world without knowing which entities were supposed to be:
that is the very question the marker exists to answer. So this asks the call
sites instead — the same place the lifecycle vocabulary already lives, per
`SpawnSessionScopedExt`, which makes the owner an argument of the spawn.

⇒ EXEMPTIONS ARE NAMED, not inferred. A spawn that genuinely outlives a match —
the select screen's UI, say — says so here once.
"""

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
RULESET = REPO / "game/ambition_demo_smash/src"
# The ruleset's moves that place things live here as modules, and the world
# spawns what they request; Smash stamps those entities by their kind.
MODULES = REPO / "game/ambition_demo_smash_modules/src"

SPAWN = re.compile(r"\.spawn\w*\(")
STAMP = "stamp_match_object"

# file -> why its spawns are not match-scoped. One line each, and each must say
# what DOES end the object.
EXEMPT = {
    "select_screen.rs": (
        "the character-select UI, which exists BETWEEN matches and is torn down "
        "with the screen; a match-scoped select screen would delete itself"
    ),
}


def _is_test_module(path):
    """A file that IS a test module, included with `mod tests;`.

    ⛔ The `#[cfg(test)]` scan below cannot see these: the attribute sits on the
    `mod` line in the PARENT file, so the whole of `tests.rs` reads as production
    to a per-file scanner. Caught on this guard's first run, which reported six
    fixture spawns as unstamped ruleset objects.
    """
    return path.name == "tests.rs" or path.stem.endswith("_tests")


# ⛔⛔ EVERY ROAD TO A MATCH OBJECT, NOT JUST THE ONE SPELLING — and this is a
# repair, not a flourish. `checked` counted files containing `.spawn(`, and the
# ruleset has SIX of them against a floor of FIVE. ⇒ One consolidation and this
# guard reddens on an IMPROVEMENT: move two spawns behind a helper and the
# corpus check fails while the invariant it guards is better held than before.
# A peer hit exactly this on `check_reload_resets_also_answer_replay.py`, which
# floors on adopters of a type and counted PARAMETER declarations — three
# adopters stopped declaring one because a generic began asking on their behalf,
# and the floor failed on the change that SPREAD the type.
#
# ⇒ A file creates match objects if it spawns, OR stamps, OR names the marker.
# Add the next road here rather than lowering the number: the floor is protecting
# against the corpus quietly emptying, and that is still worth catching.
ROADS = (
    "a `.spawn(` site",
    f"a `{STAMP}` call",
    "a mention of `MatchScoped`",
    "a module building a `ModuleEntitySpawn` or `ModuleItemSpawn`",
)

# ⭐ A MODULE'S SPAWN IS STAMPED BY ITS KIND, NOT AT ITS CALL SITE. The world
# spawns what a module requests, so the module cannot stamp it; Smash stamps
# every module entity whose kind starts with a prefix
# (`stamp_smash_module_entities`). ⇒ What the next module can get wrong is the
# KIND: a spawn named outside that prefix is never the match's, and outlives it
# exactly as the mine did. The prefix is read from the stamp itself, so the
# check and the owner cannot drift apart.
MODULE_SPAWN = re.compile(r"\b(ModuleEntitySpawn|ModuleItemSpawn)\s*\{")
MODULE_KIND = re.compile(r"\bkind:\s*([A-Z_][A-Z0-9_]*)\.into\(\)")
STAMP_PREFIX = re.compile(r'module_entity\.kind\.starts_with\("([^"]+)"\)')


def _creates_match_objects(path):
    """Whether this file is on ANY road to creating a match-scoped object."""
    text = path.read_text()
    return bool(_production_spawn_lines(path)) or STAMP in text or "MatchScoped" in text


def _module_spawn_kinds(path):
    """For each module spawn built before the file's test module: its shape,
    the constant its `kind:` names, and that constant's value (None when the
    kind is not a constant of the file)."""
    text = path.read_text().split("\n#[cfg(test)]", 1)[0]
    kinds = []
    for found in MODULE_SPAWN.finditer(text):
        tail = text[found.end():]
        named = MODULE_KIND.search(tail[: tail.find("}")])
        if not named:
            kinds.append((found.group(1), None, None))
            continue
        const = re.search(rf'const {named.group(1)}: &str = "([^"]+)";', text)
        kinds.append((found.group(1), named.group(1), const.group(1) if const else None))
    return kinds


def _module_spawning_files():
    return [path for path in sorted(MODULES.glob("*.rs")) if _module_spawn_kinds(path)]


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
        if _creates_match_objects(path):
            checked += 1
        sites = _production_spawn_lines(path)
        if not sites:
            continue
        if path.name in EXEMPT:
            continue
        body = path.read_text()
        if STAMP in body:
            stamped_files += 1
            continue
        for number, text in sites:
            unstamped.append(f"{path.name}:{number}: {text}")
    checked += len(_module_spawning_files())

    # ⛔⛔ ANTI-VACUITY FIRST, AND THIS ORDER WAS EARNED. Placed after the main
    # assertion, this arm was UNREACHABLE: renaming the stamp helper makes every
    # file fail the `STAMP in body` test, so the list below fills up and fires
    # before anything asks whether the helper still exists. The generic failure
    # then hides the specific one — "five sites are unstamped" when the truth is
    # "the thing this guard looks for was renamed". Found by poisoning, not by
    # reading, which is how the same defect turned up twice in one day.
    assert checked >= 5, (
        f"only {checked} file(s) in the ruleset create match objects; this guard "
        "has lost its corpus.\n"
        "  The roads counted are: " + ", ".join(ROADS) + ".\n"
        "  ⇒ If spawning moved behind a HELPER, add that helper as a road here. "
        "Do NOT lower the number: the floor exists so an emptied corpus fails, "
        "and a floor that counts one spelling fails on the refactor that "
        "consolidates the others."
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
        "`ambition_platformer2d::versus_match::stamp_match_object(...)`, so the "
        "match owns its end. Otherwise "
        "it outlives the match exactly as the mine did — and it will be found by "
        "a player rather than by this suite. If the object genuinely outlives a "
        "match, add it to EXEMPT in this file with the reason and say what DOES "
        "end it."
    )


def test_every_module_spawn_is_named_into_the_stamp():
    prefixes = STAMP_PREFIX.findall((RULESET / "lib.rs").read_text())
    assert len(prefixes) == 1, (
        f"Smash's module-entity stamp tests {len(prefixes)} kind prefixes "
        "(`module_entity.kind.starts_with(..)` in lib.rs); this check reads the "
        "one it uses, so a renamed or split stamp must be re-read here, not "
        "assumed"
    )
    prefix = prefixes[0]
    spawning = _module_spawning_files()
    assert len(spawning) >= 3, (
        f"only {[path.name for path in spawning]} build a module spawn; spring, "
        "bomb and mine did when this check landed, so fewer means the spawn "
        "shapes were renamed and this check reads nothing"
    )
    wrong = [
        f"{path.name}: a {shape} of kind {name} = {value!r}"
        for path in spawning
        for shape, name, value in _module_spawn_kinds(path)
        if value is None or not value.startswith(prefix)
    ]
    assert not wrong, (
        f"these module spawns are not named into the match's stamp (kinds "
        f"starting {prefix!r}):\n  " + "\n  ".join(wrong) + "\n"
        "⇒ The world spawns them and Smash stamps them by kind, so a kind outside "
        "the prefix is never the match's and outlives it. Name the kind with a "
        "constant of the module's file, inside the prefix."
    )


def test_a_spawn_that_moves_behind_a_helper_does_not_empty_the_corpus(monkeypatch):
    """⛔⛔ THE FLOOR MUST NOT FAIL ON THE REFACTOR THAT IMPROVES THE THING.

    The ruleset has SIX files that spell `.spawn(` against a floor of FIVE, so
    consolidating two spawn sites behind a helper would have reddened this guard
    while the invariant it holds got BETTER. That is a real margin, not a
    hypothetical one: a peer hit the same shape on an adopter floor that counted
    parameter declarations and failed on the change that spread the type.

    ⇒ This simulates the consolidation by making every `.spawn(` invisible, and
    the corpus must survive on the other roads — the files still stamp, and they
    still name the marker.
    """
    import sys

    monkeypatch.setattr(
        sys.modules[__name__], "_production_spawn_lines", lambda _path: []
    )
    surviving = [
        path.name
        for path in sorted(RULESET.glob("*.rs"))
        if not _is_test_module(path) and _creates_match_objects(path)
    ] + [path.name for path in _module_spawning_files()]
    assert len(surviving) >= 5, (
        "with every spawn moved behind a helper the corpus fell to "
        f"{surviving} — the floor still counts one spelling, so it would fail "
        "on the consolidation instead of on an emptied ruleset"
    )
