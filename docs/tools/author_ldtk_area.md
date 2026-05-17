# `ambition_ldtk_tools area create`

Author a new Ambition LDtk active area / level from a small YAML or JSON spec. This hides LDtk JSON details behind a high-level entity list so agents do not hand-edit editor metadata.

Use this when adding or modifying authored LDtk areas. Do not edit `sandbox.ldtk` JSON directly.

## Usage

```bash
python -m ambition_ldtk_tools area create path/to/spec.yaml --backup
python -m ambition_ldtk_tools area create spec.yaml \
    --ldtk crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk \
    --schema tools/ambition_ldtk_tools/schemas/ldtk/JSON_SCHEMA.json
python -m ambition_ldtk_tools area create spec.yaml --output /tmp/sandbox.preview.ldtk
```

The tool runs repair and validation after edits.

## Related docs

- `docs/tools/index.md`
- `docs/recipes/ldtk-authoring.md`
- `docs/systems/ldtk-world-composition.md`
- `dev/journals/lessons_learned.md`

## Validation

```bash
python -m ambition_ldtk_tools validate crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
python -m ambition_ldtk_tools repair crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk --in-place
```
