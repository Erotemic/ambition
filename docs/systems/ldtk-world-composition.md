# LDtk world composition

LDtk is the current source of truth for world/level composition.

## Current rule

- Author areas, collision layers, loading zones, and spatial entities in LDtk or via `ambition_ldtk_tools`.
- Do not hand-edit `sandbox.ldtk` JSON for semantic changes.
- Runtime code should project LDtk data into Bevy ECS and reusable engine types.
- Old RON room manifests are historical.

## Runtime concerns

LDtk changes are high-risk because they affect:

- collision categories and IntGrid values,
- loading-zone graph links,
- player starts and spawn repair,
- camera zones and visual profiles,
- web/static map embedding,
- Android packaged assets,
- editor roundtrip metadata.

## Tools

Use:

```bash
python -m ambition_ldtk_tools validate game/ambition_content/assets/worlds/sandbox.ldtk
python -m ambition_ldtk_tools repair game/ambition_content/assets/worlds/sandbox.ldtk --in-place
python -m ambition_ldtk_tools area create <spec.yaml> --backup
```

See `docs/tools/index.md` and `docs/recipes/ldtk-authoring.md`.

## Common failure modes

- Treating old RON room docs as current.
- Moving zones without updating graph links.
- Introducing entities without validator/tool support.
- Breaking static-map/web or Android asset assumptions.
- Forgetting spawn repair or transition cooldown behavior.

## Validation

Run LDtk validators and focused sandbox tests before broad tests. Search `dev/benchmark-candidates/` for LDtk collision/runtime traps.
