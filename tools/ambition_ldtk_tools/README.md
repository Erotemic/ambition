# Ambition LDtk Tools

Modal CLI for editing, validating, and repairing the Ambition
`sandbox.ldtk` world. Agents should not hand-edit the LDtk JSON; use this
package so mutations are repaired and validated before write.

Run commands from the repository root with the package directory on
`PYTHONPATH`:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools <subcommand> ...
```

## Common commands

```bash
# Validate gameplay/editor contracts without mutating the file.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools validate \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk

# Check whether the package repair pass would change the file.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools roundtrip \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk

# Run roundtrip + validate.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk

# Repair in place, then inspect the diff before committing.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools repair \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk \
  --in-place
git diff -- crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk

# Schema helpers.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools schema fetch
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools schema validate \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk \
  --schema tools/ambition_ldtk_tools/schemas/ldtk/JSON_SCHEMA.json \
  --require-schema

# Authoring helpers.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/examples/crawl_lab.yaml \
  --dry-run
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/examples/crawl_lab.yaml
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools door free-spots \
  central_hub_basement
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity add \
  tools/ambition_ldtk_tools/specs/hub_lab_door.yaml \
  --in-place
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools def register-entity \
  tools/ambition_ldtk_tools/specs/encounter_and_switch_entities.yaml \
  --in-place
```

`area create` applies its spec by default; pass `--dry-run` to preview without
writing. Mutating subcommands run the repair + validate pipeline before
returning, so a successful invocation is editor-safe. Always inspect the
resulting `git diff` for `.ldtk` changes before committing.

## CLI surface

```
validate                       Validate sandbox.ldtk
repair  <ldtk> --in-place      Fix editor metadata / canonicalize JSON
roundtrip <ldtk>               Non-mutating repair-needed smoke check
doctor <ldtk>                  Roundtrip + validate
compact <ldtk>                 Re-format JSON arrays to LDtk editor style
list-metadata <ldtk>           Print biome/music/ambient metadata per level

schema fetch                   Pull official LDtk JSON schema
schema validate <ldtk>         Schema-aware validation

area create <spec.yaml>        Author a new area / level
door free-spots <room>         List free 48x96 door slots

entity add <spec.yaml>         Add entity instance(s)
entity set-field <spec.yaml>   Set field instances on existing entities
entity move <spec.yaml>        Move an existing entity
entity even-space <room>       Even-space matching entities in a level

def register-entity <spec>     Register an entity definition
```

The following subcommands are reserved placeholders and will land later:
`entity delete`, `link {add,remove,check}`, and
`intgrid {paint,erase,summarize}`.

## Layout

```
ambition_ldtk_tools/
  __init__.py
  __main__.py
  cli.py
  validate.py            # `validate`
  repair.py              # `repair`
  roundtrip.py           # `roundtrip`
  schema.py              # `schema fetch` / schema-aware validation
  area_authoring.py      # `area create` / `door free-spots`
  edit/
    entities.py          # `entity add`
    defs.py              # `def register-entity`
    move.py              # `entity move`
    set_field.py         # `entity set-field`

specs/                   # YAML spec inputs
  examples/              # reference / lab specs
  ...                    # active specs (water_world_area.yaml, etc.)

schemas/
  ldtk/JSON_SCHEMA.json  # checked-in copy of the official LDtk JSON schema
```

## Retired standalone script names

Older docs and failure messages may mention retired top-level scripts such as
`tools/repair_ambition_ldtk.py`, `tools/validate_ambition_ldtk.py`,
`tools/check_ldtk_editor_roundtrip.py`, `tools/author_ldtk_area.py`,
`tools/add_ldtk_entity_to_level.py`, or `tools/register_ldtk_entity_def.py`.
Those shim files are no longer present in this checkout. Use the package CLI
shown above instead, and update stale docs/prints when you find old script
names.
