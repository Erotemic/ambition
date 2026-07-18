---
status: current
last_verified: 2026-07-18
---

# LDtk world composition and transactional room loading

This page records the current cross-crate implementation shape. The durable
contract is in [`../concepts/ldtk-world-composition.md`](../concepts/ldtk-world-composition.md).

## Pipeline

1. a provider owns one or more `.ldtk` payloads and a world manifest;
2. `ambition_ldtk_map` imports LDtk layers/entities/fields into typed records;
3. `ambition_world` owns reusable room specs, loading zones, collision/world
   vocabulary, validation, and lowering contracts;
4. provider/domain registrations supply content-specific lowering or staging;
5. room preflight produces an immutable `RoomConstructionPlan` without mutating
   the live room;
6. `ambition_load` tracks required/degradable work and barrier readiness;
7. the app/runtime room-transition coordinator authorizes one commit after the
   plan and required assets are ready;
8. canonical construction retires the old room and installs the target atomically;
9. `ambition_load_presentation` may cover unresolved work but cannot authorize it.

Some construction integration still lives in `ambition_actors` and app adapters
while the decomposition continues. New code should strengthen the contracts
above rather than create another loader/spawn path.

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
./run_tests.sh -p ambition_ldtk_map
./run_tests.sh -p ambition_world
./run_tests.sh -k room_transition
./run_tests.sh -k construction_plan
```
