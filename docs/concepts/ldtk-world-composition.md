---
id: ldtk-world-composition
aliases: []
status: current
authority: durable-concept
last_verified: 2026-07-18
implemented_by:
  - crates/ambition_ldtk_map
  - crates/ambition_world
  - crates/ambition_load
  - crates/ambition_platformer_provider
  - game/ambition_content/src/worlds.rs
related_adrs:
  - docs/adr/0009-world-composition-and-ldtk-authoring.md
related_docs:
  - docs/systems/ldtk-world-composition.md
  - docs/recipes/ldtk-authoring.md
---

# LDtk world composition

LDtk is Ambition's current spatial authoring backend. The reusable engine sees
typed authored world records and lowering contracts, not LDtk JSON internals.

## Stable pipeline

```text
provider-owned .ldtk
    -> ambition_ldtk_map import/conversion
    -> ambition_world typed records
    -> validation + lowering/content-staging registries
    -> immutable room construction plan
    -> load/readiness transaction
    -> atomic session-scoped commit
```

Import, lowering, preparation, readiness, and commit are separate phases. The
source room remains authoritative while the destination is prepared. A failed or
superseded transition must not leak partial target entities.

## Invariants

- Do not hand-edit `.ldtk` JSON for semantic changes; use LDtk or
  `ambition_ldtk_tools`.
- Provider crates own world payloads, room IDs, and named content.
- `ambition_ldtk_map` owns backend adaptation and schema-specific conversion.
- `ambition_world` owns reusable room/world vocabulary and canonical lowering
  seams.
- Lowering produces session-scoped canonical ECS state through one registry.
- Stable IDs, not `Entity`, cross save/snapshot/content boundaries.
- Loading zones and target zones are validated as a graph.
- Web/Android embedded/static paths use the same provider world identity.
- Reset, hot reload, room transition, and restore must converge on canonical
  construction rather than each inventing a spawn path.

## Validation

```bash
python -m ambition_ldtk_tools validate game/ambition_content/assets/worlds/sandbox.ldtk
./run_tests.sh -p ambition_ldtk_map
./run_tests.sh -p ambition_world
./run_tests.sh -k room_transition
./run_tests.sh -k construction_plan
```

See [`../recipes/ldtk-authoring.md`](../recipes/ldtk-authoring.md) for mutation
commands and [`llm-spatial-authoring-discipline.md`](llm-spatial-authoring-discipline.md)
for placement reasoning.
