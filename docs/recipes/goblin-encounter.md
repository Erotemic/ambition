---
status: current
last_verified: 2026-07-18
---

# Author or revise the goblin encounter

This is a concrete provider-content workflow, not an engine architecture. Use it
as an example of composing reusable encounter, character, world, progression,
audio, and presentation domains without adding goblin-specific core code.

## Localize current owners

```bash
python scripts/agent_query.py "goblin encounter waves music reward"
python scripts/agent_query.py tests "goblin encounter"
```

Current provider data includes an encounter spec under
`game/ambition_content/assets/data/encounters/` and LDtk placement in the
provider world. Copy live neighboring data; do not copy numeric snippets from
this recipe.

## Change loop

1. Decide whether the change is encounter sequencing, actor capability/brain,
   spatial placement, reward/progression, or presentation.
2. Edit the provider-owned data for that concern.
3. Use LDtk tooling for spatial changes; keep safe arrivals and lock-wall
   lifecycle scoped to the encounter/room.
4. Keep wave/state transitions in the reusable encounter state machine and
   provider spec—not a room-name branch in app code.
5. Bind music/SFX/VFX by semantic provider IDs.
6. Verify headlessly from room load through completion and reset.

```bash
WORLD=game/ambition_content/assets/worlds/sandbox.ldtk
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor "$WORLD"
./run_tests.sh -p ambition_content -k goblin
./run_tests.sh -p ambition_encounter
./run_tests.sh -k encounter
```

## Invariants

- Encounter entities are owned by the correct lifecycle scope.
- Completion/reward is idempotent.
- Reset/re-entry does not retain stale walls, enemies, music state, or reward
  eligibility.
- Goblins use the same actor/body/action/combat path as other characters.
- The encounter remains provider content; reusable crates know only generic
  encounter and actor vocabulary.
