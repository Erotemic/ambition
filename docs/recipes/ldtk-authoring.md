---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/concepts/ldtk-world-composition.md
  - docs/systems/ldtk-world-composition.md
  - docs/tools/ldtk-tools.md
---

# LDtk authoring

LDtk is Ambition's spatial source of truth. Use the editor or
`ambition_ldtk_tools`; do not hand-edit `.ldtk` JSON.

The main provider worlds currently live under
`game/ambition_content/assets/worlds/`. Localize a room/entity contract before
editing:

```bash
python scripts/agent_query.py "LDtk <room or entity type>"
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools --help
```

## Safe manual edit loop

```bash
WORLD=game/ambition_content/assets/worlds/sandbox.ldtk
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor "$WORLD"
# Edit and save in LDtk.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools repair "$WORLD" --in-place --backup
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools roundtrip "$WORLD"
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools validate "$WORLD"
git diff -- "$WORLD"
```

Use the exact subcommand help before mutation. Most mutators require an explicit
`--in-place` or `--output`; area creation is the notable current command that
edits its target in place by default unless `--dry-run` or `--output` is used.

## Spec-driven area creation

Specs live under `tools/ambition_ldtk_tools/specs/`.

```bash
SPEC=tools/ambition_ldtk_tools/specs/<area>.yaml

PYTHONPATH=tools/ambition_ldtk_tools \
  python -m ambition_ldtk_tools.area_authoring "$SPEC" --dry-run

# Apply to the spec/default target; make a backup when editing in place.
PYTHONPATH=tools/ambition_ldtk_tools \
  python -m ambition_ldtk_tools.area_authoring "$SPEC" --backup
```

Use `--output /tmp/review.ldtk` for a non-destructive review file and
`--replace-existing` only for a spec-owned generated level.

## Placement discipline

Read [`../concepts/llm-spatial-authoring-discipline.md`](../concepts/llm-spatial-authoring-discipline.md).
Place an object according to its purpose and the live geometry, not a guessed
coordinate. Useful read-only tools include entity query/check, IntGrid query,
door free-spots, and geometry rendering.

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity query --help
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity check --help
cargo run -p ambition_actors --example render_room_geometry -- <ROOM_ID>
```

## Representation rules

- Static collision/hazards use the canonical IntGrid vocabulary.
- Use entities for authored objects that carry identity, fields, behavior, paths,
  or dynamic lifecycle.
- Loading zones need a valid reciprocal destination and safe arrival geometry.
- Provider-stable IDs, not Bevy `Entity` values, connect authored content.
- A tool-generated diff must remain understandable in LDtk and in semantic diff
  output.

## Validation

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor "$WORLD"
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools diff semantic HEAD:"$WORLD" "$WORLD"
./run_tests.sh -p ambition_content -k ldtk
./run_tests.sh -k room_spatial_integrity
```

Use [`headless-room-verification.md`](headless-room-verification.md) for runtime
proof. CLI help and source override old recipe flags.
