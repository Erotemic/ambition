---
status: current
last_verified: 2026-10-02
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

## Discover the vocabulary before you author

Most authored vocabulary in this project is typed `String`, and the legal values
live in a Rust parser. `PickupSpawn.kind`, `Prop.kind`,
`EnemySpawn.character_id` / `respawn`, `MaryOBlock.contents`,
`KinematicPath.mode`, `LoadingZone.activation`, `BreakablePlatform.trigger` —
the entity def tells you the field exists and nothing more. Do not guess and do
not copy `entity query`'s summary column, which truncates.

```bash
WORLD=game/ambition_demo_mary_o/assets/worlds/mary_o.ldtk
# What can I place, which layers accept it, which fields, which enums —
# and a CENSUS of the values this world already authors for each field.
PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools vocabulary list \
  --ldtk "$WORLD" [--identifier EnemySpawn] [--docs]
# Placements that omit a field every other placement of their type authors,
# and enum values the enum cannot spell.
PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools vocabulary check \
  --ldtk "$WORLD" [--level mary_o_1_3]
```

The census counts content rather than restating a Rust rule, so it cannot drift.

⚠ **`validate` does NOT enforce the converter's required/refused contract.** A
room can pass `area create`, `repair` and `validate` while an `EnemySpawn` has no
`character_id`, which the converter refuses at load. Run `vocabulary check`
before you hand a room off.

## ⛔ `area create` DROPS the name of a static-collision entity

`Solid`, `OneWayPlatform`, `BlinkWall` and `HazardBlock` in an `area create`
spec are **lowered into IntGrid cells**, and `int_grid_value_to_block` names an
IntGrid-derived block `"ldtk solid"` / `"ldtk one-way"`. The author's `name` is
gone. The command reports `lowered N static-collision entities into M IntGrid
cells`; it does not say a name went with them.

That matters because **names are load-bearing**. Mary-O dresses its flagpole by
name (`goal_pole`, `goal_pole_knob`, `goal_pole_banner` in
`game/ambition_demo_mary_o/src/lib.rs`) and `authored_pole` PANICS on a room
with no `goal_pole` block. (Its cellar masonry is no longer named: a `Solid`
says its colour in its `color` field.) Author those in a **second pass** with
`entity add`, which does not lower:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/<area>.ron --ldtk "$WORLD" --replace-existing
PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools entity add \
  tools/ambition_ldtk_tools/specs/<area>_named_blocks.yaml --ldtk "$WORLD" --in-place
```

⚠ **order matters and `--replace-existing` is destructive**: regenerating the
area wipes the second pass, so re-run `entity add` after every `area create`.
Worked example: `mary_o_1_3_area.ron` + `mary_o_1_3_named_blocks.yaml`.

## Room music

A level's `music_track` is what plays in it, and its `fight_music_track` is what
plays while a fight is on in it. See [`room-music.md`](room-music.md) for the
priority order and the command that sets either field.

## Room entry cutscene

A level's `entry_cutscene` names the cutscene that plays the first time its
room becomes live. The script must be in the cutscene library, and its seen
flag stops a second play. Only one level of an active area can set it: the area
merge keeps the first value. Content validation refuses an unknown script and
a second value in one area.

The scripts are content too: Ambition's are in
`game/ambition_content/assets/data/cutscenes/*.ron` (schema
`cutscene_library`), which the content compiler checks and merges into one
library.

```bash
PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools level set-field \
  --ldtk game/ambition_content/assets/worlds/sandbox.ldtk --in-place \
  --level cutscene_lab --set entry_cutscene=cutscene_lab_intro
```

## A command while the room is live

A level's `while_live` is one authored command line, in the vocabulary a
`Switch`'s `on_activate` uses. It is asked for on each tick while the room is
live, so the verb must do nothing when its work is done. `encounter.start`
is such a verb: it starts an inactive encounter and ignores one in any other
phase. The Noether Chamber (`symmetry_room`) starts its puzzle this way:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools level set-field \
  --ldtk game/ambition_content/assets/worlds/sandbox.ldtk --in-place \
  --level symmetry_room --set "while_live=encounter.start encounter:symmetry_attunement"
```

A line that does not prepare against the composed command catalog is dropped
with a warning that names the room.

## Safe manual edit loop

```bash
WORLD=game/ambition_content/assets/worlds/<world>.ldtk
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor "$WORLD"
# Edit and save in LDtk.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools repair "$WORLD" --in-place --backup
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools roundtrip "$WORLD"
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools validate "$WORLD"
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools diff semantic HEAD:"$WORLD" "$WORLD"
```

The last command prints a verdict: `noise_only`, `changed` or `ambiguous`.
If the file changed and you do not know why, follow
[the semantic diff recipe](../../tools/ambition_ldtk_tools/README.md#when-an-ldtk-file-changed-and-you-do-not-know-why).

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


## Moving platforms and kinematic world objects

Moving platforms are authored in LDtk. The converter lowers position and size,
`speed`, horizontal `sweep_dx`, a referenced `KinematicPath`, and vertical
wrapping fields such as `loop_dy` and `loop_min_y` into `MovingPlatformSpec`.
Open work is in
[`../planning/engine/ldtk-authoring-and-world-tools.md`](../planning/engine/ldtk-authoring-and-world-tools.md)
and
[`../planning/engine/kinematic-world-objects.md`](../planning/engine/kinematic-world-objects.md).

## Placement discipline

Read [`../concepts/llm-spatial-authoring-discipline.md`](../concepts/llm-spatial-authoring-discipline.md).
Place an object according to its purpose and the live geometry, not a guessed
coordinate. Useful read-only tools include entity query/check, IntGrid query,
door free-spots, and geometry rendering.

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity query --help
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity check --help
cargo run -p ambition_platformer2d_actor_monolith --example render_room_geometry -- <ROOM_ID>
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

## ⛔ A regenerated `.ldtk` silently drops levels its specs do not know

A world generator script rebuilds the whole world from the specs it knows. A level
authored by another road (`area create` + `entity add`) is not in those specs, so
a regenerate writes a valid world without it. `doctor`, `roundtrip` and the
schema checks all stay green. A dropped level also leaves a dangling `next_room`
successor.

Before you merge any `.ldtk` change, count the levels on both sides. World files
live in the `game/ambition_map_assets` submodule, so run this inside it:

```bash
git -C game/ambition_map_assets show <ref>:ambition_demo_mary_o/worlds/mary_o.ldtk | python3 -c "
import json,sys; print([l['identifier'] for l in json.load(sys.stdin)['levels']])"
```

Prefer re-applying the semantic change over merging the file. Re-author a field
def and its instances onto the current world with `level add-field-def` /
`entity set-field`.
