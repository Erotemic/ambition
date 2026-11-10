---
status: current
last_verified: 2026-07-18
---

# LDtk world composition and transactional room loading

This page records the current cross-crate implementation shape. The durable
contract is in [`../concepts/ldtk-world-composition.md`](../concepts/ldtk-world-composition.md).

## Pipeline

1. a provider owns one or more `.ldtk` payloads and a world manifest;
2. `ambition_platformer2d_ldtk` imports LDtk layers/entities/fields into typed records;
3. `ambition_platformer2d_world` owns reusable room specs, loading zones, collision/world
   vocabulary, validation, and lowering contracts;
4. provider/domain registrations supply content-specific lowering or staging;
5. room preflight produces an immutable `RoomConstructionPlan` without mutating
   the live room;
6. `ambition_load` tracks required/degradable work and barrier readiness;
7. the app/runtime room-transition coordinator authorizes one commit after the
   plan and required assets are ready;
8. canonical construction retires the old room and installs the target atomically;
9. `ambition_load_presentation` may cover unresolved work but cannot authorize it.

Some construction integration still lives in `ambition_platformer2d_actor_monolith` and app adapters
while the decomposition continues. New code should strengthen the contracts
above rather than create another loader/spawn path.

## Which way a zone goes

A zone's way is read off the room links (`RoomSet::way_of`): both ways, out
only (its far side has no link back to this room), or in only (it has no
link out: a place to arrive at). The renderer draws a zone that is not both
ways tinted and signed over it, `ONE WAY: NO RETURN` or `NO EXIT`, and a
door's nameplate says so too (Jon, 2026-11-09: a one-way door must be
"EXTREMELY clear"). Nothing is authored for it: a link written one way is
drawn as one.

## Transition invariants

- repeated overlap with the same loading zone does not mint duplicate active
  transactions;
- a genuinely new target supersedes the prior transaction exactly;
- the source room remains playable/authoritative until commit policy says
  otherwise;
- preflight and asset discovery do not leak target entities;
- failures leave a valid source world and publish actionable load evidence;
- hidden grace, cover, loading UI, and ready-hold are presentation policy;
- readiness is contributor-neutral and headless observable;
- restore/reset/hot reload converge on canonical lowering/construction.

## Authoring and validation

Use `ambition_ldtk_tools`; do not hand-edit LDtk JSON. See
[`../recipes/ldtk-authoring.md`](../recipes/ldtk-authoring.md).

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools validate game/ambition_content/assets/worlds/sandbox.ldtk
./run_tests.sh -p ambition_platformer2d_ldtk
./run_tests.sh -p ambition_platformer2d_world
./run_tests.sh -k room_transition
./run_tests.sh -k construction_plan
```
