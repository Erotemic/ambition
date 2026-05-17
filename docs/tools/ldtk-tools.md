
# LDtk tools

Location: `tools/ambition_ldtk_tools/`

Purpose: validate, repair, roundtrip, compact, inspect metadata, initialize worlds, and author areas/entities in Ambition LDtk files.

## Use this instead of hand-editing JSON

```bash
cd tools/ambition_ldtk_tools
python -m ambition_ldtk_tools --help
python -m ambition_ldtk_tools validate ../../crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
python -m ambition_ldtk_tools repair ../../crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk --in-place
```

Area specs live under `tools/ambition_ldtk_tools/specs/`.

## Agent rules

- Validate before and after semantic LDtk edits.
- Use repair/roundtrip tooling to preserve editor-compatible shape.
- Update `docs/recipes/ldtk-authoring.md` if the workflow changes.
- Treat loading zones, collision IntGrid values, and coordinate transforms as spatial review areas.
