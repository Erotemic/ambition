---
id: bevy-native-data-driven-ecs
aliases: []
status: current
authority: durable-concept
last_verified: 2026-09-03
related_adrs:
  - docs/adr/0002-engine-must-be-bevy-native.md
  - docs/adr/0003-data-specs-and-asset-loading.md
  - docs/adr/0009-world-composition-and-ldtk-authoring.md
implemented_by:
  - crates/ambition_platformer2d_core
  - crates/ambition_platformer2d_shared_tangle
  - crates/ambition_platformer2d_world
  - crates/ambition_platformer2d_ldtk
  - crates/ambition_platformer2d_runtime
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
- LDtk owns Ambition's world authoring today. `ambition_platformer2d_ldtk` adapts LDtk;
  `ambition_platformer2d_world` owns reusable world records/lowering vocabulary.
- RON remains appropriate for compact provider catalogs, tuning, settings,
  saves, and generated-asset specifications.
- One domain owns each noncommutative state machine. Multiple append-only
  registrations are fine; multiple mutable authorities are not.

### Worked example: "multiple mutable authorities are not fine" (2026-09-03)

The rule above is easy to agree with and hard to spot, because the second
authority is usually a POSITION rather than a second system. `SwitchActivationQueue`
was drained inside the actor kernel's encounter adapter, and the drain toggled a
persisted switch inline. Four unrelated policies — a quest flag, `FlipGravity`,
four `SetGravity` faces, an encounter reset — read that toggle from inside the
same loop, and every one of them "knew" the switch's new value only because it
happened to run after the line that wrote it.

⇒ **What made it one authority again**: one system drains the queue in order,
performs the persisted write, and publishes the POST-toggle value as a fact.
Consumers react to the fact and none re-derives it. Nothing about the rule
required inventing a protocol — it required noticing that "after this line"
was doing the work an owner should.

⚠ **The guard shape is worth copying.** A second author does not look like two
`set_switch` calls; it looks like the queue being read twice. So the test leaves
the queue unconsumed and asserts the switch does NOT flip again — *"a SECOND
drain of an EMPTY queue must not toggle again"*. Poison it the other way too:
publish in reverse order and assert the consumer's order claim goes red, because
order is part of a queue's value and the checksum says so.

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
cargo run -p ambition_app_tools --bin headless -- 30
```
