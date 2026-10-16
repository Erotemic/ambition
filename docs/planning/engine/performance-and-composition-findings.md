# Performance and composition: findings and proposals

**Scope:** what makes the game cost more CPU than it must on a weak device, and
what keeps Mary-O and Sanic from paying for Ambition. **Asked by:** Jon,
2026-10-15/16 ("optimize by finding more elegant patterns, instead of chasing
milliseconds"; "run [approximations] by me first"; "Mary-O and Sanic [must] not
get bogged down by everything else that is in Ambition"). **Status:** an 8-hour
run started 2026-10-16T04:40Z; this page is its live record.

**Related pages (they own their plans; this page ranks across them):**
[system-composition-audit](system-composition-audit.md) (S1-S6),
[capability-and-runtime-composition](capability-and-runtime-composition.md),
[performance-and-iteration](performance-and-iteration.md),
[simulation-tick-rate-compatibility](simulation-tick-rate-compatibility.md),
[consolidation plan](../consolidation/consolidation-plan.md).

Each claim is **MEASURED** (a run, a census, a profile, or a read of source)
or **REASONED** (inferred). A number here has the run that made it; run it
again before you quote it.

## How to answer a cost question

| Question | Tool |
| --- | --- |
| Which systems run in a frame, by crate? | `AMBITION_PROFILE_CENSUS=1` + `AMBITION_SYSTEM_LIST=PATH`; `scripts/system_census_table.py` |
| Which Bevy system takes the time (desktop)? | `scripts/profile_desktop.sh` → `tracy_summary.md` |
| Where does the phone spend its CPU? | `scripts/profile_android.sh all ...`, then `scripts/symbolize_android_profile.py DIR` |
| Which line of OURS caused framework time (phone)? | `symbolize_android_profile.py DIR --ours` (needs `simpleperf-device-samples.txt`, which a profile made after 2026-10-16 has) |
| Which thread is saturated, and what does it do (phone)? | the `threads` table of `symbolize_android_profile.py DIR` (busy % of one core, by thread id), then `--ours --thread TID` |
| Which system allocates, and which code makes archetype churn? | the `perf_probe` feature (in progress, see the run log) |

## Measured baseline

**Phone (Pixel 5, hub with every game, Mary-O 1-1, commit 97ff2541e,
2026-10-16; `profiles/android-all-maryo-1-1`).** MEASURED.

- Frame time, 5 s windows: p50 24-46 ms, p95 37-131 ms.
- Threads: main 67%, the Bevy compute pool 22% (shares of the process; see F11
  for how busy each one is).
- Crates by own time: `core` 21%, `bevy_ecs` 13.2%, `mimalloc` 5.2%, `alloc`
  3.1%, `concurrent_queue` 2.2%, `fixedbitset` 1.2%, `async_executor` 1.0%;
  each crate of ours under 1%.
- Reading (REASONED): the time is in the framework and the allocator, and the
  framework does what our composition asks: about 1000 systems a frame, their
  access checks (`fixedbitset`), the task executor, and allocations. Which
  line of ours causes each part is the next measurement (`--ours` with the
  sample dump, and the `perf_probe`).

**System census (desktop debug, VM, 2026-10-16).** MEASURED.

| run | registered | ran in one frame |
| --- | --- | --- |
| standalone Mary-O, 1-1 | 1277 | 1036 |
| standalone Sanic, act 1 | 1264 | 1043 |
| hub Ambition, `basement_enemies` | 1964 | 1529 |

Systems that ran in each standalone game for capabilities it does not use
(Mary-O / Sanic): boss encounters 41/41, conversation 36/36, Smash grabs and
throws 14/14, Smash moveset 13/13, mounts 7/7, held items 9/9, match stocks
7/7, gizmos 36/36, Kira audio channels 30/30, developer tools and traces 18/18,
extension lowering 32/34. About 170 systems a frame (17%).

## Findings

### F0 — A third of the phone's CPU is the executor's work for each system, outside any system

MEASURED (phone, `android-all-maryo-1-1`, every sample's full call chain,
`symbolize_android_profile.py --ours`). 32.2% of all app samples are in the
schedule executor with no system body on the stack: per system, before or
after it runs. By innermost frame: access checks over component bitsets
(`bevy_ecs/query/access.rs` with `fixedbitset`, about 2.9%), the
`TypeId` test that finds apply-deferred points (`system/system.rs` and
`executor/mod.rs`, about 2.5%), archetype-cache updates
(`update_archetypes_unsafe_world_cell`, `query/state.rs`), parameter
validation (`function_system.rs`), and allocation in the executor. A further
44.8% has no frame of ours at all: the render world, the GPU driver
(`vulkan.adreno.so`), the kernel, and the task pool.

What it means (REASONED): this cost grows with the number of systems that run
each frame (about 1000) and, for the access checks, with the number of
component and resource types the world has registered (about 1900 and 1400).
It does not depend on what the systems do. So the levers are: fewer systems
installed (P1, P3), fewer and coarser systems where many small ones do one
job (the ten narrative ledgers), a set run condition that skips a whole
feature group when it has no work (a skipped system pays none of the per-system
work above), and fewer registered types (291 message types, 330 GGRS snapshot
resources).

Lines of ours that do take time (each under 1.5%): `room_instance.rs:172`
(1.4%), `room_graph.rs:432` and `:471`, `clip_material.rs:107`,
`ambition_entity_catalog/src/lib.rs:1563`, `touch_input/virtual_device.rs:50`,
`wallet_shield.rs:40`, `held_items.rs:108`, `hand.rs:63`.

**Mary-O only, against every game (phone, one run each).** MEASURED: with
`AMBITION_HUB_GAMES=mary_o` the app used 41.7 s of CPU in 30 s, against
49.9 s with every game (-17%), and 38% fewer context switches. Frame time was
not better in this pair (p50 41-69 ms against 24-46 ms), but the pair is
confounded: other commits (97ff2541e, 11130fb49), one run each, and the place
in the level. Clocks were at their maximum and thermal status 0 in both.

### F1 — The hosts install everything; the capability profiles can omit five things

MEASURED. Both demo hosts build `PlatformerEnginePlugins::fixed_tick()`
(`game/ambition_demo_mary_o_app/src/lib.rs:45,143`,
`game/ambition_demo_sanic_app/src/lib.rs:28,121`); no host calls `for_profile`.
`Capability` has five variants (`profile.rs:32`). Grabs and throws, the Smash
moveset, mounts and match stocks are installed inside `CombatSchedulePlugin`
(`ambition_platformer2d_runtime/src/combat_schedule.rs`, mounts at `:776`),
where a plugin group cannot disable them. Boss encounters CAN be omitted
already: `BossCatalog` is core state (`runtime/src/sim_core_resources.rs:194-203`)
and the `encounters-without-named-bosses` profile is witnessed; the demo hosts
simply do not ask. `BossEncounterSimulationPlugin` initializes the catalog a
second time (`boss_encounter/src/lib.rs:132`), and the run condition at
`actor_monolith/src/features/mod.rs:1094-1102` tests a resource that is now
always there. The extension phases `technique_execution` and `boss_conduct`
(about 29 systems) and `wielded_use`/`module_entity_tick` (about 20) are core,
whatever the profile. Mary-O's and Sanic's own movesets author
`smash.capture_attempt/pummel/throw`, so Grabs cannot leave a standalone game
until its fighter kit does (D4).

### F2 — Twelve narrative ledgers run in every game

MEASURED. `NarrativeInputPlugin<M>` (`ambition_conversation/src/ledger.rs:242`)
adds three systems per payload; twelve payloads (two in `ambition_conversation`,
seven in `actor_monolith/src/features/mod.rs:1491-1497`, three in
`ambition_content`) make 36 systems; a standalone game runs nine (27). The seven
monolith ones and the ten outside `ConversationPlugin` are
not removed by `Capability::Dialogue`. Every writer is in `ambition_conversation`
or `ambition_content`.

### F3 — The fixed schedules run on the multi-threaded executor

MEASURED. `serialize_frame_schedules`
(`ambition_platformer2d_runtime/src/lib.rs:741`) covers `First`..`Last`, not
`Fixed*`. Standalone games simulate in `FixedUpdate`. `dev/bevy_idle_systems`:
an idle system costs about 0.016 us on the single-threaded executor and 1.7-12 us
on the multi-threaded one. The executor policy has a second owner
(`game/ambition_app/src/app/plugins.rs:355`, an env switch the demos never reach).
Worse: the windowed demo hosts never call the policy at all.
`install_windowed_foundation` (`ambition_platformer2d/src/app.rs:1726-1810`)
does not call `serialize_frame_schedules`; its callers are the headless
foundation and `visible_composition.rs:107` only. So windowed Mary-O, Sanic and
Smash run `First`..`Last` and `Fixed*` on the multi-threaded executor. Bevy
itself runs `Main`, `FixedMain` and `RunFixedMainLoop` serially; the rollback
crate serializes `GgrsSchedule` only, not `SaveWorld`/`LoadWorld`/`ReadInputs`/
`AdvanceWorld`.

### F4 — In the hub, each game runs the other games' gameplay

MEASURED (10-14 census). While Ambition plays, Smash runs 49 systems, Mary-O 29,
Sanic 4. Scope gates: `ambition_demo_smash` 1 of 51 `add_systems`,
`ambition_demo_sanic` 2 of 33, `ambition_demo_mary_o` 11 of 52,
`ambition_content` 0 of 150. Physics debris (avian2d 20, interpolation 19) and
3D lighting (`bevy_light` 19, `bevy_pbr` 14) run in the hub; no code spawns a
light. Ambition's cast uses techniques that `ambition_demo_smash` registers
(`smash.counter`, `smash.place_spring`), so the hub cannot leave Smash out.

### F5 — Read-models rebuild each tick, inside the simulation, for the renderer

MEASURED. `SimViewPlugin` (`ambition_sim_view/src/facts.rs:1141-1180`) rebuilds
its views in the sim schedule's `FeatureViewSync` (about 36 systems); only
`ambition_render` (and the demo HUDs) read them. A headless host builds them for
no reader; on a slow frame with several ticks they are built several times.

### F6 — String identities are cloned, compared and sorted every tick

MEASURED (source). `select_actor_targets`, `collect_perception_peers`,
`rebuild_actor_anim_index`, `rebuild_body_pose_views`,
`rebuild_actor_render_index`, `rebuild_feature_view_index` and
`rebuild_nameplate_index` clone `SimId`/`FeatureId` strings, sort by string, or
key `HashMap`s by `String`, per body per tick.

### F7 — Two systems visit every entity every tick

MEASURED (source). `project_wallet_shields`
(`actor_monolith/src/features/wallet_shield.rs:38`) and
`brandish_the_playing_move_s_weapon` (`ambition_combat/src/held_items.rs:93`)
iterate a query with no required component, so they visit every entity in the
world each tick (the hub census counts about 8000 entities, 1300 live). A body
they can act on has `WornCharacter` or `BodyWalletShield`, and `MovePlayback` or
`MoveBrandishedItem`; the filters `Or<(With<WornCharacter>,
With<BodyWalletShield>)>` and `Or<(With<MovePlayback>, With<MoveBrandishedItem>)>`
keep what they do and drop the walk. Eight other queries in the scan have no
required component but are only used with `get(entity)`, which costs nothing.
`character_runtime/presentation.rs:261` walks every body only on the tick the
registry changes.

### F8 — Plumbing that scales with what is registered, not with what is used

MEASURED. Gizmos: 8 groups x 4 systems (16 for `RoomTrailGizmos0..3`); Bevy's
debug gizmo groups are on. Kira: 15 channel types x 2 systems. Message types:
291 `Messages<T>` resources, each with its own update. Developer tools: about
18 systems in shipping hosts (`DevToolsSimPlugin`, including a file-watch poll).
Input: `publish_seat_bindings` builds a `BTreeMap` and formats strings every
frame to find a rebind.

### F9 — Upstream query-state costs

MEASURED (10-13 phone profile). `virtual_joystick` is fixed in our fork.
Open: `bevy_ecs_ldtk` `clean_respawn_entities` (~0.8%), `bevy_yarnspinner`
`fetch_resources` (~0.6%), `bevy_light` `check_dir_light_mesh_visibility`
(~0.5%), `bevy_ggrs` `handle_requests` (0.2-0.4%).

### F10 — Content in engine crates (composition hygiene, not CPU)

MEASURED. `HELD_ITEMS` (`ambition_characters/src/brain/action_set/mod.rs`, 25
Ambition items), the closed `ambition_items::Item` enum and `icon_path`,
`fire_puppy_slug_gun_system` and its rollback key in every game, portal-gun
`gun_*` modules in the generic `ambition_portal2d`, `BUILTIN_STRIKE_KEYS` with a
second list of the same 11 keys, the invented `gradient_sentinel` boss.

### F11 — The main thread is the limit; the pool and the render thread are not

MEASURED (both phone runs, the `threads` table and `--thread TID`, 30 s each).
Busy, as a share of one core over the recording (Mary-O-only / all games):

| thread | busy |
| --- | --- |
| main (`android_main`) | 82% / 83% |
| Bevy render thread (also named `android_main`: an unnamed `std::thread` has its parent's name) | 25% / 26% |
| compute task pool, 4 workers | 8-9% each |

On the main thread (56.5% of the process samples in the Mary-O-only run):

- 34.9 points (62% of the thread) are under `app.run()`
  (`ambition_app/src/app/cli.rs:273`) with no system of ours below them:
  access bitset tests, `update_archetypes_unsafe_world_cell`,
  `is_apply_deferred`, `run_without_applying_deferred`, mimalloc, and Bevy's
  own systems.
- The rest is in system bodies of ours, spread thin: the largest is 1.4 points
  (`lifecycle/room_instance.rs:172`), all others under 0.6.

The render thread (17.1%) has no frame of ours: kernel, the Adreno Vulkan
driver, wgpu.

Reading (REASONED): the frame waits on the main thread. It is on-CPU 82% of
the time; the off-CPU rest is most likely the multi-threaded executor waiting
for its tasks, because the render thread is busy only a quarter of the time.
The pool does little: the parallel executor buys almost no parallel work and
costs the scheduling. This is the strongest support for P2 (serial schedules)
and P1 (fewer systems). The GPU is not the limit of these runs: the render
thread, which submits and presents, is idle three quarters of the time.

### F12 — No catch-up limit of ours on the fixed step

MEASURED (code read). The hub's GGRS loop (`bevy_ggrs` `schedule_systems.rs:55`)
runs `while accumulator >= 1/60 s` on `Time::delta()`; the standalone games use
Bevy's `FixedUpdate`. Neither has a limit of ours; only Bevy's `Time<Virtual>`
`max_delta` (250 ms) applies, so one slow frame can run up to 15 ticks. The
offline GGRS session uses `check_distance: 0`, which skips every save
(`ggrs` `sync_test_session.rs:155`).

Reading (REASONED): with p50 frames of 24-46 ms on the phone, a frame runs
1.5-2.8 ticks on average, and the read-models of `FeatureViewSync` are rebuilt
on each tick (P5). A lower catch-up limit trades real time for frame time (the
game slows down under load): D11.

### F13 — Three quarters of a standalone frame's allocations are input lookups

MEASURED (`perf_probe`, desktop debug, 578 frames, allocations per tracing
span; scratchpad `probe/`). Allocations per frame, standalone Mary-O 1-1 /
Sanic act 1:

| span | Mary-O | Sanic |
| --- | --- | --- |
| `leafwing_input_manager::systems::update_action_state` | 19,222 | 19,222 |
| multi-threaded executor | 2,293 | 2,116 |
| `view_isolation::isolate_live_rooms` | 444 | 2,773 |
| `bevy_render::render_asset::prepare_assets` | — | 1,064 |
| `hand::reconcile_effective_repertoire` | 596 | — |
| `starting_character::gate_body_control` | 295 | 123 |
| `authorize_staged_character_presentation_sources` | 203 | 210 |
| `dev::trace::actor_oob::record_actor_oob_frame_system` (a developer trace) | 196 | — |

The input number is the same in both games: it does not depend on the scene.
The hub makes 31,942 a frame (26.6k a call). Two causes in
`leafwing-input-manager` 0.21, both run twice a frame (frame and fixed loop):

- MEASURED (source read), the larger by the count per call: `InputMap::handle_clashes`
  calls `get_clashes`, which calls `possible_clashes()` on every call
  (`src/clashing_inputs.rs:190-232`): every pair of button actions, each
  binding decomposed into new `Vec`s. The comment there says "cached"; it is
  not. Our host chooses `ClashStrategy::PressAll` for chord-free maps
  (`ambition_platformer2d_host/src/lib.rs:373`), and under `PressAll` a clash
  removes nothing, so all of this work has no effect for us.
- MEASURED (source read): `CentralInputStore::pressed` and `button_value`
  (`src/user_input/updating.rs:111-140`) box a clone of the binding for each
  lookup, to use it as a hash key (`// PERF: surely there's a way to avoid
  cloning here`).

Reading (REASONED): both fixes keep behaviour exactly, in a workspace fork with
upstream PRs, as for `bevy_ggrs`: return early from `handle_clashes` under
`PressAll`, and otherwise pair only the actions that are pressed; look up
without boxing (`hashbrown::Equivalent` on `&dyn Buttonlike`). The probe counts
allocations; their time on the phone is inside the 5.2% of `mimalloc` and the
3.1% of `alloc`.

Caveat on the per-frame numbers (MEASURED): the Mary-O probe run was taken
while the VM was loaded, at about 14 fixed ticks a frame against about 1.6 at
idle, so the per-frame numbers of fixed-step systems are high; the per-call
numbers (`allocs_per_call` in the CSV) are the ones to compare. Other per-call
numbers of ours: `reconcile_effective_repertoire` ~40, `gate_body_control`
~20, `record_actor_oob_frame_system` 13, `integrate_sim_bodies` 9.6. Sanic:
`prepare_assets<HitFlashMaterial>` ~1,070 a frame (133 KB), so a hit-flash
material is changed every frame. The write is already guarded
(`hit_flash.rs:416-437`, write only a change); the material changes because
each overlay follows its body's animation frame (`uv_rect`) also at intensity
0, and overlays live as long as their bodies and stay `Visible`. REASONED: an
overlay at intensity 0 draws nothing; tracking the frame only while a flash
is on would remove the re-uploads and the draws of idle overlays. The comment
there explains why the overlay asserts `Visible` (portal compositing), so this
needs a check of the portal path first.

### F14 — Room isolation re-derives every room mask every frame

MEASURED (source read; allocations from F13). `isolate_live_rooms`
(`ambition_render/src/rendering/view_isolation.rs:216`) walks the subtree of
every entity stamped `InRoomInstance` each frame, with a new `Vec` per root,
and rebuilds each `RenderLayers` with several clones (`in_room_band`,
`without_room_layers`). The result changes only when the live-room set or its
order changes, or when an entity or child is new. The room bands are layers
256-511 and the local-view band starts at 1024; Bevy's `RenderLayers` keeps one
`u64` inline (`bevy_camera` `render_layers.rs:23`), so each mask with a room
layer is on the heap, and each clone of it allocates. Sanic: 2,773
allocations a frame; Mary-O: 444.

Reading (REASONED): change-driven, with no behaviour change: walk all roots
only when `LiveRoomInstance` changed or a room was added or removed; else walk
only `Added<InRoomInstance>` roots and `Changed<Children>`. A second, smaller
lever: put the room bands in layers below 64, so a mask fits inline.

### F15 — A `&mut` that writes nothing wakes every change filter downstream

MEASURED (probe: `reconcile_effective_repertoire` ran 901 times in 901 ticks
of Mary-O, 596 allocations a frame; it is not in Sanic's list; source read).
Mary-O's `begin_star_power` (`game/ambition_demo_mary_o/src/star.rs:62`) calls
`worn.unequip(POCKET_QUASAR_ID)` on `Mut<WornEquipment>` every tick. The
`&mut` call marks `WornEquipment` changed whether or not a row is removed, so
the change-filtered `reconcile_effective_repertoire` re-folds the whole
repertoire every tick and writes `ActionSet` and `ActorMoveset`, which marks
those changed for each of their readers. Fix (no behaviour change): test
`wears` first, through `Deref`.

Reading (REASONED): this is a class, not one bug. Bevy marks a component
changed on `DerefMut`, not on a different value. The general answer is a
detector, not a code search: the probe reports, per component type, the
entities marked changed per frame; a component changed on every frame for
every entity is either a per-frame value (expected) or this bug.

### F16 — The common style: re-derive every tick, idempotently

MEASURED (probe allocations, source read). Four of the top allocators of ours
re-derive, every tick, a value whose inputs change rarely, and say so as a
virtue ("idempotent by construction", "running every tick can only add"):

| system | re-derives each tick | inputs change when |
| --- | --- | --- |
| `isolate_live_rooms` (F14) | every room entity's `RenderLayers` | the live-room set changes, an entity is new |
| `reconcile_effective_repertoire` (F15) | the repertoire, after a false change | equipment, identity, held item change |
| `gate_body_control` (`avatar/starting_character.rs:631`) | each body's action scheme (`derive_action_scheme`); the control prompt derives it again | abilities, moveset, action set, techniques change |
| `authorize_staged_character_presentation_sources` (`character_runtime/presentation.rs:54`) | each staged provider's SFX authorization, with catalog clones; its `ResMut` write marks the selection changed every tick | the cast, a bank or a catalog arrives |

Reading (REASONED): the style is robust (no missed update, no order bug,
rollback replays it) and its cost is per tick per body. The elegant pattern is
derive on change: a run condition (`resource_changed`, `any_with_component`),
a `Changed<T>` filter, or one cached component (`DerivedActionScheme`) that
both the gate and the prompt read. Under rollback a cached value must be
rollback state or derived from inputs that a restore marks changed; check
each cache against that before it lands. Together with F15 this is P12.

### F18 — Other steady churn of ours (probe)

MEASURED (probe churn, per frame at idle):

- Sanic: `gravity_visuals.rs:29` despawns and respawns every gravity-zone
  visual every frame (1.43 each way); it runs in `Update` with no condition.
- Mary-O: `rigged.rs:923` spawns rig part slots and despawns as many, ~1.4 a
  frame each.
- Every game: `lifecycle/session.rs:316` and `:326` insert
  `SessionScopedEntity` and then `InRoomInstance` as two inserts after each
  session spawn (4-8 a frame each): three archetype moves per spawn instead of
  one. Fix: one bundle in the spawn.
- Outside our code: `FixedMainScheduleOrder` removed and inserted per fixed
  tick (Bevy), `bevy_ggrs` reinserts `Session`, `PlayerInputs` and
  `FixedTimestepData` every frame.

### F17 — The hub respawns a room's parallax every frame

MEASURED (probe churn, hub `basement_enemies`, 595 frames): exactly 1.000
`PresentedRoomParallax` root spawn, 2.000 parallax layer spawns, 2.000 layer
despawns and 2.000 `PresentedForView` inserts (`parallax.rs:623`) per frame;
also 1.000 `WieldedItemVisual` spawn per frame. A steady rate of exactly one
per frame is a loop, not a load.

Reading (REASONED, to confirm with the change detector of F15):
`refresh_parallax_layers_on_quality_change` (`parallax.rs:325`) despawns every
parallax layer and every `PresentedRoomParallax` when `GameAssets` is changed;
`present_live_room_parallax` spawns them again on the next frame. So
`GameAssets` is most likely marked changed every frame. A candidate writer:
`ensure_active_room_parallax_theme` (`parallax.rs:458`) calls
`ensure_parallax_layers_for_room` through `ResMut<GameAssets>` for a live room
whose theme is not resident and not marked "without art". The comment at
`parallax.rs:481` already names this risk for the resident case.

### F19 — A widely shared system parameter is a per-system cost

MEASURED (phone, `--ours --thread`): the largest line of ours on the main
thread, 1.36% of the process, is `lifecycle/room_instance.rs:172`, the
`#[derive(SystemParam)]` of `LiveRooms`: the fetch and validation of its two
queries (`roots`, and `stamps` over every entity with `InRoomInstance`), with
the bodies inlined. About 208 parameter declarations in `crates/` and `game/`
take `LiveRooms` or `LiveRoomOf` (git grep), so the cost is paid once per run
of each of those systems.

Reading (REASONED): the live-room set is one or two values that change on a
room transition. A resource kept by one system (the live rooms in order, and
each band) is a cheaper parameter than a query: no archetype matching and no
per-run query validation. A system that asks for the room of its own entities
can carry `Option<&InRoomInstance>` in its own query. Measure one system first.

### Not a lever (keep out of this run)

Session-scoped App globals (C03: 30 counted plus 19 uncounted instances) are
reset only on session edges: correctness work (S6), no per-frame cost. The
`ambition_platformer2d` facade (209 `pub use`) costs compile time only.
Rollback costs about 0% offline (`check_distance: 0`).

## Proposals, ranked by structural leverage

| # | Proposal | Removes | Size | Behaviour change |
| --- | --- | --- | --- | --- |
| P1 | Per-game engine profiles: `BossCatalog` always present (empty); ledgers under Dialogue; grabs, mounts, match stocks as group members with capabilities; extension lowering installed by the capability that owns the phase; demo hosts call `for_profile` | about 115 systems a tick in Mary-O and Sanic | large | no, except decisions D3, D4 |
| P2 | One executor policy (`apply_executor_policy`), called by every foundation including the windowed demos, covering `Fixed*` and `StateTransition`; A/B first | the multi-threaded executor cost of every frame and fixed system in the windowed demos | small | no |
| P3 | Hub: gate each game's session gameplay by its scope (S2); install Ambition, physics debris and 3D lighting only for the games that use them (S2b); move shared techniques out of `ambition_demo_smash` | 82+ foreign systems while one game plays, 72 hub-only systems | medium | no |
| P4 | Interned identities: an integer handle per body, made when its `SimId` is minted; read-model keys by `Entity` | String clones and sorts in 5+ per-tick systems | medium | no |
| P5 | Read-models built once per drawn frame, by change | repeat builds on slow frames; builds in headless hosts | medium | D2 |
| P6 | Change-driven systems: the two whole-world queries, input bindings, room isolation (F14) | full scans per tick; 444-2,773 allocations a frame in room isolation | small | no |
| P12 | Derive on change, not every tick (F15, F16): the four systems, then the change detector finds the next ones | per-tick re-derivations and false changes that wake change filters | small each | no |
| P11 | Leafwing fork (F13): no clash work under `PressAll`, pairs of pressed actions only otherwise; lookups without boxing; upstream PRs | 19-32k allocations a frame, ~75% of a standalone frame's | small | no |
| P7 | Plumbing: one gizmo trail group, Bevy debug gizmos off in shipping, developer tools only in developer builds, Kira channels | ~80 systems | small-medium | D6, D9 |
| P8 | Upstream query-state fixes (LDtk, Yarn, bevy_light, GGRS) | ~2% of phone CPU | small each | D5 (only for the LDtk workaround) |
| P10 | Phone render scale: a reduced main render target, scaled to the surface (`quality.rs:190` has no phone path) | fragments, when the GPU is the limit; not the limit today (F11) | medium | D10 |
| P9 | Content to data: held items, the item enum, Ambition-only item systems | Ambition work in other games; engine edits for content | large | D7 |

## Implementation plans

### P2 — one executor policy (one commit, an A/B, one commit)

- Rename `serialize_frame_schedules` to `apply_executor_policy(app)`
  (`runtime/src/lib.rs:741`), the only place that sets a main-world executor.
  It serializes each schedule that exists: `First`, `PreUpdate`,
  `StateTransition`, `Update`, `SpawnScene`, `PostUpdate`, `Last` and
  `FixedFirst`..`FixedLast`. It leaves the render sub-app and Bevy's own serial
  schedules alone; the GGRS schedules stay with the rollback crate (measure
  `SaveWorld`/`LoadWorld` in the same run).
- Call it from `install_windowed_foundation` as well; delete the second owner
  (`ambition_app/src/app/plugins.rs:348-374`); fix the false comment at
  `combat_schedule.rs:985`.
- A/B: one variable, `AMBITION_SCHEDULE_EXECUTOR` = `frame-serial` (today) |
  `serial` | `parallel`, read once, logged once, deleted after the decision.
  Standalone Mary-O needs a committed tick-only binary (`build_demo_app()`,
  `--ticks N`) so `[census] phases` is not dominated by rendering; 3+
  interleaved repetitions per arm; equal `AMBITION_SYSTEM_LIST` per arm; the
  `room_replay` test under each arm for equal state.

### P1 — per-game engine profiles (ordered commits, each green)

`supported_profiles.rs` iterates `SUPPORTED_PROFILES` and `Capability::ALL`, so
each new capability gets the step test, the not-installed probe and the
session-edge check from the existing harness.

1. **C1 Extension phases by owner.** `technique_execution` and `boss_conduct`
   into `BossExtensionPhasesPlugin`; `wielded_use` and `module_entity_tick` into
   `WieldedExtensionPhasesPlugin` (`extension_composition.rs:32-205`); install
   order inside each phase kept. Delete the second `BossCatalog` init; gate
   `drive_boss_animators` on the registry. Test: no port of an omitted phase is
   offered; a `boss_conduct` module is refused at admission. About 29 + 20
   systems.
2. **C2 Dialogue owns the ledgers and the dialog view.** The seven monolith
   ledgers into `NarrativePayloadLedgersPlugin`; `rebuild_dialog_view` out of
   `SimViewPlugin` into `DialogViewPlugin`. About 22 systems (only for a game
   that omits dialogue, D3).
3. **C3 Mounts.** `MountsPlugin` takes `combat_schedule.rs:776-800` and
   `install_mount_pose_systems`; the mount ports in the boss phases are
   installed only with mounts, decided by the composition root. 7 systems.
4. **C4 Match stocks.** `StocksMatchPlugin` takes `features/mod.rs:925-975` and
   the finish zoom; the Smash and versus rules assert it. 7 systems.
5. **C5 Grabs.** `CapturePlugin` takes `combat_schedule.rs:525-672`; the control
   gate chain splits on two capture-owned sets (same order). 14 systems, only
   after D4.
6. **C6 Game profiles.** A profile `platformer-without-bosses-mounts-stocks`;
   each game exports `ENGINE_PROFILE`; the Mary-O and Sanic hosts call
   `for_profile`; pack admission compiles against the schemas of the profile.
   About 55-85 systems a tick per standalone game; re-measure.
7. **C7** after D3 and D4: Dialogue, Grabs and the Smash moveset leave the
   standalone profiles.

## Decisions for Jon

Each of these changes what a player sees or what a reader gets. Nothing here
is done without a yes.

- **D1 — 30 Hz simulation on low tiers.** At most 13-17% of phone CPU (the
  simulation is 26-35%). Breaks unless fixed first: riders on rising one-way
  platforms (14 px a tick against 8 px slop), regions under 52 px, hitstop
  length, NPC jump prediction, AI and enemy cadence in ticks. Needs the
  tick-rate plan's P0 items.
- **D2 — Read-models per drawn frame.** RL, headless and trace readers must ask
  for them.
- **D3 — Standalone Mary-O and Sanic without Dialogue.** Both have
  `fallback_dialogue` rows. No standalone build installs a dialogue presenter
  today (only `ambition_app` installs the Yarn bridge), so those lines are
  never shown there now. Refuse such rows at admission, or ignore them?
  REASONED (code read): a standalone game can enter `GameMode::Dialogue` and
  not leave it. `DialogSimStatePlugin` is installed, its `DialogueNodeIndex` is
  unpopulated, so `entry_node` opens every non-self conversation
  (`ambition_dialog/src/context.rs:122`), and `interact.rs:395` sets
  `Dialogue`; the only system that returns to `Playing` is
  `sync_dialogue_game_mode`, which only `ambition_app` installs. Today the rows
  with dialogue are Hall characters (`tier: MainHall`), and the standalone
  level actors are hostile, so no reached path is known. Omitting Dialogue from
  the standalone profile (C2/C7) removes the path.
- **D4 — Smash moveset and grabs out of standalone Mary-O and Sanic.** Their
  own movesets author attacks and `smash.capture_attempt/pummel/throw`, so this
  means moving their fighter kits out of the standalone packs (the hub keeps
  them).
- **D5 — LDtk respawn one frame late** (a set condition) instead of waiting for
  the upstream fix.
- **D6 — Kira dynamic channels.** Changes how music layers crossfade.
- **D7 — Unknown boss ids refused at admission** instead of the invented
  fallback boss.
- **D8 — `SimId` minted by a spawn observer.** Moves where in the tick identity
  is made (determinism).
- **D9 — The per-room gizmo trail with two or more live rooms:** rebuild on
  retained gizmos, or drop.
- **D10 — Phone render scale** (P10): a lower internal resolution on the low
  tiers. Visible. Worth it only when a phone run shows the render thread or
  the GPU as the limit; today the main thread is (F11).
- **D11 — A catch-up limit** of N ticks per drawn frame (F12): below the limit
  the game keeps real time; above it, the game slows down instead of dropping
  more frames. Visible under load only.

## A second opinion, weighed (GPT, 2026-10-16)

Jon passed on GPT's diagnosis. Its points against this page's evidence:

| GPT's point | Here | Verdict |
| --- | --- | --- |
| Mary-O pays for Ambition's capabilities; install less, not faster | F0, F1, P1 | agrees; MEASURED (-17% CPU with only Mary-O in the hub; 170 unused systems a frame in standalone Mary-O) |
| Read-models rebuilt per tick, so slow frames repeat them | P5, D2, F12 | agrees; REASONED (no limit of ours on catch-up, F12) |
| Phones have no reduced render target | P10, D10 | true (`quality.rs:190`); not the limit of today's runs: the main thread is the limit and the render thread is idle 75% of the time (F11) |
| Transparent overdraw (5.3x on the HD 630) | journal 2026-08-29 | a GPU cost on a desktop iGPU; not measured on the phone; same reasoning as P10 |
| Effects composite bodies off-screen | — | agrees it is not a steady cost (direct part drawing since 10-05); spikes only; not measured here |
| Asset upload stalls cause the worst drops | — | plausible for p95; the phone runs did not separate spikes from the steady cost |
| Single-threaded executor for the standalone fixed step | P2 | agrees, and wider: the windowed demos run every schedule multi-threaded (F3), and F11 shows why it matters |
| Keep 60 Hz for now | D1 | agrees; D1 stays Jon's call |
| Do not rewrite movement, broadphase or AI | — | agrees; no system body of ours is over 1.4% on the main thread (F11) |

## Run log (2026-10-16)

- 04:40Z armed. Inputs: four read-only audits of 10-15 (composition plan
  status, consolidation C03/C08/C09, GPT's 10-11 notes and the tick-rate audit,
  warts and queue) and a fresh system census; the phone baseline above.
- Android attribution: `profile_android.sh` now keeps every sample's call chain
  (`simpleperf report-sample --show-callchain`); the `-g` reports drop branches
  under 0.25%, so they could not name a system. `symbolize_android_profile.py`
  picks the library of the sampled build (it took the first of two installs),
  and has `--lines` and `--ours` (commit 1e1ca6525's parent).
- 05:30Z phone: the sample dump of the baseline (Jon) gives F0; the
  Mary-O-only run gives -17% CPU. Jon cannot run the phone again tonight; the
  rest of the run measures on the headless desktop.
- In progress: the `perf_probe` feature (allocations per span, churn per
  source line).
- 05:40Z P1/P2 plans written (an agent's code read); F1/F2/F3 corrected (bosses
  can already be omitted; twelve ledgers; the windowed demos never serialize).
- 05:58Z `symbolize_android_profile.py`: a threads table and `--thread`. F11:
  the main thread is the limit (a first table summed it with the render
  thread, which has the same name; now split by thread id). F12: no catch-up
  limit. GPT's diagnosis weighed.
