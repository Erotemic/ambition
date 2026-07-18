---
id: bevy-native-data-driven-ecs
aliases: []
status: current
authority: durable-concept
last_verified: 2026-07-18
related_adrs:
  - docs/adr/0002-engine-must-be-bevy-native.md
  - docs/adr/0003-data-specs-and-asset-loading.md
  - docs/adr/0009-world-composition-and-ldtk-authoring.md
implemented_by:
  - crates/ambition_engine_core
  - crates/ambition_platformer_primitives
  - crates/ambition_world
  - crates/ambition_ldtk_map
  - crates/ambition_runtime
---

# Bevy-native data-driven ECS

Ambition uses Bevy and ECS directly. “Data-driven” means authored/generated data
feeds typed ECS vocabulary and canonical systems; it does not mean duplicating
live state into parallel object graphs or inventing a backend-neutral runtime
that hides Bevy.

## Durable rule

```text
authored/generated data
    -> typed parse/import
    -> validation and provider registration
    -> ECS components/resources/entities
    -> owner systems and messages
    -> read models
    -> presentation
```

- Pure math and deterministic kernels stay usable without a Bevy world.
- Runtime integration is Bevy-native when ECS identity, scheduling, lifecycle,
  queries, or resources are the natural model.
- LDtk owns Ambition's world authoring today. `ambition_ldtk_map` adapts LDtk;
  `ambition_world` owns reusable world records/lowering vocabulary.
- RON remains appropriate for compact provider catalogs, tuning, settings,
  saves, and generated-asset specifications.
- One domain owns each noncommutative state machine. Multiple append-only
  registrations are fine; multiple mutable authorities are not.

## Smells

- a second “runtime model” that must be synchronized with ECS;
- a reusable crate that names Ambition content;
- a presentation system mutating authoritative simulation;
- a resource mirroring session entities without exact invalidation;
- a generic abstraction whose only purpose is to avoid using Bevy types;
- hand-written app assembly for a subsystem that should own a plugin.

## Validation

Use the narrowest owning test plus a headless composition when the change affects
outcomes:

```bash
python scripts/agent_query.py tests "<invariant>"
./run_tests.sh -p <owning-crate>
cargo run -p ambition_app --bin headless -- 30
```
