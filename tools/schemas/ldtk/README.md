# LDtk JSON schema

Ambition's LDtk validator can optionally run LDtk's official JSON Schema through
Python's `jsonschema` package. We avoid Node/npm in this workflow.

Fetch the schema manually when you want strict editor-format validation:

```bash
mkdir -p tools/schemas/ldtk
python - <<'PY'
from urllib.request import urlretrieve
urlretrieve('https://ldtk.io/files/JSON_SCHEMA.json', 'tools/schemas/ldtk/JSON_SCHEMA.json')
PY
uv pip install jsonschema
python tools/validate_ambition_ldtk.py \
    --schema tools/schemas/ldtk/JSON_SCHEMA.json \
    --require-schema \
    crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

The official schema catches LDtk/editor-format drift. The Ambition validator then
checks game-specific contracts such as active-area stitching, loading-zone graph
validity, transition arrival safety, and direct `bevy_ecs_ldtk` spawnability.
