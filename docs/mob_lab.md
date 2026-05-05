# Mob lab encounter

A scripted basement encounter room with hallway → arena layout, a
dynamically-spawned lock wall, intro phase with music swap and camera
zoom, multi-wave enemy spawning (including delayed sub-spawns), and
a free-toggle reset switch outside the arena.

## Map (LDtk)

`mob_lab` is a single 1600×768 level with two chambers separated by a
divider gap and jambs. The whole level was authored from
`tools/specs/mob_lab_area.yaml` via:

```bash
python tools/register_ldtk_entity_def.py \
    tools/specs/encounter_and_switch_entities.yaml --in-place
python tools/register_ldtk_entity_def.py \
    tools/specs/lockwall_entity.yaml --in-place
python tools/author_ldtk_area.py tools/specs/mob_lab_area.yaml
python tools/add_ldtk_entity_to_level.py \
    tools/specs/hub_lab_door.yaml --in-place
```

```
+----------------------------------------------+
|                                              |  ← arena ceiling
|   hallway (closed)         |  arena (open)   |
|                            |                 |
|  ████ ceiling              |                 |
|                            |                 |
|  [door↓→hub]  [Switch] ──/ \── threshold ──── |
|  ████████████████████  ████████████████████  | ← floor (continuous)
+----------------------------------------------+
                               ↑
                  EncounterTrigger AABB starts the encounter
                  LockWall fills the gap when Active
```

- `PlayerStart` lands on the hallway floor — no more falling from a
  ceiling.
- `Switch mob_lab_reset_switch` is in the hallway, between the hub
  door and the divider gap. Pressing Interact toggles between
  Cleared (green) and Inactive (red); free toggle in the sandbox.
- `LockWall mob_lab_lock` fills the doorway gap (224×384) only while
  the encounter is in `Starting` or `Active`.
- `EncounterTrigger mob_lab` sits just inside the arena.
- `CameraZone mob_lab_arena` covers the arena half.

## Wave script (hard-coded in `crate::encounter::mob_lab_wave_specs`)

| Wave | Mobs                                           | Timing                  |
| ---- | ---------------------------------------------- | ----------------------- |
| 1    | 2 × `medium_striker` (one each side)           | Spawn together at wave start |
| 2    | 2 × `medium_striker` + 1 × `large_brute`       | 2 goblins immediate; big goblin at +3.5s (regardless of wave-2 clear) |
| 3    | 2 × `large_brute`                              | Spawn together         |

Sandbag archetypes are intentionally avoided: their built-in respawn
timer would prevent waves from clearing. The dynamic enemy spawner
sets `respawn_timer = 999_999.0` on every encounter spawn as belt-
and-braces.

## Phase machine

```
Inactive  ──player crosses trigger──▶  Starting{remaining}
                                         │
                                         │ tick down by dt
                                         ▼
                                       Active{wave_index, remaining_mobs}
                                         │   │   │
                                         │   │   └── wave clears ──▶ Cleared
                                         │   │
                                         │   └── all waves done ──▶ Cleared
                                         │
                                         └── player dies ──▶ Failed
```

`Starting` (intro window — 2.5s by default) is when the camera
zoom + lock wall + music swap apply, but no mobs spawn yet. Wave 1
mobs only land once `Starting` elapses.

The seldom_state state-component vocabulary
(`EncounterDormant` / `EncounterStarting` / `EncounterActive` /
`EncounterCleared` / `EncounterFailed` from
`ambition_engine::state_machines`) is mirrored onto a per-encounter
`EncounterController` Bevy entity so HUD / debug systems can query
by state component instead of by resource lookup.

## Lock wall

`sync_lock_walls` runs each frame and:

1. Builds the desired set of `(encounter_id, min, size)` tuples from
   every encounter currently in `Starting` or `Active` whose spec
   has a `LockWall`.
2. Drops any `world.blocks` whose name starts with `lockwall:` and
   isn't in the desired set.
3. Inserts a `Block::solid("lockwall:<id>", min, size)` for any
   desired entry not already present.

Block name format `lockwall:<encounter_id>` lets the system find and
remove only its own blocks; static LDtk solids are unaffected.

## Camera zoom

`EncounterRegistry::active_camera_zoom()` returns the first
`Starting`/`Active` encounter's spec.camera_zoom (defaults 1.6 for
mob_lab). `camera_follow` reads it each frame and applies it as the
orthographic projection scale. Overview-camera dev mode (`F5`) still
trumps the encounter zoom.

## Music swap

The encounter spec carries a `music_track` id (`"pulse_drift_voyage"`
for mob_lab — added to `sandbox.ron` from
`tune_examples/pulse_voyage_drift.ron`). Each frame
`update_encounters_from_world` writes the desired track id to
`EncounterMusicRequest`; the audio-feature-gated
`apply_encounter_music` system swaps Kira's music channel when the
desired track changes.

When all encounters are out of `Starting`/`Active`, the desired
track collapses back to `sandbox_data.audio.default_music_track`.

## Switch behavior

The reset switch is a colored block (red = encounter armed / Inactive,
green = encounter Cleared / disabled). Pressing Interact:

1. Toggles the encounter phase between `Cleared` and `Inactive`.
2. Toggles the persisted switch on/off (so the visual color
   reads from the save unambiguously after reload).
3. Despawns any encounter mobs spawned by previous attempts via
   `FeatureRuntime::despawn_encounter_enemies`, so a fresh attempt
   starts with a clean field.

Free toggle: pressing the switch never requires beating the encounter
first, so the sandbox can swap arming on/off whenever.

## Cancellation on leave

Encounters in `Starting` or `Active` only persist while the player is
in the matching active area. Walking back through `lab_entry` snaps
the encounter to `Inactive`, releasing the lock wall and camera
zoom + reverting the music to the default track. A fresh attempt is
available next time the player re-enters the trigger.

## Persistence

- `Cleared` and `Failed` survive reload via
  `~/.local/share/ambition/sandbox_save.ron` (or OS equivalent).
- `Inactive`, `Starting`, and `Active` collapse to `Untouched`
  in the save.
- Switch on/off persists separately in the save's `switches` list,
  but is also re-derived from the encounter phase on every save
  write so the two stay in sync.

## Tests

`cargo test -p ambition_sandbox --lib encounter::` (22 tests) covers:

- state-machine lifecycle (entry, multi-wave clear, death-during-
  active failure, retry reset, lock state, HUD summary);
- intro phase delays first wave spawn until elapsed;
- delayed sub-spawn holds then fires;
- wave clears only when both pending and alive mobs are resolved;
- `sync_lock_walls` inserts and removes the named block;
- `mob_lab_loaded_spec_has_three_waves_lockwall_and_intro` verifies
  the embedded LDtk file produces the canonical wave structure;
- switch payload parsing + registry helpers as before.

## What's still deferred

- **Smooth camera zoom interpolation.** Today the scale snaps to 1.6
  on `Starting` start. A future patch can ease over `intro_seconds`.
- **Switch sprite swap.** Today the switch is a colored block. A
  future patch can swap to authored on/off sprites.
- **Multiple encounters.** The system handles multi-encounter
  registration but only mob_lab has hard-coded waves; future
  rooms register a wave builder via the same pattern.
- **Encounter UI.** No HUD wave indicator yet — `hud_summary()`
  exists but isn't wired into the on-screen overlay.
