# Performance and iteration

**State:** OPEN engine-product program.

## Goal

Treat performance and iteration speed as part of engine quality, not as an
occasional optimization campaign.

A Unity/Godot competitor must be pleasant to build with as well as capable at
runtime. Ambition is the primary measurement customer; acceptance games and
minimal external consumers reveal different dependency/capability footprints.

## Measurement families

### Compile and dependency topology

Track:

- clean and incremental build cost of major crates/apps;
- change amplification after representative edits;
- high-fan-in foundations;
- capability footprint for minimal consumers;
- effect of actor-monolith/shared-tangle/runtime extractions.

Use measurements to choose ownership boundaries. Do not carve solely to make a
line count smaller.

### Runtime simulation

Track representative costs for:

- actor/brain phases;
- collision and dynamic world geometry;
- rollback save/load/resimulation;
- portals and unusual rendering features;
- multiple resident rooms;
- multiple local views.

### Rendering and quality profiles

Quality profiles should cover all expensive presentation features coherently,
including texture/sprite resolution, portals, lighting/VFX and multi-view
rendering. Desktop and Android defaults may differ while the authored content
identity remains the same.

### Asset residency

Continue [`../sprite-residency-and-live-quality.md`](../sprite-residency-and-live-quality.md)
and broaden the model where other asset classes need similar residency/quality
policy.

### Authoring iteration

Measure the edit -> validate -> preview/hot-reload loop for LDtk and other content
pipelines. A feature that takes minutes of opaque regeneration to inspect is an
engine ergonomics problem even if runtime is fast.

### Headless throughput

Headless simulation, replay, fighter evaluation and automated acceptance tests
should remain cheap enough to be used as development tools.

## Rules

- record the workload and machine/context with a number;
- prefer representative bounded benchmarks over permanent giant sweeps;
- optimize a measured dominant cost, not an old campaign's baseline;
- distinguish compile/link time, test execution time, asset generation time and
  runtime frame cost;
- preserve correctness/authoring clarity before micro-optimizing;
- avoid NPM-based tooling when a Python/native tool is practical for repository
  workflows.

## Measured baseline — headless sandbox, 2026-08-29

Machine: `aivm-2404`, i7-7700HQ, 6 logical CPUs, no GPU. Workload:
`scripts/profile_desktop.sh --headless` (sandbox, 1800 ticks, `profiling`
profile). Bundle: `desktop-timeline-run-20260829T000517Z`.

The scene: **64 entities, 85 archetypes, 2 bodies, 1 player** — and **876
registered systems across 11 schedules**. Over 1800 frames that is **1.44M
system executions, 1.55M run-condition evaluations, and 317K command flushes**:
~1,835 instrumented zones per frame for a scene with two bodies in it.

⚠ Absolute per-frame microseconds in that bundle are inflated: Tracy's own
threads took **55% of sampled cycles**, more than the game's 40%. Counts and
ratios are sound; treat times as an upper bound and read the observer-effect
section of the bundle.

**The unprofiled frame is 1.52ms, not the 13.6ms the Tracy bundle reported** —
Tracy inflated it ~9x. Startup is likewise 482ms of app construction unprofiled
against 1.675s under Tracy (~3.5x), because every system registration emits a
source location to the profiler. Always size an optimization against a
`--no-tracy` run.

Where that 1.52ms goes, from `[census] phases` (headless, 657-frame window):

```text
   0.990 ms   65%  Update
   0.150 ms   10%  StateTransition
   0.080 ms    5%  PostUpdate
   0.064 ms    4%  Last
   0.052 ms    3%  SpawnScene
   0.051 ms    3%  PreUpdate
   0.050 ms    3%  outside
   0.045 ms    3%  First
   0.042 ms    3%  RunFixedMainLoop
```

`SpawnScene` costing more than `First` is one surprise, and it is only visible
because the phase list is read from `MainScheduleOrder` rather than hardcoded.

⭐ **ONE SCHEDULE HOLDS 93% OF THE ENGINE.** `[census] schedules` reports the
population behind each of those times:

```text
Update=822  PostUpdate=15  PreUpdate=9  Startup=8  StateTransition=8
First=4  FixedMain=1  FixedPostUpdate=1  RunFixedMainLoop=1
```

822 of 886 systems are in `Update`. That single fact explains the frame: the
861 run-condition evaluations per frame are 822 systems each carrying their own
condition, and directions 1 and 2 below are both really "stop making `Update`
carry everything". Any per-system overhead the engine pays, it pays 822 times.

**On `StateTransition`, correcting an earlier reading of this data:** it holds
**8 systems** and still costs ~0.15ms, which is ~20x the per-system cost of
`Update`'s 822. So it is not our code being slow — it is Bevy's per-state
machinery (transition steps plus `try_run_schedule` for `OnEnter`/`OnExit`/
`OnTransition` that are all empty here) being expensive per system. Worth a
look before shipping, but it is NOT the place to start: `Update`'s 822 systems
are, because that is where both the frame and the 1.9s of plugin build live.

## Directions with strong efficiency leverage

Ordered by (measured evidence × breadth of benefit). None is an architecture
shift; each is a change in where an existing mechanism is attached.

### 1. Run conditions belong on system SETS, not on each system

**Measured:** 861 run-condition evaluations per frame.
`ambition_platformer2d_shared_tangle::schedule::gameplay_allowed` alone was
evaluated **87 times per frame** (156,774 times over the run);
`mode_scope::in_mode` 30 times per frame. Bevy evaluates a run condition once
per system that carries it, so N systems sharing one condition pay N
evaluations.

`configure_sets(..., MySet.run_if(gameplay_allowed))` evaluates it once. The
systems are already grouped by phase; the condition is what is ungrouped.

### 2. A shipped game should not schedule the experiences it does not contain

**Measured:** the sandbox run — which never entered Sanic, Smash, or Mary-O —
still evaluated `ambition_demo_sanic::ball_dash::tick_rolling`,
`ambition_demo_smash::offer_to_exit_the_match`, and
`ambition_demo_mary_o::powerups::refuse_a_weaker_form_pickup` **1802 times
each**, once per frame.

That is correct for `shell_host`, which is a launcher that can start any of
them, and it is the right default for the multi-game host. It is the wrong
default for a game built ON this engine that ships one experience: the systems
are registered at plugin-build time and their conditions run forever after.

Two seams, neither structural:

- experience-scoped systems go in a per-experience `SystemSet` gated by one
  condition (this is direction 1 applied to the largest population);
- a single-experience app composes only its own provider plugin, which
  `ambition_demo_mary_o_app` already demonstrates. The engine should make that
  the documented default for a shipped title rather than a demo-only path.

### 3. `World::query*` inside a per-frame system rebuilds a `QueryState`

`World::query_filtered` constructs a fresh `QueryState` and re-matches every
archetype in the world. In an exclusive system that runs each frame, that is a
per-frame archetype scan. `Local<Option<QueryState<..>>>` keeps it across
frames and `QueryState::iter` still updates archetypes, so nothing is missed.

Fixed in `world::gated_lock_walls::sync_authored_gated_lock_walls`
(33.1us/frame before, in the top ten systems, for a room with no gated walls).
There are ~73 `world.query*` sites outside tests; most are setup or spawn paths
where the cost is paid once and the pattern is fine. **A guard that flags a
`world.query*` reached from a system registered in a per-frame schedule would
keep this from recurring** — the existing `scripts/check_*.py` family is the
right shape for it.

### 4. Per-frame rebuilds that could be change-detection driven

`rebuild_control_prompt` (31.8us/frame), `rebuild_feature_view_index`,
`rebuild_attack_vfx_views`, `sync_ecs_actors_with_save`. Each recomputes a
derived view every frame. Their inputs change on events (a binding swap, a room
transition, a spawn), not continuously. These are individually small and
collectively the shape of the frame.

### 5. Dev instrumentation is a top-five per-frame cost in an OPTIMIZED build

`dev::trace::record_actor_oob_frame_system` (40.5us/frame) and
`dev::trace::systems::record_frame_system` (37.9us/frame) together outweigh
`tick_actor_brains`, and `dev_tools::hot_reload::poll_world_source_changes`
adds 34.5us/frame. The trace ring buffer is deliberately always-on and its
forensic value is real, so this is a budget question rather than a bug: decide
what a shipped `--ship` build carries, and make the recorder's per-frame cost
scale with bodies rather than with frames where it can.

`poll_world_source_changes` also does a **blocking `fs::metadata` on the main
thread** (max 3.9ms in this run, on virtiofs). On a network mount, Android
storage, or a slow SD card that is a frame hitch. Worth moving off-thread.

### 6. Startup is dominated by App construction, which nothing measured

**Measured:** 2.6s from process exec to first frame; the `[startup]` phase
logger reported **120.4ms** of it, because `StartupProfiler` was created partway
through plugin build and anchored its deltas there. Tracy attributed **1.9s to
`plugin build`**, of which `AmbitionGameSimulationPlugin` was 1.675s — 876
system registrations and their schedule graphs.

The anchor is fixed (`profiling::note_process_start`). The COST is not: plugin
build scales with registered systems, so directions 1 and 2 shorten startup and
the frame together. This is the number a player feels on a phone.

## Campaign 2026-08-29 — runtime efficiency, 24h

Jon armed a 24-hour goal: *"make this game run faster, more efficiently, and
elegantly"*, on evidence, with BOTH deliverables required — measurements
preserved as history, and landed work with before/after numbers. His constraint
on method: be cheap with the machine (6-CPU VM, no GPU, shared target dir — one
cargo invocation at a time, no `--workspace --tests`). Build times are fair game
opportunistically but do not outrank the frame.

### Open work, in leverage order

Each row is a lever already MEASURED in the baseline above. Do not re-derive the
baseline; extend it.

- ▢ **D-PERF-1 — hoist `gameplay_allowed` off 89 systems onto a set.** The
  baseline measured 87 evaluations per frame of this one condition. ⭐ THE
  MECHANISM ALREADY EXISTS AND IS ALREADY USED: `configure_platformer2d_simulation_phases`
  puts `simulation_authorized` on `GameplaySimulationRoot` with ONE
  `configure_sets` call. `gameplay_allowed` is a second, different condition
  (`GameMode` vs the session gate) that never got the same treatment.
  **Measured shape of the work:** 89 `.run_if(gameplay_allowed)` sites in 11
  files; 76 of them in four — `items/pickup/mod.rs` (29),
  `runtime/combat_schedule.rs` (26), `content/portal/plugin.rs` (13),
  `runtime/player_schedule.rs` (8). All four attach to `app.sim_schedule()`, so
  one set configured once covers the bulk.
  ⛔ NOT EVERY SYSTEM IN THOSE TUPLES WANTS THE GATE — `pickup/mod.rs` already
  carries a comment marking the shrine-resume system as deliberately ungated.
  The exceptions are the whole risk; find them before moving anything.
  ⭐ Bevy 0.18.1 semantics, read from `bevy_ecs/src/schedule/config.rs` rather
  than assumed: `.run_if` on a TUPLE pushes to `collective_conditions` and is
  *"evaluated at most once (per schedule run)"*; `.distributive_run_if` copies it
  onto each system. So even a tuple-level hoist helps, and a named set shared
  across files collapses the whole population to one evaluation.

- ▢ **D-PERF-2 — a shipped title should not schedule experiences it does not
  contain.** Baseline: a sandbox run that never entered Sanic, Smash or Mary-O
  still ticked `tick_rolling`, `offer_to_exit_the_match` and
  `refuse_a_weaker_form_pickup` 1802 times each. ⛔ The broad launcher host MUST
  keep working — this is per-experience `SystemSet` gating plus a documented
  single-experience composition, not deletion. `ambition_demo_mary_o_app`
  already demonstrates the composition half.

- ▢ **D-PERF-3 — per-frame rebuilds whose inputs change on events.**
  `rebuild_control_prompt` (31.8us/frame), `rebuild_feature_view_index`,
  `rebuild_attack_vfx_views`, `sync_ecs_actors_with_save`. Determine the
  COMPLETE authoritative input set before converting any of them; a missed
  invalidation path is a correctness bug, so each needs a regression test that
  covers every path.

- ▢ **D-PERF-4 — dev instrumentation is a top-five per-frame cost in an
  OPTIMIZED build.** `record_actor_oob_frame_system` (40.5us) +
  `record_frame_system` (37.9us) together outweigh `tick_actor_brains`;
  `poll_world_source_changes` adds 34.5us AND does a blocking `fs::metadata` on
  the main thread (3.9ms max on virtiofs). A budget question, not a bug: decide
  what `--ship` carries, and scale the recorder with bodies rather than frames.

- ▢ **D-PERF-5 — the runtime measurement series.** `dev/ambition_dev_measurements/`
  had only compile-cost series; there was no runtime history, so no commit could
  be compared against any other. Normalized JSONL summaries that survive the raw
  trace being deleted, with a comparability key strict enough that a lavapipe run
  can never group with a hardware-GPU run, nor a Tracy run with an unprofiled one.

### Rules this campaign is working under

- ⛔ A LAVAPIPE RUN IS NOT A GPU RUN, and a Tracy frame time is not an
  unprofiled one — Tracy inflates this app's frame ~9x. Size every optimization
  against a `--no-tracy` run; use traced runs for ATTRIBUTION only.
- ⛔ Preserve expressiveness. Portals, multiple local views, rich VFX, systemic
  simulation, extensible characters, rollback and dev tooling all stay. The goal
  is that a capability pays PROPORTIONALLY: absent costs nothing, dormant costs
  little, active costs something attributable. Deleting a capability is not an
  optimization.
- ⛔ No cargo-cult perf work: do not replace Bevy scheduling, write a custom
  sprite renderer, merge systems to shrink a count, add unsafe, or lower visual
  quality to fix an architectural inefficiency.
- ⭐ A REJECTED HYPOTHESIS IS A DELIVERABLE. Record it here so the next session
  does not re-run the dead end.

### Investigations

| # | Hypothesis | Result | Conclusion |
|---|---|---|---|
| I1 | `gameplay_allowed`'s 87 evaluations/frame need a new set-gating mechanism built | ⛔ REJECTED — the mechanism already exists and is already in use: `configure_platformer2d_simulation_phases` puts `simulation_authorized` on `GameplaySimulationRoot` in ONE `configure_sets` call | The work is not "build set gating", it is "apply the existing pattern to the second condition". And Bevy 0.18.1's `.run_if` on a tuple is ALREADY collective (`collective_conditions`, evaluated at most once per schedule run) — only `.distributive_run_if` copies per system. Read from `bevy_ecs/src/schedule/config.rs`, not assumed. |
| I2 | The `smash_it` link failure after the knockout fix was caused by that fix | ⛔ REJECTED — clean tree, `HEAD` lacks the symbol, `grep` finds no reference anywhere in the crate | A draft test compiled and then `git checkout --`'d hours earlier left an incremental `.rcgu.o` referencing the dead symbol. A build-cache lie, not a source defect. `CARGO_INCREMENTAL=0` rebuilds past it. ⛔ do not delete under `target/`. |

## Near-term opportunities

- continue actor-monolith decomposition where dependency isolation improves
  incremental builds;
- measure the dynamic-world collision overlay while moving-platform architecture
  is touched;
- establish multi-view rendering baselines before split-screen grows expensive;
- keep Android quality/residency profiles within device budgets;
- retain lean mastered soundtrack/sprite authoring diagnostics rather than
  generating maximal preview artifacts by default.

## Acceptance

The repository should be able to answer, with current measurements, what makes a
common edit slow, what capabilities a minimal game actually pulls in, what
runtime features dominate a representative frame, and which quality/residency
profile is appropriate for a target platform.
