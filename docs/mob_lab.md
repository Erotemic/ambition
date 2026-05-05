# Mob lab encounter

A scripted basement encounter room with seldom_state-vocabulary
state, camera zoom-out on engagement, and a reset switch outside the
arena that clears the persisted defeat state.

## Layout (LDtk)

The `mob_lab` active area is a single 1024×768 level with two
chambers separated by a divider wall:

```
┌──────────────────────────────────────────┐
│ [back door]  [Switch]   ┃   [arena]      │
│                          ┃   [enemy]     │
│                          ┃   [enemy]     │
│  antechamber             ┃   [enemy]     │
│                                          │
└──────────────────────────────────────────┘
                ↑                ↑
                EncounterTrigger threshold
```

- `LoadingZone lab_entry` (left edge) connects bidirectionally to
  `central_hub_complex/lab_door` in `central_hub_basement`.
- `Switch mob_lab_reset_switch` sits in the antechamber. Walking up
  to it and pressing Interact resets the encounter (clears the
  persisted defeat state and returns the registry to `Inactive` so a
  fresh attempt is available).
- `EncounterTrigger` AABB straddles the divider opening. Player
  enters → encounter advances to `Active`, exits seal, camera zooms
  to `1.6`.
- Three `EnemySpawn` markers form wave 1 (sandbag dummies for now).
- `CameraZone mob_lab_arena` covers the arena half so future
  presentation can override the camera bounds during the fight.

The whole area was authored by re-runnable Python:

```bash
python tools/register_ldtk_entity_def.py \
    tools/specs/encounter_and_switch_entities.yaml --in-place
python tools/author_ldtk_area.py tools/specs/mob_lab_area.yaml
python tools/add_ldtk_entity_to_level.py tools/specs/hub_lab_door.yaml --in-place
```

Re-run any of those if the LDtk file gets out of sync; the tools
refuse to clobber existing content and `validate_ambition_ldtk.py`
runs automatically.

## Runtime

```
Engine — seldom_state vocabulary (state_machines.rs)
  EncounterDormant
  EncounterStarting { remaining }
  EncounterActive { wave_index, remaining_mobs, total_waves }
  EncounterCleared
  EncounterFailed
  SwitchOff { id } / SwitchOn { id }

Sandbox — encounter.rs
  EncounterRegistry (Resource)
    encounters: BTreeMap<String, EncounterState>
    .ensure(id) / .get(id) / .any_lock_active() / .active_camera_zoom()
  EncounterController (Component on per-encounter entity, marker)
  SwitchActivationQueue (Resource): drained by update_encounters_from_world
  SwitchActivation { id, action, target_encounter }
    parse_custom("switch:<id>:<action>:<target>") -> Option<Self>
```

### State machine

The encounter resource is the source of truth for the live phase.
Each encounter also has a `seldom_state`-style controller entity
that carries one of the engine state components matching the live
phase, so HUD / debug / future per-entity logic can query by
component without touching the resource. Keeping both in sync is
done by `sync_encounter_controller_states`, which removes all
encounter state components and inserts the matching one whenever
the registry changes.

The `StateMachinePlugin` from `seldom_state` is registered (in
`add_simulation_plugins`) so future patches can promote the encounter
to a transition-driven state machine without restructuring.

### Camera zoom

`camera_follow` reads `EncounterRegistry::active_camera_zoom()`
each frame. When any encounter is in `Active` and its spec has
`camera_zoom > 1.0`, the orthographic camera scales by that factor.
Overview-camera dev mode still trumps encounter zoom.

### Switch → encounter reset

When the player overlaps the switch and presses `Interact`,
`FeatureRuntime::update` pushes the switch's `Custom("switch:...")`
payload into `FeatureEvents::switch_activations`. `sandbox_update`
parses each payload and pushes a `SwitchActivation` into the
`SwitchActivationQueue`. `update_encounters_from_world` drains the
queue, applies the matching reset to the targeted encounter, toggles
the persisted switch state in the save, and clears the persisted
encounter state.

## Persistence

The defeat state of each encounter and the on/off state of each
switch live in `~/.local/share/ambition/sandbox_save.ron` (XDG/macOS/
Windows-conventional path). See `docs/save_and_settings.md` for the
full file layout and load/save semantics.

`update_encounters_from_world` projects the live phase to
`PersistedEncounterState` each frame and writes via
`SandboxSave::data_mut`; the change-detection-based
`autosave_sandbox_save` system writes to disk on the next frame.
`Active` and `Inactive` collapse to `Untouched` (no entry); `Cleared`
and `Failed` survive.

`Switch` payloads with `action: "ResetEncounter"` clear the persisted
encounter state for `target_encounter` and toggle the switch's own
persisted on/off.

## Tests

`cargo test -p ambition_sandbox --lib encounter::` covers (17 tests):

- state machine lifecycle (entry, multi-wave clear, death-during-
  active failure, retry reset, lock state, HUD summary),
- `SwitchActivation::parse_custom` (full / empty target / non-switch),
- `EncounterRegistry` (ensure, camera zoom selection, fallback to 1.0),
- `EncounterState::apply_persisted` / `to_persisted` (Active collapses
  to Untouched, Cleared keeps lock off),
- `load_encounter_specs_from_ldtk` against the embedded `mob_lab`
  area (wave count, camera_zoom > 1.0, persisted state passthrough).

## What's deferred

- **Actual mob spawning during waves.** The encounter system
  surfaces `EnemySpawned` events with the spec's `kind` strings
  ("sandbag_finite") but doesn't yet plug into `FeatureRuntime`'s
  enemy spawn pipeline — the LDtk EnemySpawn markers in the level
  spawn directly via the existing JSON-adapter path. A future
  follow-up replaces those markers with encounter-spawned enemies
  so retries actually re-spawn fresh dummies.
- **Exit lock enforcement.** `EncounterRegistry::any_lock_active()`
  returns true while an encounter is Active, but `room_transition_phase`
  doesn't yet consult it. A future patch suppresses LoadingZone
  routing while a lock is active.
- **Multi-wave authoring from LDtk.** Today every EnemySpawn marker
  in the area collapses into wave 1; a future patch reads a `wave`
  Int field on each `EnemySpawn` to assemble multi-wave specs.
- **Switch toggle visual.** The switch toggles its persisted state
  but no sprite swap happens; future patches can render the on/off
  state via the `SwitchOn` / `SwitchOff` engine components.
