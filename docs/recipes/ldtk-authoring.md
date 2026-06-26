# LDtk authoring workflow

Ambition treats `crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk` as the sandbox world source. LDtk owns authored spatial data; Ambition owns runtime gameplay semantics, validation, persistence, and hot-reload policy.

Agents should not hand-edit LDtk JSON. Use the `ambition_ldtk_tools` package so edits are repaired, normalized, and validated before write.

## Standard commands

Run from the repo root:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools repair \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
  --in-place

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools roundtrip \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools validate \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk
```

### Multi-file worlds: validate with `--secondary-world`

`intro.ldtk`, `you_have_to_cut_the_rope.ldtk`, and
`hall_of_characters.ldtk` (plus any future zone file) are merged on top of
`sandbox.ldtk` by the runtime loader
(`ldtk_world/loading.rs::secondary_world_ids`), so their `LoadingZone`s
legitimately target rooms that live in `sandbox.ldtk` (e.g.
`central_hub_complex`). Validating a secondary file **in isolation**
reports those cross-file targets as `error: ... targets unknown room`
false positives. Always pass the other world(s) so the validator resolves
cross-file links:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools validate \
  crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk \
  --secondary-world crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk
```

(Use `python3` if `python` is unavailable. `--secondary-world` may be
repeated.) A clean run prints `... passes Ambition LDtk validation (N
warnings)`. Note that `intgrid paint`/`erase` re-serialize the whole file
to the tool's canonical JSON formatting; a first edit on a drifted file
produces a large whitespace diff (one-time normalization), after which
edits diff cleanly.

### Regenerating the Hall of Characters

The Hall is generated wholesale from `character_catalog.ron` into its own
secondary world, `hall_of_characters.ldtk` — it is never spliced into
`sandbox.ldtk`. One command rebuilds it (scaffolding the file on first run):

```bash
PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools \
  generate hall-of-characters
```

The hub-side door (`hall_of_characters_door` in `central_hub_main`) is
permanent hand-authored content in `sandbox.ldtk`; the generated file only
carries the hall level + its `hall_of_characters_entry` zone, which
cross-targets the hub. Validate the pair with:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools validate \
  crates/ambition_gameplay_core/assets/ambition/worlds/hall_of_characters.ldtk \
  --secondary-world crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk
```

## Visual verification (collision + entity geometry)

To *see* a room's layout without launching the game (no GPU needed), render
its collision world + every authored entity (enemy/boss spawns, NPCs,
pickups, chests, breakables, hazards, doors, spawn) to a PNG you can open
or inspect:

```bash
cargo run -p ambition_gameplay_core --example render_room_geometry            # list rooms
cargo run -p ambition_gameplay_core --example render_room_geometry -- <ROOM_ID>  # -> /tmp/room_<id>.png
```

Filled boxes are collision (gray=Solid, blue=OneWay, red=Hazard,
gold=PogoOrb); outlines are authored entities. Useful for confirming a
room is enclosed, a door rests on a surface, or a boss/encounter is placed
where you expect — the same checks the validator lints, but rendered.

## Standard edit loop

1. Run `doctor` on the current LDtk file.
2. Open the file in LDtk.
3. Edit levels, IntGrid collision, entities, loading zones, and metadata.
4. Save in LDtk.
5. Run `repair --in-place`, `roundtrip`, and `validate`.
6. Inspect the diff before committing.

## Runtime testing

Test an alternate map without recompiling:

```bash
cargo run -p ambition_app --bin ambition_game_bin -- --ldtk mods/my_world.ldtk
AMBITION_LDTK=mods/my_world.ldtk cargo run -p ambition_app --bin ambition_game_bin
```

For a self-contained executable, build with `--features static_map`; the checked-in `sandbox.ldtk` is embedded and used as fallback if the external map is missing or invalid.

For dev hot reload:

```bash
cargo run -p ambition_app --bin ambition_game_bin --features dev_hot_reload --release
```

Hotkeys:

- `F11`: validate/apply the on-disk LDtk file.
- `F12`: toggle auto-apply after file changes.
- `F5`: overview camera.

## Programmatic authoring

Area and entity specs live under `tools/ambition_ldtk_tools/specs/`.

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/goblin_encounter_area.yaml \
  --dry-run

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/goblin_encounter_area.yaml \
  --apply

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity add \
  tools/ambition_ldtk_tools/specs/hub_lab_door.yaml \
  --in-place
```

Prefer existing specs as examples. If a spec path in an old doc mentions the retired tools/examples/ldtk_specs/ directory or a top-level the retired author_ldtk_area.py script, that doc is stale.

## Collision representation

The canonical static-collision representation is the `Collision` IntGrid layer. Entity names such as `Solid`, `OneWayPlatform`, `BlinkWall`, and `HazardBlock` remain part of the editor/tool vocabulary, but current tooling lowers static rectangles into IntGrid cells where appropriate.

Common meanings:

```text
1 = Solid
2 = OneWayUp
3 = BlinkSoft
4 = BlinkHard
5 = Hazard
```

`DamageVolume` remains an entity because it may carry motion paths and per-volume damage that IntGrid cells cannot represent.

## Supported entity identifiers

```text
PlayerStart
Solid
OneWayPlatform
BlinkWall
HazardBlock
PogoOrb
ReboundPad
LoadingZone
DamageVolume
KinematicPath
NpcSpawn
PickupSpawn
ChestSpawn
Breakable
EnemySpawn
BossSpawn
DebugLabel
CameraZone
StitchedBoundary
WaterVolume
```

## Optional official schema validation

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools schema fetch
uv pip install jsonschema
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools validate \
  --schema tools/ambition_ldtk_tools/schemas/ldtk/JSON_SCHEMA.json \
  --require-schema \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk
```

The official schema catches editor-format problems. Ambition validation catches game-specific problems such as invalid loading-zone graph links, unsafe transition arrivals, missing active areas, stale definition IDs, and unknown music tracks.

Related docs: `docs/tools/ldtk-tools.md`, `docs/systems/ldtk-world-composition.md`, `docs/systems/transition-spawn-validation.md`, `docs/systems/ldtk-hot-reload.md`.
