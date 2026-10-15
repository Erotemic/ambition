# Per-game system audit and composition plan

**Scope:** which systems each game runs each frame, and the structural changes
that make a game run only what it uses. **Owners it feeds:**
[capability and runtime composition](capability-and-runtime-composition.md)
(profiles), [performance and iteration](performance-and-iteration.md) (Android
CPU). **Status:** audit measured 2026-10-14; plan reviewed 2026-10-14 (S1
and the S2 inventory approved, then a review).

## How to measure

Re-run these before you quote a number. Do not copy counts into other pages.

- `AMBITION_PROFILE_CENSUS=1` on any composition prints `[census] systems_ran`:
  the systems that ran in one frame against those registered, by schedule and
  by crate. `AMBITION_SYSTEM_LIST=PATH` writes the full list.
- `scripts/system_census_table.py A.txt B.txt ...` compares lists crate by
  crate. The recipe is in its docstring and in `docs/recipes/profiling.md`.
- `dev/bevy_idle_systems` measures what an idle system costs on each executor.

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
5. **The main-world frame schedules are single-threaded; the fixed schedules
   are not.** Reasoned from code: `serialize_frame_schedules` sets `First`
   through `Last`, and `bevy_ggrs`'s schedules are set by the rollback crate.
   The `Fixed*` schedules keep Bevy's default executor. Standalone Mary-O runs
   its simulation in `FixedUpdate` (494 of 597 systems ran), and a `perf`
   sample shows `multi_threaded::tick_executor` in that run. The hub's
   simulation is in `GgrsSchedule`, which is single-threaded.
6. **Archetypes are not the lever.** Measured: the counts are flat in play,
   and archetype matching is mostly the per-run check of each system (see
   `docs/recipes/profiling.md`). Fewer systems running reduces it; fewer
   archetypes does not.

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

### S1 — Serialize the fixed schedules in every host (small)

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

## Order

S1 and the S2 inventory first, then a review before any structural change.
S2 has the largest effect on the hub, where Android Mary-O runs. S3 is the
structural milestone, and the question it asks of the monolith is "which
capabilities put work and dependencies on games that did not ask for them".
S4 is the pass after it. Measure frame time on the phone after each step with
`profile_android.sh`.
