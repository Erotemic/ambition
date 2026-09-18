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
takes the population to 52 — measured 2026-09-16, and 50 after the
test-module stripper stopped reading three fixtures as producers.

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
import functools
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
#
# ⛔⛤ THREE ROWS LEFT ON 2026-09-17 AND THEY HAD ALL BEEN ADJUDICATING A FIXTURE.
# `CausalRecording`, `SimPhaseCensus` and `SlotControls` each had an `Update`
# side that consisted ENTIRELY of a test module calling `app.add_systems(Update,
# <a sim system>)`, which the shared `strip_test_modules` could not see because
# the module was spelled `#[cfg(all(test, feature = "causal"))]`,
# `#[cfg(all(test, not(target_arch = "wasm32")))]` or `#[cfg(all(test, feature =
# "input"))]` rather than a bare `#[cfg(test)]`. Measured, per type:
#
#   CausalRecording  update side was `a_game_publishes_something`
#                    (`platformer2d/src/lib.rs:817`, in `mod causal_sdk_tests`)
#   SimPhaseCensus   `open_sim_phase_window` registered into BOTH sides from one
#                    file — the Update half inside `runtime_census.rs`'s tests
#   SlotControls     `publish_seat_controls_when_nobody_else_does`, registered
#                    into `Update` at four sites inside `mod focus_gate_tests`
#                    and once in `portal/plugin.rs`'s test module, and into the
#                    sim from `player_schedule.rs` — the production road
#
# ⇒ `SlotControls`'s row is the one to learn from: its argument ("the Update
# writer stands down via `another_authority_publishes`") described a REAL
# mechanism, so it read as a considered verdict. The crossing it was adjudicating
# was still a fixture. A reason that is true is not evidence that its subject
# exists.
CROSSING_IS_HARMLESS: dict[str, str] = {
    # Presentation and developer instruments: the sim writes, something outside
    # draws or records. Nothing reads them back as authority.
    "ActiveUiCues": "presentation cues, republished from sim state every tick",
    "ActorTraceBuffer": "dev trace ring, never read as authority",
    "CameraShakeState": "presentation; the sim requests, the camera decays it",
    "DeveloperRuntimeState": "dev tools",
    "GameplayTraceBuffer": "dev trace ring",
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
    # ⭐ A FIFTH SPELLING, AND THE SOURCE ALREADY SAYS SO. `CausalPlugin` registers
    # `stamp_causal_frame` into `bevy::app::First` ("provides a host frame stamp");
    # `player_schedule.rs` registers the SAME function again into `sim`, with its
    # own comment: "the simulation schedule stamps again when replay state is
    # available. The writes are idempotent." Read the body to check that claim
    # rather than take it: it is a pure setter derived from fresh `Res` reads each
    # call (tick, `replaying_history`, session generation) plus a per-registration
    # `Local<RollbackEpoch>` that does not leak between the two registrations —
    # nothing it writes depends on a PRIOR write to `CausalRecording` from either
    # side. Sim runs after `First` in frame order, so its stamp is simply the last
    # write of the frame; `First`'s write is a value the sim call immediately
    # overwrites with an equal-or-refined one, never a value read as authority in
    # between.
    "CausalRecording": "`stamp_causal_frame` re-registered into `sim` after `First`; both calls are a pure derived setter, confirmed idempotent by reading the body",
    # ⭐ A SIXTH SPELLING, AND BOTH SIDES SAY IT OUT LOUD. `publish_frontend_context_prompt`
    # (Update) and `rebuild_control_prompt` (sim) each carry a doc comment
    # asserting "one writer per frame by construction": the Update side writes
    # only when a non-gameplay context (menu/launcher) owns input and returns
    # otherwise; the sim side's own opening comment says it yields on exactly
    # those frames. Verified rather than trusted: when the Update side does
    # write, it sets `entries: Vec::new()` (an empty prompt) — so the third
    # writer, `project_prompt_readiness` (sim, `.after(rebuild_control_prompt)`),
    # which mutates `entry.ready` in a `for entry in &mut prompt.entries` loop,
    # iterates zero times on exactly those frames. No writer ever reads or
    # refines a value one of the others owns.
    "ControlPrompt": "`publish_frontend_context_prompt`/`rebuild_control_prompt` are a documented one-writer-per-frame handoff; `project_prompt_readiness` refines an empty entry list on the frames the other side owns",
    # Third of the three c215d6a37 exposed. `SimPhaseCensus` holds only an
    # `Instant`, per-phase `f64` totals and a tick count — a profiling
    # accumulator, structurally identical to `ActorTraceBuffer` /
    # `GameplayTraceBuffer` above. `open_sim_phase_window` (sim, every tick)
    # marks a phase-timing window open; `report_sim_phase_census` (`Last`,
    # once per visible frame) reports the average and resets. A rollback
    # resimulation batch can call the sim-side opener more than once per
    # visible frame, which skews a TIMING AVERAGE across replayed attempts —
    # a profiling-quality concern, not a gameplay-determinism one: nothing
    # reads this resource back as sim authority.
    "SimPhaseCensus": "profiling accumulator (Instant + per-phase f64 totals), never read back as sim authority",
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

# ⛔ FILED, NOT UNEXAMINED. A row here crosses the boundary in the defect's own
# shape AND has a question in front of it, so it prints as owed rather than as a
# new finding. This census's stated job is to say *how many resources that ruling
# is responsible for*; a bucket called UNCLASSIFIED could not report that number,
# because the filed rows and the unlooked-at ones were the same pile.
#
# ⚠ It is NOT a waiver: the count of filed rows is the number a ruling has to
# cover, and a filed row leaving the crossing set is checked below the same way a
# stale harmless row is.
FILED: dict[str, str] = {
    "CutsceneAdvanceRequest": (
        "Q136 — a dismiss raised on the host side does nothing; the producer is "
        "outside the timeline and the consumer inside it"
    ),
    # ✅ `SpawnPlayerCloneRequest` was filed here on 2026-09-18 and is GONE
    # because the question was ANSWERED and the code changed, which is the only
    # sanctioned way for a filed row to leave: the spawn moved out of the
    # simulation schedule onto the mechanical-edit road, so the resource is no
    # longer written on both sides of the boundary at all. Q136's first landed
    # road. `CutsceneAdvanceRequest` below is what the ruling still owes.
}


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


# ⭐ MEMOISED, AND THE COST IS THE REASON. This re-reads every production source
# and re-runs the writer sweep; its own test file calls it once per arm. The tree
# does not change inside one process — a one-shot CLI, a pytest module — so the
# second call is the same answer at no price.
#
# ⛔⛤ **IT DOES NOTHING WITHOUT THE TEST FILE'S OWN CACHE, AND THAT IS MEASURED
# RATHER THAN ARGUED.** A fresh `importlib` module per arm threw this cache away
# every time. Three readings of `pytest scripts/tests/test_resources_crossing_the_rewind_boundary.py`:
# neither cache **271 s**, this cache alone **271 s** (the module is rebuilt, so
# it never gets a second call), the test's `load()` cache alone **260 s** (one
# module, but the sweep still re-runs per arm), **both 117 s**. Two changes that
# are each a no-op on their own and 2.3× together.
#
# ⚠ It returns a MUTABLE dict; callers read it and must not edit it, which is
# what every caller does today.
@functools.lru_cache(maxsize=1)
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
            # ⛔⛤ THE HOST SIDE IS ASKED, NOT ENUMERATED, SINCE 2026-09-18. This
            # used to test `schedule in GUARD.NON_REWINDING`, a tuple of BARE
            # labels, while the tree spells a non-rewinding schedule in QUALIFIED
            # form 39 times — so `bevy::app::PreUpdate` fell through to `continue`
            # and the write was counted on NEITHER side. A crossing needs both
            # sides to be seen, so a missed host write reads as "sim only": the
            # same failure direction the guard itself had, one step further
            # downstream. Measured across the repair: 50 crossings before, and the
            # rows it adds are host writes that were previously invisible.
            if GUARD.is_non_rewinding(schedule):
                side = "update"
            elif GUARD.normalize_schedule(schedule) in SIM_LABELS:
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
    per_frame = [name for name in candidates if not only_at_a_session_edge(found[name])]
    filed = sorted(name for name in per_frame if name in FILED)
    unclassified = sorted(name for name in per_frame if name not in FILED)
    stale = sorted(name for name in CROSSING_IS_HARMLESS if name not in found)
    stale_filed = sorted(name for name in FILED if name not in found)

    print(f"`Resource` types written on BOTH sides of the rewind boundary: {len(found)}")
    print(f"  rollback-registered: {sum(1 for n in found if n in registered)}")
    print(f"  adjudicated harmless: {len(CROSSING_IS_HARMLESS) - len(stale)}")
    print(f"  crossing ONLY at a session edge: {len(session_edge_only)}")
    for name in session_edge_only:
        print(f"    {name} — `Update` side is session teardown only")
    print(f"  FILED, awaiting a ruling: {len(filed)}")
    for name in filed:
        print(f"    {name} — {FILED[name]}")
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

    if stale_filed:
        print(
            "\n⛔ FILED names types that no longer cross the boundary: "
            f"{stale_filed}. Either the question was answered and the code changed "
            "— delete the row and the question — or the sweep lost sight of it."
        )
        return 1
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
