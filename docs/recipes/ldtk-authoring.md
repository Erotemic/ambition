---
status: current
last_verified: 2026-08-13
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

## Discover the vocabulary before you author (2026-08-15)

⛔ **most authored vocabulary in this project is typed `String`, and the legal
values live in a Rust parser.** `PickupSpawn.kind`, `Prop.kind`,
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

⭐ **the census is the discoverable source of truth, and it cannot drift** — it
counts content rather than restating a Rust rule. `respawn` reads as an opaque
string until you are told 24 of 24 enemies say `OnRoomReenter`.

⚠ **`validate` does NOT enforce the converter's required/refused contract.**
`mary_o_1_3` was authored through `area create` + `repair` + `validate`, all
three green, with six `EnemySpawn` entities carrying no `character_id` — which
`convert_enemy_spawn` refuses, i.e. the room would have panicked the game on
load. `vocabulary check` is what caught it. Run it before you hand a room off.

## ⛔ `area create` DROPS the name of a static-collision entity

`Solid`, `OneWayPlatform`, `BlinkWall` and `HazardBlock` in an `area create`
spec are **lowered into IntGrid cells**, and `int_grid_value_to_block` names an
IntGrid-derived block `"ldtk solid"` / `"ldtk one-way"`. The author's `name` is
gone. The command reports `lowered N static-collision entities into M IntGrid
cells`; it does not say a name went with them.

That matters because **names are load-bearing**. Mary-O dresses its flagpole and
its cellar masonry by name (`goal_pole`, `goal_pole_knob`, `goal_pole_banner`,
`vault_*` in `game/ambition_demo_mary_o/src/lib.rs`) and `authored_pole` PANICS
on a room with no `goal_pole` block. Author those in a **second pass** with
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

Moving platforms are **already authored from LDtk**. The current converter can
lower authored position/size plus `speed`, horizontal `sweep_dx`, referenced
`KinematicPath`/legacy path id, and vertical wrapping fields such as `loop_dy`
and `loop_min_y` into `MovingPlatformSpec`.

That means the forward task is not "add LDtk moving platforms". It is to make the
existing path Engine-1.0 quality:

- prefer native/typed `EntityRef` linkage for a platform's path instead of a
  string relation;
- make motion mode explicit/validated instead of depending on precedence among
  optional fields;
- improve path/point editing beyond coordinate strings where LDtk can represent
  the intent directly;
- surface mode-specific validation and provenance as authoring diagnostics; and
- keep runtime moving-geometry/contact semantics in the reusable world/simulation
  model rather than in an Ambition-only adapter.

See
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

## ⛔⛔ A REGENERATED `.ldtk` SILENTLY DROPS LEVELS ITS SPECS DO NOT KNOW

**Found 2026-08-15, caught one command before it landed.** A commit that added
authored enemy `facing` also **deleted `mary_o_1_3` entirely** — 139 insertions
against **1316 deletions**, and the deletions were an entire authored level.

⭐ **the cause is the authoring script, not the agent.** `author_mary_o_ldtk.py`
rebuilds the world from the specs it knows; a level authored by a *different*
road — `area create` + `entity add`, which is how `mary_o_1_3` was built — is not
in those specs, so a regenerate writes a world without it. The `.ldtk` is one
file, so "regenerate the part I own" is not a thing it can do.

⛔ **and every check downstream stays GREEN.** The result is valid LDtk, the
roundtrip passes, `doctor` passes, the schema is intact. Nothing is corrupt —
there is simply one less level, and every tool that derives the roster from the
file agrees with the smaller world. ⚠ the only thing that noticed was a diffstat
whose deletion count was suspiciously close to the size of the level added one
commit earlier.

⇒ **before merging any `.ldtk` change, COUNT THE LEVELS on both sides:**

```bash
git show <ref>:ambition_demo_mary_o/worlds/mary_o.ldtk | python3 -c "
import json,sys; print([l['identifier'] for l in json.load(sys.stdin)['levels']])"
```

⭐ **and prefer re-applying the semantic change over merging the file.** A field
def plus its instances can be re-authored onto the current world with
`level add-field-def` / `entity set-field`; a wholesale file merge inherits
whatever the other side's generator believed the world contained.

⚠ **`next_room` makes this worse than it was.** The exit chain is authored in the
file now, so a dropped level is also a dangling successor: 1-2 points at a room
that no longer exists, and the circuit test fails a level later than the deletion.
