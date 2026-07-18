---
status: current
last_verified: 2026-07-18
---

# Add a showcase room

A showcase room is a small provider-owned LDtk level that demonstrates one
mechanic through real runtime composition. Keep it intentionally narrow so it
can become a durable acceptance fixture rather than an unmaintained demo.

## Workflow

1. State the mechanic invariant the room demonstrates.
2. Find a nearby spec and the current entity/world contracts.
3. Create a new spec under `tools/ambition_ldtk_tools/specs/`.
4. Dry-run, inspect, then apply with a backup.
5. Validate the map and render geometry headlessly.
6. Add or update a focused integration test that enters the room through normal
   provider/session loading.

```bash
python scripts/agent_query.py "<mechanic> LDtk room"
SPEC=tools/ambition_ldtk_tools/specs/<showcase>.yaml

PYTHONPATH=tools/ambition_ldtk_tools \
  python -m ambition_ldtk_tools.area_authoring "$SPEC" --dry-run
PYTHONPATH=tools/ambition_ldtk_tools \
  python -m ambition_ldtk_tools.area_authoring "$SPEC" --backup

WORLD=game/ambition_content/assets/worlds/sandbox.ldtk
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor "$WORLD"
cargo run -p ambition_actors --example render_room_geometry -- <ROOM_ID>
python scripts/agent_query.py tests "<mechanic> <ROOM_ID>"
./run_tests.sh -k <focused_test_substring>
```

## Review checklist

- The level has one clear purpose and a stable ID.
- Entry/exit zones have reciprocal links and safe arrivals.
- Static geometry uses IntGrid; dynamic authored behavior uses entities.
- The test observes simulation/read-model state, not pixel color or incidental
  tuning.
- The room contains no engine-default named content; it belongs to the provider.
- The mechanic is exercised through the same body/action path used elsewhere.
