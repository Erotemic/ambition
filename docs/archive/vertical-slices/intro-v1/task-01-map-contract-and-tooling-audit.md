# Task 01: Map contract and tooling audit

## Purpose

Prepare the Act 1 intro rebuild so later agents can edit the map safely and consistently. This task should not build the world. It establishes the room contract, verifies the authoring path, inspects existing entities, and leaves a clear implementation map for tasks 02-09.

## Files to read first

- `AGENTS.md`
- `dev/vertical-slices/intro-v1/scaffold.md`
- `docs/concepts/llm-spatial-authoring-discipline.md`
- `tools/ambition_ldtk_tools/README.md`
- `docs/planning/story-gameplay-progression-draft.md`
- `docs/planning/gameplay-idea-index.md`

## Files likely to change

- `dev/vertical-slices/intro-v1/map-contract.md`


## Implementation goal

Create `dev/vertical-slices/intro-v1/map-contract.md`. This is the compact,
repo-grounded implementation contract for later agents. It should not duplicate
the full scaffold. It should answer what exists today, what is owned by this
vertical slice, what can be edited safely, and what validation commands work.

## Required contents for map-contract.md

Include these sections:

```text
1. Current intro room list with dimensions and world coordinates.
2. Current loading-zone graph.
3. Current important entity types present in each room.
4. Current area specs and ownership status.
5. Intended intro-v1 topology.
6. Coordinate/size constraints for taller or new rooms.
7. Cartography/map-layer implementation status and likely hooks.
8. Validation commands attempted and results.
9. Known tooling blockers/gotchas.
10. Task 02 handoff.
```

## Current topology target to document

The contract should ground this intended topology in the current repo:

```text
intro_wake_room
  -> intro_raid_corridor
    -> intro_escape_shaft
      -> drain_alley / Drain Market Main
        -> under_town_pipes / private route
        -> gate_stack_lower / right utility switchback
        -> forest tease / future route
        -> sky tease / future route
```

It should also name the later extension topology:

```text
under_town_pipes -> alice_relay -> bob_relay -> alice_relay return
Alice/Bob loop -> private map marks / Bob field survey / private route unlock
right_utility_switchback -> combat_calibration_lab -> first_system_boss -> return shortcut
```

## Cartography/map-layer audit

In addition to room inspection, identify whether the current codebase already has
usable hooks for map reveal, map markers, quest flags, conditional dialogue,
conditional labels, or route state. Do not implement them in this task unless a
trivial existing hook is obvious. Record likely implementation surfaces for later
Task 04 and Task 08 work.

Recommended facts to mention if no system exists yet:

```text
map_basic_unlocked
map_private_marks_unlocked
bob_field_survey_received
route_memory_received
MAP_PRIVATE / MAP_OFFICIAL / MAP_DANGER / MAP_SECRET / MAP_WATCHED labels
```

## Inspection steps

Use small scripts to inspect `intro.ldtk`; do not scroll through the raw JSON by
hand. For each level, record level identifier, size, world position, entity type
summary, player starts, loading zones, and any scripted or debug labels that
look relevant.

Record every current LoadingZone with:

```text
source level
zone id/name
activation mode if present
target room
target zone
bidirectional flag
notes about whether it is a main route, dev escape hatch, or placeholder
```

List the entity types that already appear in the intro world. Look specifically
for whether these are available: `PlayerStart`, `LoadingZone`, `Solid`,
`OneWayPlatform`, `CameraZone`, `DebugLabel`, `NpcSpawn`, `Prop`, `Switch`,
`LockWall`, `Chest`, `Pickup`, `Encounter`, `EnemySpawn`, `DamageVolume`,
`MovingPlatform`, and `WaterVolume`. Use the exact identifiers from the file if
they differ.

## Area spec ownership decision

Document these expected ownership choices, correcting them if inspection proves
otherwise:

```text
intro_wake_room_area.yaml: keep mostly stable.
intro_raid_corridor_area.yaml: tune, but do not overbuild.
intro_escape_shaft_area.yaml: rewrite as vertical ascent.
drain_alley_area.yaml: expand into Drain Market knot.
gate_stack_lower_area.yaml: reinterpret as right utility switchback.
pirate_sky_arena: future promise, not required for intro-v1 main route.
```

## Coordinate and tool questions to answer

Before Task 02 edits the shaft, answer:

```text
Can intro_escape_shaft become 768x1408 or 1024x1536 in place?
Do downstream rooms need to move in world coordinates?
Do EdgeExit zones assume horizontal right-edge movement?
Do camera zones or spawn zones assume a fixed 512px room height?
Does the LDtk tool support adding/replacing levels in intro.ldtk cleanly?
Does doctor/repair/validate work on intro.ldtk today?
```

## Non-goals

Do not rebuild rooms. Do not rename story concepts globally. Do not create the
Alice/Bob route. Do not implement progression systems. Do not add boss logic.
This task is about making later map edits safe.

## Acceptance criteria

The task is complete when `map-contract.md` exists, contains the current room
graph and entity vocabulary, names the intended room-function reinterpretations,
lists validation attempts and results, identifies tooling blockers, and ends with
a `Task 02 handoff` section.



## Validation baseline

For LDtk edits, run from the repository root:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk
```

If the area spec changed, use the relevant `area create --dry-run` command first
when the current tool supports it, then apply the edit through the LDtk tooling
and inspect the diff. If the current tooling only documents `sandbox.ldtk`, adapt
the command to `intro.ldtk` for this slice and document any mismatch.

When code or dialogue changes are made, also run the narrowest relevant checks:

```bash
cargo fmt --check
cargo test -p ambition_gameplay_core --lib
cargo run -p ambition_gameplay_core --bin headless
```

If a command fails for a known pre-existing reason, record the exact command and
the short error summary in the task handoff instead of hiding the failure.



## Required handoff note

End the task with a short handoff note in the changed doc, commit message, or a
new note beside the task. Include:

```text
what changed
what was validated
what remains placeholder
what felt fun or unreadable
which room/route should be worked on next
```
