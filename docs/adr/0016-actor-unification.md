# ADR 0016: Unify NPCs, enemies, hazards, and interactables as actor-like ECS data

## Status

Accepted direction; implementation remains incremental.

## Decision

Represent gameplay entities that can be spawned, addressed, damaged, interacted with, or used in encounters as actor-like ECS data. This is a compositional ECS vocabulary, not one mega-component.

Common reusable pieces include identity/content IDs, faction, health/damage, interaction hooks, movement/path behavior, encounter membership, and presentation messages.

## Context

Earlier patch notes separated interactions, hazards, enemies, bosses, labels, and NPCs into independent skeleton systems. That was useful while proving primitives, but it is not the enduring architecture. The current project direction is data-driven ECS: authored/generated data becomes components; systems interpret components; presentation consumes resulting state/messages.

## Consequences

- Old interaction/hazard/actor skeleton docs are historical.
- New one-off taxonomies should be avoided unless a concept genuinely cannot compose through actor-like ECS data.
- Dialogue/commerce, combat, hazards, bosses, and authored LDtk entities should converge on shared identity and interaction vocabulary where practical.

## Current implications for agents

- Before adding a new gameplay entity category, check whether actor/faction/damage/interactable components already express it.
- Keep component vocabulary small and compositional.
- Update concept/system docs when a reusable actor invariant changes.

## Faction vocabulary (2026-05-20 update)

The shared actor-faction tag landed as the `ActorFaction` component
in `crates/ambition_characters/src/actor/pose.rs`:

```rust
pub enum ActorFaction { Player, Enemy, Npc, Boss, Neutral }
```

Distinct from the existing `ActorDisposition` (which is the per-tick
hostility flag — peaceful NPCs can flip to hostile when struck):
faction is the structural "which side owns this actor" tag that
damage routing, projectile hit policy, and enemy targeting all
dispatch on. Predicates: `is_player_side()` for "is on the player's
side", `is_hostile_side()` for "participates in the combat loop"
(Enemy or Boss).

Today the spawn paths tag every actor entity (player, encounter
mob, NPC, boss). No system reads the component yet — it's a
read-model / identity tag the projectile-faction merge
(OVERNIGHT-TODO #17.7) and multiplayer-aware targeting (#17.8) will
dispatch on instead of pattern-matching on per-family components.

The mirroring `ProjectileFaction` enum in
`crates/ambition_gameplay_core/src/projectile/body.rs` agrees on the
combat-side semantics — `ProjectileFaction::Player` and
`ActorFaction::Player` describe the same side — so a future "is this
projectile hostile to this actor?" predicate can compose the two
without translation.

## Universal-brain (2026-05-24 update)

Beyond the faction tag, the actor-unification work extended to a
universal-brain interface (`crates/ambition_characters/src/brain/`):
every controllable entity now carries `Brain` + `ActionSet` +
`ActorControl` sibling components. Brains tick each frame to fill
the abstract intent frame; ActionSets resolve abstract intent into
concrete combat/effect requests. Today consumers still flow
through the shared actor/action pipeline; sandbox effect consumers translate `ActorActionMessage` into projectiles, hitboxes, boss specials, and presentation messages.

The brain + faction layers compose:
- `ActorFaction` answers "which side owns this actor" (immutable
  identity).
- `Brain::is_hostile()` answers "does this actor want to attack
  right now" (per-tick, brain-derived).
- Damage routing reads faction; per-tick attack gating reads
  brain hostility.

See `docs/systems/brain-driver.md` for the brain shape +
`docs/recipes/extending-brains-and-action-sets.md` for the
extension procedure.
