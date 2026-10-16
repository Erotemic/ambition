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

## Summary (2026-10-16)

- **One thread is the limit.** On the phone the main thread is 82% busy, the
  render thread 25%, each of 4 pool workers 8-9% (F11). 62% of the main
  thread is framework work outside any system body: per-system scheduling,
  access checks, archetype updates, allocation (F0, F11). The cost follows how
  many systems run, not what they do; no system body of ours is over 1.4%.
  Levers: fewer systems per game (P1), a serial executor for the
  standalone games (P2: MEASURED on the desktop, a standalone tick is ~6×
  cheaper and the main thread does 23% less work per second while drawing
  32% more frames, F22). In the hub (what the phone runs) the simulation is
  already serial and only ~15% of the main thread; the frame phases are the
  rest (F23).
- **Input was 82% of a standalone frame's allocations** (19.2k a frame,
  F13): leafwing rebuilt every action pair on each call though we resolve no
  clashes. Fixed in this repo: allocations a frame Mary-O 23,407 → 4,128,
  hub 46,949 → 15,257 (pixel-identical capture). On the phone the pass was
  4.5% of all CPU samples (~8% of the main thread).
- **Change detection is defeated.** About 30 components of every body are
  marked changed every tick whether they change or not (F20), and several
  resources too; one no-op `&mut` re-folded Mary-O's whole repertoire every
  tick (F15). The common style re-derives every tick (F16). The new change
  counter names each such write by source line. Levers: P12, then P5.
- **Presentation entities in immediate mode** (F21): despawn all and spawn
  all, every frame. Three loops found, all fixed.
- **Composition:** standalone Mary-O and Sanic run ~170 systems a frame for
  capabilities they do not use (P1; D3, D4 for Jon). Both hosts now build the
  engine without named boss encounters (−13 and −11 systems a frame); most
  of the rest is in the core extension phases (C1).
- **Fixed tonight, no behaviour change, each verified by the probe, a
  pixel-identical hub capture, or a test:** Mary-O's star re-fold (F15),
  Sanic's gravity visuals (F18), the hub's per-frame parallax respawn and its
  cause in `first_room_art.rs` (F17), the hub's wielded-item overlays (F21),
  room isolation walking only when a mask can be stale (F14: 443 → 7.6
  allocations a frame in Mary-O, 1,118 → 2.0 in Sanic), read-before-write
  for the SFX authorizer, the ledgers and the trail gizmo bands (F20), HUD,
  feature-visual and hit-flash writes only of a change (F20), seat bindings
  projected only on a change, the two whole-world queries filtered (F7), and
  leafwing's clash pass skipped under `PressAll` (F13).
- **Before → after tonight** (MEASURED, `perf_probe`, idle, per frame,
  without the probe's and the census's own work; `probe/*_v2` against
  `probe/*_final`):

  | | Mary-O | hub `basement_enemies` |
  | --- | ---: | ---: |
  | allocations in our systems | 2,218 → 1,156 (−48%) | 2,328 → 1,743 (−25%) |
  | all allocations | 24,772 → 23,407 | 47,478 → 46,949 |
  | of which leafwing input (fixed after this table, F13) | 19,222 | 31,942 |
  | archetype churn from our code | 10.3 → 7.2 | 22.2 → 7.2 (−68%) |
  | value changes from our code | 1,575 → 1,379 | 994 → 956 |

  After the table, the leafwing fix (F13): all allocations a frame Mary-O
  23,407 → 4,128, hub 46,949 → 15,257. Sanic act 1 over the whole night
  (`probe/sanic_act1_v2` → `probe/sanic_act1_final`): all allocations 24,034
  → 3,492 (−85%), in our systems 1,942 → 692, value changes from our code
  682 → 503. The large remaining items: the serial executor for the
  standalone games (D12, needs a yes), then P1 and P12.
- **Time, desktop** (MEASURED, `scripts/executor_ab.sh`, 3 interleaved
  repetitions, the same script early in the night and at the end; the box's
  load differed, so read the direction, not the digit). Main-schedule time
  per drawn frame: Mary-O today's policy 14.1 → 12.6 ms (−11%), with
  `serial` (D12) 8.3 → 7.0 ms; hub today's policy 13.4 → 12.4 ms (−7%), with
  `serial` 12.5 → 10.9 ms.
- **What tonight's fixes held on the phone** (MEASURED, the all-games phone
  profile of 10-16, `symbolize_android_profile.py --under NAME`, inclusive
  share of all process CPU samples): `handle_clashes` 3.14%,
  `reconcile_effective_repertoire` (the star's false change) 1.43%,
  `isolate_live_rooms` 0.47%, the SFX authorizer 0.40%,
  `publish_seat_bindings` 0.22%, the HUD 0.13%, the rest under 0.1% each:
  about 6.2% of all samples, ~11% of the main thread. The first two are gone;
  the others now run only on a change. REASONED until a new phone run
  measures it.
- **Instrumentation:** `perf_probe` (allocations per span, churn and value
  changes per source line; `profile_desktop.sh --perf-probe`); the phone
  symbolizer's thread table, `--thread`, `--ours`, `--inclusive` and
  `--under`; the executor A/B switch (`AMBITION_SCHEDULE_EXECUTOR`,
  `scripts/executor_ab.sh`, to remove with D12). Each is one switch, one
  directory or one script to remove.
- **For Jon:** D1-D12 below (D13 withdrawn: fixed without a fork). None is
  done. D12 (serial executor default for the standalone games) changes no
  behaviour and is ready.

## Next, in order

1. A phone run of the Mary-O-only hub (`profile_android.sh`, as 10-16) to
   measure tonight's fixes where they matter; compare with
   `android-maryo-only-1-1b` by `--thread` and `--under`.
2. D12: a yes for the serial executor in the standalone games (F22).
3. P1 C1: the boss and wielded extension phases installed by their owning
   capability (~49 systems a frame in Mary-O and Sanic), then C2-C6.
4. P12: the body step marks only real changes (F20); then the change-driven
   read-models (P5) can skip.
5. F19: the live-room lookups as one resource, not ~280 query parameters.
6. F24: one composed collision world per room per tick, then a layered view.
7. The hub's ~9.4k allocations a frame with no system owner (F13, open).

## How to answer a cost question

| Question | Tool |
| --- | --- |
| Which systems run in a frame, by crate? | `AMBITION_PROFILE_CENSUS=1` + `AMBITION_SYSTEM_LIST=PATH`; `scripts/system_census_table.py` |
| Which Bevy system takes the time (desktop)? | `scripts/profile_desktop.sh` → `tracy_summary.md` |
| Where does the phone spend its CPU? | `scripts/profile_android.sh all ...`, then `scripts/symbolize_android_profile.py DIR` |
| Which line of OURS caused framework time (phone)? | `symbolize_android_profile.py DIR --ours` (needs `simpleperf-device-samples.txt`, which a profile made after 2026-10-16 has) |
| Which functions of ours cost the most, framework time under them included (phone)? | `symbolize_android_profile.py DIR --inclusive [--thread TID]` |
| How much phone CPU passes through one function (inclusive)? | `symbolize_android_profile.py DIR --under NAME` (repeatable; with `--thread TID`) |
| Which thread is saturated, and what does it do (phone)? | the `threads` table of `symbolize_android_profile.py DIR` (busy % of one core, by thread id), then `--ours --thread TID` |
| Which system allocates, and which code makes archetype churn? | `profile_desktop.sh --perf-probe` → `alloc_by_span.csv`, `churn_by_site.csv` (recipe `profiling.md` §1d) |
| Which code marks a component or resource changed every frame (wakes every `Changed<T>` reader)? | the same run → `changes_by_site.csv` |

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
(`bevy_ecs/query/access.rs` with `fixedbitset`, about 2.9%), the <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->
`TypeId` test that finds apply-deferred points (`system/system.rs` and <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->
`executor/mod.rs`, about 2.5%), archetype-cache updates <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->
(`update_archetypes_unsafe_world_cell`, `query/state.rs`), parameter <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->
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
(1.4%), `room_graph.rs:432` and `:471`, `clip_material.rs:145`,
`ambition_entity_catalog/src/lib.rs:1563`, `crates/ambition_touch_input/src/virtual_device.rs:50`,
`wallet_shield.rs:45`, `held_items.rs:108`, `hand.rs:63`.

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
`Capability` has five variants (`crates/ambition_platformer2d_runtime/src/profile.rs:32`). Grabs and throws, the Smash
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
whatever the profile. Since 2026-10-16 the Mary-O and Sanic hosts build the
engine without `Capability::BossEncounters` (their integration tests pass:
95 and 42). MEASURED: systems run a frame Mary-O 1,079 → 1,066, Sanic 1,086
→ 1,075; the rest of the boss systems are in the core extension phases (C1).
Mary-O's and Sanic's own movesets author
`smash.capture_attempt/pummel/throw`, so Grabs cannot leave a standalone game
until its fighter kit does (D4).

### F2 — Twelve narrative ledgers run in every game

MEASURED. `NarrativeInputPlugin<M>` (`ambition_conversation/src/ledger.rs:247`)
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
Worse: the windowed demo hosts did not call the policy at all; only the
headless foundation and `game/ambition_app/src/app/visible_composition.rs`
did. So windowed Mary-O, Sanic and Smash run `First`..`Last` and `Fixed*` on
the multi-threaded executor. Since 2026-10-16 all three foundations call
`apply_executor_policy`, with unchanged defaults (F22, D12). Bevy
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
(`actor_monolith/src/features/wallet_shield.rs:32`) and
`brandish_the_playing_move_s_weapon` (`ambition_combat/src/held_items.rs:91`)
iterate a query with no required component, so they visit every entity in the
world each tick (the hub census counts about 8000 entities, 1300 live). A body
they can act on has `WornCharacter` or `BodyWalletShield`, and `MovePlayback` or
`MoveBrandishedItem`; the filters `Or<(With<WornCharacter>,
With<BodyWalletShield>)>` and `Or<(With<MovePlayback>, With<MoveBrandishedItem>)>`
keep what they do and drop the walk (both applied 2026-10-16; the
wallet-shield, held-item, Mary-O and Sanic tests pass). Eight other queries in the scan have no
required component but are only used with `get(entity)`, which costs nothing.
`project_prepared_character_definitions` (`character_runtime/presentation.rs`)
walks every body only on the tick the registry changes.

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

MEASURED (code read). The hub's GGRS loop (`bevy_ggrs` `schedule_systems.rs:55`) <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->
runs `while accumulator >= 1/60 s` on `Time::delta()`; the standalone games use
Bevy's `FixedUpdate`. Neither has a limit of ours; only Bevy's `Time<Virtual>`
`max_delta` (250 ms) applies, so one slow frame can run up to 15 ticks. The
offline GGRS session uses `check_distance: 0`, which skips every save
(`ggrs` `sync_test_session.rs:155`). <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->

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
  (`ambition_platformer2d_host/src/lib.rs:379`), and under `PressAll` a clash
  removes nothing, so all of this work has no effect for us.
- MEASURED (source read): `CentralInputStore::pressed` and `button_value`
  (`src/user_input/updating.rs:111-140`) box a clone of the binding for each
  lookup, to use it as a hash key (`// PERF: surely there's a way to avoid
  cloning here`).

Split (MEASURED, `crates/ambition_input/tests/leafwing_allocations.rs`, the
game's own seat map): one `process_actions` call makes 9,611 allocations (the
probe's per-call figure exactly), and 9,522 of them (99.1%) are
`handle_clashes`. The boxed lookups are the other ~90.

Fixed in this repo, with no fork (2026-10-16):
`ambition_input::press_all_action_update` removes leafwing's two
`update_action_state` systems (`App::remove_systems_in_set`) and adds one that
skips the clash pass under `PressAll` and calls `process_actions` under any
other strategy; the first half of `process_actions` is copied, and a test
compares the two on held keys. Verified (MEASURED, `probe/mary_o_1_1_pressall`):
all allocations of a standalone Mary-O frame 23,407 → 4,128; the input update
19,222 → 178. The hub (`probe/basement_enemies_pressall`, what the phone
runs): 46,949 → 15,257, the input update 31,942 → 258, the capture
pixel-identical.

On the phone (MEASURED, the existing profiles, `symbolize_android_profile.py
--under handle_clashes`): 4.5% of all process samples in the Mary-O-only run
(3.1% with every game) were inside `handle_clashes`, about 8% of the
saturated main thread. REASONED: that is the CPU this fix removes on the
phone; a new phone run will measure it. Tests: `ambition_input` 152, Mary-O 95, Sanic 42. Left for an
upstream PR: skip the pass in `process_actions` itself, and look up without
boxing (`hashbrown::Equivalent` on `&dyn Buttonlike`). The probe counts
allocations; their time on the phone is inside the 5.2% of `mimalloc` and the
3.1% of `alloc`.

Idle re-runs (MEASURED, `probe/mary_o_1_1_v2`, `probe/basement_enemies_v2`):
standalone Mary-O makes 26.0k allocations a frame in all spans, 19.2k of them
in `update_action_state` (74%); the hub makes 61.4k, 31.9k in it (67% without
the probe's own 13.4k). `gate_body_control` makes 189 a call,
`authorize_staged_character_presentation_sources` 202.

Caveat on the per-frame numbers (MEASURED): the first Mary-O probe run was taken
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

Open (MEASURED, `probe/basement_enemies_pressall`): after the fix, ~9.4k of
the hub's 15.3k allocations a frame are on schedule spans, not on a system:
`GgrsSchedule` 3,877, `Update` 3,067, `PostUpdate` 1,558, `PreUpdate` 857. In
standalone Mary-O (parallel executor) the same work is on system spans.
Two explanations are ruled out (MEASURED, source read): command
application has its own `(commands)` rows (Bevy enters `system_commands` in
`CommandQueue::apply`), and only Bevy's `DefaultPlugins` are built before
`capture_scene` installs the subscriber, so no system of ours has a dead span.
REASONED, left: observers and component hooks, which run with no system span
(the hub spawns and inserts more than Mary-O), and the executor. Next probe
step: a span in the probe's own observer path, or a Tracy look at
`GgrsSchedule`.

### F14 — Room isolation re-derives every room mask every frame

MEASURED (source read; allocations from F13). `isolate_live_rooms`
(`ambition_render/src/rendering/view_isolation.rs:216`) walks the subtree of
every entity stamped `InRoomInstance` each frame, with a new `Vec` per root,
and rebuilds each `RenderLayers` with several clones (`in_room_band`,
`without_room_layers`). The result changes only when the live-room set or its
order changes, or when an entity or child is new. The room bands are layers
256-511 and the local-view band starts at 1024; Bevy's `RenderLayers` keeps one
`u64` inline (`bevy_camera` `render_layers.rs:23`), so each mask with a room <!-- cite-ok: a source file of a dependency (bevy_ecs, bevy_camera, bevy_ggrs, ggrs), not of this repo -->
layer is on the heap, and each clone of it allocates. Sanic: 2,773
allocations a frame; Mary-O: 444.

Fixed (2026-10-16): the walk runs only when a mask can be stale (the
live-room set or its order changed; a stamp, a child or a `PresentedForView`
is new or changed; a mask was removed; or, while rooms are banded, another
system wrote a mask). Verified (MEASURED): allocations a frame Mary-O 443 →
7.6, Sanic 1,118 → 2.0, hub 274 → 19; the hub capture is pixel-identical; the
seven view-isolation tests pass. A second, smaller lever is left: put the room
bands in layers below 64, so a mask fits inline.

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

### F17 — The hub respawns a room's parallax every frame

MEASURED (probe churn, hub `basement_enemies`, 595 frames): exactly 1.000
`PresentedRoomParallax` root spawn, 2.000 parallax layer spawns, 2.000 layer
despawns and 2.000 `PresentedForView` inserts (`parallax.rs:623`) per frame;
also 1.000 `WieldedItemVisual` spawn per frame. A steady rate of exactly one
per frame is a loop, not a load.

Cause (MEASURED with the change counter, `probe/basement_enemies_v2`):
`GameAssets` was marked changed on 593 of 593 frames, by a caller inside
`Option::as_deref_mut`. `refresh_parallax_layers_on_quality_change`
(`parallax.rs:325`) despawns every parallax layer when `GameAssets` is changed,
and `present_live_room_parallax` spawns them again. The writer:
`prepare_first_room_art_system` (`ambition_app/src/app/world_flow/first_room_art.rs`)
took `GameAssets`, the atlas layouts and the character load states through
`as_deref_mut()` every frame, also with no session to prepare. Fixed (it
returns before the borrows when every published session is answered).
Verified (`probe/basement_enemies_v3`): `GameAssets` changed on 0 frames, no
parallax spawn or despawn.

### F18 — Other steady churn of ours (probe)

MEASURED (probe churn, per frame at idle):

- Sanic: `gravity_visuals.rs:29` despawns and respawns every gravity-zone
  visual every frame (1.43 each way in the first probe run); it runs in
  `Update` with no condition. Fixed; verified by a unit test (an unchanged
  zone keeps its entities). The later Sanic runs, before and after the fix,
  had no live gravity zone at the captured point, so the probe cannot show
  the difference.
- Mary-O: `crates/ambition_render/src/rendering/actors/rigged.rs:923` spawns rig part slots and despawns as many, ~1.4 a
  frame each.
- Every game: `lifecycle/session.rs:316` and `:326` insert
  `SessionScopedEntity` and then `InRoomInstance` as two inserts after each
  session spawn (4-8 a frame each): three archetype moves per spawn instead of
  one. Fix: one bundle in the spawn.
- Outside our code: `FixedMainScheduleOrder` removed and inserted per fixed
  tick (Bevy), `bevy_ggrs` reinserts `Session`, `PlayerInputs` and
  `FixedTimestepData` every frame.

### F19 — A widely shared system parameter is a per-system cost

MEASURED (phone, `--ours --thread`): the largest line of ours on the main
thread, 1.36% of the process, is `lifecycle/room_instance.rs:172`, the
`#[derive(SystemParam)]` of `LiveRooms`: the fetch and validation of its two
queries (`roots`, and `stamps` over every entity with `InRoomInstance`), with
the bodies inlined. About 208 parameter declarations in `crates/` and `game/`
take `LiveRooms` or `LiveRoomOf` (git grep), so the cost is paid once per run
of each of those systems.

The same holds for the two room-spec parameters of
`ambition_platformer2d_world/src/rooms/room_graph.rs`: `LiveRoomSpecs` (its
derive line, 0.51%; it embeds `LiveRooms`) and `SoleLiveRoomSpec` (0.32%).
Together the room-lookup parameters are ~2.2% of phone samples. All
parameter fetches together (`get_param`, inclusive, main thread of the
all-games run) are 3.59%. Users by a
`git grep` of parameter declarations: `LiveRooms` ~120, `LiveRoomSpecs` ~90,
`LiveRoomOf` ~72, `SoleLiveRoom` ~17, `SoleLiveRoomSpec` 5. (The next phone
line of ours, `crates/ambition_touch_input/src/virtual_device.rs:50`, 0.46%, is the touch button's
`decompose`, called by leafwing's clash pass; it went with F13.)

Reading (REASONED): the live-room set is one or two values that change on a
room transition. A resource kept by one system (the live rooms in order, and
each band) is a cheaper parameter than a query: no archetype matching and no
per-run query validation. A system that asks for the room of its own entities
can carry `Option<&InRoomInstance>` in its own query. Measure one system first.

### F20 — Every body's whole state is marked changed every tick

MEASURED (probe `changes_by_site.csv`, standalone Mary-O 1-1, 561 frames
after warm-up, `scratchpad/probe/mary_o_1_1_v2`). About 2,300 component values
are marked changed a frame. Of ours:

- ~30 simulation components of each of the ~17 bodies, every frame
  (`BodyKinematics`, `BodyHealth`, `BodyLedgeState`, `BodyDodgeState`,
  `BodyRestartLatch`, ... at `actor_clusters.rs:170-198`, and
  `update.rs:1508-1543`). Cause (source read): `as_actor_mut`
  (`actor_clusters.rs:163`) takes `&mut` of each cluster through `Mut<T>`,
  which marks it changed whether or not the step writes it.
- More per-body writes each frame: `BodyLifeStats` (`features/mod.rs:280`),
  `ResolvedTechniqueEdges` (`starting_character.rs:705`), `BodyMelee`,
  `RangedRefire`, `AttackGestureState`, `ResolvedAttackGesture`,
  `PogoTargetVolumes`, `BodyAnimFacts`, `PresentedPose`, `BodyPoseView`.
- Resources changed every frame by our code: `ActiveAudioSelection`
  (`authorize_staged_character_presentation_sources`, F16, fixed since), `GizmoConfigStore`
  (`avatar/trail.rs:677`), `CameraShakeState`, `FinishZoomState`
  (`camera_ease.rs`), `SeatControlFrameModes`, the narrative ledgers
  (`prune_narrative_inputs` and `release_narrative_inputs`, both fixed since)
  in a game with no dialogue, and two developer-tool
  resources (`EditablePlayerStats`, `PlayerStatsSyncSnapshot`).
- Rendering: rig part slots (`rigged.rs:1426-1434`, ~148 a frame:
  animation), the HUD's `Visibility` (`hud/declared.rs:784`, 30 a frame),
  ambient motes, hit-flash overlays (`hit_flash.rs:416`, fixed since).

Readers today (MEASURED, `git grep` for `Changed<T>`, `Ref<T>` and
`is_changed` per component): none for `BodyLifeStats`,
`ResolvedTechniqueEdges`, `BodyMelee`, `RangedRefire`, `AttackGestureState`,
`ResolvedAttackGesture`, `PogoTargetVolumes`, `BodyAnimFacts`,
`PresentedPose`, `BodyPoseView`, `DamageableVolumes`, `ActorRoll`,
`BodyHealth`; three for `BodyKinematics`, one for `ActionSet`. So the per-body
false changes cost nothing now; the resource false changes did (F15, F17).

Reading (REASONED): a false change costs little by itself. It matters
because it makes change detection useless: no `Changed<Body…>` filter, and no
read-model "built by change" (P5, P12), can skip a body while all of its state
is marked changed every tick. So the order is: first make writes mark only
real changes (pass `Mut<T>` into the step, or compare before writing), then
the change-driven read-models pay. The detector verifies each step.

Sanic act 1 (MEASURED, `probe/sanic_act1_v2`, 561 frames idle): ~1,200
values changed a frame; 79 components and resources of ours are changed on
about every frame. Among them the portal presentation takes `Assets<Mesh>`,
`Assets<Image>` and `Assets<ColorMaterial>` through `&mut` every frame
(`ambition_portal2d_presentation/src/view_cones.rs:1035-1037`) in a game with
no portals: a composition cost (P1) and a false change. Also every frame: the
input-system resources (`SeatControlFrameModes`, `SeatRawFrames`,
`MenuInputState`, ...), the camera read-models, the music and audio state,
the lifecycle resources of the provider (`ShellRouteHolds`,
`PreparedPlatformerSessions`, ...), the two trace buffers, and all the
narrative and host-intent ledgers.

Fixed with read-before-write, verified with the counter (MEASURED,
`probe/mary_o_1_1_v3`): `ActiveAudioSelection` (the SFX authorizer now runs
only when one of its inputs changed; its ~200 allocations a frame are gone),
`GizmoConfigStore` (trail bands), the host-intent ledgers (prune). The
narrative ledgers had a second every-frame writer, the release
(`ambition_conversation/src/ledger.rs:194`), now fixed the same way (covered
by the conversation tests).

Write-only-a-change in the presentation (MEASURED, `probe/mary_o_1_1_v5`
against `_v4`): the HUD helpers `set_shown`/`set_hidden` took
`&mut Visibility`, and a `Mut` coerced to it at the call is marked changed
before the helper's test runs, so every HUD node changed every frame; now
generic over the `Mut`. The feature visuals (`actors/mod.rs`) and the
hit-flash overlays now use `set_if_neq`. Per frame: `Visibility` changes 74 →
1, transforms propagated (`GlobalTransform`) 224 → 182, all value changes
2,216 → 2,017. The hub capture is pixel-identical.

Verified with it (MEASURED): the `star.rs` fix (F15) works. After it,
`ActionSet` and `ActorMoveset` were changed 17 times in 561 frames (loading),
and `reconcile_effective_repertoire` has no allocations left.

### F21 — Presentation entities drawn in immediate mode

MEASURED (probe churn): three presentation systems despawned all their
entities and spawned them again every frame: gravity-zone visuals (Sanic,
fixed), the parallax refresh (hub, F17, fixed at its cause), and wielded-item
overlays (`wielded_item_visuals.rs:153`, hub, 1.000 a frame; rewritten to
update in place; verified: no churn, pixel-identical capture). A code search for the same shape
(despawn every entity of a marker, then spawn) also finds
`item_visuals.rs`, `projectile_visuals.rs`, `mark_beacon.rs`, `hud/declared.rs`
and the portal presentation (`visuals.rs`, `gun_visuals.rs`, `far_side.rs`);
not all of them run every frame.

Reading (REASONED): this is immediate-mode drawing written with retained
entities. Each frame pays two archetype moves per entity, new render
entities and extraction state, and change marks for everything downstream.
The elegant pattern is one shared helper: keyed reconcile (key → entity:
update in place, spawn the new, despawn the gone), as the gravity and wielded
systems now do by hand.

### F22 — The serial executor makes a standalone tick about 6× cheaper

MEASURED (desktop VM, standalone Mary-O 1-1 offscreen, `capture_mary_o
--warmup 1200`, profiling build, 3 interleaved repetitions per arm, medians;
`scripts/executor_ab.sh`, `scripts/executor_ab_report.py`; the switch is
`AMBITION_SCHEDULE_EXECUTOR`, `apply_executor_policy` in
`ambition_platformer2d_runtime/src/lib.rs`):

| arm | sim wall per tick | main schedules, ms per second | frames per second | process CPU, s |
| --- | ---: | ---: | ---: | ---: |
| `parallel` (the windowed demos today) | 10.70 | 968 | 68.5 | 103.0 |
| `frame-serial` (`First`..`Last` serial; the hub's frame policy) | 10.78 | 918 | 73.1 | 99.4 |
| `serial` (also `Fixed*`, `StateTransition`, `SpawnScene`) | 1.68 | 745 | 90.1 | 96.6 |

The spread between repetitions is small (sim per tick 1.67-1.76 serial,
10.52-10.72 parallel). Process CPU is mostly software rendering on this VM
(llvmpipe), so the schedule times are the measure. Serializing only the frame
schedules does almost nothing for a standalone game: its simulation is in
`FixedUpdate` (501 systems), which stays multi-threaded.

Reading (REASONED): with ~500 small systems in `FixedUpdate`, the
multi-threaded executor's per-system dispatch costs more than the systems.
The phone shows the same shape (F11: main thread saturated, pool idle). The
hub's simulation is in `GgrsSchedule`, which the rollback crate already runs
serially, so this is a standalone-game lever (Mary-O, Sanic, Smash). The
single-threaded executor runs systems in a valid order of the declared
constraints; the multi-threaded one already may run unordered systems in any
order, so no new order is possible. Recommendation: make `Serial` the default
of every foundation, then remove the switch; check `room_replay` under it
first. Not done: it is a one-line default.

### F23 — In the hub the executor is not the lever; the frame phases are

MEASURED (hub `basement_enemies`, `capture_scene`, same method as F22,
`CAPTURE="capture_scene basement_enemies 0,0 {png} 1280x720" scripts/executor_ab.sh`):

| arm | sim wall per tick | main schedules, ms per second | frames per second |
| --- | ---: | ---: | ---: |
| `parallel` | 1.84 | 946 | 57.6 |
| `frame-serial` (the hub today) | 1.90 | 847 | 63.2 |
| `serial` | 1.93 | 823 | 66.1 |

The hub's simulation runs in `GgrsSchedule`, which the rollback crate already
runs serially, so it costs ~1.9 ms a tick in every arm, and `serial` adds only
3% less main-thread time over today. Main-thread time per second of the hub
today, by phase: `RunFixedMainLoop` 227 ms (the simulation is ~120 of it),
`Update` 193, `PreUpdate` 190, outside any schedule 159, `PostUpdate` 137,
`StateTransition` 53 (16 systems: about 0.8 ms a frame), `Last` 19,
`SpawnScene` 18. Standalone Mary-O (F22, parallel) is the opposite shape:
`RunFixedMainLoop` 673 of 968.

Reading (REASONED): the phone runs the hub app, so for the phone the levers
are the frame phases: fewer systems in them (P1, P3), the false changes and
re-derivations (P12, P13), and input (P11). The executor switch (P2) is the
standalone games' lever. `StateTransition` at ~0.8 ms a frame for 16 systems
is worth one Tracy look.

### F24 — The collision world is copied whole, by each system that asks

MEASURED (phone, all-games profile, main thread,
`symbolize_android_profile.py --inclusive --thread 8529`): under
`integrate_sim_bodies` (2.21% inclusive), `world_with_sandbox_solids` takes
0.58% (`world_with_moving_platforms`, 0.56%, is inside it). A generic
`clone` frame at 1.76% sits beside `integrate_body`, but the name is too
generic (and identical code is folded) to say what it copies.
Source read: `RoomCollision::solids`
(`ambition_platformer2d_world/src/collision.rs`) borrows the authored world
only when the room has no moving platform and an empty overlay; otherwise it
builds a new `ae::World`: a copy of the whole room, then the platforms, the
overlay's blocks, lines, gate solids, water and portal carves. The body step
does it once per room per step (`ComposedRooms`); about twenty other systems
(`git grep` of `.solids()`: brains' traversal, body modes, damage, held items,
boss ticks, pets, the traces, ...) each compose their own when they run.

Reading (REASONED): two steps, each without behaviour change. First, one
composed world per room per tick, kept on the room root and rebuilt when its
geometry, platforms or overlay changed, which every reader borrows. Then a
layered view (the authored world plus the dynamic parts, read in place) so
that nothing copies the room at all; `HostSurfaces` already reads two
sources in place this way.

### Not a lever (keep out of this run)

Session-scoped App globals (C03: 30 counted plus 19 uncounted instances) are
reset only on session edges: correctness work (S6), no per-frame cost. The
`ambition_platformer2d` facade (209 `pub use`) costs compile time only.
Rollback costs about 0% offline (`check_distance: 0`).

## Proposals, ranked by structural leverage

| # | Proposal | Removes | Size | Behaviour change |
| --- | --- | --- | --- | --- |
| P1 | Per-game engine profiles: `BossCatalog` always present (empty); ledgers under Dialogue; grabs, mounts, match stocks as group members with capabilities; extension lowering installed by the capability that owns the phase; demo hosts call `for_profile` | about 115 systems a tick in Mary-O and Sanic | large | no, except decisions D3, D4 |
| P2 | One executor policy (`apply_executor_policy`), called by every foundation including the windowed demos, covering `Fixed*` and `StateTransition`. Switch and A/B done (F22, F23): a standalone tick ~6× cheaper, the hub +3%; the default is D12 | the multi-threaded executor cost of every frame and fixed system in the windowed demos | small | no |
| P11 | DONE for the clash pass, in-repo (F13): `press_all_action_update`; an upstream PR for the pass and the boxed lookups remains | 19-32k allocations a frame, ~80% of a standalone frame's | small | no |
| P12 | Derive on change, not every tick (F15, F16, F20): first writes that mark only real changes (the cluster step, the resources of F20), then the four systems; the change detector (`changes_by_site.csv`) finds the next ones and verifies each | per-tick re-derivations and false changes that wake change filters | small each | no |
| P14 | One composed collision world per room per tick, shared; then a layered view with no copy (F24) | ~0.6% of phone CPU in the body step, plus each other composer | medium | no |
| P13 | One keyed-reconcile helper for presentation entities (F21), and the remaining immediate-mode systems moved onto it | per-frame despawn and spawn of presentation entities | small-medium | no |
| P3 | Hub: gate each game's session gameplay by its scope (S2); install Ambition, physics debris and 3D lighting only for the games that use them (S2b); move shared techniques out of `ambition_demo_smash` | 82+ foreign systems while one game plays, 72 hub-only systems | medium | no |
| P6 | Change-driven systems: the two whole-world queries, input bindings, room isolation (F14) | full scans per tick; 444-2,773 allocations a frame in room isolation | small | no |
| P4 | Interned identities: an integer handle per body, made when its `SimId` is minted; read-model keys by `Entity` | String clones and sorts in 5+ per-tick systems | medium | no |
| P5 | Read-models built once per drawn frame, by change | repeat builds on slow frames; builds in headless hosts | medium | D2 |
| P7 | Plumbing: one gizmo trail group, Bevy debug gizmos off in shipping, developer tools only in developer builds, Kira channels | ~80 systems | small-medium | D6, D9 |
| P8 | Upstream query-state fixes (LDtk, Yarn, bevy_light, GGRS) | ~2% of phone CPU | small each | D5 (only for the LDtk workaround) |
| P10 | Phone render scale: a reduced main render target, scaled to the surface (`crates/ambition_render/src/quality.rs:190` has no phone path) | fragments, when the GPU is the limit; not the limit today (F11) | medium | D10 |
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

Two items are ready and change no behaviour, but each needs a yes for
another reason:

- **D12 — `Serial` as the default executor of the windowed demos** (F22).
  One line in `install_windowed_foundation`. Mary-O's 208 tests pass with
  `AMBITION_SCHEDULE_EXECUTOR=serial`. It changes how every standalone game
  runs, so it should land with a play test, and the switch is then removed.
- **D13 — withdrawn.** The leafwing clash pass is skipped in this repo with
  no fork (F13). An upstream PR (skip the pass under `PressAll` in
  `process_actions`; lookups without boxing) is an outward action for Jon.

## A second opinion, weighed (GPT, 2026-10-16)

Jon passed on GPT's diagnosis. Its points against this page's evidence:

| GPT's point | Here | Verdict |
| --- | --- | --- |
| Mary-O pays for Ambition's capabilities; install less, not faster | F0, F1, P1 | agrees; MEASURED (-17% CPU with only Mary-O in the hub; 170 unused systems a frame in standalone Mary-O) |
| Read-models rebuilt per tick, so slow frames repeat them | P5, D2, F12 | agrees; REASONED (no limit of ours on catch-up, F12) |
| Phones have no reduced render target | P10, D10 | true (`crates/ambition_render/src/quality.rs:190`); not the limit of today's runs: the main thread is the limit and the render thread is idle 75% of the time (F11) |
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
- 05:30Z an agent starts the `perf_probe` feature (allocations per span,
  churn per source line).
- 05:40Z P1/P2 plans written (an agent's code read); F1/F2/F3 corrected (bosses
  can already be omitted; twelve ledgers; the windowed demos never serialize).
- 05:58Z `symbolize_android_profile.py`: a threads table and `--thread`. F11:
  the main thread is the limit (a first table summed it with the render
  thread, which has the same name; now split by thread id). F12: no catch-up
  limit. GPT's diagnosis weighed.
- 07:41Z `perf_probe` committed (6f14f4eb7): allocations per span, churn and
  value changes per source line. Findings F13-F20 from its runs (desktop,
  standalone Mary-O and Sanic, hub `basement_enemies`). Fixes with no
  behaviour change, verified by the probe or a test: Mary-O's star no longer
  re-folds the repertoire every tick (F15); gravity-zone visuals are no
  longer respawned every frame (F18). Next: the hub run with the change
  counter, to confirm F17.
- 09:13Z F17 cause found with the change counter and fixed (`first_room_art.rs`);
  wielded-item overlays updated in place (pixel-identical capture). Executor
  A/B: standalone Mary-O (F22) and hub (F23); switch committed, defaults
  unchanged. Sanic change census. Read-before-write fixes for the SFX
  authorizer, the narrative and host-intent ledger prunes and the trail gizmo
  bands, verified with the change counter (F20).
- 09:56Z Room isolation walks only when a mask can be stale (F14); HUD,
  feature-visual and hit-flash writes only of a change (F20); seat bindings
  projected only on a change; Mary-O and Sanic hosts without named boss
  encounters (F1: −13 and −11 systems a frame). Before/after table in the
  summary. Remaining large items need a yes (D12) or are P1's C1.
