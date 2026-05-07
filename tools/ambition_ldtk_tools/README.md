# Ambition LDtk Tools

Modal CLI for editing, validating, and repairing the Ambition
sandbox.ldtk world. Agents should not hand-edit the LDtk JSON; use this
package so mutations are repaired and validated before write.

## CLI

```
python -m ambition_ldtk_tools validate                       # validate sandbox.ldtk
python -m ambition_ldtk_tools repair  <ldtk> --in-place      # fix editor metadata
python -m ambition_ldtk_tools roundtrip <ldtk>               # non-mutating smoke check
python -m ambition_ldtk_tools doctor    <ldtk>               # roundtrip + validate

python -m ambition_ldtk_tools schema fetch                   # pull official LDtk JSON schema
python -m ambition_ldtk_tools schema validate <ldtk>         # schema-only validation

python -m ambition_ldtk_tools area create <spec.yaml>        # author a new area / level
python -m ambition_ldtk_tools area create <spec.yaml> --apply  # write the file (dry-run is the default for `--dry-run`)
python -m ambition_ldtk_tools door free-spots <room>         # list free 48x96 door slots

python -m ambition_ldtk_tools entity add <spec.yaml>         # add entity instance(s)
python -m ambition_ldtk_tools def register-entity <spec.yaml>  # register an entity definition
```

`area create` defaults to applying its spec (it is the original behavior of
`tools/author_ldtk_area.py`); pass `--dry-run` to preview without writing.
The mutating subcommands run the full repair + validate pipeline before
returning, so a successful invocation is always editor-safe.

The following subcommands are reserved (placeholders) and will land later:
`entity set-field`, `entity move`, `entity delete`, `link {add,remove,check}`,
`intgrid {paint,erase,summarize}`.

## Layout

```
ambition_ldtk_tools/
  __init__.py
  __main__.py
  cli.py
  validate.py            # legacy validator (now `validate`)
  repair.py              # legacy repair pass (now `repair`)
  roundtrip.py           # non-mutating round-trip check
  schema.py              # official LDtk JSON schema fetcher
  area_authoring.py      # large authoring tool (now `area create`/`door free-spots`)
  edit/
    entities.py          # `entity add` (legacy add_ldtk_entity_to_level.py)
    defs.py              # `def register-entity` (legacy register_ldtk_entity_def.py)

specs/                   # YAML spec inputs
  examples/              # reference / lab specs
  ...                    # active specs (water_world_area.yaml, etc.)

schemas/
  ldtk/JSON_SCHEMA.json  # checked-in copy of the official LDtk JSON schema
```

## Compatibility shims

The old script paths still work but print a deprecation note:

- `tools/validate_ambition_ldtk.py`
- `tools/repair_ambition_ldtk.py`
- `tools/check_ldtk_editor_roundtrip.py`
- `tools/fetch_ldtk_schema.py`
- `tools/author_ldtk_area.py`
- `tools/add_ldtk_entity_to_level.py`
- `tools/register_ldtk_entity_def.py`

Each one forwards its argv to the corresponding `python -m
ambition_ldtk_tools` subcommand.
