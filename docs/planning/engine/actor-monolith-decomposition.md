# Actor residual-kernel decomposition

> **Baseline:** `54d99e7fb`, measured 2026-09-07 (P1 landed).
> Historical carve notes live in Git history. This file contains only the rules
> needed to make the next decomposition decisions.

**State:** ACTIVE. The authority prerequisites for C2 are already crossed; this
work now runs in parallel with capability/plugin composition.

Executable work is in
[`actor-monolith-work-frontier.md`](actor-monolith-work-frontier.md). The
post-P4 design ledger is
[`actor-monolith-hard-core-edge-ledger.md`](actor-monolith-hard-core-edge-ledger.md).

## Goal

Reduce `ambition_platformer2d_actor_monolith` to a coherent actor/body kernel.
A carve succeeds only when an authority moves to its semantic owner and the old
dependency disappears from production source.

The residual kernel may own:

```text
body state and actor-local lifecycle
accepted control/intent projection
movement/contact integration
core body reaction/action application
narrow observation/decision seams
```

It must not remain the owner merely because code historically landed there for:

```text
session/world lifecycle
persistence/save mirrors
independent item/projectile domains
boss/encounter/dialogue orchestration
presentation/UI/audio
host/dev policy
named product content
```

## Current graph

Measure with:

```bash
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
```

Baseline nontrivial SCCs:

```text
 9: abilities, construction, control, features, items, projectile,
    session, shrine, world

 2: assets, character_sprites
```

P1 (`character_runtime -> features`, 11 -> 9) is done. The next three packets are
already designed. They are not invitations to choose a different low-count edge:

```text
P2  projectile -> features           expected  9 -> 8
P3  shrine -> session                expected  8 -> 7
P4  construction -> world            expected  7 -> 6
```

After P4, stop mechanical carving. Remeasure and finish the hard-core ledger
before modifying the expected six-module SCC:

```text
abilities, control, features, items, session, world
```

## Rules that every packet must obey

### One packet, one ownership claim

Do not combine unrelated cleanup with an SCC packet. A packet may update the
callers, tests, rollback registration and facade paths needed by the moved
authority; it should not opportunistically redesign neighboring systems.

### The old internal path must disappear

A definition moved to a new owner but still reached through the old monolith
module is not a completed carve. Internal consumers must name the semantic owner.
Do not add compatibility re-exports under the old internal module solely to keep
imports unchanged.

Stable public facades may re-export only when they are already the intended API
surface and do not restore the internal graph edge.

### Move wire implementations with their types

If a moved canonical type implements `SnapshotState`, the implementation moves
to the crate that owns the type. Keep existing rollback wire IDs byte-for-byte
unless the packet explicitly changes the wire format. A type move alone is not a
reason to renumber or rename rollback state.

### Scheduling uses semantic sets

A moved system installs itself against a public semantic set. Do not replace a
module dependency with `.after(other_crate::private_function)` or
`.before(other_crate::private_function)`.

### Lifetime travels with authority

For every moved resource/component record:

```text
creation
mutation owner
retirement/retraction
rollback status
session/match/attempt/stock/process lifetime
```

If any row is unknown, stop the packet before moving the state.

### Tests must witness the production consequence

A unit test of a moved helper is support, not acceptance. Each packet in the
frontier names production behavior that must still be covered. A poison should
fail if the old dependency or behavior is restored.

## Packet completion receipt

Every P1-P4 commit must include, in its commit message or adjacent planning
receipt:

```text
baseline head
new head
old edge reference count
new edge reference count
largest SCC before -> after
focused Rust tests run
source/architecture guards run
rollback wire changes: none | explicit list
```

Run after each packet:

```bash
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
python3 scripts/modules_md.py
python3 scripts/check_planning_citations.py
python3 scripts/check_doc_links.py
```

Also run `git diff --check` and the focused Rust tests named by the packet. Do not
freeze a new metric baseline until the source movement is understood and
intentional.

## The post-P4 hard-core gate

P5 is complete only when
[`actor-monolith-hard-core-edge-ledger.md`](actor-monolith-hard-core-edge-ledger.md)
contains **every production dependency edge whose source and destination are
inside the measured six-module SCC** and every row has all of:

```text
source module + file + symbol
destination module + symbol/dependency
edge class
semantic owner
disposition
new target/API if CUT or MOVE
production poison/acceptance
```

Allowed dispositions:

```text
KEEP_DOWNWARD       legitimate dependency inside one proposed package/layer
MOVE_TYPE           vocabulary is filed beside the wrong consumer
MOVE_SYSTEM         mutation/install authority belongs elsewhere
PUBLISH_SET         dependency exists only for concrete system ordering
SPLIT_RESOURCE      one state object contains facts with different owners/lifetimes
GROUP_PACKAGE       modules should move together; internal cycle is accepted
DELETE_DEAD         production edge has no current customer
```

There must be **no `TBD` disposition** before the first hard-core implementation
packet begins.

P5 must end by writing one exact P6 packet with the same standard as P1-P4:
files, symbols, destination, forbidden end state, tests and expected SCC effect.
If the ledger does not support such a packet, the correct outcome is a package
map that keeps the remaining SCC together.

## Satellite SCCs

`actor_spawn` is no longer a monolith module: it is the crate
`ambition_platformer2d_actor_spawn`, whose contract is construction only (spawn
requests, spawn routines, body/brain builders, spawn-time NPC policy). The live
actor view and every system that mutates a spawned body stay in the kernel;
`scripts/tests/test_actor_spawn_boundary.py` holds that line, because this
module graph cannot see a crate boundary. `character_runtime` sits in no cycle.

`assets <-> character_sprites` is already a separate 2-module SCC and is also a
grouped-extraction question. Keep it off the P1-P5 critical path.

## Exit

D33 exits when either:

1. the residual monolith matches the controlled-character kernel and no unrelated
   authority remains inside it; or
2. the remaining SCC is explicitly accepted as one coherent package by a filled
   edge ledger and package map.

Acyclicity is useful evidence. It is not the product requirement.

## Post-carve safety map

A carve that moves one of these owner files must update the corresponding absence
contract in the same commit. This table belongs here rather than in `queue.md`
because it is durable decomposition doctrine.

| If your carve moves… | Update these absence contracts |
|---|---|
| `crates/ambition_combat/src/moveset/mod.rs` | `ending-a-move-goes-through-the-one-teardown-path` |
| `crates/ambition_characters/src/brain/fighter`, `crates/ambition_characters/src/brain/state_machine/mod.rs`, `crates/ambition_characters/src/snapshot_impls.rs`, or `crates/ambition_characters/src/brain/mod.rs` | `the-generic-brain-does-not-grow-new-platform-fighter-edges` |
| `crates/ambition_platformer2d_actor_monolith/src/character_runtime/match_activation.rs` or `game/ambition_app/src/app/versus.rs` | `a-second-writer-of-a-match-global-must-answer-ownership` |
| `crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs` or `game/ambition_app/src/dev/rollback_observatory.rs` | `the-seat-topology-has-one-engine-side-creator` |
| `game/ambition_app/src/app/versus.rs` or `game/ambition_demo_smash/src/lib.rs` | `the-global-roster-is-retired-only-by-its-owner` |
| `tools/ambition_ldtk_tools/ambition_ldtk_tools/ldtk/paths.py` or `tools/ambition_ldtk_tools/tests/test_ldtk_core_helpers.py` | `the-worlds-path-is-confined-to-ldtk-paths` |
| `crates/ambition_characters/src/prepared.rs`, `crates/ambition_combat/src/worn_kit.rs`, or `crates/ambition_characters/src/actor/character_catalog/mod.rs` | `the-catalog-default-action-set-is-confined-to-one-file` |
| `crates/ambition_platformer2d_actor_monolith/src/character_runtime/presentation.rs` | `the-provider-resolver-is-confined-to-one-file` |
| `crates/ambition_characters/src/prepared.rs`, `crates/ambition_platformer2d_actor_monolith/src/avatar/starting_character.rs`, or `crates/ambition_characters/src/actor/character_catalog/mod.rs` | `the-catalog-axis-tuning-is-confined-to-one-file` |
| `crates/ambition_platformer2d_actor_monolith/src/avatar/starting_character.rs` or `crates/ambition_platformer2d_actor_monolith/src/avatar/mod.rs` | `the-movement-tuning-resolver-is-confined-to-one-file` |
| `crates/ambition_platformer2d_actor_monolith/src/avatar/starting_character.rs` | `the-motion-model-resolver-is-confined-to-one-file` |
