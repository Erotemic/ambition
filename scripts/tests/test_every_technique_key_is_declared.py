"""⛔⛔ A HAND-KEPT LIST OF TECHNIQUE KEYS IS THE SHAPE A11b ALREADY DELETED TWICE.

`installed_techniques_are_declared.rs` asks the BUILT APP whether each technique
key is declared, which is the right question and the only one that can prove a
declaration survived composition. But its list of keys is written by hand, and
this repository has been bitten by exactly that: A11b replaced two hand-kept
effect-site lists that each read two of the four real sites, one of them a census
whose own doc recorded having already missed a site once.

⭐ SO THIS IS THE COMPLETENESS HALF, AND ONLY THAT. It does not ask whether a key
is declared — the Rust guard does, against the real app. It asks whether the
Rust guard's list still names every technique key that EXISTS. A new
`smash_*.rs` module with a new key is then a red test naming the key, rather
than a silent hole in a guard that keeps passing.

⚠ SCOPED BY WHERE TECHNIQUE KEYS LIVE, not by an exception list: the game's are
`pub const`s in `crates/ambition_characters/src/smash_*.rs`, and the ENGINE's
live in that crate's `technique.rs`, whose own doc calls itself "the authored
schemas of engine techniques". `SMASH_SELECT_EXPERIENCE = "smash.select"` is
also a `smash.` string, and it is a SHELL ROUTE ID declared in the demo's
`lib.rs` — excluded by the scope rather than by naming it, so a future route id
does not have to be added here.

⛔ THE ENGINE HALF WAS MISSING FROM THE FIRST VERSION OF THIS FILE, and that is
exactly the hole it exists to close: `pogo_bounce` is an engine technique, it was
authored by 36 characters, nothing declared it, and a guard scoped to `smash_*`
could never have said so. Found by walking the prepared corpus instead — see
`authored_effects_are_admitted.rs`.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
KEY_HOME = REPO / "crates" / "ambition_characters" / "src"
GUARD = REPO / "game" / "ambition_app" / "tests" / "installed_techniques_are_declared.rs"

#: ⛔ THE PATTERN IS PER-MODULE, and widening it to one shared regex was wrong.
#: A game's technique key is NAMESPACED (`smash.sleep`); the engine's is a bare
#: word (`pogo_bounce`). Accepting bare words everywhere pulled in
#: `SHARK_CLASS = "shark"` from `smash_ride.rs` — a character class id sitting
#: beside the technique key, which is a real const that is really not a
#: technique. Requiring the namespace where the namespace is the convention
#: excludes it by the rule rather than by an exception list.
GAME_CONST = re.compile(r'pub const ([A-Z][A-Z_0-9]*): &str = "(smash\.[a-z_0-9]+)"')
ENGINE_CONST = re.compile(r'pub const ([A-Z][A-Z_0-9]*): &str = "([a-z_0-9]+)"')


def _defined_keys() -> dict[str, str]:
    """{CONST_NAME: "smash.key"} for every technique key module."""
    found: dict[str, str] = {}
    for path in sorted(KEY_HOME.glob("smash_*.rs")):
        for name, value in GAME_CONST.findall(path.read_text(encoding="utf-8")):
            found[name] = value
    engine = KEY_HOME / "technique.rs"
    if engine.exists():
        for name, value in ENGINE_CONST.findall(engine.read_text(encoding="utf-8")):
            found[name] = value
    return found


def test_the_technique_key_home_is_not_empty() -> None:
    """⭐ THE PREMISE. An empty scan makes every assertion below vacuous — the
    failure mode where a file move silently turns a guard into a no-op."""
    defined = _defined_keys()
    assert len(defined) >= 20, (
        f"only {len(defined)} technique key(s) found under {KEY_HOME}. "
        "Either the keys moved, in which case this guard is now measuring "
        "nothing, or the scan is wrong. Both are defects in this file."
    )


def test_the_app_guard_names_every_technique_key_that_exists() -> None:
    defined = _defined_keys()
    listed = GUARD.read_text(encoding="utf-8")
    missing = sorted(
        f"{name} ({value})"
        for name, value in defined.items()
        # The guard names each key by its CONSTANT path, e.g.
        # `ambition_platformer2d::characters::smash_sleep::SLEEP`.
        if not re.search(rf"::{re.escape(name)}\b", listed)
    )
    assert not missing, (
        "these technique keys exist and the app-level declaration guard does not "
        f"name them, so nothing checks that the shipped composition installs a "
        f"handler for them:\n    " + "\n    ".join(missing) + "\n"
        f"Add them to `{GUARD.relative_to(REPO)}`. If one is genuinely not a "
        "technique, it does not belong in `smash_*.rs` beside the ones that are."
    )
