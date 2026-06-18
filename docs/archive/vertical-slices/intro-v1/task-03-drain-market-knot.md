# Task 03: Drain Market knot

## Purpose

Rebuild `drain_alley` into the first small platforming town and route knot: a decompression room that is fun to traverse and visibly connects the intro to future Act 1 branches.

## Files to read first

- `dev/vertical-slices/intro-v1/scaffold.md`
- `dev/vertical-slices/intro-v1/map-contract.md`
- `tools/ambition_ldtk_tools/specs/drain_alley_area.yaml`
- `docs/concepts/llm-spatial-authoring-discipline.md`
- `docs/planning/gameplay-idea-index.md`

## Files likely to change

- `tools/ambition_ldtk_tools/specs/drain_alley_area.yaml`
- `crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk`


## Design target

Drain Market is not a large RPG town. It is a compact platforming knot. The
player should enjoy moving through it before any dialogue is final. It should
also contain the first readable Alice/Bob cartography mark, even if the player
does not yet know how to interpret it.

Target function:

```text
vertical_escape_shaft -> Drain Market Main
Drain Market Main -> under-town pipes / private cartography route
Drain Market Main -> right utility switchback
Drain Market Main -> forest/ninja tease
Drain Market Main -> sky/pirate tease
Drain Market Main -> return to escape shaft
```

Suggested room size is `1536 x 768` or `1600 x 768`. It should have three layers:
upper roofs/awnings, middle main street, and lower pipes.

## Layout sketch

```text
+----------------------------------------------------------------+
| sky chain / pulley / pirate crate                              |
| upper optional climb                                           |
|                                                                |
| awnings / roofs / high pickup / locked balcony                  |
|                                                                |
| forest gate / trees -- market street + Oiler -- utility exit    |
|                                                                |
| pipe grates / under-town entrance / Alice map mark                              |
|                                                                |
| old drain / blocked military-service hint                      |
+----------------------------------------------------------------+
```

## Required beats

### Arrival from the shaft

The player should arrive near a drain grate, broken pipe, or market edge. Put
Oiler nearby but do not block movement. Main street should be visible quickly.

### Oiler repair anchor

Oiler is the first warm social anchor. His mechanical function is `P1: Oiler
Stabilizer / Compact Calibration`. If a real item cannot be granted yet, place a
DebugLabel and optionally use an existing dialogue or switch pattern to set a
flag. The under-town route can remain open in test mode as long as it is labelled
as a story-mode P1 gate.

### Roof path and high pickup

Add an upper traversal route over the market street. Include one visible high
pickup or reward. Basic movement should almost reach it; future movement or a
skilled route should finish the access. Missed jumps should drop the player back
to town, not into death.

### Under-town entrance

Create a lower pipe/grate entrance leading to `under_town_pipes` or a labelled
placeholder. It must be visible from the main street, tied to Oiler/Stabilizer in
intended progression, and labelled as the main good/private route.

### Right utility exit

Preserve or replace the connection to `gate_stack_lower`, but treat it as the
right mechanical/system route rather than a fixed story concept. It is the
neutral and official-ish path into combat/system content.

### Future promises

Add a forest/ninja promise on the left or down-left and a sky/pirate promise
above. Use blocked gates, tree-line labels, falling leaves, sky chain, pulley,
cloud dock sign, contraband crate, or debug labels. Do not make either route
required.

### Military/service hint

Add one out-of-place sealed drain hatch, crate, or label below the market or near
the shaft return. Keep it small.

## Combat and branch hooks

No hostile combat on the main street. Optional lower-pipe skitter or hazard is
fine.

Label hooks for:

```text
Good/private: under-town route after Oiler.
Evil/lawful: right utility/official route.
Neutral: continue right without Alice/Bob.
Chaotic: roof sequence-break candidate.
Famous: public street/NPC path.
Private: roofs and pipes avoid the main street.
```

## Acceptance criteria

`drain_alley` functions as a layered Drain Market, with upper/middle/lower
traversal, Oiler or a repair stub, one high reward, under-town entrance, right
utility exit, visible forest and sky promises, no main-street combat lock-in,
and validated LDtk changes.



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

### Alice/Bob map mark

Place one subtle private cartography mark near the pipe entrance or roof/pipe
transition. It can be a DebugLabel if art is unavailable:

```text
MAP_PRIVATE: unread Alice/Bob route mark
```

The mark should foreshadow that maps can reveal different kinds of routes. Do not
start the full Alice/Bob quest here unless it is already trivial. Task 04 owns the
under-town cartography route.
