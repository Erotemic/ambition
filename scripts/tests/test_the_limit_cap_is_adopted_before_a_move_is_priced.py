"""A meter that arrives FULL must be emptied before anything prices a move.

⛔⛔ THIS GUARD EXISTS BECAUSE DYING GRANTED THE LIMIT. A stock loss keeps the
body entity and calls `reset_body_clusters`, which assigns
`BodyMana::default()` — a 100-point pool that starts FULL, because
`ResourceMeter::new` sets `current` to `max`. That runs in `CombatSet::Settle`.
The Limit's emptying lived only in `fill_limit_meters`, registered in
`CombatSet::ContentFlavor`, which runs AFTER `CombatSet::Trigger` where a move's
cost is checked. ⇒ On the frame after a respawn the meter read 100/100 against a
Limit priced at the match's cap, and respawn protection permits a swing.

⭐ THE UNIT TEST PROVES THE SYSTEM; THIS PROVES THE PLACEMENT.
`a_meter_that_arrives_as_a_full_mana_pool_is_not_spendable_when_a_move_is_priced`
builds its own three-system schedule, so it is green no matter where the ruleset
actually registers the adoption. The bug WAS the registration, and a test that
constructs its own order cannot see it.

⚠ TEXTUAL, AND THAT IS HONEST HERE: the fact being guarded IS a line of
registration source. What it cannot see is a `.before(...)` that Bevy resolves
differently than it reads — for that, the end is `sim_phase_pins`, which pins
placement in the assembled app.
"""

from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
RULESET = REPO / "game/ambition_demo_smash/src/lib.rs"
LIMIT = REPO / "game/ambition_demo_smash/src/limit.rs"
ADOPT = "adopt_the_limit_cap"


def test_the_adoption_is_registered_before_the_phase_that_prices_moves():
    ruleset = RULESET.read_text()

    # ⛔ ANTI-VACUITY FIRST, and in the order that survives a rename: if the
    # system is gone, every pattern below fails to match and the guard would
    # report a placement problem for a system that no longer exists.
    assert f"pub fn {ADOPT}(" in LIMIT.read_text(), (
        f"`{ADOPT}` is gone from the ruleset's limit module — this guard is "
        "checking where a system runs that no longer exists. If the adoption "
        "moved, point this at its new home; do not delete it."
    )
    assert f"crate::limit::{ADOPT}" in ruleset, (
        f"`{ADOPT}` exists but the ruleset never registers it, so nothing "
        "empties a meter that arrives as a full mana pool"
    )

    # The registration and what it is ordered against, comments stripped so the
    # prose explaining the rule cannot satisfy it.
    code = "\n".join(line.split("//", 1)[0] for line in ruleset.splitlines())
    match = re.search(
        rf"crate::limit::{ADOPT}\s*\.(before|in_set)\(([^)]*)\)", code, re.S
    )
    assert match is not None, (
        f"`{ADOPT}` is registered with no ordering at all. Bevy is then free to "
        "run it after `CombatSet::Trigger`, which is the frame a respawned "
        "fighter spends a Limit they did not earn."
    )
    how, against = match.group(1), match.group(2)
    assert how == "before" and "CombatSet::Trigger" in against, (
        f"`{ADOPT}` is ordered `{how}({against.strip()})`. It must be "
        "`.before(CombatSet::Trigger)`: Trigger is where a move's cost is "
        "priced, and a meter reset in `Settle` is still wearing its 100-point "
        "mana-pool shape until this runs."
    )
