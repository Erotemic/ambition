---
status: current
last_verified: 2026-11-10
---

# Add a showcase room

A showcase room is a provider-owned LDtk level designed to demonstrate one
mechanic in the running game. Keep its purpose clear and use a stable room ID.

## Workflow

1. Identify the game/provider that owns the room and the behavior to demonstrate.
2. Examine a nearby area spec and the current LDtk entity and world contracts.
3. Author a spec under `tools/ambition_ldtk_tools/specs/` and dry-run it.
4. Inspect the geometry, then apply with a backup.
5. Enter the room in the game and exercise the mechanic. Use headless simulation
   when that makes a difficult interaction easier to reproduce.

```bash
python3 scripts/agent_query.py "<mechanic> LDtk room"

PYTHONPATH=tools/ambition_ldtk_tools \
  python3 -m ambition_ldtk_tools.area_authoring tools/ambition_ldtk_tools/specs/<showcase>.yaml --dry-run
PYTHONPATH=tools/ambition_ldtk_tools \
  python3 -m ambition_ldtk_tools.area_authoring tools/ambition_ldtk_tools/specs/<showcase>.yaml --backup

PYTHONPATH=tools/ambition_ldtk_tools \
  python3 -m ambition_ldtk_tools doctor game/ambition_content/assets/worlds/sandbox.ldtk
```

See [LDtk authoring](ldtk-authoring.md) for coordinate, collision and
loading-zone rules. Use the LDtk room renderer or `render_room_geometry` when
visual inspection is more helpful than reading coordinates.

Add a permanent integration test only if it covers a consequential regression
or acceptance condition not easily verified through playtesting.

## Review

- The room is reachable and demonstrates the intended behavior.
- Arrival points and loading-zone destinations are safe.
- Static collision comes from IntGrid; interactive mechanics use authored entities.
- Game-specific content remains owned by the provider.
- The mechanic uses its normal runtime authority, not a showcase-only path.
