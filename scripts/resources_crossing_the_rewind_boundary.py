#!/usr/bin/env python3
"""Which resources are mutated on BOTH sides of the rewind boundary?

⛔⛤ **THIS IS A DIFFERENT QUESTION FROM `check_rollback_mutators_run_in_sim.py`,
AND THE DIFFERENCE IS WHY THREE MEASURED DEFECTS WERE INVISIBLE TO IT.** That
guard asks *"is a ROLLBACK-REGISTERED type mutated from a schedule that never
rewinds"*, so its population is the canonical registry and a type carrying no
registration at all cannot appear in it. This one asks the complementary
question: is any `Resource` written by a system registered into a literal
`Update`/`PreUpdate`/`PostUpdate`/`FixedUpdate` **and** by a system registered
into `app.sim_schedule()`?

That shape is how a request gets lost. Three instances are measured, each with an
in-sim control arm, in
`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`:

    NewGameResetRequested    menu press -> 0 commits (in-sim control: 1)
    OwnedItems               Update grant -> never lands (in-sim control: 3 -> 4)
    CutsceneAdvanceRequest   host dismiss -> beat stays (in-sim control: advances)

⇒ All three are the same defect: a producer outside the timeline, a consumer
inside it. `docs/planning/awaiting-maintainer-decision.md`'s Q136 is the ruling
that has to cover them, and **this census exists to say how many resources that
ruling is responsible for** rather than to gate anything.

# # It REPORTS. It is not a gate, and the axis is not sharp enough to be one.

⚠ Crossing the boundary is NECESSARY for the defect and nowhere near sufficient.
A presentation resource legitimately written by the sim and read-modified by a
renderer crosses it too and is perfectly correct. What distinguishes the three
above is DESTRUCTIVE CONSUMPTION inside the sim — `mem::take`, a drain, a bool
reset to false — which is what makes the outside write unrecoverable rather than
merely late. Detecting that statically is not attempted here; it is named so the
next reader knows which residue matters.

⭐⛤ **AND TRIAGING THIS CENSUS FOUND FOUR SHIPPED WAYS TO CROSS THE BOUNDARY
SAFELY, WHICH IS A MORE USEFUL RESULT THAN THE ROW COUNT.** Each was read in the
body, not taken from a comment:

```text
the UPDATE writer stands down   SlotControls, via another_authority_publishes(latches, rollback)
the SIM writer stands down      LoadCoordinator / RoomTransitionLoadState, via
                                `if simulation_host.is_rollback() { return; }`
the SIM consumer stands down    SlotControlLatches, via `if replay.replaying_history`
                                -- and a replayed tick is fed from GGRS's stored input
the READ is routed              SeatRawFrames, via `seat_frame_this_tick` choosing the
                                authoritative table for this host
```

⇒ `CutsceneAdvanceRequest` uses **none** of them, which is why it is the whole
residue. The fix has four precedents to choose from rather than needing a new
mechanism — and the third row carries the invariant: destructive consumption
inside the sim is safe when the intent ALSO rides a channel the replay can
re-read.

⛔ THE FIRST VERSION OF THIS SWEEP REPORTED 67 TYPES AND THE NOISE WAS
STRUCTURAL: `App`, `Commands`, `NextState`, `Anchor` and `Sprite` are not
resources at all, they are ubiquitous parameters that the mutable-param regex
matches. Requiring `#[derive(..Resource..)]` removes them by construction and
takes the population to 52.

⚠ Every parsing rule here is IMPORTED from the sibling guard rather than
rewritten — paren-balanced `add_systems` bodies, `#[cfg(test)]` stripping,
`run_if`/`after`/`before` not counting as registrations, and `SystemParam`
bundles resolved transitively so one identifier can stand for 25 `ResMut`
fields. Each of those cost that guard a wrong answer first.

Usage:
    python3 scripts/resources_crossing_the_rewind_boundary.py
    python3 scripts/resources_crossing_the_rewind_boundary.py --all
"""

from __future__ import annotations

import argparse
import collections
import importlib.util
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# The sibling guard owns the parsing. Loading it by path keeps ONE copy of the
# rules that were paid for three times over.
_SPEC = importlib.util.spec_from_file_location(
    "rollback_mutator_guard", REPO / "scripts/check_rollback_mutators_run_in_sim.py"
)
GUARD = importlib.util.module_from_spec(_SPEC)
sys.modules["rollback_mutator_guard"] = GUARD
_SPEC.loader.exec_module(GUARD)

# The idiomatic spellings of the rewinding schedule at an `add_systems` call
# site. `app.sim_schedule()` is usually bound to a local first.
SIM_LABELS = frozenset({"sim", "sim_schedule", "label"})

RESOURCE_STRUCT = re.compile(
    r"#\[derive\([^)]*\bResource\b[^)]*\)\]\s*"
    r"(?:#\[[^\]]*\]\s*)*"
    r"(?:pub(?:\([^)]*\))?\s+)?struct\s+(\w+)"
)

# ⭐ ADJUDICATED, WITH THE ARGUMENT — not a waiver list. A row here asserts that
# crossing the boundary is HARMLESS for this type, and says why. A row that stops
# crossing is reported, the way the per-attempt census reports a stale entry,
# because a name kept here after its subject changed is worse than no list.
CROSSING_IS_HARMLESS: dict[str, str] = {
    # Presentation and developer instruments: the sim writes, something outside
    # draws or records. Nothing reads them back as authority.
    "ActiveUiCues": "presentation cues, republished from sim state every tick",
    "ActorTraceBuffer": "dev trace ring, never read as authority",
    "CameraShakeState": "presentation; the sim requests, the camera decays it",
    "CausalRecording": "dev recording buffer",
    "DeveloperRuntimeState": "dev tools",
    "GameplayTraceBuffer": "dev trace ring",
    "SimPhaseCensus": "instrument that counts phase execution",
    "RelativisticOpticalView2d": "derived presentation view",
    "RelativisticTargetingView2d": "derived presentation view",
    "RelativityClockView2d": "derived presentation view",
    "RelativitySignalView2d": "derived presentation view",
    "WorldlineHistoryView2d": "derived presentation view",
    # Already adjudicated by CUTSCENE-ROLLBACK-DECISION in docs/planning/queue.md.
    "BossEncounterRegistry": "authored read-only catalog behind a specs_loaded latch",
    "CutsceneSkipHold": "input-local skip accumulator the sim never reads",
    # Capture binaries, not the game: `game/*/src/bin/capture_*.rs` drive a
    # scripted run to take screenshots.
    "Warmup": "capture-binary shutter countdown (src/bin/capture_*.rs)",
    # ⭐ THE CROSSING IS HANDLED EXPLICITLY, and this is the precedent Q136 wants.
    # `publish_seat_controls_when_nobody_else_does` opens with
    # `another_authority_publishes(latches, rollback)` and RETURNS: under a
    # rollback host the sim-side publisher owns the value and the `Update`
    # fallback stands down. That is what a sanctioned boundary crossing looks
    # like -- the outside writer asks whether it is still the authority.
    "SlotControls": "Update writer stands down under rollback via `another_authority_publishes`",
    # ⭐ THE MIRROR OF THE `SlotControls` ARGUMENT: here the SIM-side writer is the
    # one that stands down. `commit_ready_room_transition_system` opens with
    # `if simulation_host.is_rollback() { return; }` -- verified in the body, not
    # taken from its param comment -- because a rollback-host room change must go
    # through `commit_confirmed_lifecycle`'s rebase instead. So under rollback
    # these two are written only from `Update`, and there is no crossing left.
    "LoadCoordinator": "sim-side writer returns early under `SimulationHost::Rollback`",
    "RoomTransitionLoadState": "sim-side writer returns early under `SimulationHost::Rollback`",
    # ⭐ A THIRD SPELLING OF THE SAME ARGUMENT, and the one closest to the defect.
    # `publish_latched_slot_controls` DESTRUCTIVELY consumes the latches
    # (`latches.take(slot)`) inside the sim -- the exact shape that loses a menu
    # press -- but it opens with
    # `if replay.is_some_and(|replay| replay.replaying_history) { return; }`, so
    # the take happens only on a live tick and a replayed tick is fed from GGRS's
    # stored input instead. ⇒ Destructive consumption inside the sim is safe when
    # the consumer knows it is replaying. That is the property `CutsceneAdvanceRequest`
    # lacks.
    "SlotControlLatches": "sim consumer returns early while `replaying_history`",
    # ⭐ THE FOURTH SPELLING, and the only one where nobody stands down: the READ
    # is routed instead. `seat_frame_this_tick` is
    # `if another_authority_publishes(latches, rollback) { slots.get(slot) } else
    # { raw.get(slot) }`, so under a rollback host the authoritative table is
    # `SlotControls` and the raw row is written only to be folded into the encoded
    # rollback input. Its own doc: *"Writing the table that is not authoritative
    # is harmless -- it is overwritten by the authority that owns it."*
    "SeatRawFrames": "host-aware read predicate picks the authoritative table (`seat_frame_this_tick`)",
}


# ⭐ A SESSION BOUNDARY IS NOT A PER-FRAME PRODUCER, and conflating them buried
# the rows that matter. `reset_session_scoped_resources_on_activation` and
# `..._on_retire` write nearly every session-scoped resource from `Update`, once,
# at a session edge — which is a decision `teardown.rs` already argues at length
# (*"the write is at a point no rewind crosses"*). A type whose ONLY `Update`
# writer is one of these is reported separately: it crosses the boundary, but not
# in the shape that loses a player's press.
SESSION_EDGE_WRITERS = frozenset({
    "reset_session_scoped_resources_on_activation",
    "reset_session_scoped_resources_on_retire",
})


def resource_types() -> set[str]:
    found: set[str] = set()
    for _path, text in GUARD._production_sources():
        found.update(RESOURCE_STRUCT.findall(text))
    return found


def mutable_types_by_system() -> dict[str, set[str]]:
    """system name → every type its signature borrows mutably.

    ⚠ NOT intersected with the rollback registry, which is the whole point: the
    sibling guard intersects, and that is precisely what hides an unregistered
    request.
    """
    bundles = GUARD.system_param_mutables()
    out: dict[str, set[str]] = {}
    for _path, text in GUARD._production_sources():
        for match in GUARD._PUB_FN.finditer(text):
            params = GUARD._params(text, match.end())
            mutated = set(GUARD._MUTABLE_PARAM_TYPE.findall(params))
            for identifier in re.findall(r"\b([A-Z][A-Za-z_0-9]*)\b", params):
                mutated |= bundles.get(identifier, frozenset())
            if mutated:
                out.setdefault(match.group(1), set()).update(mutated)
    return out


def crossings() -> dict[str, dict[str, set[str]]]:
    """resource → {"update": {sites}, "sim": {sites}} for types written on both."""
    by_system = mutable_types_by_system()
    resources = resource_types()
    sides: dict[str, dict[str, set[str]]] = collections.defaultdict(
        lambda: {"update": set(), "sim": set()}
    )
    for path, text in GUARD._production_sources():
        for body in GUARD.add_systems_bodies(text):
            schedule, _, rest = body.partition(",")
            schedule = schedule.strip()
            if schedule in GUARD.NON_REWINDING:
                side = "update"
            elif schedule in SIM_LABELS:
                side = "sim"
            else:
                continue
            rest = GUARD.strip_run_conditions(rest)
            for name, types in by_system.items():
                if not re.search(rf"\b{name}\b", rest):
                    continue
                for candidate in types & resources:
                    sides[candidate][side].add(f"{name} ({path.name})")
    return {
        name: side
        for name, side in sides.items()
        if side["update"] and side["sim"]
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--all", action="store_true", help="also list the adjudicated-harmless rows"
    )
    args = parser.parse_args()

    found = crossings()
    registered = GUARD.rollback_types()
    def only_at_a_session_edge(sides: dict[str, set[str]]) -> bool:
        return all(
            site.split(" (")[0] in SESSION_EDGE_WRITERS for site in sides["update"]
        )

    candidates = [
        name
        for name in found
        if name not in registered and name not in CROSSING_IS_HARMLESS
    ]
    session_edge_only = sorted(
        name for name in candidates if only_at_a_session_edge(found[name])
    )
    unclassified = sorted(
        name for name in candidates if not only_at_a_session_edge(found[name])
    )
    stale = sorted(name for name in CROSSING_IS_HARMLESS if name not in found)

    print(f"`Resource` types written on BOTH sides of the rewind boundary: {len(found)}")
    print(f"  rollback-registered: {sum(1 for n in found if n in registered)}")
    print(f"  adjudicated harmless: {len(CROSSING_IS_HARMLESS) - len(stale)}")
    print(f"  crossing ONLY at a session edge: {len(session_edge_only)}")
    for name in session_edge_only:
        print(f"    {name} — `Update` side is session teardown only")
    print(f"  UNCLASSIFIED with a per-frame `Update` writer: {len(unclassified)}")
    for name in unclassified:
        sides = found[name]
        print(f"    {name}")
        print(f"      update: {', '.join(sorted(sides['update'])[:3])}")
        print(f"      sim   : {', '.join(sorted(sides['sim'])[:3])}")
    if args.all:
        print("\n  adjudicated harmless (see CROSSING_IS_HARMLESS for each argument):")
        for name, why in sorted(CROSSING_IS_HARMLESS.items()):
            if name in found:
                print(f"    {name} — {why}")

    if stale:
        print(
            "\n⛔ CROSSING_IS_HARMLESS names types that no longer cross the "
            f"boundary: {stale}"
        )
        print(
            "  Check WHICH happened before deleting the entry: the type stopped "
            "being written on one side (delete it), or the sweep lost sight of it "
            "(fix the sweep). A stale entry silently absorbs the next type to "
            "take its place."
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
