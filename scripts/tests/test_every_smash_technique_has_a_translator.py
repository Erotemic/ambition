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
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO / "scripts"))

from lib.rust_sources import file_is_test_only, is_test_path  # noqa: E402
# Where the authored vocabulary is DECLARED.
VOCABULARY = REPO / "crates" / "ambition_entity_catalog" / "src"
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
    return is_test_path(path)


#: ⛔⛤ **A FLOOR ON THE HAYSTACK, AND THIS COPY'S RISK RUNS THE OTHER WAY FROM
#: ITS SIBLINGS.** `_is_test_file` is one of FIVE copies of "is this file
#: test-only" in `scripts/`, drifted into five different answers; this one is the
#: narrowest (no `*_tests.rs`, no `test.rs`). For the other copies a WIDENED
#: exclusion is the silent danger, because they report cleaner when they see
#: less. Here it is loud: a smaller haystack produces MORE orphans, so
#: over-exclusion fails the test rather than hiding a defect.
#:
#: ⚠ The silent direction here is the opposite one — an exclusion too NARROW lets
#: a test file into the haystack, and then a const named only by a test reads as
#: a connected technique. That is the recorded defect in `_is_test_file`'s own
#: docstring. ⇒ So when the five copies are consolidated onto one owner, this
#: call site must not LOSE exclusions, and the floor below cannot see that. It
#: is here to make the consolidation reviewable, not to make it safe.
#:
#: MEASURED 2026-09-16: 282 ruleset files scanned, 86 excluded as tests. Then
#: the consolidation onto `lib.rust_sources` GAINED exclusions rather than
#: losing them — `*_tests.rs` and `test.rs` by name, plus whole-file
#: `#![cfg(test)]` — and the reading fell to 257 kept of 368, 109 excluded by
#: name and 2 by the attribute. That is this guard's loud direction, and the
#: floor is lowered to match a haystack that is correctly smaller.
RULESET_FILE_FLOOR = 245


def _ruleset_text() -> str:
    parts: list[str] = []
    for root in RULESETS:
        for path in sorted(root.rglob("*.rs")):
            if _is_test_file(path):
                continue
            text = path.read_text(encoding="utf-8")
            if file_is_test_only(text):
                continue
            parts.append(text)
    assert len(parts) >= RULESET_FILE_FLOOR, (
        f"this guard now reads only {len(parts)} ruleset file(s), floor is "
        f"{RULESET_FILE_FLOOR}. Something narrowed its reach — most likely the "
        "test-file exclusion — and a verdict over this corpus would be a claim "
        "about the scan rather than about the rulesets"
    )
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
