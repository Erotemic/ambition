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

⚠⚠ **THE HOSTNAME IS NOT THE MACHINE, and this row is why.** As written below
this baseline was taken on `aivm-2404`, i7-7700HQ, 6 logical CPUs. A session on
2026-08-29 introspected a box ALSO called `aivm-2404` and found an **i9-11900K,
12 logical CPUs, 64 GB** (`machine_id ec9af5ee…`, kernel 6.8.0-110-generic,
Ubuntu 24.04.4, kvm). ⛔ DO NOT ASSUME EITHER LINE IS THE ERROR — these agents
run on more than one host, the name is reused, and neither run recorded a
`machine_id` to tell them apart. The row below is left exactly as it was
recorded.

⇒ ⛔ **treat these absolute milliseconds as belonging to an UNIDENTIFIED host**
and do not compare a new timing against them. Counts, ratios and populations are
hardware-independent and remain usable. This is precisely the failure the
machine-readable series exists to prevent, and every row it writes carries a
`machine_id` for exactly this reason.

Machine, as recorded at the time: `aivm-2404`, i7-7700HQ, 6 logical CPUs, no
GPU. Workload:
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

### That baseline is now a series, not a paragraph

The bundle above is gone; these numbers survived only as prose. They are now the
first two rows of `dev/ambition_dev_measurements/runtime_frame_cost.jsonl` —
marked `backfilled: true`, because a transcription is not a measurement.

- record a run: `scripts/lib/profile_bundle_to_history.py <bundle-dir>`
  (it refuses, and writes nothing, for a bundle whose game never started);
- read it back: `scripts/perf_history.py list | compare A B | latest --against X
  | scenario sandbox | report`.

⛔ Each row carries a `comparable_key` over scenario, machine, renderer and
instruments, and the tool REFUSES to subtract rows whose keys differ, naming the
field. The Tracy and `--no-tracy` baselines above are two such rows: same commit,
same scenario, 9x apart, and permanently un-comparable to each other.

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

- ◐ **D-PERF-2 — a shipped title should not schedule experiences it does not
  contain.** ⛔⛔ **ITS STATED EVIDENCE DID NOT SURVIVE RE-MEASUREMENT, 2026-08-29.**
  The row said a sandbox run "still ticked `tick_rolling`,
  `offer_to_exit_the_match` and `refuse_a_weaker_form_pickup` 1802 times each".
  The demos are in fact gated, and gated the RIGHT way:
  `SanicRulesPlugin::hosted()` puts `run_if(in_mode(SANIC_MODE))` on whole
  TUPLES (`rules`, `milestone_sfx`, `badniks`, `ring_loss`), and a tuple-level
  `run_if` in Bevy 0.18 is COLLECTIVE — one anonymous set, evaluated at most once
  per schedule run. Sanic's 28 systems cost ~4 evaluations, not 28, and none of
  them execute while the mode is inactive. The whole app now carries 61
  per-system and 29 set conditions in total.
  ⭐ **WHAT IS REAL, measured with `[census] owners`: 154 of 780 registered
  systems — 19.7% — belong to four experiences the sandbox never enters**
  (`mary_o=44`, `twintrack=44`, `smash=38`, `sanic=28`). ⇒ the cost is
  REGISTRATION — plugin build time, graph size, memory — NOT per-frame
  execution. Still worth doing, as a startup and composition win; it is not the
  frame emergency the old row implied.
  ⛔ The broad launcher host MUST keep working — per-experience `SystemSet`
  gating plus a documented single-experience composition, not deletion.
  `ambition_demo_mary_o_app` already demonstrates the composition half.
  ⚠ To settle it properly someone needs an EXECUTED-systems-per-frame
  instrument; the census counts registrations and conditions, not executions.

- ◐ **D-PERF-3 — per-frame rebuilds whose inputs change on events.**
  ⚠ **PARTLY STALE, re-checked 2026-08-29.** `rebuild_control_prompt` (the row's
  headline, 31.8us/frame) has been fully change-driven since `2fe4ba42a`
  (2026-07-23) — a `Local` cache key over `Ref<>` authorities plus resource
  presence bits, careful enough to invalidate on a rebind AND on picking up a
  different pad. Its residual cost is COMPUTING THE KEY, not rebuilding, which
  is a different fix. `rebuild_feature_view_index` looked like it allocated a
  `String` per feature per frame; `insert_if_absent` already does a `get_mut`
  first, so only a genuinely new id allocates.
  ⇒ **what is left and genuinely uncached:** `rebuild_feature_view_index` still
  makes a linear pass over 7+ query families every frame,
  `rebuild_attack_vfx_views` and `sync_ecs_actors_with_save` have NO change
  detection at all (zero `is_changed`/`Ref`/`Local` in either file).
  ⛔ Determine the COMPLETE authoritative input set before converting any of
  them; a missed invalidation path is a correctness bug, so each owes a
  regression test covering every path.

- ◐ **D-PERF-4 — dev instrumentation is a top-five per-frame cost in an
  OPTIMIZED build.** ⚠ **PARTLY STALE ALREADY, re-checked 2026-08-29** — this is
  why the campaign rule is grep before you build. `poll_world_source_changes` was
  fixed in `6e6a5ce12` (2026-08-29 02:10Z): its debounce countdown ticked through
  a `ResMut`, which marks the resource changed on DEREFERENCE, so the watcher
  announced a change every frame of every run and cost every reader its change
  detection for a false claim. The countdown is a `Local` now and the resource is
  touched mutably only when the watch actually moves.
  ⇒ **what is left of this row:** (a) the blocking `fs::metadata` on the main
  thread, measured at 3.9ms on virtiofs and now debounced to ~3Hz — invisible on
  a local disk, a hitch on a network mount, Android storage or a slow card, and
  its own commit deliberately deferred moving it off-thread; (b)
  `record_actor_oob_frame_system` (40.5us) and `record_frame_system` (37.9us),
  which together outweigh `tick_actor_brains`. Both recorders are per-frame and
  the oob one takes a `CollisionWorld` plus a body query — the budget question is
  what `--ship` carries, and whether the cost can scale with BODIES rather than
  with frames.

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

### The instrument this campaign added, and why it is not a profiler

`[census] conditions` (in `ambition_dev_tools::runtime_census`) reports, per
run, `system_conditions`, `set_conditions`, `sets_with_conditions`, and a ranked
breakdown naming each condition attached four or more times.

⭐ IT IS STRUCTURAL, NOT SAMPLED. The schedule graph already knows every
attachment; `Schedule::graph()` exposes `systems.get_conditions()` and
`system_sets.get_conditions()` in Bevy 0.18. So the number costs no profiler, has
no observer effect, and is DETERMINISTIC — which is what makes it a usable
regression gate, unlike a wall-clock millisecond on a shared VM.

⛔ IT COUNTS ATTACHMENTS, NOT EVALUATIONS, and the gap is the entire point.
Bevy evaluates a system's conditions once per system per run, and a SET's
conditions once per run no matter how many systems the set holds. Hoisting one
shared condition off N systems onto a set moves N out of `system_conditions` and
1 into `set_conditions`. That is precisely the shape of D-PERF-1's improvement,
and it is a change no timing measurement on this machine is quiet enough to
resolve.

⚠ The earlier "861 run-condition evaluations per frame" came from Tracy zone
counts, on a run whose frame time was inflated ~9x. The structural count
supersedes it as the metric to move: same fact, no distortion, and it does not
need the profiler installed to be read.

### The host THIS campaign measured on, introspected 2026-08-29

⛔ Recorded rather than assumed, and not inherited from the row above — the two
may or may not be the same box.

```text
hostname       aivm-2404          machine_id  ec9af5ee73e34e07a46bddf870f96f2e
cpu            11th Gen Intel Core i9-11900K @ 3.50GHz    logical_cpus  12
memory         64 GB              virt        kvm
kernel         6.8.0-110-generic  os          Ubuntu 24.04.4 LTS
rustc          1.95.0 (59807616e 2026-04-14)
graphics       NO /dev/dri, no vulkaninfo, no glxinfo, DISPLAY unset, session=tty
               ⇒ headless only. Any GPU number from this host would be software
               rendering, and it is not comparable to a desktop GPU run.
target dir     bindmount BOUND to ext4 (/dev/vda1), 158G — not virtiofs
```

⚠ `boot_id` changes per boot and is deliberately not part of any comparability
key; `machine_id` is what identifies the host.

### D-PERF-1 measured, before

This host, debug profile, headless `sandbox`, 120 ticks. ⭐ THE PROFILE DOES
NOT MATTER FOR THIS NUMBER — it is a property of the schedule graph, not of
optimization, which is why the before/after cost no `profiling` rebuild.

```text
[census] conditions system_conditions=139 set_conditions=33 sets_with_conditions=30
         gameplay_allowed=78 {{closure}}=22 resource_exists=10 spacetime_is_active=8 Assets=7
[census] schedules  schedules=20 systems=887 Update=822 PostUpdate=15 PreUpdate=9 ...
```

⭐⭐ **`gameplay_allowed` IS 78 OF 139 — 56% of every per-system run-condition
attachment in the app is one question about `GameMode`, asked 78 times per run
for the same answer.** Nothing else comes close; the next entry is 22, and that
is 22 DIFFERENT closures rather than one condition repeated.

⚠ 78 counted against 83 source sites: this census sees what PLUGIN BUILD
registered in this composition, and the sandbox does not install every content
plugin. The source count and the runtime count are answering slightly different
questions and both are right.

### D-PERF-1 measured, after — ⭐⭐ 83 EVALUATIONS PER RUN BECOME 1

```text
[census] conditions system_conditions=61 set_conditions=29 sets_with_conditions=26
         {{closure}}=22 resource_exists=10 spacetime_is_active=8 Assets=7
```

| metric | before | after | delta |
|---|---|---|---|
| `system_conditions` | 139 | **61** | **−78** |
| `set_conditions` | 33 | 29 | −4 |
| `sets_with_conditions` | 30 | 26 | −4 |
| `gameplay_allowed` in the per-system breakdown | **78** | **absent** | gone |

`system_conditions` fell by exactly the 78 the breakdown attributed to
`gameplay_allowed`, and 56% of the app's per-system condition attachments went
with it.

⛔⛔ **AND THE `set_conditions` MOVED THE WRONG WAY, WHICH IS THE INTERESTING
PART.** One new conditioned set should have made it 34, not 29. It is not a
regression: **five of the 83 sites were TUPLE-level**
(`(a, b, c).chain().run_if(gameplay_allowed)`), and in Bevy 0.18 a tuple-level
`run_if` builds an ANONYMOUS SET carrying the condition — those were already
costing one evaluation each, not one per member, and were counted under
`set_conditions` rather than in the per-system breakdown. Converting them to
membership retired five anonymous sets and added one named one: 33 − 5 + 1 = 29,
and `sets_with_conditions` 30 − 5 + 1 = 26. Both arithmetics close exactly.

⇒ **the honest before/after is 78 per-system + 5 collective = 83 evaluations of
`gameplay_allowed` per schedule run, against 1 now.** ⚠ Had I not chased the
wrong-signed −4, the headline would have been "−78" and the five collective
sites would have been silently double-counted as a win they were not.

### The guard, and the proof it can fail

`the_gameplay_gate_is_carried_by_the_set` (in `app_it`) builds the SHIPPED app
and asserts three things: a `GameplayGated` set exists, it carries a run
condition, and ZERO systems still carry `gameplay_allowed` individually.

⭐ IT READS THE APP, NOT THE CONFIGURATOR. Calling
`configure_platformer2d_simulation_phases` in the test and then asserting it
configured something would pin the function and say nothing about whether the
shipped composition ever calls it — which is the actual hole.

⛔ IT MUST NOT PUMP A FRAME FIRST, for the reason in I3: one `update()` drains
the graph and the assertion becomes `0 == 0`.

⭐⭐ POISONED, because a gate guard that passes on the first try has proved
nothing. Dropping the `.run_if` from the one `configure_sets` call fails it with
the message it was written for:

```text
`GameplayGated` exists in ["GgrsSchedule"] but carries NO run condition.
Every system in it now runs at a menu, and nothing else in this build
would have said so
```

⚠ And the poison named the schedule: in the shipped app the sim schedule IS
`GgrsSchedule`, so this gate lives inside the rollback schedule rather than
`Update`.

### Fresh frame baseline on current main, 2026-08-29

⛔ NOT COMPARABLE TO THE 2026-08-28 ROW ABOVE — different host (that one's
machine is unidentified; see the warning at the top) and a `dev` build rather
than `profiling`. Recorded as its own series start, not as a delta.

This host (i9-11900K, 12 logical CPUs, `machine_id ec9af5ee`), `dev` profile,
headless `sandbox`, 1800 ticks, NO Tracy, census only:

```text
[census] frame  frames=948 mean=0.93 p50=0.87 p95=1.18 p99=1.84 min=0.76 max=11.93
[census] phases Update=0.724 StateTransition=0.058 PostUpdate=0.031 Last=0.025
                SpawnScene=0.024 outside=0.018 PreUpdate=0.017 First=0.014
                RunFixedMainLoop=0.014
[census] ecs    entities=64 archetypes=85 components=994 bodies=2 players=1
```

⭐ **`Update` IS 78% OF THE FRAME** (0.724 of 0.93ms), consistent with it holding
822 of 887 systems. `StateTransition` is 6% on 8 systems, which remains a high
per-system cost and remains Bevy's machinery rather than ours.

⚠⚠ **AND THE HONEST READING IS THAT THIS FRAME IS NOT IN TROUBLE.** 0.93ms mean
for a scene with TWO BODIES in it. ⛔ Do not optimize this number — chasing
microseconds in a 2-body scene is how a campaign spends itself on nothing. The
open question is not "why is the frame slow", it is **how does cost GROW**, and
that is a scaling question this repository still cannot answer.

⚠ `max=11.93ms` against a `p99` of 1.84 is a real outlier worth a look before it
is explained away; one frame in 948 costing 13x the median is the shape of a
blocking call or an allocation spike, not of ordinary variance.

### ⛔⛔ THE HEADLESS WORKLOAD IS NEARLY EMPTY, AND THAT UNDERMINES THE BASELINES

Measured 2026-08-29, sweeping `--start-room` over the headless sandbox at 20Hz
census, 3000 ticks each:

| start room | entities | archetypes | bodies | mean | p95 | max |
|---|---|---|---|---|---|---|
| *(default)* | 64 | 85 | 2 | 0.87ms | 0.99 | 1.97 |
| `goblin_encounter` | 64 | 57 | 1 | 0.84ms | 0.96 | 2.34 |
| `central_hub_complex` | 64 | 85 | 2 | 0.84ms | 0.92 | 0.96 |

⭐⭐ **EVERY ROOM GIVES 64 ENTITIES AND ONE OR TWO BODIES.** The room argument
reaches the binary — `run_game.sh --print-plan` confirms `--start-room` is
passed through as a game arg — and it moves the archetype count, so something
loads. But no room produces a populated scene.

⇒ ⛔ **EVERY HEADLESS MEASUREMENT IN THIS DOCUMENT, INCLUDING THE 2026-08-28
BASELINE, DESCRIBES A TWO-BODY WORLD.** They are honest measurements of the
engine's FIXED OVERHEAD and say nothing about gameplay cost, scaling, or the
sprite-heavy room that motivated the campaign. That is not a small caveat: the
phase split, the per-system rankings and the frame budget are all
fixed-overhead numbers, and the brief's headline question — *why does a room
with hundreds of sprites chug* — cannot be asked on this path at all.

⚠ TWO POSSIBILITIES, NOT YET SEPARATED, and they need separating before any
scaling work: either the headless host deliberately loads a minimal scene and
never populates the room, or room content fails to load headlessly and nothing
reports it. ⛔ Do not build a scaling benchmark on this path until that is
answered — a curve measured on an empty world is worse than no curve.

### An instrument gotcha this cost a run to find

⛔ A headless run finishes in well under a wall-clock SECOND, and the census
samples on WALL time. At the default 1Hz the only sample a short run produces is
the startup frame — which reports `frames=1` and `Update=127ms`, because that is
plugin build, not a frame. It is very easy to read that as a catastrophic frame
time. Set `AMBITION_PROFILE_CENSUS_HZ` high enough that the run outlives the
first interval, and ⛔ distrust any `[census] frame` row with a small `frames=`.

### ⭐⭐ THE FIRST REAL SMASH MATCH PROFILE, 2026-08-29 — and it inverts the ranking

`game/ambition_app_tools/src/bin/smash_match_profile.rs` exists because nothing
else profiled a MATCH: `run_game.sh smash` opens on character select, and every
headless room measures a two-body world. It drives the SHIPPED composition
(`build_visible_app(NoWindow)` + `smash_roster` + `GoTo(SMASH_GAMEPLAY_ROUTE)`),
waits for the round to actually go live rather than counting frames, and checks
at the end that seats still exist — a profile of a match that quietly ended is a
profile of a results screen.

This host, `dev` profile, headless, 2 fighters, 3000 ticks, no Tracy:

| phase | empty sandbox | SMASH MATCH | ratio | share of match frame |
|---|---|---|---|---|
| **PreUpdate** | 0.017ms | **1.98ms** | **116x** | **45%** |
| Update | 0.724ms | 1.31ms | 1.8x | 30% |
| PostUpdate | 0.031ms | 0.57ms | 18x | 13% |
| RunFixedMainLoop | 0.014ms | 0.32ms | 23x | 7% |
| StateTransition | 0.058ms | 0.10ms | 1.7x | 2% |
| **frame mean** | **0.87ms** | **4.41–4.82ms** | **5x** | |
| entities / archetypes | 64 / 85 | **2048 / 376** | 32x / 4.4x | |

⛔⛔⛔ **THE TABLE ABOVE COMPARES TWO DIFFERENT APP COMPOSITIONS, AND MY FIRST
READING OF IT WAS WRONG. Corrected same day.**

`PreUpdate = 45%` does NOT mean a mysterious phase got expensive. It means
**THE SIMULATION TICK IS 45% OF THE FRAME**, and the sim runs there:

- `bevy_ggrs::GgrsPlugin` registers exactly ONE system into `PreUpdate` —
  `run_ggrs_schedules`, an EXCLUSIVE system — which runs `ReadInputs`, calls
  `advance_frame()`, and dispatches `AdvanceWorld` → `world.run_schedule(GgrsSchedule)`;
- `AmbitionRollbackPlugin` calls `set_sim_schedule(GgrsSchedule)`, so
  **`SimSchedule` IS `GgrsSchedule`** — 236 `add_systems(sim, …)` call sites land
  inside that one `PreUpdate` system.

⇒ ⛔ **MY "NINE SYSTEMS, THEREFORE ~220us EACH" ARITHMETIC WAS NONSENSE.** One of
those nine contains several hundred more. The repo's own D-PERF-1 poison message
had already said `GameplayGated exists in ["GgrsSchedule"]`, and I did not
connect it.

⛔⛔ **AND THE 116x IS A COMPOSITION DIFFERENCE, NOT A SCALING ONE.** The sandbox
baseline goes through `run_headless` (`--direct` ⇒ `cli_direct_entry`), which
builds `MinimalPlugins` + a few plugins and NEVER calls `set_simulation_host` —
so `SimulationHost` falls to its default `RenderFrame`, the sim runs in `Update`,
and `GgrsSchedule` does not exist. That is why that row reads `Update=822,
PreUpdate=9`. The smash profile uses `build_visible_app` → `SimulationHost::Rollback`
+ `DefaultPlugins`. Between the two runs `PreUpdate` gained the ENTIRE SIMULATION
and the whole DefaultPlugins input/UI/picking front end. Most of the 116x is
that.

⇒ **the honest statement is "the sim tick costs 1.98ms over a 2048-entity
world"**, and the two rows are not a scaling pair. Making them one needs
`build_visible_app(NoWindow)` measured idle at the launcher AND on the smash
stage.

⭐ RULED OUT FROM SOURCE, and it removes the obvious suspect: the shipped local
session is a SyncTest with `check_distance: 0`, and ggrs SKIPS the save request
entirely at zero (`sync_test_session.rs`: *"we can skip all the saving if the
check_distance is 0"*). `SaveWorld` and `LoadWorld` never run, so every snapshot
plugin and all ~19 rollback registrations cost ZERO per frame in this build.
Rollback re-simulation is not what is happening. ⚠ unless the observatory's F9
proof pulse raised `check_distance` mid-run — settle that by printing
`RollbackExecutionStats`, where a nonzero load count proves rollback ran.

⚠ `PreUpdate` is forced `ExecutorKind::SingleThreaded` by
`serialize_frame_schedules`, as is `GgrsSchedule`. So this is honest serial CPU
time, not executor overhead.

⚠ Absolute numbers here are a `dev` build on this host; they are for RATIOS and
for ranking, and a `profiling`-profile run is owed before any of them is quoted
as a budget.

### The architecture this campaign feeds

A GPT architecture review of 2026-08-29 is synthesised, ranked and
measurement-annotated in
[`runtime-efficiency-architecture.md`](runtime-efficiency-architecture.md) —
ten directions, the target shape, an explicit do-NOT list, and the three
campaigns worth executing first. ⚠ Its top-ranked item rests on the D-PERF-2
evidence corrected above, and the correction changes that item from a frame-time
problem into a startup one.

### Investigations

| # | Hypothesis | Result | Conclusion |
|---|---|---|---|
| I1 | `gameplay_allowed`'s 87 evaluations/frame need a new set-gating mechanism built | ⛔ REJECTED — the mechanism already exists and is already in use: `configure_platformer2d_simulation_phases` puts `simulation_authorized` on `GameplaySimulationRoot` in ONE `configure_sets` call | The work is not "build set gating", it is "apply the existing pattern to the second condition". And Bevy 0.18.1's `.run_if` on a tuple is ALREADY collective (`collective_conditions`, evaluated at most once per schedule run) — only `.distributive_run_if` copies per system. Read from `bevy_ecs/src/schedule/config.rs`, not assumed. |
| I3 | The condition census can sample the schedule graph on the census interval, like every other census row | ⛔ REJECTED BY ITS OWN FIRST MEASUREMENT — it reported `system_conditions=0` beside `systems=886` | `Schedule::initialize` MOVES conditions out of `ScheduleGraph` into the private executable, and there is no public accessor for them afterwards. Any read after the first run of a schedule sees an empty graph. ⇒ the census is a ONE-SHOT `PreStartup` topology dump, and it now prints `unavailable=graph_already_initialized` rather than a confident zero. ⚠ it therefore counts what PLUGIN BUILD registered; systems added later on session activation are not in it. |
| I7 | Archetype fragmentation from spurious components is what makes `PreUpdate` cost 2ms in a match | ⛔ **REJECTED BY THE AFTER-MEASUREMENT.** Removing 1297 spurious `AttackVfxView` components (64 archetypes, 376 -> 312) moved the frame NOT AT ALL: mean 4.41-4.82ms before, 4.84ms after; PreUpdate 1.98 -> 2.11ms | The defect was real and the fix is kept ON CORRECTNESS GROUNDS — a presentation fact was being stamped onto falling-sand chunks and UI nodes — but it buys no measurable time and must not be reported as if it did. ⇒ **PreUpdate's 2ms remains unexplained**, and whatever causes it does not scale with archetype count. |
| I6 | Frame deltas of a few percent can be read off single runs | ⛔ REJECTED — the same binary and scenario produced means of 4.41, 4.51, 4.82 and 4.84ms across runs | Run-to-run spread here is ~10%, so ⛔ no single-run comparison below about 15% means anything. Repeated runs and a stated range are the minimum for any claim smaller than that, and the ledger's job is to make that discipline automatic. |
| I5 | The D-PERF rows name work that still needs doing | ⛔ REJECTED, THREE FOR THREE — every row checked so far was already addressed | D-PERF-1's set-gating mechanism already existed and was already used (`simulation_authorized` on `GameplaySimulationRoot`); D-PERF-2's "1802 ticks each" premise is wrong (the demos are tuple-gated, which Bevy makes collective); D-PERF-3's headline candidate `rebuild_control_prompt` has been change-driven since 2026-07-23, a MONTH before the measurement that named it, and `rebuild_feature_view_index`'s `insert_if_absent` already avoids the per-frame `String` allocation it looked like it made. ⇒ **the written rows are not a reliable work list.** The engine is in better shape than they say, and the next D-PERF decision needs FRESH per-system attribution rather than another row. |
| I4 | `CARGO_INCREMENTAL=0` fixes the stale-object link failure | ◐ PARTLY — it BUILDS past it but does not CLEAR it; the next ordinary build resurrected the same undefined symbol | The poisoned state lives in the package's incremental dir and survives any number of `CARGO_INCREMENTAL=0` runs. `cargo clean -p <pkg>` is the durable fix, and it is cargo's own operation rather than a delete under `target/`. ⛔ do not reach for `rm -rf`. |
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
