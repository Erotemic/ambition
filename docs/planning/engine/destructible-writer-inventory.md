# A5: who writes destructible state

**Delivered 2026-09-10.** A5's hold reads *"HOLD until A2's contact contract is
established AND writer inventory is complete."* A2 is closed. This is the second
half. It is the same shape as the [accepted-control writer
map](accepted-control-writer-map.md), which lifted half of A4's hold.

⛔ **MEASUREMENT ONLY. IT DECIDES NOTHING.** Q96 asks whether a projectile should
collide with an ECS breakable's published surface. That is a behaviour ruling and
it is not answered here. This page records who writes destructible state TODAY, so
that a later ownership change has a subject and Q96 has a population. **Nothing
here licenses an extraction.**

## The state, and where the machine lives

| carrier | crate |
|---|---|
| `Breakable` (health, state, trigger, collision) | `ambition_interaction` |
| `BreakableState { Intact, Cracking, Broken }` | `ambition_interaction` |
| `BreakableFeature { breakable }` — the ECS component | `ambition_combat` |

⭐⭐ **THE TRANSITION IS NOT IN THE MONOLITH.** `Breakable::apply_damage`
(`ambition_interaction/src/lib.rs:255`) owns the whole state machine: it damages
the health, sets `Broken` or `Cracking`, and returns `true` on the break. Its
`#[must_use]` says the caller owes the break its consequences. ⇒ Every ECS writer
below is an ORCHESTRATOR of that one method, not a second interpretation of it.

## Writers of destructible state — the complete production set

| site | writes | authority |
|---|---|---|
| `monolith features/ecs/spawn_static.rs:613` | constructs `BreakableFeature` | authored placement spawn |
| `ambition_combat/src/breakables.rs:128` | constructs `BreakableFeature` | respawn re-insert |
| `ambition_combat/src/breakables.rs:41` | `state = Intact` | respawn transition |
| `monolith features/ecs/damage/mod.rs:397` | `&mut BreakableFeature` | damage transition |
| `ambition_interaction/src/lib.rs:258,260` | `state = Broken` / `Cracking` | the domain state machine |

**Five writers, three crates.** `monolith damage/mod.rs` also calls
`begin_ecs_breakable_respawn` at `:592` and `:989`; that function is
`ambition_combat`'s, so the respawn authority is one place called from two.

## Readers — measured, and NOT writers

`damage_predicates.rs`, `projectile/systems.rs`, `projectile/intercept.rs`,
`world/physics.rs`, `world/overlay.rs`, `features/ecs/anim_helpers.rs`,
`construction/mod.rs`, `world/rooms/reconstitution.rs`, `features/ecs/summon.rs`,
`features/ecs/damage/boss_hit.rs`.

⚠ **`features/ecs/target_volumes.rs` LOOKS like a writer and is not.** It takes
`(&CenteredAabb, &BreakableFeature, &mut DamageableVolumes)`: it READS the
breakable and WRITES a different component. A first pass of this inventory counted
it as a destructible writer on the strength of a `&mut` on the same line. **A
`&mut` in a query is not a `&mut` on the thing you are counting**, and the file
with the most references (`spawn_static.rs`, 53) turned out to hold mostly
spec-to-domain converters and exactly one construction.

## What this says about A5's premise

The packet's destination is *"one logical destructible-object owner"*. ⇒ The
inventory does not find scattered authority to consolidate. It finds **a domain
state machine with one method, and four ECS sites that call or construct around
it, split across three crates.** The split is by crate, not by duplicated
interpretation.

⛔ That is a statement about writers and nothing else. It does not say the split is
wrong, and it cannot: *"moving all destructible state first would preserve an
incorrect split interpretation"* is the frontier's own warning, and whether the
interpretation is correct is what Q96 decides.

## Reproduce

```
grep -rn "&mut .*BreakableFeature" --include=*.rs crates/ game/
grep -rn "BreakableFeature::new" --include=*.rs crates/ game/
grep -rn "\.state = BreakableState::" --include=*.rs crates/
```
Read every hit before classifying one. Two of the three greps above produce a
false positive that a count alone would keep.
