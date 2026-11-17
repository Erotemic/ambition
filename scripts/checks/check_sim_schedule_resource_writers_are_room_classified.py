#!/usr/bin/env python3
"""Which global `Resource` writers in the rewinding schedule ignore room ownership?

Room-local simulation rewinds one live room at a time, but a `Resource` is one
value for the whole App. The room replay contract (`docs/systems/room-replay.md`)
says a writer of room-owned state asks `RoomReplay::allows(owner)`, or writes a
shared store with a room write stamp (`RoomWrite`) that a room's rewind retracts.
Nothing found a writer that did neither, so each one so far was found by a
cross-room desync (Jon, 2026-11-16: room-local semantics were adopted faster than
global resources were partitioned).

This is the census the island-rollback plan asked for. Each `ResMut<T>` binding of
a sim-schedule system (the population `check_sim_schedule_resource_memory_is_
adjudicated.py` scans) is one of:

    room-gated      the system asks `.allows(` of `RoomReplay`
    room-stamped    the system writes with a `RoomWrite` (or `from_room`) stamp
    derived         its type is rebuilt from live state each run, and the scan
                    CHECKS that: no binding of the type accumulates unless the
                    same system resets it or the resource-memory guard names the
                    reset-before-contribute ordering
    instrument      its type is an observation, trace or dev/demo tool that no
                    gameplay system reads
    unclassified    none of the above

The unclassified count is a ratchet: it may only fall, and the ceiling must be
lowered in the change that lowers it. Zero is the goal the plan states.
"""

from __future__ import annotations

import sys as _sys
from pathlib import Path as _Path
_sys.path.insert(0, str(_Path(__file__).resolve().parents[2]))
_sys.path.insert(0, str(_Path(__file__).resolve().parents[1]))

import functools
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from scripts.checks import check_rollback_mutators_run_in_sim as sim  # noqa: E402
from scripts.checks import check_sim_schedule_resource_memory_is_adjudicated as memory  # noqa: E402

#: A type rebuilt from live state on each run of its writers. The reading names
#: the writer; the scan checks it does not accumulate.
DERIVED: dict[str, str] = {
    # The read models of `ambition_sim_view`: each is rebuilt by its own
    # `rebuild_*`/`publish_*` system from the entities of this tick.
    **{
        ty: "a sim_view read model, rebuilt each run from this tick's entities (read 2026-11-17)"
        for ty in (
            "ActorAnimIndex", "ActorRenderIndex", "BlinkPreviewFact", "BodyClocksView",
            "BossFrameIndex", "BossRenderIndex", "CombatGeometryView", "ControlPrompt",
            "ControlledBodiesView", "DynamicFeatureViews", "FeatureViewIndex",
            "GroundItemsView", "GuardBreaksView", "HeldItemView", "HostileWieldedItemsView",
            "LaunchedBodiesView", "MarkBeaconsView", "NameplateIndex", "NearestInteractable",
            "PassageViews", "PlayerHudFacts", "ShieldRingsView", "ShrinesView",
            "WorldItemsView", "WornGearView",
        )
    },
    **{
        ty: "a relativity read model, published each run from this tick's worldlines (read 2026-11-17)"
        for ty in (
            "RelativisticTargetingView2d", "RelativityClockView2d",
            "RelativitySignalView2d", "WorldlineHistoryView2d",
        )
    },
    "BodyGeometry": "published before and after motion from this tick's bodies and portals (read 2026-11-17)",
    "ForceZones": "`collect_force_zones` gathers this tick's zone entities (read 2026-11-17)",
    "GravityZones": "`collect_gravity_zones` gathers this tick's zone entities (read 2026-11-17)",
    "PortalCarves": "`publish_portal_carves` publishes this tick's portal openings (read 2026-11-17)",
    "PerceptionProjectiles": "`collect_perception_projectiles` gathers this tick's projectiles (read 2026-11-17)",
    "BodyContactSnapshot": "`snapshot_body_contact` snapshots this tick's contacts (read 2026-11-17)",
}

#: A type no gameplay system reads: an observation census, a trace, or a tool
#: outside the shipped compositions.
INSTRUMENT: dict[str, str] = {
    "BodyIdentityCensus": "an observation census (see the resource-memory guard's reading)",
    "UnmintedBodyCensus": "an observation census (see the resource-memory guard's reading)",
    "BrainActionCounter": "an observation census with no reader (resource-memory guard)",
    "LdtkRuntimeSpineStats": "an observation census with no reader (resource-memory guard)",
    "SimPhaseCensus": "`ambition_dev_tools`' phase census",
    "RollbackExecutionStats": "counts rollback runs; observing the rewind is its purpose",
    "RollbackProofState": "the dev rollback proof pulse; observing the rewind is its purpose",
    "GameplayTraceBuffer": "the gameplay trace recorder",
    "ActorTraceBuffer": "the actor trace recorder",
    "InputStreamRecorder": "records the input stream for replay files",
    "Forced": "a Smash capture probe (`ambition_demo_smash_app` tools)",
    "StandAt": "a Sanic capture tool (`ambition_demo_sanic_app`)",
    "Warmup": "a Mary-O capture tool (`ambition_demo_mary_o_app`)",
}

#: The unclassified bindings, a ratchet: lower it with the change that lowers
#: the count, never raise it.
UNCLASSIFIED_CEILING = 147

#: A floor, not a target: a scan that suddenly sees far less has broken.
POPULATION_FLOOR = 200


@functools.cache
def rows(repo: Path = REPO) -> tuple[tuple[str, str, str, str], ...]:
    """`(system, file, type, class)` per sim-schedule `ResMut` binding."""
    population = memory.memory.sim_schedule_systems(repo)
    found: list[tuple[str, str, str, str]] = []
    for src, text in sim._production_sources(repo):
        for match in sim._PUB_FN.finditer(text):
            name = match.group(1)
            if name not in population:
                continue
            params = sim._params(text, match.end())
            binds = memory._RESMUT.findall(params)
            if not binds:
                continue
            brace = text.find("{", match.end() + len(params))
            if brace < 0:
                continue
            body = sim._braced(text, brace)
            rel = str(src.relative_to(repo))
            for _bind, ty in binds:
                short = ty.rsplit("::", 1)[-1]
                if ".allows(" in body or "RoomReplay" in params:
                    kind = "room-gated"
                elif "RoomWrite" in body or "from_room" in body:
                    kind = "room-stamped"
                elif short in DERIVED:
                    kind = "derived"
                elif short in INSTRUMENT:
                    kind = "instrument"
                else:
                    kind = "unclassified"
                found.append((name, rel, short, kind))
    return tuple(sorted(set(found)))


def derived_that_accumulate(repo: Path = REPO) -> list[tuple[str, str, str]]:
    """`(type, system, file)` for a `derived` ruling the scan contradicts."""
    # An accumulation the resource-memory guard has read (a generation stamp
    # swept each run, or a named reset-before-contribute ordering) is a rebuild.
    adjudicated = set(memory.SIBLING_RESET_ADJUDICATED)
    read = set(memory.ADJUDICATED)
    bad = []
    for name, rel, ty, hits, reset in memory._scan(repo):
        if ty in DERIVED and hits and not reset and ty not in adjudicated and name not in read:
            bad.append((ty, name, rel))
    return sorted(set(bad))


def main() -> int:
    found = rows()
    counts: dict[str, int] = {}
    for *_rest, kind in found:
        counts[kind] = counts.get(kind, 0) + 1
    unclassified = [row for row in found if row[3] == "unclassified"]
    print(
        f"{len(found)} `ResMut` binding(s) in the rewinding schedule: "
        + ", ".join(f"{counts.get(k, 0)} {k}" for k in ("room-gated", "room-stamped", "derived", "instrument", "unclassified"))
    )
    types = sorted({ty for _n, _f, ty, _k in unclassified})
    print(f"unclassified types ({len(types)}): {', '.join(types)}")
    failed = False
    if len(found) < POPULATION_FLOOR:
        print(f"⛔ the population shrank to {len(found)} (floor {POPULATION_FLOOR}): the scan broke")
        failed = True
    for ty, name, rel in derived_that_accumulate():
        print(f"⛔ `{ty}` is ruled derived, but `{name}` ({rel}) accumulates it without a reset")
        failed = True
    stale = sorted((set(DERIVED) | set(INSTRUMENT)) - {ty for _n, _f, ty, _k in found})
    for ty in stale:
        print(f"⛔ a ruling for `{ty}`, which no sim-schedule system writes any more: delete it")
        failed = True
    if len(unclassified) > UNCLASSIFIED_CEILING:
        print(
            f"⛔ {len(unclassified)} unclassified binding(s), over the ceiling of "
            f"{UNCLASSIFIED_CEILING}. A new global writer in the rewinding schedule asks "
            "`RoomReplay::allows`, writes with a `RoomWrite`, or gets a ruling here"
        )
        for name, rel, ty, _k in unclassified:
            print(f"    {ty}  via {name}  ({rel})")
        failed = True
    elif len(unclassified) < UNCLASSIFIED_CEILING:
        print(
            f"⛔ {len(unclassified)} unclassified binding(s), under the ceiling of "
            f"{UNCLASSIFIED_CEILING}: lower `UNCLASSIFIED_CEILING` to {len(unclassified)}"
        )
        failed = True
    if failed:
        return 1
    print(f"ok: {len(unclassified)} unclassified, at its ceiling")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
