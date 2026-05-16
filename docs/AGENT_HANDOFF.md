# Agent handoff guide

This document is for future AI agents and human contributors who need to get productive quickly without re-learning the entire project history.

## First files to read

1. `README.md`
2. `docs/CURRENT_STATE.md`
3. `docs/GOAL_STATE.md`
4. `docs/adr/README.md`
5. The focused doc for the subsystem you are editing

Historical docs are useful, but ADRs and `CURRENT_STATE.md` supersede older constraints.


## LDtk and world composition

Read `docs/adr/0009-world-composition-and-ldtk-authoring.md` and `docs/ldtk_world_composition.md` before changing sandbox level authoring. The central hub basement is a physical area below the hub, not a separate loading-zone room. The current LDtk adapter composes levels sharing the same `activeArea` field into one runtime active area, and the sandbox also spawns a hidden `bevy_ecs_ldtk` `LdtkWorldBundle` whose `LevelSet` mirrors the active Ambition area. Keep the plugin-owned root hidden until individual LDtk layers/entities are intentionally promoted to typed Ambition runtime bundles; otherwise unregistered LDtk placeholders can render as large dark rectangles over rooms. The old sandbox rooms and feature-lab doors are now represented as LDtk active areas linked by `LoadingZone` entities; do not put the boss directly in the stitched hub/basement area.

Validate LDtk edits with:

```bash
python tools/validate_ambition_ldtk.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

Do not treat LDtk JSON as the canonical gameplay model. Add adapter/validator code when new LDtk entity identifiers or fields become meaningful. The LDtk file should stay first-class editor-shaped JSON: keep root `iid`, `worldLayout`, `defs.layers`, and `defs.entities` in sync with instances. `EdgeExit` zones must be physically reachable: split wall collision around them instead of placing the trigger inside a solid wall. Use `AMBITION_REVIEW(spatial)` around chunk composition, seams, camera bounds, wall openings, and spawn/collision assumptions.

## Repository state and patch packaging

Before producing a patch, verify whether you have a full repo checkpoint or only partial context. A full checkpoint should include the root files, `crates/`, `docs/`, and any relevant assets. If you do not have the full repo state, say so explicitly in the response and keep the patch narrowly scoped to files you can inspect.

Patch zips may contain only modified files to save bandwidth, but name them as patches, for example:

```text
ambition-some-feature-patch.zip
```

Patch zips should preserve repo-relative paths exactly. Crate files belong under `crates/ambition_engine/` or `crates/ambition_sandbox/`; do not create duplicate top-level crate directories such as `ambition_sandbox/`. Documentation belongs under `docs/` unless it is the root `README.md`.

A patch zip cannot reliably delete accidentally created files or directories when applied with `bsdtar -xf ... --strip-components 1`. If cleanup is needed, include an explicit `rm` command in the response.

## Working style

- Prefer small patches with a clear intent.
- Preserve compile logs and user feedback as design information.
- Do not claim `cargo` testing unless you actually ran it.
- Patch responses should include:
  - download link,
  - apply/run commands,
  - what changed,
  - known testing limitations,
  - markdown paragraph commit message.
- When a patch creates or changes a feature, add or update a focused doc.
- When a decision supersedes older guidance, add or update an ADR.

## Environment caveat

Some agent environments do not have `cargo`, `rustc`, or `rustfmt`. If so:

- do not claim compile success,
- do structural/text checks where possible,
- keep patches smaller,
- rely on user compile logs for correction,
- prefer comments/docs/tests that are syntactically low-risk.

## Source-of-truth hierarchy

Use this order when documents disagree:

1. Fresh user instructions in the current conversation.
2. `docs/adr/*.md` for recorded decisions.
3. `docs/CURRENT_STATE.md` for active state.
4. Focused subsystem docs.
5. Historical notes and older patch docs.

If an older doc is misleading, do not delete history by default. Add a supersession note or ADR pointer.

## Spatial reasoning review convention

The project has many geometry-heavy systems where subtle mistakes are easy:

- local/world/Bevy coordinate conversion,
- camera clamping,
- loading-zone placement,
- transition arrival repair,
- AABB strict-overlap vs edge-touch semantics,
- blink shape casts,
- moving platforms and hazards,
- non-Euclidean seams/chart transforms,
- procedural room generation and reachability.

When editing such code, add a nearby comment with this marker if the logic deserves future review:

```rust
// AMBITION_REVIEW(spatial): explain the coordinate/geometry assumption here.
```

Use the marker for code that is correct enough to proceed but would benefit from a stronger spatial-reasoning pass, visualization, or property test later. Do not use it as a substitute for fixing known bugs.

Suggested follow-up searches:

```bash
grep -R "AMBITION_REVIEW" -n crates docs
```

## Testing priorities

Prefer lightweight tests before heavyweight Bevy app tests:

- pure movement step tests,
- collision and blink destination tests,
- room graph validity tests,
- spawn repair tests,
- boss schedule snapshots,
- procedural geometry finite/containment checks,
- RON round-trip tests,
- input-buffer timer tests.

`insta` snapshots are best for reviewable generated outputs. `proptest` is best for invariants over many small randomized cases.

## Data-driven direction

Favor specs that describe gameplay intent:

```text
Enemy(kind: GradientSeeker, attack: TelegraphLunge(...))
```

Avoid making data mirror low-level Bevy bundles unless a presentation system specifically needs that.

## Documentation discipline

- README: stable project portal only.
- `CURRENT_STATE.md`: current truth and known transient areas.
- `GOAL_STATE.md`: long-term direction.
- ADRs: durable decisions and supersessions.
- Patch docs: what a patch changed and why.
- Storyline docs: narrative/worldbuilding candidates.

## LDtk asset loading lesson

Keep `LdtkPlugin` before `init_collection::<loading::SandboxAssetCollection>()`. The asset collection allocates a typed `Handle<bevy_ecs_ldtk::assets::LdtkProject>`, so the LDtk asset type and loader must already be initialized before `bevy_asset_loader` creates the collection.

## LDtk hot reload operational note

For LDtk/world-composition patches, include one pasteable apply/validate/run
bash block. Prefer running the sandbox with:

```bash
cargo run -p ambition_sandbox --features dev_hot_reload --release
```

The runtime supports `F11` to validate/apply the on-disk LDtk project and `F12`
to toggle auto-apply after file changes. Keep the validator as the safety gate;
failed reloads must preserve the currently running world. When adding new map
entities, update both the Rust adapter and `tools/validate_ambition_ldtk.py`, and
make sure hot reload rebuilds the corresponding map-authored runtime state.

## LDtk FieldDef schema shape

Do not only validate entity field definitions. LDtk level fields such as
`activeArea` are also `FieldDef` records and must include the same first-class
schema keys expected by `bevy_ecs_ldtk`, including `allowedRefs`,
`allowedRefTags`, `allowOutOfLevelRef`, `autoChainRef`, `editorDisplayScale`,
`editorLinkStyle`, `editorShowInWorld`, `exportToToc`, `searchable`, and
`symmetricalRef`. A missing key can produce Bevy asset-loader errors even when
Ambition's custom JSON adapter can still read and hot-reload the map.

Field definitions have two type-looking fields with different meanings:
`__type` is the human-readable type string used in instances/docs, while `type`
is the editor's internal `FieldType` constructor. For example, a string field
should use `__type: "String"` and `type: "F_String"`. Do not write
`type: "String"`; LDtk 1.5.3 rejects that with `No such constructor String`
and may crash while trying to inspect project settings. The validator must keep
checking this for both entity field definitions and level field definitions.

## LDtk defUid compatibility lesson

Direct `bevy_ecs_ldtk` spawning depends on LDtk instance `defUid` values, not
only on `__identifier` strings. If entity instances use stale `defUid` values
that do not match `defs.entities[*].uid`, `bevy_ecs_ldtk` can panic in
`calculate_transform_from_entity_instance` while spawning a level. Keep entity
instance `defUid`, entity field instance `defUid`, and level field instance
`defUid` synchronized with their definitions whenever generating or patching
`sandbox.ldtk`, and keep `tools/validate_ambition_ldtk.py` strict about this.


## LDtk runtime spine migration

Ambition is moving from a custom LDtk JSON adapter toward `bevy_ecs_ldtk` as the runtime spine. The sandbox now registers every current Ambition LDtk entity identifier as a lightweight plugin-spawned marker bundle, keeps the LDtk world root active, disables LDtk level-background rendering, and records plugin-spawned entity lifecycle in HUD/debug state. LDtk now builds runtime `RoomSet` data directly through `RoomSet::from_parts`; the old RON-shaped world manifest structs and builders have been removed.

Official LDtk JSON Schema validation should use Python `jsonschema`, not npm. `tools/validate_ambition_ldtk.py` supports optional `--schema` and `--require-schema` flags while continuing to run Ambition-specific semantic validation without the schema file.

## LDtk / RON ownership update

Do not add new spatial/world content to `assets/ambition/sandbox.ron`. `SandboxDataSpec` no longer has a `rooms` field at runtime; RON owns non-spatial sandbox tuning/audio data only. LDtk owns spatial authoring. If a future patch needs to change rooms, levels, solids, doors, hazards, actors, or debug labels, edit `crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk` and update the LDtk validator if needed.

Do not reintroduce RON-shaped room/world manifest structs for sandbox spatial authoring. Prefer promoting plugin-spawned LDtk marker entities into typed Ambition components or extending the LDtk-native `RoomSet::from_parts` path over adding any new LDtk-to-intermediate-manifest conversion logic.

The old checked-in RON room block has been removed from `crates/ambition_sandbox/assets/ambition/sandbox.ron`. Do not reintroduce it as a fallback. If a patch needs fallback/fixture maps, put them in explicit test fixtures instead of the main sandbox tuning/audio manifest.

## LDtk bridge migration note

Prefer `LdtkProject::to_room_set()` for LDtk-derived runtime rooms. It materializes `RoomSpec`, `ae::World`, loading zones, room objects, and graph links directly. Do not reintroduce LDtk-to-RON-manifest call sites.

## LDtk editor round-trip lesson

When generating or patching `.ldtk` directly, do not leave `realEditorValues` empty for field instances that have non-null `__value`. Runtime parsers primarily read `__value`, but the LDtk editor uses `realEditorValues` while saving modified levels. Empty editor values can cause all custom fields in the touched level to be rewritten as null after a simple move operation.

Run this before handing off generated LDtk files for GUI editing:

```bash
python tools/validate_ambition_ldtk.py --normalize-editor-values crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
python tools/validate_ambition_ldtk.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

Startup should not use `.expect()` on LDtk room construction. If the embedded LDtk file is invalid, print all validator errors and exit nonzero; hot reload should continue to reject invalid edits while preserving the live world.


## LDtk authoring tooling rule

Before handing a generated or patched `.ldtk` file to the user for LDtk GUI editing, run `python tools/repair_ambition_ldtk.py --in-place crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk` and then `python tools/check_ldtk_editor_roundtrip.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk`. Use Python `jsonschema` only for official LDtk schema validation; do not add npm/Node validation tooling. If a new Ambition LDtk entity is introduced, add an LDtk entity definition with docs/colors/default field values, register it with `bevy_ecs_ldtk`, update the validator, and update `docs/ldtk_authoring.md`.
## LDtk runtime-spine update

The first promoted plugin-spawned LDtk categories are `PlayerStart`, `LoadingZone`, `DebugLabel`, and `CameraZone`. `bevy_ecs_ldtk` owns their entity lifecycle; Ambition rebuilds a runtime-spine index from spawned entities each frame for HUD/debug overlays and future direct gameplay promotion. Hot reload now prepares a replacement world transaction before mutating live state and rejects edits that delete the current active area or leave missing graph links.

## Gameplay flight recorder

- `F8` writes a `debug_traces/ambition_trace_*.json` + `.md` pair.
- The recorder also auto-dumps when it detects player OOB (non-finite
  pos/vel, AABB outside world envelope by more than the margin, AABB
  inside a `Solid`, or absurd velocity).
- Buffer lives in `crate::trace::GameplayTraceBuffer` (sandbox-side
  resource); dumps work in headless and visible builds.
- Attach both files to OOB / collision-escape bug reports. The `.md`
  is a 120-frame summary; the `.json` is the full 240-frame ring.
- See `docs/gameplay_trace_recorder.md` and the tests in
  `crates/ambition_sandbox/src/trace.rs`.

## Tier-1 player primitives in the engine

`ambition_engine::player_state` provides:

- `LocomotionState` (Grounded/Airborne/Dashing/Blinking/WallSlide/
  Crouching/MorphBall/GrappleAiming/…)
- `BodyMode` + `BodyShape::fits_at` (collision-safe resize query)
- `ResourceMeter` (stamina/mana/ammo/charge with regen+decay)

These are the backends; mechanics (crouch, morph ball, grapple,
projectile, parry, functional zip) are listed in
`crate::mechanics::MechanicsRegistry` with maturity. See
`docs/mechanics/body_modes.md`.

## LDtk roadmap step 1 (spine indices + room projection)

Step 1 of the LDtk runtime-spine roadmap promotes authored LDtk data out of
JSON-only ad hoc readers and into typed Ambition indices / room specs.

`Solid`, `OneWayPlatform`, and `DamageVolume` (with the legacy `HazardBlock`
alias) carry typed components plus sibling per-frame index resources rebuilt in
active-area-local coords. `LdtkRuntimeSpineParity` compares the collision-heavy
indices to the JSON-derived `ae::World::blocks` and logs a single deduped
warning whenever they diverge; the JSON adapter still owns collision authority
until parity holds across boot, hot reload, and every active area.

`CameraZone` and `KinematicPath` are promoted through the room projection:
`CameraZone` entities land in `RoomSpec::camera_zones`, and LDtk
`KinematicPath` entities land in `RoomSpec::kinematic_paths` while also being
mirrored through `ae::World::objects` for older feature consumers. Moving
platforms, NPC patrols, enemy patrols, and moving hazards can now consume the
typed path index via `path_id`; new systems should prefer the typed `RoomSpec`
fields. See
`docs/ldtk_runtime_spine.md`.

## Settings / input architecture

`crate::settings` is a real module with `audio` / `controls` /
`gameplay` / `video` submodules and a `UserSettings` resource
(serializable, defaults today; persistence is a focused follow-up).
The pause overlay is a renderer/controller that walks a page
stack and dispatches per-row `apply_action` calls; mutation logic
stays close to the field. See `docs/settings_system.md` for the
add-a-setting recipe. `SandboxAction` has a dedicated
`Menu*` action seam; the sandbox menu reads only those + analog
left-stick repeat, never gameplay actions. Controller deadzone
applies before the engine sees the move axis (fixes Xbox 360 +Y
drift on blink aim); dash uses hysteretic trigger edges with
configurable thresholds (fixes held-trigger re-fire).

## Player projectile + motion input

`ambition_engine::projectile` is the reusable backend
(`ProjectileSpawner`, `ProjectileBody`, `MotionInputBuffer`); the
sandbox `crate::projectile::update_projectiles` wires them into
gameplay. F (kbd) / West face button fire a Fireball; performing a
half-circle motion before pressing fires a Hadouken. See
`docs/mechanics/projectiles_and_motion_inputs.md`.

## Encounter / mob lab foundation

`crate::encounter` is a tested state-machine resource for wave-
based encounters with lock / fail / retry semantics. The mob-lab
LDtk room is wired end-to-end: `EncounterTrigger` enters
`Starting`, the camera zoom + lock wall + music swap apply,
`mob_lab_wave_specs` drives the hard-coded wave script, and a
hallway `Switch` free-toggles `Cleared / Inactive` for sandbox
iteration. Persistence (Cleared / Failed) survives reload. See
`docs/mob_lab.md` for layout, save shape, and remaining deferred
items (smooth camera ease, switch sprite swap, multi-encounter
authoring, on-screen wave indicator).

## Player-owned movement mechanics

Ledge grab and swim are now engine-owned movement mechanics, not
post-`sandbox_update` sandbox mutators. `ae::Player::ledge_grab` is advanced by
`ae::update_player_simulation_with_tuning`, and water/swim behavior is handled
by the same simulation tick through `World::water_at`.

When changing ledge behavior, update `crates/ambition_engine/src/ledge_grab.rs`
and the movement tests. The sandbox module `crates/ambition_sandbox/src/ledge_grab.rs`
is only a presentation/test shim that re-exports timing constants. Avoid adding
new systems that write `runtime.player.pos` after `sandbox_update`; extend the
engine movement state instead.


## Character AI shared evaluator

`ambition_engine::character_ai` provides
`CharacterAiSnapshot` + `CharacterAiMode` and a pure evaluator
that picks the mode from the snapshot. It is used today as an
*observed* signal — `EnemyRuntime` / `BossRuntime` populate
`ai_mode` for HUD/debug — but the movement / attack branches still
read the old timer fields. The path to making it authoritative is
in `docs/character_ai_refactor.md`. Hostile NPC conversion already
routes through `EnemyRuntime` so a single AI loop covers authored
enemies and runtime-converted hostiles, but bosses and per-brain
knobs are still parallel implementations.

## Player state is ECS-authoritative — no god-object resource

The `SandboxRuntime` god-resource was deleted by the 2026-05-16 ECS
player migration. Player state now lives on a single Bevy entity
marked with `PlayerEntity`, carrying these canonical components
(see `crates/ambition_sandbox/src/player/components.rs`):

- `PlayerMovementAuthority { player: ae::Player }` — authoritative
  movement, abilities, body shape, fly, dash, mana, etc.
- `PlayerBody` — compact read-model snapshot for queries that don't
  need every movement-internal field. Rewritten each frame by
  `write_player_ecs_components`.
- `PlayerHealth` — HP.
- `PlayerCombatState` — flash / hitstop / hitstun / damage-invuln
  timers + the mirrored `attacking` flag.
- `PlayerAnimState` — presentation-only animation timers.
- `PlayerInteractionState` — interact buffer, double-tap-up/down
  pending edges.
- `PlayerBlinkCameraState` — blink-in + camera-snap timers.

Spawn this set with `PlayerSimulationBundle::new(player, health)`
(in `crates/ambition_sandbox/src/player/bundles.rs`). Headless
drivers and tests can build the same shape without any presentation
plugin in scope. Standalone Bevy resources cover what isn't truly
per-player: `SandboxSimState`, `SandboxDevState`,
`MovingPlatformSet`, `CurrentPlayerAttack`.

Do NOT re-introduce `SandboxRuntime` / `FeatureRuntime`. The
`legacy_runtime_guardrail` integration test scans `src/` and fails
if those identifiers reappear; the `plugin_minimal_app` test
asserts the deleted resources are not silently inserted at startup.

## Multiplayer-readiness policy (single player today, but…)

The game currently spawns exactly one local player. The architecture
intentionally avoids hard-coding "one player" as a permanent
assumption. Identity components on every player entity:

- `PlayerEntity` — *any* player entity. Use this when a system wants
  every player regardless of locality or slot.
- `PlayerSlot(u8)` — per-player slot id. `PlayerSlot::PRIMARY` (= 0)
  is the local primary player today.
- `PrimaryPlayer` — the player the camera, HUD, and dev tools follow
  by default. Exactly one entity carries this; today every player is
  also primary.
- `LocalPlayer` — input comes from this machine. Today every player
  is also local; future remote-network players would not be.

`PlayerSimulationBundle::new(player, health)` spawns the
single-player default with all four tags set. The
`PlayerIdentityBundle` is the smaller "just the tags" bundle for
tests that want to spawn a second player without the simulation
chain.

Helper queries live in `crate::player::queries`:
[`PrimaryPlayerOnly`](crates/ambition_sandbox/src/player/queries.rs)
filter, [`primary_player_entity`], and
[`sort_players_by_slot`]. Use these in new systems instead of
`single_mut::<…, With<PlayerEntity>>` whenever the singleton intent
matters.

Working rules:

1. **New player-bearing messages MUST include identity.** Either
   `Entity` or `PlayerSlot` — never a bare `PlayerHealRequested`
   that silently means "the player." Old messages like
   `PlayerHealRequested` and `PlayerDamageEvent` are grandfathered
   until they touch this rule.
2. **New systems that operate on the camera / HUD / dev-tool target
   should filter on `PrimaryPlayer`** (or use
   `crate::player::queries::PrimaryPlayerOnly`), not on
   `PlayerEntity` alone.
3. **Avoid adding new singleton "player" resources.** Prefer
   per-player components. `ControlFrame`, `CurrentPlayerAttack`,
   and `SandboxSimState::last_safe_player_pos` are documented
   singleton-for-now resources; expand them only as a last resort.
4. **The `second_player_entity_spawns_with_unique_slot_and_no_extra_primary`
   test in `plugin_minimal_app.rs` is a canary.** If you break it,
   you have likely deepened a singleton assumption — fix the new
   code, don't relax the test.

What we explicitly are NOT doing yet: networking, split-screen
rendering, per-player input devices, per-player room simulation, or
camera/HUD rework. The pass above is *readiness*, not
implementation.

