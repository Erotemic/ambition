# Adding a sandbox showcase room

This walkthrough documents the workflow used to land
`crawl_lab` / `morph_lab` / `ladder_lab` / `cutscene_lab` / `quest_lab`
in 2026-05-07. Use it as a recipe when adding the next
basement-reachable showcase room.

## Step 1 — find a free door slot in the basement

```bash
python tools/author_ldtk_area.py <any-spec.yaml> --list-free-spots central_hub_basement
```

Output lists every free 48x96 gap along the basement door row at
y=832 with a suggested centerpoint. Pick a gap with at least ~150px
width so the door sits cleanly between its neighbors.

## Step 2 — write a yaml spec

Copy `tools/examples/ldtk_specs/cutscene_lab.yaml` as a starting
template — it's the shortest one (no IntGrid layers, no chest,
no special wiring beyond floor/walls/PlayerStart/LoadingZone). Edit:

- `id` and `level_id`: pick a unique active-area name
- `world_x` / `world_y`: pick coords that don't overlap any existing
  level. The current convention is to put new labs on a row at
  y=1024, x=26000+ (each lab gets ~1000-2000px width).
- `px_wid` / `px_hei`: room size, multiples of 16
- `biome: lab` (required — every active area must declare a biome,
  pinned by the `embedded_ldtk_active_areas_have_biome_metadata` test)
- `connect_to.target_room: central_hub_complex` (NOT
  `central_hub_basement` — `target_room` is the destination's
  activeArea, and the basement is part of `central_hub_complex`)
- `connect_to.target_zone: <new_room>_door` (matches the source-side
  LoadingZone's `target_zone`)
- `connect_to.px`: the px from step 1
- `entities[*]`: see existing specs for floor / walls / ceiling /
  PlayerStart / LoadingZone / DebugLabel patterns. ChestSpawn fields
  are `name: Chest` and `reward: health:N` — the spec rejects any
  other fields.

## Step 3 — dry run

```bash
python tools/author_ldtk_area.py tools/examples/ldtk_specs/<new>.yaml --dry-run
```

Verify the preview matches intent (entity counts, exit links,
IntGrid cell totals, reciprocal LoadingZones). Fix any errors
before writing.

## Step 4 — apply

```bash
python tools/author_ldtk_area.py tools/examples/ldtk_specs/<new>.yaml
```

The tool runs repair + validate as a post-pass. On success, the LDtk
file has the new level appended and `central_hub_basement` has a
matching reciprocal LoadingZone.

## Step 5 — run smoke checks

```bash
cargo test -p ambition_sandbox --lib                            # 376+ lib tests
cargo run -p ambition_sandbox --bin headless -- 30              # boots the sim
cargo run -p ambition_sandbox --bin rl_smoke -- 50              # visits every room
```

The basement-reachability test in `ldtk_world.rs::tests::embedded_ldtk_includes_basement_reachable_body_mode_rooms`
pins the showcase rooms by name; if you want your new room to be
required-reachable (most do), add it to the test's required list.

## Step 6 — wire any system bindings

If the room demonstrates a system that needs side-channel hookup:

- **Cutscene**: add a `<room>_intro` script to
  `default_cutscene_library()` and a binding in
  `RoomCutsceneBindings::defaults()`. Mirror `cutscene_lab`'s shape.
- **Quest**: add a `QuestSpec` to `default_quest_specs()` and
  auto-start it in `populate_quest_registry`. Mirror
  `quest_lab_visit`'s shape.
- **Switch / persistence**: drop a `Switch` entity in the entities
  list. The Switch system fires `set_flag("test_switch_toggled")`
  on toggle (already wired in `encounter.rs`).
- **Climbable cells**: add an `intgrid.climbable` block to the spec.
  Tooling automatically lowers cells into the project's Climbable
  IntGrid layer. Cell values: 1=Ladder, 2=Vine, 3=Wall.

## Step 7 — update FEATURES.md and TODO.md

Add an entry in FEATURES.md under "Sandbox showcase rooms" with the
mechanic the room demonstrates. If the room closes a TODO row, mark
the row `[x]` and link to the new room.

## Anti-patterns to avoid

- **Don't put `target_room` in a LoadingZone equal to a level
  identifier.** The runtime treats it as the destination's
  activeArea name, not a level id. The basement is
  `central_hub_complex`, not `central_hub_basement`.
- **Don't reuse a `target_zone` value across LoadingZones**. Each
  zone id must be globally unique.
- **Don't pick a `connect_to.px` without running `--list-free-spots`
  first.** The basement door row gets crowded fast; overlapping
  placements are rejected by the tool but can produce confusing
  error messages.
- **Don't forget `biome: lab` (or other biome).** The
  `embedded_ldtk_active_areas_have_biome_metadata` test will fail
  without it.
- **Don't author ChestSpawn with `id`, `reward_amount`, or other
  fields not in the entity def.** `name` and `reward` are the
  authored set; reward format is `health:N` (or `mana:N`,
  `dash_charge:N`).
