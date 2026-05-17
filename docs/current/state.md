# Current state

Update this when current architecture or active design direction changes.

Related split:

- [`risks.md`](risks.md)
- [`next.md`](next.md)

This document is the current high-level source of truth for Ambition. Update it when the architecture or active design direction changes. Keep transient implementation details in focused patch docs or source comments.

## One-sentence summary

Ambition is a Rust/Bevy 2D metroidvania/platformer sandbox plus reusable mechanics engine, built code-first around excellent movement feel, data-driven rooms, generated assets, and eventually mathematical/story-driven progression.

## Active architecture

```text
ambition_engine
  Bevy-native reusable mechanics and data vocabulary.

ambition_sandbox
  Playable Bevy shell, LDtk-authored sandbox world-composition POC, RON tuning/audio data,
  debug tools, visual/audio adapter, and current experimental feature rooms.

future story/game crates
  Campaign content, progression, dialogue, world variants, and presentation choices.
```

The engine may depend on Bevy and Bevy-adjacent crates when useful. It should still avoid owning sandbox presentation details such as colors, HUD layout, inspector windows, and temporary visual experiments.

## 2026-05-16 ECS player migration complete — SandboxRuntime deleted

`SandboxRuntime` (the god-object resource that shadowed player state) has been
deleted entirely (Stage 12, commit `690e268`). Player state is now authoritative
on `PlayerMovementAuthority { player: ae::Player }`, an ECS component on the
player entity. All prior shadow-write patterns (`runtime.player = player.clone()`)
are gone. Other fields that were on `SandboxRuntime` are now standalone Bevy
resources: `SandboxSimState`, `SandboxDevState`, `MovingPlatformSet`,
`CurrentPlayerAttack`. The guardrail in `lib.rs`: *"Do not introduce a god-object
runtime resource; add narrow resources or ECS components instead."*

## 2026-05-09 UI, radio, audio-generation, and module-split update

Recent mobile/menu polish made dialog choices behave like menu rows: larger
touch targets, compact/windowed option lists, drag/wheel/joystick navigation,
and mouse-as-touch proxy behavior where appropriate. The radio is now a real
track-selection page rather than a single Music row: it exposes the simple-track
lineup, remembers a user-selected station in `RadioStationState`, and lets
adaptive encounter music temporarily take over before resuming the selected
station.

The active music score YAMLs live under
`tools/ambition_music_renderer/scores/active/`; generated audio under `assets/`
remains ignored output. The audio generator should skip unchanged YAML by
default using timestamp/hash/version checks and should support an explicit force
path when renderer code changes or authors want to rebuild everything. Python
generators should prefer `rich.print` and render file paths as terminal links
when possible.

The large-file refactor has also moved from planning into implementation:
`movement`, `encounter`, `audio`, `music`, `input`, and `trace` now use stable
facade files with child modules. Remaining stale refactor notes that describe
those files as unsplit should be read historically unless they call out a deeper
crate-extraction or gameplay-state redesign that is still pending.

## Current stack

- Bevy 0.18
- Leafwing Input Manager for semantic controls
- serde / RON / `bevy_common_assets` for tuning/audio manifests
- LDtk JSON authoring via an Ambition adapter, with `bevy_ecs_ldtk` now used as a first-class Bevy asset/hidden `LdtkWorldBundle` path
- `bevy_asset_loader` foundation for future explicit loading states
- `petgraph` for room transition graphs
- The plugin-owned LDtk world root is hidden until individual LDtk layers/entities are promoted to visible typed Ambition runtime bundles; this avoids placeholder LDtk rectangles rendering over the sandbox.
- `bevy-inspector-egui` and Bevy Gizmos for dev tooling
- `parry2d` for reusable geometry queries
- FunDSP for startup-rendered generated audio and `bevy_kira_audio` for sandbox playback, mixing channels, looping, and fades
- Bevy/glam math types such as `Vec2` and `Aabb2d`
- Bevy `States` for app-wide modes such as playing, paused, dialogue, transitions, and cutscenes
- `seldom_state` foundation for per-entity state machines
- `insta` and `proptest` as lightweight testing foundations

## Active gameplay state

The sandbox currently has:

- an all-abilities movement testbed (walk / run / wall jump / wall climb /
  dash / blink / fly / glide / fast-fall / pogo / rebound / fireball /
  Hadouken / swim / ledge-grab),
- `BodyMode` driver for stance-aware kinematics (Standing / Crouching /
  Crawling / Sliding / MorphBall) with `BodyShape::fits_at`-gated transitions,
- per-player engine state on `Player` for `damage_multiplier`, `invincible`,
  `mana: ResourceMeter`, and `was_riding_platform` diagnostics,
- input presets through Leafwing, with per-controller-profile filter
  defaults (Xbox 360 widens deadzones + trigger band; PlayStation tightens),
- pause/game-mode gating,
- adaptive generated music tied to encounter phases (mob_lab fires
  intro → wave1 → wave2 [+brute reinforcement] → wave3 → outro), with
  LDtk × audio cross-validation for `music_track` field references,
- pause-menu music track switching + per-controller / video / audio /
  gameplay settings persistence,
- LDtk-authored active-area composition for the central hub POC, with a live
  `bevy_ecs_ldtk` `LevelSet` synced to the active Ambition room,
- a central hub with a literal drop-down basement stitched into one continuous
  active area, with the old sandbox doors and feature labs ported into
  LDtk-authored active areas,
- central-hub side `EdgeExit` wall collision split around the exits so those
  zones are physically reachable,
- test rooms for hazards, enemies, boss patterns (Gradient Sentinel intro /
  phase1 / phase2 / stagger / enraged), breakables (`OnHit` / `OnStand` /
  `Either` triggers), pickups/chests, NPC talk hooks, and a scripted
  encounter (`mob_lab` with lock-wall slam + 3-wave spawning),
- a sim → presentation message seam (`SfxMessage` / `VfxMessage` /
  `DebrisBurstMessage` / `PlayerDiedMessage`) covered by
  `tests/scripted_gameplay.rs`,
- debug labels over loading zones, dedicated quest panel, full-screen
  map with zoom controls + room-name labels,
- feature runtime behavior for current prototype entities,
- input-feel helpers such as jump/coyote/dash/interaction buffering,
- proptest coverage for `BodyShape::fits_at`, wall-jump start positions,
  and `ResourceMeter` envelope invariants,
- GitHub Actions CI: engine + sandbox lib tests + `cargo run --bin headless`
  smoke,
- early state-machine/test/asset-loader scaffolding.

These prototype feature rooms are not the final game. They exist to validate reusable mechanics before content is curated.

## Current data location

The canonical sandbox tuning/audio manifest is:

```text
crates/ambition_sandbox/assets/ambition/sandbox.ron
```

The current sandbox level-authoring POC is:

```text
crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

The sandbox normally loads this LDtk file from disk at runtime so map edits and
modded maps do not require recompiling Rust. Override it with `--ldtk <path>` or
`AMBITION_LDTK=<path>`. Build with `--features static_map` to also embed the
checked-in map in the binary and fall back to it when the external map cannot be
read or parsed.

Older root-level asset paths are obsolete unless a patch explicitly says otherwise.

The audio manifest uses a track-list shape: `default_music_track` selects the startup track, and `music_tracks` contains procedural/declarative arrangements. The old 32-beat loop remains available as `original_lofi_loop`; the current default is a longer generated track for sandbox iteration. These names are implementation data, not final game promises.

## Stable decisions

- Movement feel matters more than final art or story polish right now.
- The game should remain fun with raw collision/debug geometry.
- Reusable mechanics should migrate into `ambition_engine` or reusable data specs.
- The sandbox should remain an adapter/lab, not a second engine.
- App-wide modes use Bevy `States`; per-entity behavior should move toward `seldom_state` gradually.
- Room/content authoring should be data-driven where practical. LDtk is now the first external level-editor adapter target; Ambition typed data remains canonical.
- Generated assets should remain inspectable, reproducible, and connected to gameplay semantics.
- Patches should include documentation notes, testing limitations, and a markdown paragraph commit message.

## Experimental / not final

The following areas are intentionally provisional:

- Feature basement layout and entity visuals. The boss is no longer in the stitched hub basement; it belongs in the separate `basement_boss` lab reached through a restored basement door.
- Enemy and boss behavior details.
- NPC dialogue presentation.
- Generated visual style.
- Asset-loading boot flow.
- Exact control presets and HUD layout.
- Roguelike / run-based modes.
- Non-Euclidean and non-metric space mechanics.

Do not document these as final game promises in the README. Use focused docs and ADRs.

## LDtk asset initialization ordering note

`SandboxAssetCollection` contains a typed `Handle<bevy_ecs_ldtk::assets::LdtkProject>`, so `LdtkPlugin` must be registered before `init_collection::<loading::SandboxAssetCollection>()`. If the collection is initialized first, Bevy panics while allocating the typed LDtk handle because `LdtkProject` has not been registered yet.

## LDtk hot reload foundation

The sandbox now has a first hot-reload loop for the LDtk-authored world. During
development, run with `--features dev_hot_reload` to enable Bevy file watching;
the sandbox also polls the LDtk file timestamp. `F11` validates and applies the
current LDtk file, while `F12` toggles automatic apply after detected changes.
Rejected reloads leave the live Ambition world untouched and report validator
errors in stderr/HUD. Applied reloads rebuild the LDtk-derived `RoomSet`, active
`GameWorld`, transient feature runtime, moving-platform state, room visuals, and
LDtk level index while preserving the player and repairing their position if the
edited map would put them in collision.

## LDtk loader schema-noise fix

A user run confirmed hot reload worked in practice, but Bevy logged that
`bevy_ecs_ldtk` could not deserialize `sandbox.ldtk` because the level field
`activeArea` was missing `allowedRefs`. This was not a gameplay crash because the
Ambition reload path reads the file with its own adapter, but it meant the typed
LDtk asset path was still unhealthy. The sandbox LDtk file now includes the
first-class LDtk `FieldDef` keys for level fields, and the validator checks both
entity field definitions and level field definitions for the required
`bevy_ecs_ldtk`/LDtk schema fields.

LDtk editor round-trip also requires `FieldDef.type` to use LDtk's internal
`FieldType` constructor names, such as `F_String`, not the human-readable
`__type` values such as `String`. The `activeArea` level field previously used
`type: "String"`, which Bevy's early custom path tolerated but the LDtk 1.5.3
editor rejected with `No such constructor String`. The sandbox LDtk file now
uses `type: "F_String"` for `activeArea`, and the validator checks entity and
level field definitions for this editor-openable shape.

## LDtk defUid spawn fix

A user run confirmed that after the `allowedRefs` schema fix, `bevy_ecs_ldtk`
progressed to actual level spawning but panicked in
`calculate_transform_from_entity_instance`. The cause was stale hand-authored
entity-instance `defUid` values that did not match `defs.entities[*].uid`. The
sandbox LDtk file now synchronizes entity instance `defUid` values and field
instance `defUid` values with their definitions, and the validator checks this
before runtime.

## LDtk runtime spine migration

Ambition is moving from a custom LDtk JSON adapter toward `bevy_ecs_ldtk` as the runtime spine. The sandbox now registers every current Ambition LDtk entity identifier as a lightweight plugin-spawned marker bundle, keeps the LDtk world root active, disables LDtk level-background rendering, and records plugin-spawned entity lifecycle in HUD/debug state. LDtk builds runtime `RoomSet` data directly through `RoomSet::from_parts`; the old RON-shaped world manifest structs and builders have been removed.

Official LDtk JSON Schema validation should use Python `jsonschema`, not npm. `tools/validate_ambition_ldtk.py` supports optional `--schema` and `--require-schema` flags while continuing to run Ambition-specific semantic validation without the schema file.

## LDtk-only world bootstrap step

The sandbox RON asset is no longer the runtime owner of the room/world manifest. `SandboxDataSpec` intentionally contains only abilities, movement tuning, and generated-audio configuration. Startup and LDtk hot reload build the active `RoomSet` directly from `assets/ambition/worlds/sandbox.ldtk` instead of writing the LDtk-derived manifest back into `SandboxDataSpec.rooms`.

The old `rooms` data may still exist in historical artifacts for reference, but new gameplay/world patches should treat it as deleted legacy data. `SandboxDataSpec` is non-spatial; LDtk is the sandbox world source.

The legacy `rooms` block has also been removed from `crates/ambition_sandbox/assets/ambition/sandbox.ron`; the LDtk file is now the only checked-in sandbox world definition. If older docs mention RON room authoring, treat that as historical unless explicitly scoped to tests/fixtures.

## LDtk runtime bridge confinement update

LDtk is now the only checked-in sandbox world definition, and startup / hot-reload call sites use `LdtkProject::to_room_set()` directly. `ldtk_world.rs` now materializes runtime `RoomSpec`, `ae::World`, loading zones, objects, and `RoomLink`s directly. The next migration step is to consume plugin-spawned `EntityInstance` data category by category and eventually reduce custom JSON parsing.

## LDtk editor round-trip repair

A user edited `sandbox.ldtk` in LDtk 1.5.3 by moving existing entities, and LDtk saved the touched hub levels with null custom fields because generated field instances had parser-facing `__value` values but empty `realEditorValues`. The repaired sandbox LDtk file now includes editor values for all non-null field instances. The validator now rejects this lossy shape and supports `--normalize-editor-values` to fill editor values from existing `__value` data before opening the file in LDtk.

The repaired map preserves the intended NPC move and the lower-door horizontal move, removes an accidental empty 1x1 `LoadingZone`, and lifts the lower-door trigger slightly so transition arrival no longer intersects the hub floor.

## LDtk editor-native tooling

The LDtk workflow now has dedicated Python tools: `tools/repair_ambition_ldtk.py` repairs generated/agent-patched editor metadata, `tools/check_ldtk_editor_roundtrip.py` verifies the file is editor-roundtrip clean without mutating it, and `tools/fetch_ldtk_schema.py` fetches the official LDtk JSON Schema for optional Python `jsonschema` validation. The sandbox LDtk project now defines all supported Ambition entity identifiers, including `CameraZone` and `StitchedBoundary`, with colors/docs/default fields so supported objects can be added from the LDtk editor. See `docs/ldtk_authoring.md`.

## LDtk runtime-spine update

The first promoted plugin-spawned LDtk categories are `PlayerStart`, `LoadingZone`, `DebugLabel`, and `CameraZone`. `bevy_ecs_ldtk` owns their entity lifecycle; Ambition rebuilds a runtime-spine index from spawned entities each frame for HUD/debug overlays and future direct gameplay promotion. Hot reload now prepares a replacement world transaction before mutating live state and rejects edits that delete the current active area or leave missing graph links.

## Headless simulation entry point (Phase 2 — full gameplay loop)

The sandbox crate is a library + multi-binary package. The visible binary (`cargo run -p ambition_sandbox`) is unchanged; an additional `cargo run -p ambition_sandbox --bin headless [TICKS]` runs the actual gameplay loop (`sandbox_update` and helpers) on a `MinimalPlugins`-based Bevy `App` with no rendering, audio, or windowing.

ADR 0012's Phase 2 events refactor (commits c49c1e5–81900dd) routed every sim-emitted side-effect through typed buffered messages: `SfxMessage` (audio.rs), `VfxMessage` (fx.rs), `DebrisBurstMessage` (physics.rs). The simulation pushes into per-frame `Vec` collectors and `sandbox_update` drains via `MessageWriter::write_batch` at every return point. Presentation-side subscribers (`audio_play_sfx_messages`, `vfx_spawn_messages`, `physics_spawn_debris_messages`) consume the messages and perform the actual playback / particle spawn / debris burst. Headless omits the subscribers; queues drain harmlessly.

Library structure: `lib.rs` declares `pub mod app;`. The public App-builder surface remains `app::run_visible`, `init_sandbox_resources`, `add_simulation_plugins`, `add_ldtk_runtime_plugin`, `add_presentation_plugins`, and `sandbox_update`, but the implementation is now split under `crates/ambition_sandbox/src/app/` by responsibility: CLI/runtime entry, resource bootstrapping, plugin/schedule wiring, setup systems, the `sandbox_update` orchestrator, phase helpers, hot-reload/dev runtime, room/world flow, HUD, and feedback queues. `bevy_ecs_ldtk::LdtkPlugin` and Avian2D `AmbitionPhysicsPlugin` live in the visible-only halves because they need `RenderApp` / `SceneSpawner` respectively; headless still has the JSON-derived `RoomSet` for collision and runs the runtime-spine systems as no-ops without LDtk-spawned entities. `handle_debug_hotkeys` remains a presentation-side Bevy system so `sandbox_update` does not read `Res<ButtonInput<KeyCode>>`. See `docs/systems/headless-simulation.md` and `docs/archive/historical-roadmaps/events-refactor-plan.md`.

## Menu input + controller deadzone / trigger fixes

`SandboxAction` now has a dedicated menu seam:
`MenuNavigate{Up,Down,Left,Right}`, `MenuSelect`, `MenuBack`,
`MenuStick` (analog), and the new `Projectile`, `DashAnalog`,
`AimStick` actions. The pause menu reads only the `Menu*` actions
through a new `MenuInputState` resource that handles analog repeat
(initial delay + interval, both configurable); cardinal D-pad /
arrow-key edges always fire immediately. `Enter` is now a real
binding on `MenuSelect` rather than a hardcoded check inside the
settings page.

`ControlFrame::read_gameplay_with_settings` applies the configured
left-stick deadzone before any walk-modifier / blink-aim consumer
sees the movement axis, fixing Xbox 360 +Y drift gradually pushing
precision blink aim upward. The new `PlayerDashTriggerState`
resource together with `update_trigger_edge` collapses analog
right-trigger jitter into a single press edge using configurable
press / release thresholds, fixing held-Dash from re-firing.

## Player projectile + Hadouken

`ambition_engine::projectile` adds the reusable primitives:
`ProjectileKind { Fireball, Hadouken }`, `ProjectileSpec`,
`ProjectileBody`, `ProjectileSpawner` (cooldown + `ResourceMeter`),
and `MotionInputBuffer` for half-circle / quarter-circle motion-input
recognition. The sandbox `crate::projectile::update_projectiles`
system samples the deadzoned axis into the motion buffer, ticks
in-flight projectiles against the active world, and fires Fireball
on press or Hadouken when a half-circle precedes the press. Trace
events go through new `GameplayTraceEvent::Projectile`. F (kbd) and
West face button are the default fire bindings. See
`docs/mechanics/projectiles_and_motion_inputs.md`.

## LDtk runtime-spine: OneWayPlatform + DamageVolume promoted

`LdtkOneWayPlatform` and `LdtkDamageVolume` typed components are now
attached on plugin-spawned LDtk entities, with sibling
`LdtkRuntimeOneWayIndex` / `LdtkRuntimeDamageIndex` resources
rebuilt every frame in active-area-local coordinates. New
`LdtkRuntimeSpineParity` resource compares the index counts to the
JSON-derived `ae::World::blocks` (`Solid` / `OneWay` / `Hazard`) and
logs a single deduped warning whenever they diverge. JSON adapter
authority is unchanged pending parity verification across hot reload
and the full active-area set; once parity holds the JSON arms can
retire. See `docs/ldtk_runtime_spine.md`.

## Encounter / wave system foundation

`crate::encounter` is the wave / lock / fail state machine
(`Inactive | Starting | Active{wave_index, remaining_mobs} |
Cleared | Failed`) plus `EncounterEvent`s for trace plumbing. The
mob-lab LDtk room is wired end-to-end: `EncounterTrigger` enters
`Starting`, the camera zooms, the music swap fires, the lock wall
materializes through `sync_lock_walls`, and the hard-coded wave
script in `mob_lab_wave_specs` drives spawning. Death-during-active
fails the encounter; the `Switch` in the hallway free-toggles
between `Cleared` and `Inactive`. Persistence (Cleared / Failed)
survives reload via `sandbox_save.ron`. See `docs/recipes/mob-lab.md` for
the full layout, persistence, and what is still deferred (smooth
camera ease, switch sprite swap, multi-encounter authoring, HUD
wave indicator).

## Player-owned movement mechanics

Ledge grab and swim now live in `ambition_engine::movement` instead of running
as post-`sandbox_update` player mutators:

- `Player::ledge_grab` is engine-owned state.
  `update_player_simulation_with_tuning` latches, holds, drops, and climbs ledges
  in the same tick that owns gravity, wall contact, water, and collision. The
  ledge probe accepts any standable pull-up surface (`Solid`, `BlinkWall`, or
  `OneWay`) and rejects candidates whose pull-up body would be blocked.
- Water/swim behavior is also engine-owned: the movement tick queries
  `World::water_at`, drowns/reset when `!abilities.swim`, converts buffered
  jump presses into `MovementOp::SwimStroke`, and applies buoyancy/drag/fall
  caps while submerged.

The remaining post-update body-mode driver still mutates `Player::body_mode`
for crouch/morph/climbing entry/exit, but it explicitly avoids fighting active
ledge/swim states. F3 stat edits remain sandbox-side debug tooling.

## Character AI: pure evaluator, not yet authoritative

`ambition_engine::character_ai` is now the canonical
`CharacterAiSnapshot → CharacterAiMode` evaluator (Idle / Patrol /
Chase / Telegraph / Attack / Recover / Stunned / Dead). It is pure,
Bevy-free, and unit-tested. Hostile NPC conversion already routes
through `EnemyRuntime`, so a single AI implementation drives both
authored enemies and runtime-converted hostiles.

The evaluator is *observed*, not yet *authoritative*: `EnemyRuntime`
and `BossRuntime` build a snapshot and stash the resulting mode
for HUD/debug, and sandbox enemies now consume the engine `CharacterAiOutput`
for coarse hold / patrol / chase / attack intent. Archetype-specific speeds,
contact damage, and collision remain sandbox-side tuning; bosses are still a
separate follow-up. See `docs/systems/character-ai-refactor.md` for the remaining
data-table per-brain work.

## LDtk roadmap step 1 (Solid promotion, partial)

Step 1 of the LDtk runtime-spine roadmap is in progress: collision-heavy entities are being promoted from JSON-only adapter output to typed Ambition components on plugin-spawned entities.

The first runtime-spine / room-projection categories are now promoted in stages. Collision-heavy entities (`Solid`, `OneWayPlatform`, `DamageVolume`) have typed spine indices plus JSON-adapter parity checks before they become collision authority. `CameraZone` and `KinematicPath` are promoted in the room projection: authored camera zones land in `RoomSpec::camera_zones`, and LDtk `KinematicPath` entities land in `RoomSpec::kinematic_paths` while still mirroring through `ae::World::objects` for older consumers; moving platforms, NPC patrols, enemy patrols, and moving hazards can now consume the typed path index via `path_id`. The raw-LDtk-vs-runtime debug overlay remains the verification gate before retiring JSON adapter arms.

## Gameplay flight recorder + Tier-1 mechanic primitives

The sandbox now has a rolling per-frame trace recorder and three engine-side
mechanic primitives:

- `crate::trace::GameplayTraceBuffer` records 240 player snapshots and 240
  events, dumps to `debug_traces/ambition_trace_*.json` + `.md` on `F8` or
  on automatic OOB detection (NaN/inf pos/vel, AABB outside world envelope
  with margin, AABB inside `Solid`, absurd velocity).
  See `docs/gameplay_trace_recorder.md`.
- `ambition_engine::LocomotionState` (Grounded / Airborne / Dashing /
  Blinking / WallSlide / Crouching / MorphBall / GrappleAiming /
  CurveRiding / …) is the explicit movement-mode enum.
- `ambition_engine::BodyMode` + `BodyShape::fits_at` is the
  alternate-body-shape vocabulary plus the collision-safe resize query
  used to gate stand-up / unmorph against ceilings.
- `ambition_engine::ResourceMeter` is the generic stamina/mana/ammo/charge
  primitive (regen + decay tick, `try_spend`, `fraction` for HUD bars).

These primitives are Bevy-free so they survive both the visible binary and
headless. The HUD now shows current locomotion / body-mode / mechanic-count
summary / latest trace status. F8 was previously bound to exclusive
fullscreen; that binding was removed (F7 borderless covers the dev case).

`crate::mechanics::MechanicsRegistry` is a small in-memory catalog of
playable verbs and Planned mechanics (crouch / morph / grapple / projectile /
parry / functional zip) that the HUD can summarize and that future patches
can append to without restructuring.

See `docs/mechanics/body_modes.md` for how to build mechanics on top of
these primitives.

## Settings architecture (real)

`crate::settings` is now a real module with submodules `audio`,
`controls`, `gameplay`, `video` and a top-level `UserSettings`
resource. The pause overlay renders a page stack
(`Top → Video / Audio / Controls / Gameplay → row pages`) backed by
serializable per-category structs. Audio volumes (master / music /
sfx / mute) are pushed to the Kira channels by
`apply_audio_settings`; difficulty + assist + the gameplay damage
multiplier compose into a single scalar that scales incoming player
damage. Flashes / colorblind / camera-zoom / trace-auto-dump
settings exist as data and will be consumed where wiring lands.
See `docs/settings_system.md` for the architecture and the
add-a-setting recipe; `docs/pause_menu_settings.md` for the page
layout and bindings.

## Trace recorder hardening (post-baseline)

The recorder is now useful as the first-line collision/OOB debugging
tool:

- **Filenames** include unix seconds, sub-second nanoseconds, and a
  process-wide atomic counter, so dumps in the same nanosecond cannot
  overwrite each other.
- **Synthesized events** are diffed each tick from the previous sim
  snapshot — input edges, locomotion / body changes, dash / double
  jump / jump heuristics, blink start + precision, damage / death,
  reset, room transition, and (the smoking-gun event for the active
  OOB bug) `CollisionCorrection` for unexplained position deltas
  larger than what the player's velocity can produce. The recorder
  stays a passive observer; phase helpers can still push events
  directly when they have non-state-derivable info.
- **Moving platforms** populate `GameplayTraceFrame.moving_platforms`
  with pos / size / AABB / direction / riding / distance fields so
  platform-related tunneling becomes visible in the trace.
- **BodyMode** is now an authoritative field on `ambition_engine::Player`.
  Sandbox systems that drive crouch / morph / slide should write
  `player.body_mode`; the trace and HUD consult `BodyMode::from_player`
  which reads the field. Single source of truth.

Settings extracted into `crate::settings` (vocabulary +
mutation logic). The pause menu became a thin renderer/controller
that decodes `ActionState` into a compact `NavInput` and dispatches
to `settings::handle_action`. Audio-off (`--no-default-features
--features input`) compiles and runs with the Music row replaced by
a placeholder.

## Body-mode mechanics: crouch + morph ball wired

`crate::body_mode::update_body_mode` runs in the progression chain
after `sandbox_update` and turns the engine's existing `BodyMode` /
`BodyShape::fits_at` primitives into two playable mechanics:

- Down held + grounded → `Crouching`. Releasing Down attempts a
  collision-safe stand-up via `try_change_body_mode`; a low ceiling
  rejects the transition and the player stays crouched.
- Double-tap-down + grounded → `MorphBall`. Jump-pressed (or
  Up-pressed) inside MorphBall tries Standing; a low ceiling keeps
  the ball curled. The signal is `PlayerInteractionState::double_tap_down_pending`
  (an ECS component on the player entity), set by `input_timer_system`
  and consumed by the body-mode driver via `mem::take`. The engine's
  airborne fast-fall path still uses the same gesture and gates on
  `!on_ground` so there is no input crosstalk.

`Player::base_size` is the new canonical Standing-stance size; the
engine helper `try_change_body_mode` adjusts `pos.y` to keep the
player's feet planted, runs `BodyShape::fits_at` against the target
shape, and rejects the transition if the new AABB would overlap any
caller-matched block. Mid-action mechanics (dash, blink-aim,
wall-cling/climb, ledge hang/climb, in-water swim) own their own posture; the
body-mode driver no-ops while any of them are active. `Player::reset_to` rebuilds
the struct so death/respawn always restores Standing.

## Biome / room-music metadata seam

LDtk levels can declare optional `biome` / `music_track` /
`ambient_profile` / `visual_theme` strings in their level fields
(added by `tools/add_biome_level_fields.py`). The runtime reads
these into `RoomSpec::metadata` per active area (first non-empty
value wins when an area spans multiple levels), mirrors the active
room's metadata into `crate::rooms::ActiveRoomMetadata`, and pushes
the `music_track` value through `RoomMusicRequest` for the audio
system. `audio::apply_encounter_music` resolves the desired track
as: encounter override > room music_track > sandbox-wide
`default_music_track`. Unknown track ids are silently ignored at
the audio layer so a typo can't stall playback. Every gameplay
level in the embedded LDtk now declares a biome; only `mob_lab`
sets a non-default `music_track`. The HUD shows the active
metadata under `ROOM:`; the diagnostic
`python tools/list_ldtk_metadata.py` prints the per-area merged
metadata for offline auditing.

## Cutscene skip UX

Holding `Reset` (Backspace / Delete / pad-Select) for
`SKIP_HOLD_THRESHOLD_SECS = 1.2` seconds during a cutscene flips
`CutsceneAdvanceRequest::skip_cutscene = true`; the cutscene
runtime takes its existing `skip()` branch and the seen flag is
recorded. The HUD shows a 12-segment progress bar while the hold
is in flight. Reset was chosen rather than Start so the pause
toggle still works during cutscenes and Interact / Jump still
advance dialogue normally. Closes the corresponding tech-debt
entry.

## Programmatic LDtk authoring (agent-friendly)

`tools/author_ldtk_area.py` now supports `--dry-run` (build the
level entirely in memory, print a structured summary, do not
mutate the file), top-level `connect_to:` (insert reciprocal
LoadingZone entities into existing target levels), top-level
biome metadata fields, and difflib-backed "Did you mean ...?"
suggestions on unknown entity types and field identifiers. Four
starter specs ship under `tools/examples/ldtk_specs/` (crawl_lab,
water_lab, mob_arena, music_biome_lab). The smoketest still
covers the live path; a new
`tools/author_ldtk_area_features_test.py` exercises every new
feature against a copy of the live `sandbox.ldtk`. See
`docs/ldtk_authoring.md`.
