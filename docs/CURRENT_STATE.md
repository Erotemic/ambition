# Current state

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

- an all-abilities movement testbed,
- input presets through Leafwing,
- pause/game-mode gating,
- generated lo-fi music tracks and sound effects,
- pause-menu music track switching,
- LDtk-authored active-area composition for the central hub POC, with a live `bevy_ecs_ldtk` `LevelSet` synced to the active Ambition room,
- a central hub with a literal drop-down basement stitched into one continuous active area, with the old sandbox doors and feature labs ported into LDtk-authored active areas,
- central-hub side `EdgeExit` wall collision split around the exits so those zones are physically reachable,
- test rooms for hazards, enemies, boss patterns, breakables, pickups/chests, and NPC talk hooks,
- debug labels over loading zones,
- feature runtime behavior for current prototype entities,
- input-feel helpers such as jump/coyote/dash/interaction buffering,
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

## Known high-risk areas

Spatial reasoning and geometry code need extra review. In particular:

- LDtk chunk-to-active-area composition,
- LDtk validator checks for `EdgeExit`/solid overlap and transition arrivals that would start outside the target active area or inside authored solids,
- room transition arrival repair,
- loading-zone placement and labels,
- camera/world coordinate conversion,
- collision edge-touch semantics,
- blink destination search,
- moving hazards/platforms,
- non-Euclidean seams or chart transforms.

When touching these systems, add an `AMBITION_REVIEW:` comment if the logic is easy to get subtly wrong, and add tests or debug visualization when practical. See `docs/AGENT_HANDOFF.md`.

## Current next good moves

1. Fix compile/runtime issues from user logs before adding new features.
2. Convert one enemy to the `seldom_state` path instead of migrating everything at once.
3. Build a small first-level vertical slice rather than only adding isolated labs.
4. Expand tests around room graphs, blink/collision, input buffering, and generated schedules.
5. Add a render/preview lab for procedural visuals before committing to a final style.
6. Keep updating ADRs when decisions supersede older notes.

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

Library structure: `lib.rs` declares `pub mod app;` (`crates/ambition_sandbox/src/app.rs` ~1500 lines) with four App-builder helpers — `init_sandbox_resources`, `add_simulation_plugins`, `add_ldtk_runtime_plugin`, `add_presentation_plugins` — that both binaries compose. `bevy_ecs_ldtk::LdtkPlugin` and Avian2D `AmbitionPhysicsPlugin` live in the visible-only halves because they need `RenderApp` / `SceneSpawner` respectively; headless still has the JSON-derived `RoomSet` for collision and runs the runtime-spine systems as no-ops without LDtk-spawned entities. `handle_debug_hotkeys` moved to a presentation-side Bevy system so `sandbox_update` no longer reads `Res<ButtonInput<KeyCode>>`. See `docs/headless_simulation.md` and `docs/events_refactor_plan.md`.

## LDtk roadmap step 1 (Solid promotion, partial)

Step 1 of the LDtk runtime-spine roadmap is in progress: collision-heavy entities are being promoted from JSON-only adapter output to typed Ambition components on plugin-spawned entities.

The first collision category, `Solid`, is now partially promoted. Every plugin-spawned `Solid` entity carries a typed `LdtkSolid` Ambition component, and a sibling `LdtkRuntimeSolidIndex` resource holds the active-area-local view rebuilt each frame. The JSON adapter still produces `ae::Block::solid()` entries for `ae::World::blocks` so runtime collision authority is unchanged for now; the JSON Solid path is marked transitional. The raw-LDtk-vs-runtime debug overlay (Step 2) is the verification gate before retiring the JSON path and letting `LdtkRuntimeSolidIndex` become collision authority. `OneWayPlatform`, `DamageVolume`, `KinematicPath`, and the remaining `CameraZone` work follow the same shape and ship in subsequent step-1 patches.
