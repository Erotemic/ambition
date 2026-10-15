# Per-game system audit and composition plan

**Scope:** which systems each game runs each frame, and the structural changes
that make a game run only what it uses. **Owners it feeds:**
[capability and runtime composition](capability-and-runtime-composition.md)
(profiles), [performance and iteration](performance-and-iteration.md) (Android
CPU). **Status:** audit measured 2026-10-14; the plan is a draft for Jon.

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

Each step names the count that shows it worked. The ratchet is
`[census] systems_ran` per game: hub Mary-O should come down to standalone
Mary-O, and standalone Mary-O should come down to what Mary-O uses.

### S1 — Serialize the fixed schedules in every host (small)

Put `FixedFirst` through `FixedLast` in `serialize_frame_schedules`, or measure
first with `AMBITION_MAIN_SCHEDULES_SINGLE_THREADED=1` (it already covers
them) in `scripts/headless_room_frame.sh`. Evidence: finding 5 and the earlier
headless measurement in `serialize_frame_schedules`'s comment. Done when a
standalone demo's frame time with and without the change is recorded.

### S2 — A game's own content runs only in its session (medium)

Each game's plugin puts its systems in one set with one run condition: "this
game's session is active". The condition is evaluated once for the set, so
a game that is not active costs one check, not its systems (finding 4). The
shell already knows the active route. Candidates, by count: `ambition_content`,
`ambition_demo_smash`, `ambition_demo_mary_o`, `ambition_app`'s gameplay
systems, `ambition_demo_sanic`. Done when, in the hub, no game crate runs
systems while another game plays.

### S3 — Each game declares the engine capabilities it uses (large)

Extend `Capability` with the features finding 2 names (boss encounters are
there; add Smash grabs and throws, the Smash moveset, mounts, conversation and
the others the census shows), and make each demo host build
`PlatformerEnginePlugins::for_profile` with its own profile. In the hub, the
same capabilities are gated per session, as in S2. This follows the existing
contract in [capability and runtime composition](capability-and-runtime-composition.md):
supported absence is a tested profile, not an `Option` sprinkled over systems.
Done when standalone Mary-O does not register the systems of a capability it
omits, and its supported-profile tests pass.

### S4 — Gate per feature, at the set, by work (medium, after S2 and S3)

Inside a capability that a game uses, a system with no work still runs. Group
each feature's systems in a set behind one cheap condition (an entity with the
feature's marker exists, or its message queue is not empty). Prefer one set
condition to one condition per system: the hub registers 286 system
conditions and 87 set conditions. Done when the ran/registered share falls well under the
present 75%.

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

S1 first: it is small and finding 4 says it can be large. Then S2, which has
the largest effect on the hub, where Android Mary-O runs. S3 is the structural
change; S4 is the pass after it. Measure frame time on the phone after each
step with `profile_android.sh`.
