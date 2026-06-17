# LDtk JSON schema

Ambition's LDtk validator can optionally run LDtk's official JSON Schema through
Python's `jsonschema` package. We avoid Node/npm in this workflow.

Fetch the schema when you want strict editor-format validation:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools schema fetch
uv pip install jsonschema
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools schema validate \
    crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
    --schema tools/ambition_ldtk_tools/schemas/ldtk/JSON_SCHEMA.json \
    --require-schema
```

You can also use the round-trip checker with the schema:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools roundtrip \
    crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
    --schema tools/ambition_ldtk_tools/schemas/ldtk/JSON_SCHEMA.json \
    --require-schema
```

The official schema catches LDtk/editor-format drift. The Ambition validator then
checks game-specific contracts such as active-area stitching, loading-zone graph
validity, transition arrival safety, direct `bevy_ecs_ldtk` spawnability, and
editor round-trip safety.
