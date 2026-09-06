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
