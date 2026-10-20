# Per-game system audit and composition plan

**Scope:** which systems each game runs each frame, and the structural changes
that make a game run only what it uses. **Owners it feeds:**
[capability and runtime composition](capability-and-runtime-composition.md)
(profiles), [performance and iteration](performance-and-iteration.md) (Android
CPU). **Status:** the runtime audit was measured on 2026-10-14; S1 landed on
2026-10-17. The source and ECS index were reviewed on 2026-10-20 at
`be412a299768`. S7 is a proposed consolidation campaign, not a measured fix.

## How to measure

Re-run these before you quote a number. Do not copy counts into other pages.

- `AMBITION_PROFILE_CENSUS=1` on any composition prints `[census] systems_ran`:
  the systems that ran in one frame against those registered, by schedule and
  by crate. `AMBITION_SYSTEM_LIST=PATH` writes the full list.
- `scripts/system_census_table.py A.txt B.txt ...` compares lists crate by
  crate. The recipe is in its docstring and in `docs/recipes/profiling.md`.
- `dev/bevy_idle_systems` measures what an idle system costs on each executor.

## Static ECS inventory (2026-10-20)

The generated `.agent/ecs_inventory/project.md` is from commit
`be412a299768` (`.agent/index/generation_stamp.json`). It counts 1,202
registered systems in 92 crates. It also counts 2,114 registration records
and 1,753 system-like functions. These are **different measures**. The
inventory counts source registration sites. It does not count the systems that
one selected game runs in one frame. It does not include Bevy's dependency
systems in the same way as the runtime census. Do not compare its 1,202 with
the runtime census's 1,950 or 2,059 as a before/after speed result.

| Source crate | Static registered systems | Why inspect it |
| --- | ---: | --- |
| `ambition_platformer2d_actor_monolith` | 204 | Large actor, world and feature implementation |
| `ambition_render` | 155 | Character and effect presentation |
| `ambition_platformer2d_runtime` | 137 | Installs and orders many systems from other crates |
| `ambition_app` | 131 | Host, session and developer composition |
| `ambition_content` | 59 | Game-specific behavior |
| `ambition_platformer2d_host` | 41 | Host adapters |
| `ambition_demo_twintrack` | 40 | Game-specific simulation and presentation |
| `ambition_dev_tools` | 38 | Often absent from the phone build |
| `ambition_sim_view` | 38 | Simulation read-model publication |
| `ambition_game_shell` | 31 | Shell and session lifecycle |
| `ambition_platformer2d_rollback_ggrs` | 26 | Rollback and confirmation |
| `ambition_demo_mary_o` | 22 | Game-specific behavior |
| `ambition_touch_input` | 20 | Phone touch controls and UI |

These 13 crates account for 942 of the 1,202 registrations. The remaining
260 are in other crates. The `runtime` crate defines only 48 system-like
functions locally but has 137 registrations. It is often a composition site,
not the owner of each system body. Source size and registration count alone
cannot rank fusion work.

The source index is enough to find candidates and their registered schedule,
set and direct resource accesses. The runtime census is still needed to find
which systems execute in the hub, Mary-O, Smash and Ambition. Keep the
static and runtime reports separate.

## Findings (2026-10-14, debug builds, VM)

All counts are measured, without the census's own systems. The hub is
`capture_scene` (`ambition_app`, every game installed); the standalone is
`capture_mary_o` (`ambition_demo_mary_o_app`, `all_capabilities`).

| run | systems registered | systems ran in one frame |
| --- | --- | --- |
| hub, Ambition | 1950 | 1512 |
| hub, Mary-O | 2059 | 1551 |
| hub, Sanic | 1950 | 1531 |
| hub, Smash | 1950 | 1516 |
| standalone Mary-O | 1275 | 1034 |

1. **In the hub, every game pays for every other game.** Measured, in hub
   runs: while Ambition plays, `ambition_demo_mary_o` runs 29 systems,
   `ambition_demo_smash` 49 and `ambition_demo_sanic` 4. In every game,
   `ambition_content` (Ambition's content) runs 81 and `ambition_app` about 54.
   The engine crates run the same count in every game (`actor_monolith` 232,
   `ambition_render` 138, `ambition_combat` 77).
2. **A standalone game runs engine capabilities it does not use.** Measured,
   standalone Mary-O, systems that ran: boss encounters 34 (Mary-O's castle
   boss is an ordinary enemy body, not an engine boss encounter), conversation
   32, Smash grabs and throws (`ambition_combat::capture`) 14, the Smash
   moveset 13, mounts 7. The `Capability` profiles can omit only cutscenes,
   dialogue, boss encounters, held use and inventory, and the demo hosts take
   all of them.
3. **Almost every registered system runs.** About 75% to 80% in each run.
   Most of our systems do not have a run condition that turns them off when
   their feature has no work. They run and find no entity.
4. **On the multi-threaded executor, an idle system is expensive.** Measured
   with `dev/bevy_idle_systems` (release, 12 cores): about 0.016 us an idle
   system on the single-threaded executor. On the multi-threaded executor,
   1.7 us each in a schedule of 100 systems that conflict, and 12 us each in
   a schedule of 1000. A system skipped by a false run condition costs 0.04 us.
5. **Every main-world schedule is single-threaded (S1, done 2026-10-17).**
   `serialize_main_schedules` sets `First` through `Last`, `StateTransition`,
   `SpawnScene` and `FixedFirst` through `FixedLast`, in every foundation;
   `bevy_ggrs`'s schedules are set by the rollback crate. Before it, the
   `Fixed*` schedules kept Bevy's multi-threaded executor, and standalone
   Mary-O runs its simulation in `FixedUpdate`.
6. **Archetypes are not the lever.** Measured: the counts are flat in play,
   and archetype matching is mostly the per-run check of each system (see
   `docs/recipes/profiling.md`). Fewer systems running reduces it; fewer
   archetypes does not.

7. **The hub is Ambition's app, with the other games added.** Measured with
   `AMBITION_HUB_GAMES=mary_o` (the hub installs only the named games'
   providers; the launcher then lists only them): Mary-O ran 1494 systems a
   frame, against 1551 with every game and 1034 standalone. Sanic, Smash and
   Versus were 57 of the difference. The rest is Ambition's, and the switch
   cannot remove it: the host's base composition (`ambition_app`
   `plugins.rs`, `resources.rs`) installs Ambition's content, presentation and
   item roster, and loads Ambition's world manifest and audio catalogs. With
   those left out, the host panics (`WorldManifest` is required by
   `setup_host_presentation_system` and the LDtk loader). And Ambition's cast
   uses techniques that Smash registers (`smash.counter`,
   `smash.place_spring`, ...): without Smash, 20 of its moves are refused at
   admission.

Not measured yet: the frame time that each change below saves on a phone. The
VM draws with a software renderer, which takes about 70% of its samples.

## Plan

A smaller `systems_ran` is evidence, not the goal. Each step reports these
separately: fixed-step CPU time, frame time (median and tail), systems
registered and run, capabilities installed, the Cargo closure, and the
composition code the change added. A change that saves CPU by adding a
registry, a global resource or a dependency edge is not accepted without a
review. The hub has a frontend that standalone games do not, so its target is
no foreign game's gameplay running, not standalone's total.

### S1 — Serialize the fixed schedules in every host (small) — DONE 2026-10-17

Decision D12 (approved): `serialize_main_schedules` is the one policy of
every foundation. The A/B switch (`AMBITION_SCHEDULE_EXECUTOR`,
`apply_executor_policy`, `scripts/executor_ab.sh`) is deleted with the
decision; F22 keeps its measurements. The record below is history.

**Measured 2026-10-16** ([findings](performance-and-composition-findings.md),
F22 and F23): the shared switch exists (`AMBITION_SCHEDULE_EXECUTOR`,
`apply_executor_policy`, `scripts/executor_ab.sh`). Standalone Mary-O: a
fixed tick took 10.7 ms in parallel and 1.7 ms serial; the hub gains about 3%,
its simulation being serial already. The default for the standalone games is
decision D12 there. Mary-O's (95, room replay among them) and Sanic's (42)
integration tests pass with the switch at `serial`. Context switches and
simulation-state equality across arms were not measured.

Put `FixedFirst` through `FixedLast` in `serialize_frame_schedules`. Measure
first, with care:

- `AMBITION_MAIN_SCHEDULES_SINGLE_THREADED=1` is read only by `ambition_app`.
  It does not reach a standalone demo, whose host does not use that path. The
  switch must be in the shared composition (`serialize_frame_schedules`) so
  one executable runs both arms.
- Bevy 0.19 cannot say which executor a schedule has. The run reports the
  executor that the switch installed, per schedule, and the A/B counts context
  switches, which a serial executor makes fewer of.
- Same optimized executable, warm-up, repeated runs. Fixed-step time and
  frame time apart; ticks per frame; simulation state equal in both arms.
  Headless standalone Mary-O and the hub's Mary-O route, with enemies in the
  room, then a GPU host and the phone.
- The render sub-app and the GGRS schedules are out of scope (the render app
  keeps its own executor; GGRS is serial already).

One owner of the executor policy: `ambition_app`'s switch is removed or calls
the shared function. The report says what the switch requested; context
switches corroborate it and do not prove it.

Done when both arms are recorded. If one composition gains and another loses,
the host chooses the executor.

### S2 — A game's own content runs only in its session (medium)

Inventory first (`AMBITION_SYSTEM_LIST`): each game-owned system that runs
while another game plays, in one of five groups: session gameplay (sleeps),
process services (run always), preparation and activation, retirement, and
presentation (its own view lifetime). Only the first group is gated.

Use the scope that exists: `in_rules_scope` and `in_base_mode`/`in_mode`
(`ambition_platformer2d_runtime::mode_scope`), which Mary-O and Sanic already
use. No new active-game resource. The scope follows the room, not the
frontend route, so an incoming session can prepare and an outgoing one can
retire. One set per game per schedule, since a game's systems are in more than
one schedule.

Done when the hub runs no foreign game's gameplay, and a lifecycle test passes:
Ambition, Mary-O, Smash, Mary-O, with a room transition and a replay; the
second Mary-O behaves as the first. And a two-live-room case: `CurrentRoom::in_scope`
is true when any live room is in scope, so it decides whether a game's systems
run, not which actors they touch. Each system still filters its subjects by
their room, and the test proves that one room's rules do not act on the other
room's population.

### S2b — A neutral hub: Ambition installs only when selected (medium-large)

From finding 7. **Step 1 done (2026-10-14):** `app::hub_games` owns the
selection, and the hub's base composition installs Ambition's content,
presentation, item roster, yarn project, rollback state, LDtk runtime and
world only when `ambition` is selected. Without it, the host builds its asset
catalog with no LDtk world and keeps Ambition's audio catalog, which holds the
frontend's music and menu sounds (data, no systems). Measured, Mary-O in the
hub, systems run in one frame: 1551 with every game, 1381 with
`AMBITION_HUB_GAMES=mary_o`, 1034 standalone. No `ambition_content` system
runs. The rest of the gap is the hub's own: its frontend (`ambition_app` 50,
`bevy_lunex` 35, `bevy_egui` 16), touch controls (29, which a phone needs),
the GGRS host (26), portals (39), physics debris (`avian2d` and
interpolation, 39) and 3D lighting (`bevy_light`, `bevy_pbr`, 33).

Next steps, each its own commit:

- Move the selected-Ambition installs into Ambition's provider, so that the
  hub names no Ambition plugin. The scattered `installs("ambition")` checks
  are the inventory of what to move.
- Install the portal, physics debris and 3D lighting plugins only for a
  selected game that uses them (S3 profiles).
- Move the techniques more than one game uses out of `ambition_demo_smash`.
- A Mary-O-only APK (`AMBITION_HUB_GAMES=mary_o ./build_for_android.sh`),
  measured on the phone against the full hub.

Found on the way: `capture_scene` retargeted every camera to its capture
image, including the offscreen cameras of the rig impostor atlases and the
portal view cones. On some frames that failed render validation and quit the
app; on the others those cameras drew into the capture instead of their
atlases. It now adopts only the cameras that draw to a window.

### S3 — Each game declares the engine capabilities it uses (large)

Inventory first: for each feature finding 2 names, its consumers and a
boundary that makes sense as one capability (grabs and throws together; the
Smash moveset only after the generic actions Mary-O uses are separated out;
mounts only with their lifecycle duties). Not a list of system switches. Then
extend `Capability` and make each demo host build
`PlatformerEnginePlugins::for_profile` with its own profile. In the hub, the
same capabilities are gated per session, as in S2. This follows the existing
contract in [capability and runtime composition](capability-and-runtime-composition.md):
supported absence is a tested profile, not an `Option` sprinkled over systems.
Report not installed and not linked apart. Target three profiles made from the
same capabilities: a minimal platformer, Mary-O, and a Smash fighter. Done when
standalone Mary-O does not register the systems of a capability it omits, and
its supported-profile tests pass, with content that needs an omitted
capability refused at admission.

### S4 — Gate by work, only where it is measured (after S2 and S3)

On a single-threaded schedule an idle system with nothing to match is about as
cheap as one behind a false condition (`dev/bevy_idle_systems`), so a
condition on a cheap system saves nothing. Gate only a group whose cost a
timing shows, with a condition that is cheap and that cannot hide work a
producer makes later in the same tick. A few gates, not one per system.

### S5 — Third-party per-frame costs (small, parallel)

`virtual_joystick` has an upstream PR. The others named in
[performance and iteration](performance-and-iteration.md#p1--android-cpu-mary-o-measured-2026-10-13)
remain. `bevy_kira_audio` runs 34 systems and `bevy_gizmos` 36 in standalone
Mary-O; check whether a game that draws no gizmos needs the gizmo plugin.

### S6 — Per-player and per-room state off global resources (separate)

This is multiplayer correctness, not speed. The list of suspect resources is in
the 2026-10-14 resource census (`AMBITION_RESOURCE_LIST`). Plan it with the
owners of those resources; do not mix it with S1-S4.

### S7 — Consolidate coherent active ECS systems (proposed 2026-10-20)

**Objective:** reduce the cost of invoking active Bevy systems on Android.
Keep narrow Rust helpers and meaningful schedule phases. Do not trade one
large `World`-exclusive system for several declared systems just to lower the
registration count. The system boundary is a scheduling choice; the crate
boundary is an ownership choice. A crate carve must not create new scheduled
units by default.

**Evidence:** The phone profile described in
[performance and composition findings](performance-and-composition-findings.md#f0--a-third-of-the-phones-cpu-is-the-executors-work-for-each-system-outside-any-system)
found substantial schedule-executor work outside system bodies. This predates
S1, scoped capability gates and the recent rendering changes. It justifies a
new controlled test. It is **not** a prediction of the gain from any fusion.
The 2026-10-17 read-model measurement in that same document is also from an
earlier head and must not be used as the new baseline.

#### S7a — Establish the current active baseline (first)

On commit `be412a299768` or a later named head, collect
`AMBITION_PROFILE_CENSUS=1` and `AMBITION_SYSTEM_LIST=PATH` for the hub's
Mary-O 1-1, Mary-O 1-2, Ambition hub, Smash, and standalone Mary-O. Include
Sanic when its capability path changes. Record a separate count for each
executed schedule, and separate **simulation ticks per frame** from
**rendered frames**. Tag each record with commit, app profile, device, build
mode and active room. `scripts/system_census_table.py` compares the lists.

On the Android device, measure frame-time median and tail, fixed-tick cost,
executor and parameter-setup time, and system-body time. Warm each scene;
repeat baseline and candidate runs on the same device and build settings.
Include the first room load and steady play; do not hide a new load hitch
inside a steady-state average. The debug VM can screen an experiment but
cannot decide phone acceptance.

#### S7b — First low-risk fusion: touch prompt UI

Source: `crates/ambition_touch_input/src/bevy_plugin.rs`.
`update_button_verb_from_prompt` and
`sync_touch_button_visibility_from_prompt` both run in `Update` and read
`ControlPrompt`. Try one `reconcile_touch_action_prompt` system with two
ordinary helpers. Retain the current distinction between button readiness,
label text, action availability and input-context ownership. Repoint the
`render_touch_button_text` ordering to the new producer. Check that unchanged
values still do not mark `Text`, `Visibility` or `ButtonVerb` changed.

**Do not fuse across these boundaries:** `read_joystick_messages` and
`update_buttons_from_interactions` must still run in `PreUpdate` before
Leafwing `Unify`; `offset_joystick_art_within_footprint` and
`drive_joystick_knob_from_axis` remain on their opposite sides of the
`JoystickSystems::UpdateUI` barrier in `PostUpdate`. Preserve the
`TouchPresentationSet` placement order and the `InputSet::Route` read.

**Test:** touch button visibility and wording in Gameplay, Menu, Dialogue
and Empty contexts; Sanic's missing Attack button; preset/device changes;
real touch press timing, held input and keyboard/gamepad use. Compare a quiet
frame's changed-component counts. Measure the Android delta. Reject the
fusion if it adds an ordering exception or does not show repeatable savings.

#### S7c — Simulation-view publication (measured second candidate)

Source: `crates/ambition_sim_view/src/lib.rs`, `facts.rs`, `pose_view.rs`
and `view_index.rs`. The `FeatureViewSyncSchedulePlugin` publishes related
indices, pose and presentation facts from the simulation tail. Inspect
`rebuild_actor_render_index`, `rebuild_actor_anim_index` and
`rebuild_body_pose_views` for shared body reads. Fuse only a subset that
uses one observation cut and has a safe shared input traversal. Keep the
`FeatureViewSync` phase, the nameplate-after-feature order, the animation
clock advance before its read, and passage views after body pose and
animation. Preserve the `ControlPromptRebuilt` and readiness ordering.

**Do not** move these publishers wholesale into rendered-frame `Update`.
Rollback, a multi-tick rendered frame, a headless consumer or a same-tick
reader may need each simulation snapshot. Do not hide an expensive body
query inside one very large exclusive system. Compare input query work,
active invocations and snapshot contents under rollback and two live rooms.
If the profiles show small CPU cost, defer this packet.

#### S7d — Rendering reconciliation (design and profile before fusion)

Source: `crates/ambition_render/src/rendering/actors/` and related feature
rendering. Identify independent passes that visit the same drawable and
publish one presentation result. Prefer a keyed, change-aware reconciliation
that reuses existing presentation entities over repeated spawn/despawn.
Do not combine portal partitioning, animation selection, transform writes,
asset readiness and teardown into an opaque catch-all. Retain root versus
rigged-part ownership, deferred visibility and render-sub-app boundaries.

Measure entity churn, archetype changes, component writes and renderer CPU
alongside active system count. A smaller system count with more structural
writes fails this packet.

#### S7e — Low-priority generic fan-out

`crates/ambition_conversation/src/ledger.rs` installs release, prune and
forget systems for each `NarrativeInputPlugin<M>`. Check whether a shared
ledger phase can do the work with fewer active invocations. Keep each
message type's retention and activation boundary. Do not add type erasure,
a central registry or a `World` scan to save a few cheap calls. Try this only
if the new profile shows that this group costs enough to matter.

#### Acceptance and rejection gates

For each candidate, record before/after: registered and **executed** system
counts by schedule; the exact removed registration(s); system input accesses;
order and deferred-command edges; change-detection writes; Android median,
tail and fixed-tick cost; headless and rollback results. Compare the normal
and selected-capability paths. One fewer registration is not proof of a
faster frame, and a render-frame win cannot hide a simulation-tick regression.

- Preserve the same action, geometry, visibility and read-model outputs for
  the same input frames, with the same flush and rollback behavior.
- Keep tests for initial install, removal, re-entry, two live rooms, and
  multiple simulation ticks per rendered frame where the packet needs them.
- Retain ordinary Rust helper functions for independent policy and tests.
  Give the outer system only the access it needs; do not use a general
  `&mut World` escape hatch.
- Report repeated Android measurements, including variance. Revert a fusion
  that adds coupling, shifts timing or has no repeatable benefit.
- Do not update a performance baseline merely to accept a regression.

S7 runs alongside the
[actor decomposition plan](actor-monolith-decomposition.md). Its first
implementation experiment is the touch prompt pair. A monolith crate split
can move an installed system without adding another system to the schedule;
a later fusion can reduce invocations without merging the crate owners.

## Order

S1 is complete. Continue S2 and S3 where a selected game still runs an
unneeded capability. For S7, collect the new runtime/phone baseline first,
then test the touch prompt fusion. Review its measurements before a wider
read-model or rendering pass. S4 stays a measured gate-only activity after
S2 and S3. The [execution queue](../queue.md) selects when an experiment
runs; this page defines its tests and stop conditions. Use
`profile_android.sh` after each accepted change.
