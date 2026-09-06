"""Every authored `smash.*` technique key must be recognised by a ruleset.

⛔⛔ THE FAILURE THIS EXISTS FOR IS SILENT AND I HIT IT TWICE IN ONE DAY. A
technique is authored on a move as an `EffectRef` key; a ruleset adapter matches
that key and turns it into a typed request. If the adapter arm is missing, the
key falls through `_ => continue` — which is DELIBERATE, so that other rulesets'
techniques pass by untouched — and the move plays its animation, spends its
recovery, and does nothing at all. Nothing warns. No test fails.

⭐ The two that happened, both on 2026-09-05:
  · `smash.capture_carry` shipped with its adapter arm written, and the fixture
    that would have caught a missing one only caught it because a SEPARATE system
    failed parameter validation.
  · `close_on_transit` was a whole parameter nothing read — the same shape one
    level down, and it took a census to find.

⇒ So this checks the cheap half mechanically: a key that exists in the authoring
vocabulary must be NAMED somewhere in a ruleset crate. It cannot prove the arm is
correct; it can prove somebody wrote one.

⚠ WHAT IT DELIBERATELY DOES NOT DO. It does not require the key to be authored by
a fighter — a technique with no customer is a design question (see the campaign's
dormant-capability census), not a defect. This guard is only about the ENGINE side
of the road being connected.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
# Where the authored vocabulary is DECLARED.
VOCABULARY = REPO / "crates" / "ambition_characters" / "src"
# Where a ruleset may recognise it. A key named in any of these is connected.
RULESETS = (
    REPO / "game" / "ambition_demo_smash" / "src",
    REPO / "crates" / "ambition_combat" / "src",
    REPO / "crates" / "ambition_platformer2d_actor_monolith" / "src",
)

KEY = re.compile(r'pub const ([A-Z_]+): &str = "(smash\.[a-z_]+)"')


def _declared() -> dict[str, str]:
    """Every `smash.*` key, mapped to the file that declares it."""
    found: dict[str, str] = {}
    for path in sorted(VOCABULARY.rglob("*.rs")):
        for _name, key in KEY.findall(path.read_text(encoding="utf-8")):
            found[key] = str(path.relative_to(REPO))
    return found


def _is_test_file(path: Path) -> bool:
    """A test naming the key does not connect it to anything.

    ⛔⛔ THE FIRST VERSION OF THIS GUARD PASSED ITS OWN POISON because of exactly
    this. Deleting the mine's adapter arm — replacing `PLACE_MINE` with a bare
    string literal — left the const still named in `mine/tests.rs`, so the
    haystack matched and the guard reported a connected technique with no
    production reader. ⇒ A guard whose evidence can come from a test is a guard
    that passes when the feature is dead, which is the only failure mode that
    matters here.
    """
    return path.name == "tests.rs" or "tests" in path.parts


def _ruleset_text() -> str:
    parts: list[str] = []
    for root in RULESETS:
        for path in sorted(root.rglob("*.rs")):
            if _is_test_file(path):
                continue
            parts.append(path.read_text(encoding="utf-8"))
    return "\n".join(parts)


def test_every_authored_technique_key_is_named_by_a_ruleset() -> None:
    declared = _declared()
    assert declared, (
        "no `smash.*` technique keys were found at all — the pattern this guard "
        "scans for has changed shape, and a guard that matches nothing passes "
        "forever"
    )
    haystack = _ruleset_text()
    # ⭐ MATCH ON THE CONST NAME, NOT THE STRING. Rulesets compare against the
    # imported constant (`key.as_str() != PLACE_MINE`), and nothing outside the
    # declaring crate should ever spell the string literal — so searching for the
    # literal would report every correctly-written adapter as missing.
    orphans = []
    for path in sorted(VOCABULARY.rglob("*.rs")):
        for name, key in KEY.findall(path.read_text(encoding="utf-8")):
            if name not in haystack:
                orphans.append(f"{key} (`{name}`, declared in {declared[key]})")
    assert not orphans, (
        "authored technique keys that no ruleset names, so a move using one "
        "plays its animation, spends its recovery and does nothing:\n  "
        + "\n  ".join(orphans)
    )
