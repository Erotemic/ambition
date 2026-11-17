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

    room-gated      the system asks `.allows(` of `RoomReplay`, or its
                    registration runs it only outside a replay
                    (`.run_if(not(resource_exists::<RoomReplay>))`)
    room-stamped    the system writes with a `RoomWrite` (or `from_room`) stamp
    retracted       a registered room-write retractor (`register_room_write_
                    retractor`) takes back what this write did: the type is one
                    of the retractor's own logs (`*InRooms`, `*SinceCheckpoint`),
                    or a store it restores that this writer also logs to, or a
                    store a tail logger diffs whole each tick (`TAIL_LOGGED`),
                    or a one-tick queue whose retractor drops the rewound
                    room's rows (`ONE_TICK_QUEUES`)
    host-rewound    the room rollback host (`ambition_hosted_world`'s
                    `room_rollback.rs`) saves and sets back its type around a
                    replay: the shared clock in `struct Clock(..)`, which a
                    rewind sets to the saved tick and the replay runs forward
                    again, and the seat inputs `rewind_room` copies out
                    before the replay and puts back after it
    derived         the system overwrites its own section of a store from
                    live state (`MIRRORS`), or
                    its type is rebuilt from live state each run, and the scan
                    CHECKS that: no binding of the type accumulates unless the
                    same system resets it or the resource-memory guard names the
                    reset-before-contribute ordering
    instrument      its type is an observation, trace, dev/demo tool or
                    presentation-only state that no gameplay system reads
    world-owned     its type is host state a rewind does not change, by a
                    ruling that quotes the type's own doc (`WORLD_OWNED`), or
                    the system applies an act from outside the simulation
                    that a replay does not read again (`OUTSIDE_ACTS`), or
                    it follows a world fact a replay does not change
                    (`WORLD_FACT_WRITERS`), or it runs once at the
                    session's start (`SESSION_START`)
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
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from scripts.checks import check_rollback_mutators_run_in_sim as sim  # noqa: E402
from scripts.checks import check_sim_schedule_resource_memory_is_adjudicated as memory  # noqa: E402

#: A store a retractor restores whose every write is captured, because one
#: system diffs the whole value into the retractor's log at the tick's tail.
TAIL_LOGGED: dict[str, str] = {
    "ActiveConversation": (
        "`log_conversation_changes_in_rooms` diffs it into `ConversationsInRooms` "
        "in `FeatureViewSync`; `retract_room_conversations` opens or closes it "
        "back (`ambition_conversation/src/rooms.rs`, read 2026-11-17)"
    ),
}

#: A queue that holds one tick's rows, each naming its room: the retractor drops
#: a rewound room's rows outright, since every one was written after the tick
#: the room was saved at, and the replay writes them again.
ONE_TICK_QUEUES: dict[str, str] = {
    "PendingPlayerHitEvents": (
        "staged at the end of one tick, drained by the next; "
        "`retract_room_pending_player_hits` drops the rows whose victim is in the "
        "rewound room, and `apply_player_hit_events` keeps the other rooms' rows "
        "through a replay (`ambition_damage/src/lib.rs`, read 2026-11-17)"
    ),
}

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
    # Each reading quotes the type's own doc (read 2026-11-17).
    "NavigationAdvice": "\"This tick's advice, by body. Written again each tick\" (`advise_navigation` clears it)",
    "RoomNavigation": "\"a derived cache, never rollback state\": each surface graph is built whole on first use",
    "PerceptionPeers": "\"Per-frame snapshot of EVERY live body's peer data\" (`collect_perception_peers` truncates it)",
    "ActorDecisionFacts": "the observation phase's per-frame hand-off (`observe_actor_decision_inputs` clears it)",
    "ActorSteering": "\"Per-frame steering context\", computed once by `observe_actor_decision_inputs`",
    "ActorDecisionFrames": "\"Frame-local output of the autonomous DECIDE phase\" (`tick_actor_brains` clears it)",
    "EncounterSwitchIndex": "\"Rebuilt each frame from `SwitchFeature` and the save's switch map\"",
    "AuthoredSwitchCommands": "\"derived, not rollback state\": the active room's prepared switch verbs",
    "AuthoredRoomCommands": "\"Derived from the room set and the catalog, so it is not rollback state\"",
    "ClearedEncounters": "one row per cleared occurrence, cleared and republished by `publish_cleared_encounters`",
    "RelativisticOpticalView2d": "\"rollback-derived simulation state\": one image per observer, cleared each publish",
    "PortalHostDepthsByRoom": "each portal's measured host depth by live room, cleared and remeasured by `sync_portal_host_depths`",
    "LdtkRuntimeSpineIndex": "\"Rebuilt every frame from plugin-spawned LDtk entities\"",
    "ClassBRemapLog": "\"The frame-scoped ledger of Class-B remaps. Cleared at the head of every sim frame\"",
    "GravityField": "resolved each tick from the base gravity and the zones by `resolve_active_gravity`",
    "ControlledSubject": "the entity primary local control drives, resolved each tick by `resolve_controlled_subject`",
    "WorldTime": "\"Per-simulation-step dt snapshot\", refreshed from the clock each step",
    "MapMenuState": (
        "synced from the save by `sync_map_from_save`; read by the menu, the "
        "feedback layer and the headless observation (read 2026-11-17)"
    ),
    "SimDt": "\"Mirror of the host's sim clock\", written each frame by `mirror_sim_dt_into_runtime`",
    "ResolvedSwitchActivations": (
        "\"REPLACED each tick, never appended\": `drain_switch_activations` clears "
        "it first and fills it from the drained queue (`ambition_encounter/src/switches.rs`)"
    ),
    "ControlFrame": (
        "the primary seat's mirror: both writers overwrite it whole from "
        "`SlotControls` (`mirror_primary_slot_to_control_frame`; `publish_ggrs_input`, "
        "\"Read the mirror from the table\"), which the room host sets back around a replay"
    ),
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
    # Presentation only: set in the simulation, read only by effects and
    # visuals, so a room rewind that replays a write repeats an effect at worst.
    "ShrineActivationPulse": "presentation only (the resource-memory guard's `tick_shrine_activation_pulse` reading)",
    "CutRopeBossArenaState": "presentation only (the resource-memory guard's `detect_cut_rope_rope_cut` reading)",
    "GameplayBanner": (
        "presentation only: read by the app's feedback layer and by its own "
        "ticker's run condition (`the_banner_is_up`); no simulation decision reads "
        "it (read 2026-11-17)"
    ),
}

#: A capture tool's own binary: its systems drive a scene for a screenshot and
#: run in no shipped composition, whatever gameplay type they write.
TOOL_SOURCES: dict[str, str] = {
    "game/ambition_app_tools/src/bin/capture_scene.rs": (
        "the scene capture tool (`[[bin]] capture_scene` of `ambition_app_tools`): "
        "it dials gates, turns switches and stands the player for a shot"
    ),
}

#: Host state that a rewind, whole or of one room, does not change.
WORLD_OWNED: dict[str, str] = {
    "LiveMatchTicks": (
        "a match's, written only while one is active; no room replays alone while a match is prepared or active (`register_room_replay_refusal`, `session/src/match_lifecycle.rs`, consulted by `take_arrived_inputs`, read 2026-11-17)"
    ),
    "StocksMatchSettled": (
        "a match's, written only while one is active; no room replays alone while a match is prepared or active (`register_room_replay_refusal`, `session/src/match_lifecycle.rs`, consulted by `take_arrived_inputs`, read 2026-11-17)"
    ),
    "SuddenDeathEntered": (
        "a match's, written only while one is active; no room replays alone while a match is prepared or active (`register_room_replay_refusal`, `session/src/match_lifecycle.rs`, consulted by `take_arrived_inputs`, read 2026-11-17)"
    ),
    "VersusMatch": (
        "the versus round's, written only while the versus route is played (`versus_stage_is_active`); no room replays alone then (`register_room_replay_refusal` in `compose_versus_experience`, read 2026-11-17)"
    ),
    "ActiveRoundScope": (
        "the versus round's, written only while the versus route is played (`versus_stage_is_active`); no room replays alone then (`register_room_replay_refusal` in `compose_versus_experience`, read 2026-11-17)"
    ),
    "SimulationReplayState": (
        "\"Host-owned marker for a historical replay pass\": the rollback host "
        "raises and clears it around a replay (`ambition_sim_schedule`, read 2026-11-17)"
    ),
    "PeerVerdicts": (
        "\"Host state, and never rewound: a confirmed input does not change\" "
        "(`ambition_platformer2d_rollback_ggrs/src/peer_input.rs`, read 2026-11-17)"
    ),
    "RoomTransitionContentEpoch": (
        "\"Monotonic identity for the App-local inputs that define room "
        "construction\": it compares the room set by value, so \"a restore "
        "reproduces them exactly and bumps nothing\" (`room_transition/loading.rs`, "
        "read 2026-11-17)"
    ),
    "ProjectileSeqCounter": (
        "an order, not an identity: \"a global counter is a perfectly good *total "
        "order*, which is all this uses it for. The identity itself comes from the "
        "owner's own `SimIdCounter`\" (`mint_spawned_sim_ids`), a component the "
        "room's snapshot restores. A replay fires its shots again with fresh, higher "
        "ids in the same order, and the host compares no checksum with a guest "
        "(read 2026-11-17)"
    ),
}

#: Systems that overwrite their own section of a shared store from live state
#: each run, so the section is a projection of its source and follows whatever
#: the source's own class does under a rewind. Keyed by SYSTEM: the store's
#: other writers are not mirrors.
MIRRORS: dict[str, str] = {
    "persist_inventory_to_save": (
        "\"Mirror the live inventory + wallet into the save whenever they differ\": "
        "the bag and the primary wallet (`ambition_platformer2d_items/src/persist.rs`)"
    ),
    "persist_minted_item_horizon_to_save": (
        "rebuilds the save's minted rows from the live mints and the occurrence "
        "ledger when either changed (`pickup/minted_horizon.rs`)"
    ),
    "persist_occurrence_horizon_to_save": (
        "rebuilds the save's occurrence rows from `AuthoredOccurrences` and the "
        "live custody (`session/src/durable_horizon.rs`)"
    ),
    "keep_life_ledger": (
        "\"Keep each living body's facts and plan hour in the ledger\": each row "
        "is overwritten from its body's `Life`, a component the room's snapshot "
        "restores; a replay sees only its own room's bodies (`body/life.rs`)"
    ),
    "record_placed_bodies": (
        "rewrites each resting body's `Placed` row from its live kinematics; a "
        "replay sees only its own room's bodies, which the room's snapshot "
        "restored (`construction/src/whereabouts.rs`)"
    ),
    "persist_module_records_to_save": (
        "\"Mirror the session's durable module records into the save when they "
        "differ from the saved rows\" (`runtime/src/module_save_road.rs`)"
    ),
}

#: Systems whose write follows a world fact a room replay does not change.
WORLD_FACT_WRITERS: dict[str, str] = {
    "track_room_visits": (
        "sets a room's visited flag while the room is live, and only on the edge; "
        "a room replay does not change which rooms are live (it hides the others "
        "and they were visited), and the flag is never cleared but by a load "
        "(`ambition_menu/src/map/systems.rs`, read 2026-11-17)"
    ),
}

#: Systems that run once at a session's start and then never again in it,
#: each behind its own flag: the durable restore (`SaveRestored`) or a
#: registry's `initialized`. The host saves a guest room only once the session
#: is live, so no room rewind reaches back across them.
SESSION_START: dict[str, str] = {
    "populate_quest_registry": "returns at once while `registry.initialized` (`ambition_content/src/quest.rs`)",
    "populate_boss_encounter_registry": "returns at once while `registry.specs_loaded` (`boss_encounter/src/systems.rs`)",
    "complete_durable_restore": "returns at once while `restored.0`; it is the one place that sets it (`durable_horizon.rs`)",
    "restore_inventory_from_save": "\"Apply the saved inventory + wallet to the live state once\" (`items/src/persist.rs`)",
    "restore_lives_from_save": "returns at once while `restored.0` (`actor_monolith/src/body/life.rs`)",
    "restore_checkpoint_on_session_start": "\"Runs once per session: `SessionStartupResume` remembers which generation it has resolved\" (`session/src/checkpoint.rs`)",
    "activate_the_prepared_match": "the match's activation, once per session; a whole-world rewind before it re-runs it from the borrowed plan (`session/src/match_activation.rs`)",
}

#: Systems that apply an act from outside the simulation: a live message
#: read through `ReplayMessageReader`, whose live cursor a room replay does not
#: advance, so a replay neither reads it again nor should take it back.
OUTSIDE_ACTS: dict[str, str] = {
    "apply_ambient_gravity_requests": (
        "the developer's gravity cycle (`AmbientGravityRequest` from the `\\` hotkey "
        "and the developer menu), read through `ReplayMessageReader` (read 2026-11-17)"
    ),
}

#: The unclassified bindings, a ratchet: lower it with the change that lowers
#: the count, never raise it.
UNCLASSIFIED_CEILING = 29

#: A floor, not a target: a scan that suddenly sees far less has broken.
POPULATION_FLOOR = 200


_RETRACTED_TYPE = re.compile(
    r"(?:resource_mut|get_resource_mut|session_world_component_mut)\s*::<\s*((?:\w+::)*[A-Z]\w*)\s*>"
    r"|Mut<\s*((?:\w+::)*[A-Z]\w*)\s*>"
)


@functools.cache
def _function_text(repo: Path, name: str) -> str:
    """The signature and body of every production `fn name`."""
    out = []
    for _src, text in sim._production_sources(repo):
        for match in re.finditer(rf"\bfn\s+{re.escape(name)}\s*[<(]", text):
            brace = text.find("{", match.end())
            if brace >= 0:
                out.append(text[match.start():brace] + sim._braced(text, brace))
    return "\n".join(out)


@functools.cache
def retracted_types(repo: Path = REPO) -> frozenset[str]:
    """The types a registered room-write retractor mutates."""
    found: set[str] = set()
    for _src, text in sim._production_sources(repo):
        for match in re.finditer(r"register_room_write_retractor\s*\(", text):
            depth, end = 0, match.end() - 1
            for end in range(match.end() - 1, len(text)):
                if text[end] == "(":
                    depth += 1
                elif text[end] == ")":
                    depth -= 1
                    if depth == 0:
                        break
            call = text[match.end():end]
            # A retractor passed by name: read that function too.
            named = re.search(r"((?:\w+::)*)(\w+)\s*,?\s*$", call)
            if named and "|" not in call:
                call += _function_text(repo, named.group(2))
            for hit in _RETRACTED_TYPE.finditer(call):
                ty = hit.group(1) or hit.group(2)
                found.add(ty.rsplit("::", 1)[-1])
    return frozenset(found)


#: Where the room rollback host names the shared clock it saves with each tick.
HOST_CLOCK = "crates/ambition_hosted_world/src/room_rollback.rs"


@functools.cache
def host_rewound_types(repo: Path = REPO) -> frozenset[str]:
    """The types the room rollback host saves and sets back around a replay."""
    path = repo / HOST_CLOCK
    if not path.exists():
        return frozenset()
    text = path.read_text(encoding="utf8")
    found: set[str] = set()
    match = re.search(r"struct\s+Clock\s*\(([^;]*)\)\s*;", text)
    if match:
        found |= {part.strip().rsplit("::", 1)[-1] for part in match.group(1).split(",") if part.strip()}
    # The seat inputs `rewind_room` copies out before the replay drives the
    # seats with their records, and puts back after it.
    body = re.search(r"fn\s+rewind_room\b.*?\n}\n", text, re.S)
    if body:
        found |= set(re.findall(r"get_resource::<(?:[\w:]+::)?(\w+)>\(\)\.copied\(\)", body.group(0)))
    return frozenset(found)


_REPLAY_GATED = re.compile(
    r"\b(\w+)\s*\.run_if\(\s*(?:[\w:]+::)?not\(\s*(?:[\w:]+::)?resource_exists::<\s*(?:[\w:]+::)?RoomReplay\s*,?\s*>"
)


@functools.cache
def replay_gated_systems(repo: Path = REPO) -> frozenset[str]:
    """Systems registered to run only outside a room replay."""
    return frozenset(
        match.group(1) for _src, text in sim._production_sources(repo) for match in _REPLAY_GATED.finditer(text)
    )


def _is_log(ty: str) -> bool:
    """A retractor's own log, as opposed to a store it restores."""
    return ty.endswith(("InRooms", "SinceCheckpoint"))


@functools.cache
def rows(repo: Path = REPO) -> tuple[tuple[str, str, str, str], ...]:
    """`(system, file, type, class)` per sim-schedule `ResMut` binding."""
    population = memory.memory.sim_schedule_systems(repo)
    found: list[tuple[str, str, str, str]] = []
    retracted = retracted_types(repo)
    host_rewound = host_rewound_types(repo)
    replay_gated = replay_gated_systems(repo)
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
                if ".allows(" in body or "RoomReplay" in params or name in replay_gated:
                    kind = "room-gated"
                elif "RoomWrite" in body or "from_room" in body:
                    kind = "room-stamped"
                elif short in retracted and (
                    _is_log(short)
                    or short in TAIL_LOGGED
                    or short in ONE_TICK_QUEUES
                    or any(_is_log(log) and re.search(rf"\b{log}\b", params + body) for log in retracted)
                ):
                    kind = "retracted"
                elif short in host_rewound:
                    kind = "host-rewound"
                elif short in DERIVED:
                    kind = "derived"
                elif short in INSTRUMENT:
                    kind = "instrument"
                elif short in WORLD_OWNED or (name in OUTSIDE_ACTS and "ReplayMessageReader" in params):
                    kind = "world-owned"
                elif rel in TOOL_SOURCES:
                    kind = "instrument"
                elif name in MIRRORS:
                    kind = "derived"
                elif name in WORLD_FACT_WRITERS or name in SESSION_START:
                    kind = "world-owned"
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
        + ", ".join(f"{counts.get(k, 0)} {k}" for k in ("room-gated", "room-stamped", "retracted", "host-rewound", "derived", "instrument", "world-owned", "unclassified"))
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
    stale = sorted((set(DERIVED) | set(INSTRUMENT) | set(TAIL_LOGGED) | set(ONE_TICK_QUEUES) | set(WORLD_OWNED)) - {ty for _n, _f, ty, _k in found})
    for ty in stale:
        print(f"⛔ a ruling for `{ty}`, which no sim-schedule system writes any more: delete it")
        failed = True
    for name in sorted(set(OUTSIDE_ACTS) - {n for n, _r, _t, k in found if k == "world-owned"}):
        print(f"⛔ an outside-act ruling for `{name}`, which no longer reads its act live: delete it")
        failed = True
    for name in sorted((set(MIRRORS) | set(WORLD_FACT_WRITERS) | set(SESSION_START)) - {n for n, _r, _t, _k in found}):
        print(f"⛔ a system ruling for `{name}`, which writes no resource in the sim schedule any more: delete it")
        failed = True
    for rel in sorted(set(TOOL_SOURCES) - {rel for _n, rel, _t, _k in found}):
        print(f"⛔ a tool ruling for `{rel}`, which holds no sim-schedule writer any more: delete it")
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
