# LDtk JSON schema

Ambition's LDtk validator can optionally run LDtk's official JSON Schema through
Python's `jsonschema` package. We avoid Node/npm in this workflow.

Fetch the schema when you want strict editor-format validation:

```bash
python tools/fetch_ldtk_schema.py
uv pip install jsonschema
python tools/validate_ambition_ldtk.py \
    --schema tools/schemas/ldtk/JSON_SCHEMA.json \
    --require-schema \
    crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

You can also use the round-trip checker with the schema:

```bash
python tools/check_ldtk_editor_roundtrip.py \
    --schema tools/schemas/ldtk/JSON_SCHEMA.json \
    --require-schema \
    crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

The official schema catches LDtk/editor-format drift. The Ambition validator then
checks game-specific contracts such as active-area stitching, loading-zone graph
validity, transition arrival safety, direct `bevy_ecs_ldtk` spawnability, and
editor round-trip safety.
