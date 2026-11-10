---
status: current
related_docs:
  - docs/systems/actors-brains-and-character-content.md
  - docs/concepts/content-and-provider-boundaries.md
---

# Factions and targeting

`crates/ambition_combat/src/targeting.rs` owns the relationship query used
for combat decisions. Named factions and their narrative roles remain
provider content.

## Relationships and damage

`FactionRelations` has **directed** `hostile` and `allied` tables. An alliance
from A to B means A fights beside B; it does not automatically make B an
ally of A. A body also has an effective faction that can change with its
controller, as well as optional team and grudge information.

`combat_relation` resolves a source/victim pair as `Foe`, `Ally`, or
`Neutral`. A grudge has precedence, then match teams, then the effective
factions and their relations. **Target selection and damage authorization
are distinct.** Neutral bodies are not automatically selected as enemies,
but `damage_lands_between` can still allow a physical strike to hurt one.
Damage to allies depends on `DamagePolicy` (including its participant
policy), not solely on the hostility table. A same-faction or allied result
is therefore not a blanket guarantee against every kind of damage.

## Provocation and allied intervention

The shared aggression systems can give a struck or challenged peaceful body
an authored provoked brain and grudge without replacing its character kit.
In a room marked `dangerous`, `body::rally_allies_to_the_fight` may enlist
a nearby peaceful ally when a foe is actually fighting the faction that
ally supports. Merely seeing a hostile actor is insufficient. The rally
ends when the relevant fight ends, restoring the authored brain.

Ambition authors the `dangerous` room field in LDtk and declares its NPC
alliance in provider setup. For gameplay changes, inspect
`crates/ambition_platformer2d_actor_monolith/src/body/rally.rs` and the
related combat targeting policy rather than duplicating faction logic in a
named NPC or boss.

## Extension guidance

Use the shared relation, team, grudge, and damage-policy interfaces for
new roles such as possession, summons, allies, and local participants.
Presentation labels and colors must not decide relationships. Exercise a
real targeting/damage situation when changing the policy; broad unrelated
combat suites need not run for a content-only faction change.
