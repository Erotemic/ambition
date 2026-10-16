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
| Which system allocates, and which code makes archetype churn? | the `perf_probe` feature (in progress, see the run log) |

## Measured baseline

**Phone (Pixel 5, hub with every game, Mary-O 1-1, commit 97ff2541e,
2026-10-16; `profiles/android-all-maryo-1-1`).** MEASURED.

- Frame time, 5 s windows: p50 24-46 ms, p95 37-131 ms.
- Threads: main 67%, the Bevy compute pool 22%.
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
where a plugin group cannot disable them. Boss encounters cannot be omitted:
`demo_fixture::simulation_world` and `PlatformerAssetsPlugin` need
`Res<BossCatalog>`, which only `BossEncounterSimulationPlugin` makes.

### F2 — Ten narrative ledgers run in every game

MEASURED. `NarrativeInputPlugin<M>` (`ambition_conversation/src/ledger.rs:242`)
adds three systems per payload; ten payloads (two in `ambition_conversation`,
seven in `actor_monolith/src/features/mod.rs:1491-1497`, one in
`ambition_content`) make 30 systems. The eight outside `ConversationPlugin` are
not removed by `Capability::Dialogue`. Every writer is in `ambition_conversation`
or `ambition_content`.

### F3 — The fixed schedules run on the multi-threaded executor

MEASURED. `serialize_frame_schedules`
(`ambition_platformer2d_runtime/src/lib.rs:741`) covers `First`..`Last`, not
`Fixed*`. Standalone games simulate in `FixedUpdate`. `dev/bevy_idle_systems`:
an idle system costs about 0.016 us on the single-threaded executor and 1.7-12 us
on the multi-threaded one. The executor policy has a second owner
(`game/ambition_app/src/app/plugins.rs:355`, an env switch the demos never reach).

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

### Not a lever (keep out of this run)

Session-scoped App globals (C03: 30 counted plus 19 uncounted instances) are
reset only on session edges: correctness work (S6), no per-frame cost. The
`ambition_platformer2d` facade (209 `pub use`) costs compile time only.
Rollback costs about 0% offline (`check_distance: 0`).

## Proposals, ranked by structural leverage

| # | Proposal | Removes | Size | Behaviour change |
| --- | --- | --- | --- | --- |
| P1 | Per-game engine profiles: `BossCatalog` always present (empty); ledgers under Dialogue; grabs, mounts, match stocks as group members with capabilities; extension lowering installed by the capability that owns the phase; demo hosts call `for_profile` | about 115 systems a tick in Mary-O and Sanic | large | no, except decisions D3, D4 |
| P2 | Serialize the `Fixed*` schedules in the one executor policy (S1), A/B first | the multi-threaded executor cost of ~500 idle fixed systems | small | no |
| P3 | Hub: gate each game's session gameplay by its scope (S2); install Ambition, physics debris and 3D lighting only for the games that use them (S2b); move shared techniques out of `ambition_demo_smash` | 82+ foreign systems while one game plays, 72 hub-only systems | medium | no |
| P4 | Interned identities: an integer handle per body, made when its `SimId` is minted; read-model keys by `Entity` | String clones and sorts in 5+ per-tick systems | medium | no |
| P5 | Read-models built once per drawn frame, by change | repeat builds on slow frames; builds in headless hosts | medium | D2 |
| P6 | Change-driven systems: the two whole-world queries, input bindings | full scans per tick, ~30 allocations a frame | small | no |
| P7 | Plumbing: one gizmo trail group, Bevy debug gizmos off in shipping, developer tools only in developer builds, Kira channels | ~80 systems | small-medium | D6, D9 |
| P8 | Upstream query-state fixes (LDtk, Yarn, bevy_light, GGRS) | ~2% of phone CPU | small each | D5 (only for the LDtk workaround) |
| P9 | Content to data: held items, the item enum, Ambition-only item systems | Ambition work in other games; engine edits for content | large | D7 |

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
- **D3 — Standalone Mary-O without Dialogue.** Her rows with
  `fallback_dialogue`: refuse at admission, or ignore?
- **D4 — Smash moveset out of standalone Mary-O and Sanic.** Then a standalone
  game cannot grant attack.
- **D5 — LDtk respawn one frame late** (a set condition) instead of waiting for
  the upstream fix.
- **D6 — Kira dynamic channels.** Changes how music layers crossfade.
- **D7 — Unknown boss ids refused at admission** instead of the invented
  fallback boss.
- **D8 — `SimId` minted by a spawn observer.** Moves where in the tick identity
  is made (determinism).
- **D9 — The per-room gizmo trail with two or more live rooms:** rebuild on
  retained gizmos, or drop.

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
