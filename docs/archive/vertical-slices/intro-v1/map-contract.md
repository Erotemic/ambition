# Intro v1 map contract

Status: live audit. Original Task 01 snapshot was 2026-05-21; Section 1 was
re-synced at the end of Tasks 02–08 in the same session. This document
grounds Task 02–09 in the actual repo state. It is not the scaffold; see
`scaffold.md` for the full design north star. See `playtest-handoff.md` for
the post-Task-08 route graph and durable-state inventory.

When this file disagrees with the live LDtk, the live LDtk wins. Update this
contract whenever a room is resized, renamed, or rewired.

## 1. Current intro room list (post-Task 08)

Live JSON inspection (`crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk`):

```text
intro_wake_room          1024 x 384    world (0,        0)     biome=lab
intro_raid_corridor      1600 x 512    world (1024,     0)     biome=lab
intro_escape_shaft       1280 x 1280   world (2624,     0)     biome=lab     ★ Task 02 reshape
drain_alley              1024 x 1024   world (3904,     0)     biome=cave    ★ Task 03 reshape
gate_stack_lower         1600 x 768    world (4928,     0)     biome=lab     ★ Task 05 patches
under_town_pipes         1024 x 768    world (3904,  1024)     biome=cave    ★ Task 04 new
alice_relay              1024 x 768    world (4928,  1024)     biome=cave    ★ Task 04 new
bob_relay                1024 x 768    world (5952,  1024)     biome=cave    ★ Task 04 new
combat_calibration_lab   1280 x 768    world (6528,     0)     biome=lab     ★ Task 06 new
first_system_boss        1280 x 768    world (7808,     0)     biome=lab     ★ Task 07 new
pirate_sky_arena         2400 x 1024   world (108000, -1024)   biome=outdoor
```

11 levels. The Task 04 cartography row (under_town_pipes / alice_relay /
bob_relay) sits at worldY=1024 directly under drain_alley / gate_stack_lower
so the gridvania view reads "Drain Market on top, pipes underneath".

`worldLayout` is `GridVania`. The world grid cell is 16×16 (px). The intro feed
runs left-to-right in a single horizontal strip at world_y=0 (every room sits
flush at the top of the gridvania row, regardless of pxHei). pirate_sky_arena
is parked far to the right (worldX=108000) so it does not collide with the
intro spine. Any new intro-v1 room must either slot into the right tail or
park at a clean offset away from the spine, then rewire the corresponding
LoadingZone targets.

## 2. Current LoadingZone graph

```text
intro_wake_room
  Door     wake_room_arrival       -> central_hub_complex / intro_wake_door    (bidi, cross-world)
  EdgeExit wake_to_raid            -> intro_raid_corridor / raid_from_wake     (bidi)

intro_raid_corridor
  EdgeExit raid_from_wake          -> intro_wake_room / wake_to_raid           (bidi)
  EdgeExit raid_to_escape          -> intro_escape_shaft / escape_from_raid    (bidi)

intro_escape_shaft
  EdgeExit escape_from_raid        -> intro_raid_corridor / raid_to_escape     (bidi)
  EdgeExit escape_to_drain         -> drain_alley / drain_from_escape          (bidi)

drain_alley
  EdgeExit drain_from_escape       -> intro_escape_shaft / escape_to_drain     (bidi)
  EdgeExit drain_to_gate_stack     -> gate_stack_lower / gate_stack_from_drain (bidi)

gate_stack_lower
  EdgeExit gate_stack_from_drain   -> drain_alley / drain_to_gate_stack        (bidi)
  Door     intro_portal_zone       -> central_hub_complex / intro_wake_door    (one-way, cross-world)
  EdgeExit pirate_sky_up           -> pirate_sky_arena / sky_arrival           (bidi, ceiling exit)

pirate_sky_arena
  EdgeExit sky_arrival             -> gate_stack_lower / pirate_sky_up         (bidi, floor entry)
```

Reading: today's spine is a flat 5-room left-to-right corridor (wake → raid →
escape → drain → gate stack) plus an upward EdgeExit to the pirate sky tease.
There is no under-town branch, no Alice/Bob relay, no combat-lab branch, and
no system-boss room yet. Two Door zones cross over into `sandbox.ldtk`'s
`central_hub_complex` (the intro-v1 main route does not depend on them but
they exist for the existing cold-launch path).

## 3. Current entity vocabulary in intro.ldtk

Per-room counts (from `entityInstances`):

```text
intro_wake_room       PlayerStart×1  LoadingZone×2  CameraZone×1  DebugLabel×2  Prop×4  NpcSpawn×1
intro_raid_corridor   PlayerStart×1  LoadingZone×2  CameraZone×1  DebugLabel×2  EnemySpawn×2  NpcSpawn×1
intro_escape_shaft    PlayerStart×1  LoadingZone×2  CameraZone×1  DebugLabel×2
drain_alley           PlayerStart×1  LoadingZone×2  CameraZone×1  DebugLabel×4  NpcSpawn×2
gate_stack_lower      PlayerStart×1  LoadingZone×3  CameraZone×1  DebugLabel×10  NpcSpawn×2  Prop×2
                       OneWayPlatform×2  Switch×1
pirate_sky_arena      PlayerStart×1  LoadingZone×1  CameraZone×1  DebugLabel×1  EnemySpawn×3  OneWayPlatform×1
```

All intro rooms ship two IntGrid layers: `Collision` (load-bearing) and
`Climbable` (currently zero cells across every intro room). No room currently
authors a `WaterVolume`, `DamageVolume`, `HazardBlock`, `MovingPlatform`,
`KinematicPath`, `LockWall`, `ChestSpawn`, `PickupSpawn`, `BlinkWall`,
`PogoOrb`, `BreakablePlatform`, `BreakablePogoOrb`, `BossSpawn`,
`EncounterTrigger`, `StitchedBoundary`, or `ReboundPad`. Their entity
definitions all exist in `defs.entities` (see Section 4) so Task 02+ can
place them through `entity add` without registering new defs.

## 4. Entity definition vocabulary available in intro.ldtk

Field-by-field reference for the entities Task 02–09 will likely place:

```text
PlayerStart        : name
Solid              : name
LoadingZone        : id, name, activation, target_room, target_zone, bidirectional
OneWayPlatform     : name
BlinkWall          : name, tier
PogoOrb            : name
NpcSpawn           : name, prompt, dialogue_id, patrol_radius, path_id
DebugLabel         : name, text, category
EnemySpawn         : name, brain, path_id
HazardBlock        : name
ReboundPad         : name, impulseX, impulseY
DamageVolume       : name, damage, path_points, path_speed, path_mode, path_id
KinematicPath      : name, points, speed, mode, id
BossSpawn          : name, brain
BreakablePlatform  : name, max_hp, respawn, respawn_seconds, collision, trigger
BreakablePogoOrb   : name, max_hp, respawn, respawn_seconds
PickupSpawn        : name, kind
ChestSpawn         : name, reward
CameraZone         : id, name, mode, priority, zoom, target_offset_x/y, easing_hz,
                     cinematic_lock, clamp_mode
StitchedBoundary   : id, name, seam
EncounterTrigger   : id, name, camera_zoom
Switch             : id, name, prompt, target_encounter, action
LockWall           : id, name
WaterVolume        : id, name, gravity_scale, drag, max_fall_speed, swim_up_impulse
MovingPlatform     : name, sweep_dx, speed, path_id
Prop               : name, kind
```

This is the toolbox. Tasks 02–07 should pull from these before proposing new
entity defs. If a new entity def is truly needed, register it via
`def register-entity` and update this contract.

## 5. Current area spec ownership

Specs in `tools/ambition_ldtk_tools/specs/`:

```text
intro_wake_room_area.yaml         OWNED  keep stable; wake room is anchor
intro_raid_corridor_area.yaml     OWNED  tune (do not overbuild)
intro_escape_shaft_area.yaml      OWNED  reinterpret as vertical ascent in Task 02
                                          ⚠ spec world_x=104000 disagrees with live
                                          intro.ldtk world_x=2624 — spec drifted from
                                          live data. Treat live LDtk as truth; either
                                          fix the spec or do not re-apply it blindly.
drain_alley_area.yaml             OWNED  expand into Drain Market knot in Task 03
gate_stack_lower_area.yaml        OWNED  reinterpret as right utility switchback in
                                          Task 05
(no spec)                                 pirate_sky_arena ships only as raw LDtk —
                                          future promise, not on intro-v1 main route
```

New specs Task 02–07 will likely need:

```text
intro_escape_shaft_area.yaml      reshape from 1280×512 horizontal corridor →
                                  ≤1024×1536 vertical ascent (Task 02)
drain_market_knot_area.yaml       drain_alley expansion or sibling level (Task 03)
under_town_pipes_area.yaml        new (Task 04)
alice_relay_area.yaml             new (Task 04)
bob_relay_area.yaml               new (Task 04)
combat_calibration_lab_area.yaml  new (Task 06)
first_system_boss_area.yaml       new (Task 07)
```

## 6. Intended intro-v1 topology

Primary spine:

```text
intro_wake_room
  -> intro_raid_corridor
    -> intro_escape_shaft (rebuilt vertical)
      -> drain_alley / Drain Market Main
        -> under_town_pipes              [Task 04]
        -> gate_stack_lower              [Task 05 reinterpretation]
        -> [forest tease / future]
        -> [sky tease via pirate_sky_up — exists]
```

Extension topology:

```text
under_town_pipes -> alice_relay -> bob_relay -> alice_relay return        [Task 04]
right_utility_switchback (gate_stack_lower) -> combat_calibration_lab    [Task 05/06]
combat_calibration_lab -> first_system_boss -> return shortcut           [Task 07]
```

## 7. Coordinate & size constraints

Findings from the spec/file audit, recorded here so Task 02+ does not have to
re-derive them:

- worldLayout is `GridVania` with a 16×16 cell grid. Level pxWid / pxHei must
  be multiples of 16. The current intro rooms all comply.
- All five intro spine rooms sit at `worldY = 0`. Their right edges meet the
  next room's left edge exactly. Resizing a room laterally (changing pxWid)
  shifts every downstream room's worldX. Task 02 should bias toward keeping
  the escape-shaft's footprint ≤1024px wide so the existing drain_alley and
  gate_stack_lower coordinates can stay put.
- Vertical growth: making intro_escape_shaft taller (e.g. 1024×1280 or
  1024×1536) does NOT collide with neighbors because nobody else is below
  worldY=0 in the intro strip. Camera zones currently match level size 1:1
  so the camera follows pxHei out of the box. Confirm CameraZone gets resized
  in lockstep when the new spec is applied.
- EdgeExit zones assume horizontal movement: today's intro EdgeExits are all
  16-wide vertical slabs glued to the left or right edge of a level, with
  bidirectional partners on the neighbor. The pirate sky link is the
  exception (16-tall horizontal slab on a top/bottom edge). Vertical-stack
  EdgeExits are supported but rare; Task 02's ascent should still land in a
  rightward EdgeExit at the top of the shaft so the gridvania flow stays
  intelligible.
- Door zones (e.g. `wake_room_arrival`, `intro_portal_zone`) target
  `central_hub_complex` which lives in `sandbox.ldtk`. Cross-file resolution
  works at runtime (and at validation time via `--secondary-world`). Do not
  reuse the same zone id across worlds.
- PlayerStart pixel positions in the existing intro rooms are placed ~110px
  in from the left edge and a couple of tiles above the floor (e.g. drain
  spawn = (120, 434) with floor at y≈480). Keep this convention; the
  cold-launch path picks the first PlayerStart in the active room.

## 8. Cartography / map-layer implementation status

Existing code surfaces (so Task 04 / Task 08 can wire to them rather than
inventing parallel systems):

```text
crates/ambition_gameplay_core/src/map_menu/model.rs
  MapMenuState resource (open, minimap_enabled, visited:BTreeSet<String>,
                        rooms:Vec<MapRoomNode>, zoom).
  Auto-fills visited via push_room_entered_quest_events on RoomEntered.

crates/ambition_engine/src/interaction.rs
  PickupKind::StoryFlag { flag: String }
  -> PickupSpawn entities can drop a string flag into the save / quest stream.

crates/ambition_engine/src/quest.rs
  QuestAdvanceEvent::FlagSet(String)
  QuestStepCondition::FlagSet(String)
  Quest registry persists changed_ids into SandboxSave.

crates/ambition_content/src/quest.rs
  push_room_entered_quest_events fires RoomEntered when active room flips.
  apply_quest_advance_events drains pending events into the registry.
  Existing flag usage: 'met_any_hub_npc', 'test_switch_toggled',
                       'npc_pirate_admiral_talked'.
```

What is missing for Task 04 / 08:

- No `map_basic_unlocked`, `map_private_marks_unlocked`, `bob_field_survey_received`,
  `route_memory_received` flags are written anywhere yet. They can be added as
  plain story-flag strings (no schema change) once a PickupSpawn or Switch
  fires them.
- No per-room map labels (MAP_PRIVATE / MAP_OFFICIAL / MAP_DANGER / MAP_SECRET /
  MAP_WATCHED). Suggested implementation: use `DebugLabel.category` (currently
  always `Custom`) as the channel — extend the enum / branch on category at
  runtime if/when a label-aware HUD lands. Until then, encode the label class
  in DebugLabel.name (e.g. `name = "map_label_private:alice_path"`).
- No "private route" door type. `LockWall` plus a story-flag condition is the
  cheapest first cut; the lockwall's gate condition would check a flag like
  `map_private_marks_unlocked`.

These are recommendations, not commitments. Task 04 should re-evaluate and
choose the lightest mechanism that lets the slice ship.

## 9. Validation commands attempted (2026-05-21)

```bash
PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools list-metadata \
  --ldtk crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk
# OK — prints biome/music per area.

PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools doctor \
  crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk
# Fails (4 errors): LoadingZones in intro_wake_room and gate_stack_lower
# target central_hub_complex which lives in sandbox.ldtk. doctor does not
# accept --secondary-world.

PYTHONPATH=tools/ambition_ldtk_tools python3 \
  tools/ambition_ldtk_tools/ambition_ldtk_tools/validate.py \
  --secondary-world crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
  crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk
# OK: 0 warnings. This is the correct validation incantation for intro.ldtk
# whenever a LoadingZone crosses into central_hub_complex.
```

`cargo fmt --check` / `cargo test -p ambition_gameplay_core --lib` / `cargo run
-p ambition_gameplay_core --bin headless` were not run as part of this task (no
code edits). Task 02 should run them after the shaft rewrite.

## 10. Known tooling blockers / gotchas

- `entity query` is a placeholder in the CLI (`entity query [filters]` exits
  immediately with `usage: ...`). Use ad-hoc Python over `intro.ldtk` JSON
  for inspection (see `/tmp/inspect_intro.py` and `/tmp/inspect_zones.py`
  patterns used to build sections 1–4 of this contract). Filing a TODO in
  this contract rather than the tool: extending `entity query` to take
  `--ldtk`, `--level`, `--type`, and `--field key=value` would simplify the
  next audit.
- `doctor` does not currently forward `--secondary-world` to the underlying
  validator, so it produces false-positive cross-world errors on intro.ldtk.
  Until that is fixed, always validate intro.ldtk with the explicit
  `validate.py --secondary-world ... sandbox.ldtk` form documented above.
- `area create --dry-run` is supported on individual area YAMLs; pass
  `--ldtk crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk` so the
  add lands in the intro world, NOT in sandbox.ldtk (which is the CLI
  default).
- `intro_escape_shaft_area.yaml` ships with `world_x: 104000` while the live
  level sits at `worldX: 2624`. Do not re-apply the spec as-is or you will
  duplicate the level off in some far corner of the world. Task 02 should
  rewrite the spec (and the world_x/world_y) in lockstep with whatever new
  shape it picks for the shaft.
- `Climbable` IntGrid is authored everywhere but populated nowhere. If
  Task 02 wants ladders / climbs in the vertical shaft, that layer is the
  intended target (rather than a brand-new entity).
- `cargo` and `rustc` live at `~/.cargo/bin/`; ensure that's in PATH before
  running the validation commands in section 9.

## Task 02 handoff

Concrete starting points for `task-02-vertical-escape-shaft.md`:

1. Plan the new shaft footprint. Recommended: 1024×1280 (keeps lateral
   neighbors put, gives ~2.5 screens of vertical travel at the existing
   512-tall camera height). 1024×1536 is also fine and parallels the
   pirate_sky_arena's vertical headroom. Avoid going wider than 1024 unless
   you also shift drain_alley and gate_stack_lower right; that work is out
   of scope for Task 02.
2. Decide entry/exit. The left-edge EdgeExit `escape_from_raid` lives at
   `(0, 0) – (16, 480)`. Keep its band low so the player lands on the floor
   of the new shaft. The exit to drain_alley (`escape_to_drain`) should
   remain a right-edge EdgeExit but climb to a band near the top of the
   shaft so "reaching the top" is the gate. drain_alley's matching zone
   (`drain_from_escape`) sits at `(0, 0) – (16, 480)`, so either keep it
   at y=0 of drain_alley or move it; both options are open.
3. Use existing entities (`OneWayPlatform`, `Solid`, `HazardBlock`,
   `ReboundPad`, `BreakablePlatform`, `MovingPlatform`, `Climbable` IntGrid)
   before introducing anything new. Single readable mechanic per beat; see
   the encounter ladder in `scaffold.md` §"Encounter and puzzle ladder".
4. Fix the spec drift on `intro_escape_shaft_area.yaml`: either update
   world_x/world_y to match the live position (2624, 0) before reshaping,
   or rewrite the spec from scratch as the source of truth and re-apply it
   to intro.ldtk with `--ldtk crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk`.
5. After the shape lands, re-run the section-9 validate command and a quick
   `cargo run -p ambition_gameplay_core --bin headless` smoke check. Update this
   contract's section 1 with the new pxWid/pxHei and update section 5's
   ownership note.

What was validated for this task: intro.ldtk + sandbox.ldtk pass the
secondary-world validate pass. No code or LDtk content was changed.

What remains placeholder: every cartography hook, every Alice/Bob room,
every Task 04+ branch. This contract names them but does not implement them.

What felt fun or unreadable: not playtested. Empty observation.

Which room/route should be worked on next: `intro_escape_shaft` (Task 02).
