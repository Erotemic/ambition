# Add a showcase room

Use this when adding a small LDtk room that demonstrates one mechanic or content slice. Do not hand-edit LDtk JSON.

## Workflow

1. Pick an existing spec under `tools/ambition_ldtk_tools/specs/` as a starting point.
2. Copy it to a new descriptive spec name under the same directory.
3. Run a dry run.
4. Apply the spec.
5. Repair/roundtrip/validate the LDtk file.
6. Run a focused gameplay test or visible smoke pass.

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/<new_area>.yaml \
  --dry-run

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/<new_area>.yaml \
  --apply

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk
```

## Review checklist

- The room has a clear mechanic purpose.
- Loading zones have safe arrivals.
- Collision uses IntGrid cells for static solids/hazards.
- Dynamic hazards use entities only when they need entity-only data.
- Rewards or progression flags use stable IDs.
- The map diff is inspectable after repair/roundtrip.

Related docs: `docs/recipes/ldtk-authoring.md`, `docs/tools/ldtk-tools.md`.
