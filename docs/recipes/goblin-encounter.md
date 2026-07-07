# Mob lab encounter workflow

The goblin encounter is a showcase / regression area for enemy spawns, encounters, lock walls, rewards, and movement bugs around runtime-inserted geometry.

## Current authoring path

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/goblin_encounter_area.yaml \
  --dry-run

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/goblin_encounter_area.yaml \
  --apply

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  game/ambition_content/assets/worlds/sandbox.ldtk
```

## Review checklist

- Encounter trigger starts the fight only from intended entry paths.
- Lock-wall geometry does not create unsafe wall-cling or ceiling-snap corrections.
- Rewards persist through save/load once collected.
- Enemy archetypes match the intended test coverage.
- Trace reproduction is added for any movement bug discovered in the room.

Related docs: `docs/planning/tech-debt-log.md`, `docs/systems/gameplay-trace-recorder.md`, `docs/systems/boss-encounter-architecture.md`.
