#!/usr/bin/env python3
"""Who writes a request the SIMULATION consumes, and was that set reviewed?

A resource that is drained inside the rewinding schedule and written from
outside it loses the write on every rewind: the restore puts the resource back
and nothing re-produces the request. `OutstandingCheckpointRequest` carries that
sentence at its own registration — *"A REQUEST THAT OUTLIVES ITS FRAME MUST
REWIND WITH THE WORLD"* — and the cutscene pair has no equivalent.

⛔⛤ **THE SUBJECT IS A SET, NOT A SCHEDULE, AND SAYING SO IS THE WHOLE POINT.**
`CutsceneTriggerQueue` is correct TODAY for a reason nobody had written down:
every one of its producers happens to run inside the sim schedule, so a replay
re-produces whatever the rewind dropped. That is an invariant held by
coincidence. The moment a producer appears in `Update` — exactly where
`apply_menu_frame_to_cutscene_request` already sits — the queue becomes the
measured defect its sibling already is (`Q136`).

⚠ **THIS SCRIPT DOES NOT ATTRIBUTE SCHEDULES AND MUST NOT BE READ AS IF IT
DID.** `System::name()` is *"<Enable the debug feature to see the name>"* in this
workspace's build and a schedule's per-system access set is `pub(crate)` in Bevy
0.19, so neither a name filter nor an access query is available here. What this
checks is that the WRITER SET has not changed without a human writing down which
schedule the new writer runs in. Each entry's reason is that human's reading of
the body, dated; the ratchet is that a writer nobody adjudicated fails the lane.

⛔⛤ **AND THE LIMITATION ABOVE HAS A CONSEQUENCE THIS DOCSTRING USED TO LEAVE
FOR THE READER: MOVING AN EXISTING WRITER FROM THE SIM SCHEDULE INTO `Update`
LEAVES THIS GREEN.** The set is unchanged, so the ratchet sees nothing, while the
invariant it is named after is exactly what broke. ⇒ This is a REVIEW LEDGER, not
rollback coverage, and it should not be counted as the latter.

⛔⛔ **THE REPLACEMENT ARM THIS DOCSTRING PRESCRIBED WAS BUILT ON 2026-09-17 AND
DOES NOT HOLD THE PROPERTY. THE PRICE IS A MID-SESSION ROOM TRANSITION.** The
prescription was *"drive a cutscene trigger across a rewind in a sync-test world
and assert the cutscene still starts"*.
`an_authored_room_cutscene_starts_with_and_without_a_rewind`
(`game/ambition_app/tests/a_room_cutscene_starts_under_a_rewind.rs`) does exactly
that and is green — and stayed green under BOTH poisons: moving
`auto_trigger_room_cutscenes` into `Update`, and deleting the `cutscene.last_room`
registration. The reason is a layer neither poison touches: the room a world
BOOTS into fires its binding at the first tick, before any rewind window has
opened, and `ActiveCutscene` is itself rollback state, so once playing every
rewind restores it playing and the drain returns early. The queue's cross-frame
life in that fixture is one frame, at boot, outside the window.

⛔⛔ **AND THE REPLACEMENT THIS DOCSTRING THEN PRESCRIBED — "a room TRANSITION
mid-session, well inside the check distance" — WAS BUILT ON 2026-09-18 AND CANNOT
HOLD THE PROPERTY EITHER. THE PRESCRIPTION IS WITHDRAWN.**
`a_room_cutscene_taken_mid_session_starts_under_a_rewind` (same file) settles 40
frames in `central_hub_complex` and then crosses the authored door into
`cutscene_lab`. It stayed green under the `Update` poison, under that poison
TOGETHER with the deleted `cutscene.last_room` registration, and the cutscene
started on the first step after arrival with no delay to measure. ⇒ A ROOM
TRANSITION IS THE WRONG SUBJECT BY CONSTRUCTION: `detect_room_transition_system`
runs Track B under a rollback host, recording a `PendingLifecycleCommit` the host
commits only once the recording frame is CONFIRMED. A room change can never occur
on a speculative frame, so no trigger keyed on one can be driven across a rewind.
⇒ Whatever replaces this ratchet has to drive a producer that fires on a
SPECULATIVE frame — which is what `CutsceneAdvanceRequest` is, and why item 1 of
`CUTSCENE-ROLLBACK-DECISION` has a failing witness and this subject does not. Both
arms are kept for what they do pin — the binding resolving under a rollback
composition, at boot and across a mid-session crossing — and NEITHER is this
ratchet's replacement.

⛔⛤ **AND THIS DOCSTRING'S MECHANISM IS WRONG FOR ITS OWN FIRST SUBJECT.** *"The
restore puts the resource back and nothing re-produces the request"* is the
failure mode of a REGISTERED resource drained in-sim. Measured against
`game/ambition_app/tests/rollback_schema_baseline.txt`: `CutsceneTriggerQueue` is
**not registered at all** — only `cutscene.playback` (`ActiveCutscene`) and
`cutscene.last_room` (`LastCutsceneRoom`) are. A rewind therefore does not put
the queue back; it leaves both the write and the DRAIN where the rolled-forward
frames left them, which is a different hazard in the opposite direction: a
request drained on a frame that is then rolled back is simply gone, and only a
producer whose own edge state rewinds will re-make it. ⚠ The ratchet is still
worth having — a new writer is still worth a human reading — but it must not be
cited for the mechanism above. No arm replaces it today and none is prescribed;
taking a ratchet down is part of landing its replacement, not a separate cleanup.

⭐ **THE CLASS WAS ENUMERATED 2026-09-17, SO NOBODY PAYS FOR THAT MEASUREMENT
TWICE, AND `SUBJECTS` IS STILL ONE TYPE FOR A MEASURED REASON.** A guard written
around one hand-picked subject is usually a guard whose population nobody looked
for. Joining *"drained by `std::mem::take` / `.drain(..)` in production"* against
the multi-writer census gives **six resource types**:

    QuestRegistry           6 writers   registered
    DialogState             4 writers   NOT registered — Yarn presentation state
    CutsceneAdvanceRequest  2 writers   NOT registered — item 1 of CUTSCENE-ROLLBACK-DECISION, blocked on `Q136`
    CutsceneTriggerQueue    2 writers   NOT registered — this file's subject
    VersusMatch             2 writers   registered
    PendingPlayerHitEvents  1 writer    registered

⇒ Of the three unregistered ones, two are the cutscene pair already filed and
`DialogState` is the Yarn RUNNER's presentation mirror — line reveal, options,
speaker — consumed by `ambition_dialog`'s bridge to drive the runner, with the
authoritative half in the registered `ActiveConversation`. No new subject.

⚠ **AND THAT ENUMERATION IS A FLOOR.** The pattern reads `std::mem::take(&mut
*param)`, `take(&mut param.field)` and `param.drain(..)` where `param` is a
`ResMut<T>` in the same signature. `.pop()`, `.clear()`, `mem::replace`, an
`Option::take` on a field, and any drain reached through a `SystemParam` bundle
are all outside it. A type this pattern cannot see is not a type that is safe.

⚠ **TEST PATHS ARE COUNTED AND NOT RATCHETED.** A fixture that writes the queue
is how the in-sim control arms are built, and requiring the table to track test
churn would make the ratchet noisy for no gain. They are printed so a production
writer that has been moved into a test helper is visible rather than silently
dropped from the population.

    python3 scripts/check_sim_consumed_request_writers.py
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# type name -> (why the set matters, {"<path>::<owner>": "the reading, dated"})
SUBJECTS: dict[str, tuple[str, dict[str, str]]] = {
    "CutsceneTriggerQueue": (
        "drained by `drain_cutscene_triggers` inside the sim schedule's `Cutscene` "
        "phase, and it holds across frames while `ActiveCutscene` and "
        "`LastCutsceneRoom` rewind around it — so a writer outside the rewinding "
        "schedule loses its trigger on every rewind",
        {
            "crates/ambition_platformer2d_actor_monolith/src/cutscene.rs::auto_trigger_room_cutscenes": (
                "PRODUCER, in `Platformer2dSimulationPhaseMonolith::Cutscene` — in the "
                "sim schedule, so a replay re-produces it (read 2026-09-17)"
            ),
            "crates/ambition_platformer2d_actor_monolith/src/cutscene.rs::drain_cutscene_triggers": (
                "CONSUMER — the `std::mem::take`, in the sim schedule (read 2026-09-17)"
            ),
            "crates/ambition_boss_encounter/src/systems.rs::update_boss_encounters": (
                "PRODUCER, in `ProgressionSet::BossAdvance` — in the sim schedule "
                "(read 2026-09-17)"
            ),
            "crates/ambition_boss_encounter/src/events.rs::publish_events": (
                "PRODUCER, a helper `update_boss_encounters` calls, so it inherits that "
                "system's schedule and has no registration of its own (read 2026-09-17)"
            ),
            "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs::struct SessionScopedResources": (
                "SESSION EDGE, not a per-frame producer: teardown CLEARS the queue so "
                "one session's trigger cannot start in the next. A different boundary "
                "from the rollback one (read 2026-09-17)"
            ),
        },
    )
}

FN = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([a-z_][a-z0-9_]*)\s*[(<]")
STRUCT = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?struct\s+([A-Za-z_][A-Za-z0-9_]*)")


def mutable_write_pattern(type_name: str) -> re.Pattern[str]:
    """`ResMut<T>` in any qualification and lifetime, or a `&mut T` parameter.

    ⛔ THE LIFETIME IS PART OF THE SPELLING. A `SystemParam` struct writes
    `ResMut<'w, T>`, and a pattern without the optional lifetime misses exactly
    the shape that hides a writer inside a bundle of parameters — which is where
    this subject's session-edge writer lives.
    """
    qualified = r"(?:[A-Za-z0-9_]+::)*" + re.escape(type_name)
    return re.compile(
        rf"ResMut\s*<\s*(?:'[a-z_]+\s*,\s*)?{qualified}\s*>|&\s*mut\s*{qualified}\b"
    )


def owner_of(lines: list[str], index: int) -> str:
    """The `fn` or `struct` a line belongs to, whichever is nearer above it.

    ⛔ A `struct` ANSWER IS NOT A FALLBACK, IT IS THE RIGHT ANSWER for a
    `SystemParam` bundle: the write is declared on the struct and the systems
    that take it inherit it. Reporting `None` there, or crediting the previous
    `fn` in the file, would both name the wrong thing.
    """
    for i in range(index, -1, -1):
        fn = FN.match(lines[i])
        if fn:
            return fn.group(1)
        st = STRUCT.match(lines[i])
        if st:
            return f"struct {st.group(1)}"
    return "<file scope>"


def is_test_path(path: str) -> bool:
    return "/tests/" in path or path.endswith("tests.rs") or path.endswith("_tests.rs")


def tracked_rust_files() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "*.rs"], cwd=ROOT, capture_output=True, text=True, check=True
    )
    return out.stdout.split()


def main() -> int:
    files = tracked_rust_files()
    failures: list[str] = []
    for type_name, (why, table) in SUBJECTS.items():
        pattern = mutable_write_pattern(type_name)
        found: dict[str, str] = {}
        in_tests: list[str] = []
        hits = 0
        for rel in files:
            text = (ROOT / rel).read_text(encoding="utf-8")
            if type_name not in text:
                continue
            lines = text.splitlines()
            for i, line in enumerate(lines):
                # Prose about a type is not a write of it. A parameter list holds
                # no string literals, so cutting at `//` is safe here.
                if not pattern.search(line.split("//")[0]):
                    continue
                hits += 1
                key = f"{rel}::{owner_of(lines, i)}"
                if is_test_path(rel):
                    in_tests.append(f"{key}  (line {i + 1})")
                else:
                    found[key] = f"line {i + 1}"

        print(f"`{type_name}` — {why}")
        print(f"  {hits} mutable-write site(s); {len(found)} outside test paths, {len(in_tests)} in them")

        # ⛔ THE FLOOR. A regex that matches nothing reports an empty set, and an
        # empty set satisfies "every writer is adjudicated" vacuously.
        if not found:
            failures.append(
                f"`{type_name}`: ZERO mutable-write sites found outside test paths. "
                "Either the type was deleted — in which case delete its entry here — "
                "or the pattern no longer matches how a write is spelled, and this "
                "check has been passing over nothing."
            )
            continue

        unadjudicated = sorted(set(found) - set(table))
        if unadjudicated:
            failures.append(
                f"`{type_name}`: {len(unadjudicated)} writer(s) nobody adjudicated:\n    "
                + "\n    ".join(f"{k}  ({found[k]})" for k in unadjudicated)
                + "\n  ⇒ Read the body, decide WHICH SCHEDULE it runs in, and add it to "
                "`SUBJECTS` in this script with that reading. If it runs outside the "
                "rewinding schedule, it is a rollback defect and not a table entry: a "
                "request written from `Update` and drained in the sim is lost on every "
                "rewind (see `Q136`)."
            )
        stale = sorted(set(table) - set(found))
        if stale:
            failures.append(
                f"`{type_name}`: {len(stale)} adjudicated writer(s) no longer exist:\n    "
                + "\n    ".join(stale)
                + "\n  ⇒ A reason that outlives its code is a hole with a comment over "
                "it. Delete the entry in the same change that removed the writer."
            )
        for key in sorted(found):
            reason = table.get(key, "UNADJUDICATED")
            print(f"    {key}\n      {reason}")
        for row in sorted(in_tests):
            print(f"    (test) {row}")

    if failures:
        print("\n⛔ FAILED\n")
        for row in failures:
            print(f"  {row}\n")
        return 1
    print("\nok: every writer of a sim-consumed request is adjudicated")
    return 0


if __name__ == "__main__":
    sys.exit(main())
