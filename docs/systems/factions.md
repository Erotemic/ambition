---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/systems/actors-brains-and-character-content.md
  - docs/concepts/content-and-provider-boundaries.md
---

# Factions and targeting

Faction is reusable simulation vocabulary for relationship/targeting policy. A
named game faction, its characters, dialogue, art, and music remain provider
content.

## Model

An actor has stable identity and live affiliation data. Targeting and combat
resolve an effective relationship from source, target, current control/brain or
status, and provider/domain policy. “Player” is not itself a universal faction
rule; possession, allies, confusion, summons, and multiplayer require explicit
relationships.

## Ownership

- Entity/character domains expose stable identity and affiliation components.
- Combat owns target filtering and accepted hit relationships.
- Brains/perception consume relationship queries when selecting intent.
- Provider content owns named faction IDs, rosters, dialogue, reputation,
  presentation, and game-specific relationship tables.
- Persistence stores only durable provider-level facts; effective runtime
  relationships are reconstructed.

## Allies, provocation and the rally

- `FactionRelations` has two directed tables: `hostile` (who hunts whom) and
  `allied` (who fights at whose side). `fights_beside(from, foe)` gives the
  faction a `from` body fights for against `foe`.
- An alliance changes damage too: a body does not hurt the faction it fights
  beside (`combat_relation` resolves `Ally`). The table is directed, so the
  side it fights for can still hit it.
- Provocation: a struck peaceful body (past its strike threshold) or a
  challenged body becomes `Hostile`, holds a grudge against its attacker, and
  gets its character's provoked policy as its mind. Its kit does not change.
- Rally (`body::rally_allies_to_the_fight`): a peaceful body with a kit and a
  provoked policy, in a room authored `dangerous`, takes as its foe the nearest
  body that is hostile to a faction it fights beside and that has come to blows
  with that faction (a body of the faction struck it, or it swings at one now),
  inside its provoked policy's `aggro_radius`. A foe that only watches the
  player starts no rally, so the ally stays talkable at its post. It is the same flip
  as a provocation, but the aggression mode stays `RetaliatesWhenHit`. When it
  has no foe, or its foe fights nobody on its side any more, the rally ends the
  grudge and restores its authored mind (`BrainCommand::restore_default`).
- `RoomMetadata::dangerous` is the LDtk level bool `dangerous` (specs state it
  as `dangerous: true`). Ambition sets it on its out-of-town rooms with
  enemies, and declares `Npc` allied with `Player`
  (`ambition_content::plugin`).
- Test: `game/ambition_app/tests/npcs_fight_beside_you.rs`.

## Invariants

- No reusable rule branches on a named Ambition character/faction.
- Human and brain control do not bypass the same targeting policy.
- Friendly-fire, neutral, hostile, self, owner/summon, and temporary override
  behavior is explicit.
- Relationship changes are observable and snapshot/reset safe.
- Presentation colors/icons do not determine affiliation.

## Validation

```bash
python scripts/agent_query.py "faction targeting effective faction"
python scripts/agent_query.py tests "friendly fire faction"
./run_tests.sh -p ambition_combat -k faction
./run_tests.sh -p ambition_content -k faction
```
