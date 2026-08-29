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

⭐ **FOLLOWED UP 2026-08-29 — THE DIRECTION IS SOUND AND THE REMAINING HEADROOM IS
SMALL.** After the `gameplay_allowed` hoist, a Smash match measures **1719.9
run-condition evaluations per frame**, and `[census] conditions` names where they
sit: **196 conditions on individual SYSTEMS** against 70 on 67 SETS, led by
`resource_exists`=42, `Assets`=31 (Bevy's own), a `{{closure}}`=22,
`session_world_exists`=12, `input_system_is_enabled`=10.

⛔ **BUT DO NOT SELL A HOIST AS A FRAME WIN, AND THIS IS THE PART I HAD WRONG.**
`gameplay_allowed` paid because gating the SET OFF retired its systems' WORK
(0.95ms shed against 0.93ms measured) — **not because 83 condition evaluations
were expensive.** A hoist that changes nothing about what RUNS saves only the
evaluations — ⚠ **and my back-of-envelope "tens of nanoseconds each" UNDERSTATED
that.** The measured figure elsewhere in this document is **339.5us/frame of
condition evaluation, ~141us real after the 2.4x Tracy inflation**, i.e. **~3% of
a 4.45ms `dev` frame and ~5% of a `profiling` one** — a genuinely metered cost
class, not a rounding error. ⇒ a hoist pays roughly in proportion to the
evaluations it removes, so retiring most of the 196 per-system conditions is worth
perhaps **1–2%**;
and the 12 systems carrying `session_world_exists` already skip individually. ⇒
hoist for CLARITY and for the ability to retire a class in one place; expect the
frame not to move.

⭐ The number worth watching is not the evaluation count but **how many systems a
single condition can retire at once**.

### 2. A shipped game should not schedule the experiences it does not contain

⛔⛔ **THIS DIRECTION'S HEADLINE EVIDENCE DID NOT SURVIVE RE-MEASUREMENT — see
D-PERF-2 and I5 below.** The demos ARE gated, and gated the right way. Removing
four whole experiences later moved neither frame time NOR startup registration.
⇒ read the paragraph below as the ORIGINAL claim, not as a current fact.

~~**Measured:** the sandbox run — which never entered Sanic, Smash, or Mary-O —
still evaluated `ambition_demo_sanic::ball_dash::tick_rolling`,
`ambition_demo_smash::offer_to_exit_the_match`, and
`ambition_demo_mary_o::powerups::refuse_a_weaker_form_pickup` **1802 times
each**, once per frame.~~

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
where the cost is paid once and the pattern is fine.

⭐⭐ **CLOSED 2026-08-29 BY COUNTING, AND THE PROPOSED CHECKER IS NOT NEEDED.** With
the sim schedule finally enumerable, all 147 `world.query*` sites were
cross-referenced against the 1189 system names the membership census reports
across `PreUpdate`, `Update`, `PostUpdate`, `StateTransition`, `RunFixedMainLoop`
and `GgrsSchedule`. **Exactly ONE is a function registered in a per-frame
schedule:** `apply_summon_effects`
(`features/ecs/spawn_actors.rs:2323`).

And it is not a per-frame cost: the `world.query` sits inside
`for … in board_after_commit`, a loop that is EMPTY unless a summon happened that
frame. ⚠ It does rebuild the `QueryState` once per summon rather than once per
call, which is a real inefficiency and a negligible one — summons are rare and
few. ⛔ Left alone deliberately: at this stage that is polish.

⇒ **an earlier note here proposed a `scripts/check_*.py` guard to stop the
pattern recurring. Do not build it** — the population it would police is ONE
site, and it is benign. Counting the population beat writing the checker.

⚠ **Method limits, stated so the "one" is read correctly:** systems are matched
by FUNCTION NAME against the census, so a system registered under a closure or a
generic instantiation would not match, and a `world.query*` inside a HELPER
called by a per-frame system is not followed.

### 4. Per-frame rebuilds that could be change-detection driven

`rebuild_control_prompt` (31.8us/frame), `rebuild_feature_view_index`,
`rebuild_attack_vfx_views`, `sync_ecs_actors_with_save`. Each recomputes a
derived view every frame. Their inputs change on events (a binding swap, a room
transition, a spawn), not continuously. These are individually small and
collectively the shape of the frame.

⭐⭐ **CLOSED 2026-08-29. ONE WAS ALREADY DONE, AND THE OTHER THREE ARE BOUNDED
TOO SMALL TO FUND.**

**`rebuild_control_prompt` already carries the gate this section asks for** — and
a careful one: it keys on resource-PRESENCE bits as well as `is_changed()`,
because `Option<Res<T>>` cannot report its own removal, and it invalidates on
rebind, on pad swap (spelling changes without the binding moving) and on a naming
flip. Its own comment records the payoff: *"this was ~1.4% of frame CPU
re-deriving an identical scheme"*. ⛔ **The 31.8us/frame above is STALE.**

**The other three are priced by the phase they sit in, using the sim-phase census
already in hand — no new measurement needed:**

| system | phase | phase total |
|---|---|---|
| `rebuild_feature_view_index` | `FeatureViewSync` | **0.000ms** |
| `sync_ecs_actors_with_save` | `Progression` | 0.054ms |
| `rebuild_attack_vfx_views` | `PresentationVisualSync` + `PresentationSync` | 0.074 + 0.007ms |

⇒ all three together are bounded UNDER ~0.14ms of a 4.45ms frame, and each is a
FRACTION of its phase — `FeatureViewSync` rounds to zero, so that one cannot pay
at all. ⛔ Against that, a change-detection conversion risks a STALE DERIVED VIEW,
which is a player-visible bug (a prompt naming the wrong button), and the
control-prompt implementation shows how much care correctness takes. **Not
funded.**

⭐ The method is the point: **bound the prize from data already collected before
paying for the work.** A phase total is a ceiling on every system inside it.

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
storage, or a slow SD card that is a frame hitch.

⭐⭐ **UPDATED 2026-08-29 — THE WATCHER IS OUT OF THE SIM, AND THE RECORDER IS NOT
A SMASH COST.**

`poll_world_source_changes` no longer runs on the deterministic tick at all: it
was in `WorldPrep` and now runs in `Update`, beside its readers (see the section
on it above). The blocking stat remains, debounced to ~3Hz; move it off-thread
only if it ever shows up in a frame.

**And the trace recorder's headline numbers do not describe a Smash match.**
Measured on the `Trace` sim phase, same host, same day:

| workload | bodies | `Trace` phase |
|---|---|---|
| Smash match, 2 fighters | 2 | **0.015ms** |
| Smash match, 4 fighters | 3–4 | **0.016ms** |
| `mockingbird_arena` | 2 | 0.130ms |
| `goblin_encounter` | 1 | 0.149ms |
| `hall_of_characters` | 130 | 0.281ms |

⇒ **in a Smash match the whole `Trace` phase is 15-16us — 0.35% of the frame**,
an order of magnitude under the 78us/frame this section quotes for its two
recorders. ⛔ Those figures came from a different workload and should not be
carried into a Smash budget.

⚠ **AND IT DOES NOT SCALE WITH `bodies`, WHICH IS WHAT THIS SECTION ASKED FOR.**
`goblin_encounter` has ONE body and costs 0.149ms; a 2-fighter Smash match has
TWO and costs 0.015ms — ten times less with more bodies. So the driver is
something else (actor/NPC population is the obvious candidate) and is **not
identified here**. ⭐⭐ **THE DRIVER IS `players`, NOT `bodies` — IDENTIFIED SAME DAY, FROM A COLUMN
ALREADY IN THE CENSUS.** `record_frame_system` is **slot-0 by design**: it walks
the PRIMARY PLAYER's body cluster and records one body, so it is O(1) in actors.
`record_actor_oob_frame_system` is the one that iterates every `BodyKinematics`.
And `[census] ecs` reports **`players=0` in a Smash match** against `players=1` in
every room — which is why the phase nearly vanishes there.

| workload | players | bodies | `Trace` | model |
|---|---|---|---|---|
| Smash, 2 fighters | **0** | 2 | 0.015ms | ~0.00 |
| `mockingbird_arena` | 1 | 2 | 0.130ms | 0.132 |
| `goblin_encounter` | 1 | 1 | 0.149ms | 0.131 |
| `hall_of_characters` | 1 | **130** | 0.281ms | 0.281 |

⇒ **`Trace` ≈ 0.13ms whenever a PRIMARY PLAYER exists, plus ~1.16us per body.**

⚠ **ONE CENSUS ROW IN THIS DOCUMENT DISAGREES AND IS NOT EXPLAINED:** a Smash
`sim_phases` line records `Trace=0.090`, six times the 0.015 the model predicts
for `players=0`. ⇒ treat the model as PROVISIONAL — it reproduces four
measurements including a 130-body outlier it was not fitted to, but a sixfold
outlier inside the same census is unresolved, and whoever needs this number
should re-measure rather than trust the fit.
The constant is the player cluster walk; the slope is the OOB pass. The model
reproduces all four measurements, including the 130-body outlier it was not
fitted to.

⇒ so direction 5's instruction is now answerable: **the part that is per-FRAME
rather than per-body is the 0.13ms player cluster walk**, and that is the piece to
make event-driven if it ever needs paying down. ⛔ Not funded now — it is ~2% of a
room frame, zero in a Smash match, and the forensic value is deliberate.

⭐ The lesson repeats: the answer was in a column of a row I had already printed
dozens of times. `players` sat beside `bodies` in every `[census] ecs` line.

### 6. Startup is dominated by App construction, which nothing measured

**Measured:** 2.6s from process exec to first frame; the `[startup]` phase
logger reported **120.4ms** of it, because `StartupProfiler` was created partway
through plugin build and anchored its deltas there. Tracy attributed **1.9s to
`plugin build`**, of which `AmbitionGameSimulationPlugin` was 1.675s — 876
system registrations and their schedule graphs.

The anchor is fixed (`profiling::note_process_start`). The COST is not: plugin
build scales with registered systems. This is the number a player feels on a
phone.

⛔⛔ **BUT "directions 1 and 2 shorten startup and the frame together" WAS TESTED
AND IS FALSE.** Removing 61 `Update` systems moved plugin registration
**372.3ms → 380.8ms** — the wrong way, inside noise. ⇒ registration cost is
Bevy's schedule-graph construction, not a function of OUR system count in any
way this repo can exploit. ⚠ And the 2.6s here is a WINDOWED figure carrying
window creation and shader compilation; the windowless composition is **608ms**.
Both are recorded below.

## Campaign 2026-08-29 — runtime efficiency, 24h

### ⛔ A NOTE ON "THE BRIEF" AND ON DIRECTION NUMBERS — read before following any pointer

**"The brief"** in this document means the two GPT-authored prompts Jon supplied
when the campaign started. They are NOT in this repository and are not linked
anywhere, so ⛔ **a reference to "the brief's direction N" cannot be followed** —
including one to a "direction 8", which does not exist under any numbering here.

⚠ **AND "direction N" IS AMBIGUOUS IN THIS FILE.** It means two different things:
the SIX directions defined in this document (§1 run conditions on sets … §6
startup), and the TEN in
[`runtime-efficiency-architecture.md`](runtime-efficiency-architecture.md). They
do NOT correspond — this document's §5 is dev instrumentation, while the
architecture synthesis' §5 is world residency. ⇒ **when a pointer says "direction
N", check which document it means before acting on it**; several passages below
use the token both ways.

### ⭐ READ THIS FIRST — the campaign in one screen

Everything below is the working record, including retractions. This is what
survived.

**WHAT IS TRUE ABOUT THE FRAME** (all from `NoWindow` runs, where the phase
census is trustworthy — see the render caveat below):
- a 2-fighter Smash match is **4.5–5.0ms** (`dev`) / **2.8–3.2ms** (`profiling`);
- the whole gameplay simulation is **0.83–0.93ms across 17 phases**;
- ⭐⭐ **THE FRAME IS FULLY ATTRIBUTED** (4.45ms, 2 fighters, windowless): 0.83ms
  marked gameplay sim · **0.21ms GGRS driver overhead** (cheap, as
  `check_distance: 0` predicts) · **0.93ms `PreUpdate` OUTSIDE the driver** ·
  1.23ms `Update` · 0.51ms `PostUpdate` · 0.31ms `RunFixedMainLoop`;
- ⛔ **`PreUpdate` IS NOT "THE SIMULATION TICK"** — the sim is 0.83 of its 1.98ms.
  And the remainder is not purely `DefaultPlugins`: the membership census shows
  our OWN systems there too (falling sand's particles, chunk loading, effect
  release) alongside 31 asset trackers and ~12 picking systems;
- **that 0.93ms is BREADTH, not a hot spot** — 0.93ms over ~135 systems is
  **6.9us each**, and `Update` is 1.23–1.42ms over **494–497** systems
  (**2.4–2.9us each**) — ⚠ the low end sits just BELOW the 2.9–15.6us band's
  floor, which is the point: these are the CHEAPEST systems in the app, not
  hidden expensive ones. Gating `Update`'s two largest groups recovered nothing;
- **there is no hot system.** ~630 systems at 2.9–15.6us each.

⭐⭐⭐ **AND RESPONSIVENESS IS A DIFFERENT QUESTION FROM THROUGHPUT: SMASH'S FRAME
SPIKES ARE NOT IN THE SIMULATION.** 340 intervals quartile-split by worst frame:
sim total 0.837 → 0.849ms (**+1.4%**) while the frame max goes **4.64 → 8.28ms**;
every gameplay phase moves ≤4 MICROseconds. ⇒ making the sim cheaper is worth
**~zero for responsiveness** and remains correct for throughput. What the spike
IS needs a host with a GPU.

⭐⭐ **THE COST MODEL — the sheet to price a feature against:**

| quantity | price | where it lands |
|---|---|---|
| baseline frame, 2-fighter match | **~4.5ms** | ~630 systems, 2.9–15.6us each, NO hot one |
| **per FIGHTER** | **~125–240us** ⚠ 2 reps | ~1/3 `PostUpdate`; shares stabler than the total |
| per BODY | **~16us** | `WorldPrep` |
| per VISIBLE SPRITE | **~1.4us** | render extraction |
| a frame SPIKE | **+3.6ms** at the tail | ⛔ NOT the sim |

**Fighter count is the ONLY thing measured that scales with a player's choice**,
and a fighter is NOT a body — at ~125–240us it is 8–15x the per-body constant, because
it carries a sprite rig and a brain and combat state, not just kinematics. ⛔
Optimising only the sim addresses a THIRD of a fighter.

⭐ **AND THE INPUT PATH IS ALREADY CORRECT.** Responsiveness to a player is input
LATENCY, not frame time. `bevy_ggrs` declares `RunGgrsSystems.after(InputSystems)`,
so input is read and consumed in the SAME frame by construction — ⛔ a grep finds
no such constraint in this repo because it lives in the dependency, where it
belongs; do not add a duplicate. True device-to-sim latency cannot be measured
headlessly (the latch needs device authority) and joins the real-hardware item.

**WHAT IS TRUE ABOUT THE PREMISES WE STARTED FROM** — most did not survive:
- ⭐ *"a room with hundreds of sprites chugs"* — **HALF TRUE, corrected 2026-08-29
  after sweeping ALL 72 ROOMS.** The room exists: `mockingbird_arena` bursts to
  **295 visible sprites** for about one second. But it costs **+0.36ms of mean
  (4.9%)**, at 1.40us per visible sprite. ⇒ the sprites are REAL and CHEAP; the
  cost lands in the TAIL (worst frame 11.66 → **17.30ms**, past the 60Hz budget),
  not in throughput. ⛔ The earlier "no room exceeds 46–87 visible sprites" was a
  FOUR-ROOM SAMPLE and is retired — every sample photographed rooms AT REST, and
  the peak is an EVENT;
- ⛔ *"inactive experiences participate in the frame"* — removing FOUR whole
  experiences moved neither frame time nor startup registration;
- ⛔ *"the presentation projection rewrites unchanged state"* — on real content,
  **55 of 2515 transforms** and **32 of 151 sprites** changed;
- ⛔ *"Smash renders the world more than once"* — `world_rendering=1`;
- ⛔ *"rollback snapshots cost"* — `check_distance: 0`, ggrs skips saving entirely;
- ⛔ entity population predicts cost — **three separate probes**, all null.

**THE ONE THING THAT WORKED**, and why: `gameplay_allowed` hoisted from 83
per-run evaluations to 1. It worked because it retired a WHOLE CLASS at once,
which is the only lever shape this frame responds to. Gating a set off DOES
reclaim its systems' work (measured: 0.95ms shed against 0.93ms).

**⚠ THE TRACY OBSERVER EFFECT IS ~2.4x ON THIS SCENARIO, NOT THE ~9x IN THE
2026-08-28 ROW.** Measured 2026-08-29: 7.04ms traced against 2.82–3.19ms clean,
with the ingest reporting a 6.7% profiler share. Two different scenarios, two
different ratios — ⛔ do not carry the sandbox's 9x onto smash work. ⚠ And this
CPU advertises no invariant TSC (`tracy.caveat` in every bundle says so), so
Tracy RATIOS are sound and its absolute microseconds are approximate.

**⛔⛔ THE INSTRUMENT RULE THAT COST THE MOST:** `[census] phases` attributes
WALL TIME between markers, so **GPU blocking lands in whichever phase brackets
it**. Raising a render target 320x240 → 1280x960 took `StateTransition` from
0.169 to 1.822ms — a state phase scaling with PIXELS. Phase splits are valid ONLY
from non-rendering runs. `fragment_shader_invocations=0` does NOT make them safe.

**⭐⭐ ONE THING TRACY FOUND THAT NOBODY HAD NAMED: A DEBRIS EFFECT INSTALLS A
PHYSICS ENGINE.** `AmbitionPhysicsPlugin` is gated on a feature called
`physics_debris` and installs `PhysicsPlugins::default()` — the whole of avian2d.
A Smash match therefore runs, every fixed step:

```text
RunFixedMainLoop 823us → FixedMain 677 → FixedPostUpdate 589
    → PhysicsSchedule 370us   → SubstepSchedule ~6.4 substeps/frame
```

⚠ Times are Tracy's (~9x inflated, and this CPU has no invariant TSC — the
bundle's own `tracy.caveat` says treat RATIOS as sound and microseconds as
approximate), so the real cost is perhaps 40–60us/frame, ~1% of the frame and
BELOW the noise floor. ⇒ ⛔ not a performance fix, and consistent with everything
else this campaign measured. **It is recorded because it is the invariant the
brief states, violated exactly:** *capability installed but dormant ⇒ very small
fixed cost.* Here a cosmetic effect costs a physics engine's schedules whether
or not one debris body exists. ⭐ **DONE:** `pause_physics_when_no_debris_exists` pauses avian while no
`RigidBody` exists and unpauses the frame one appears.

⛔ PAUSE, NOT A SCHEDULE GATE, deliberately — `PhysicsTime::pause` is avian's own
supported API, so its schedules still run their bookkeeping and a body spawned
this frame initialises normally; it simply does not step. Skipping the schedules
would reach past the library's contract for the same microseconds, and that is
how a debris body ends up never waking.
⛔ AND IT WRITES ONLY ON A CHANGE: `ResMut` marks its resource changed on DEREF,
so touching `Time<Physics>` every frame would announce a change that did not
happen to every reader — the defect fixed in the hot-reload watcher that same
morning.
⭐⭐ **VERIFIED STRUCTURALLY, and the proportionality is exactly right:**

| Tracy zone | before | after | |
|---|---|---|---|
| `PhysicsSchedule` calls | 1975 | **359** | **−82%** |
| `SubstepSchedule` calls | 11850 | **2154** | **−82%** |
| `FixedPostUpdate` | 589us | **295us** | **−50%** |
| `RunFixedMainLoop` | 823us | 506us | −39% |

Physics now runs on ~18% of frames rather than all of them — a Smash match DOES
spawn debris on hits, so the right answer was never zero. ⭐ And its per-call
cost ROSE (370 → 457us) because when it runs it now has real bodies to solve.
**That is the invariant working: the capability pays for what it does.**

⚠ Tracy numbers, so ~2.4x inflated — the RATIOS are the result, not the
microseconds. And still NO frame-time claim: the whole thing is ~1% of a frame,
below this machine's noise floor, sized before the code was written.

**⭐⭐ THE ONLY NAMED PER-SYSTEM COST TRACY FOUND WAS egui — AND IT IS NOW GATED.**
Per-system attribution on the trustworthy `NoWindow` path ranked everything, and
every other entry was a schedule wrapper. The exception:

```text
egui::Context::run   87.6us x 1853 frames    pass{tag="0"}  85.9us
begin_pass 40.8   end_pass 31.1   plugin hooks ~36   tessellate ~11
```

egui ran a FULL CONTEXT PASS every frame with no window open and no inspector on
screen: `run_if(inspector_visible)` gates the inspector WIDGETS, while
`EguiPlugin` runs its pass unconditionally. Gating
`EguiPreUpdateSet::BeginPass` + `EguiPostUpdateSet::EndPass` on
`inspector_visible || world_inspector_visible` removes it.

⛔ BOTH ENDS OR NEITHER: begin opens a pass and end closes it, so gating only the
expensive half leaves egui with a pass that never closes.

⭐ VERIFIED STRUCTURALLY, which beats a wall-clock claim — zones exist or they do
not, with no noise floor to argue about:

| Tracy zones | before | after |
|---|---|---|
| `Context::run` / `pass{}` | present, 87.6us/frame | **0** |
| other egui work zones | present | **0** |
| egui `check_conditions` | — | 24 (the gate itself) |

⚠ Sized honestly: ~36us real after the 2.4x Tracy inflation, ~1.2% of a
`profiling` frame — BELOW the noise floor, so no frame-time claim. ⭐ And it is
Jon's steer done literally: the inspector still works the instant it is shown, it
just stops running a full egui pass to draw nothing.

**⭐⭐ RUN-CONDITION EVALUATION IS ~5% OF THE FRAME, AND THAT IS WHY THE ONE
LEVER WORKED.** The Tracy export carries 1452 `check_conditions` zones totalling
**339.5us/frame** — ~141us real after the 2.4x inflation, about 5% of a
`profiling` frame. ⇒ the `gameplay_allowed` hoist (83 evaluations → 1) was
cutting into a genuinely metered cost class, not a theoretical one. Dearest
individual conditions, per frame:

```text
14.78us  bevy_ecs::apply_deferred          2.98  XpbdSolverPlugin::build closure
 2.56us  forget_unclaimed_feature_views_while_dormant
 2.35us  ambition_dev_tools::runtime_census::mark_sim_phase   ← MY OWN INSTRUMENT
 2.10us  avian2d ...forces::apply_local_acceleration
```

⛔ **AND THE REST OF THAT RANKING WAS CHASED AND IS NOT ACTIONABLE.**
`forget_unclaimed_feature_views_while_dormant` (2.56us) looked promising — its
NAME says it runs while dormant — but its body is `if !ids.is_empty() { clear() }`,
which already avoids the `DerefMut` on a quiet frame, and its cost is the
CONDITION `not(session_presentation_is_ready)`: one `roots.single()` query, ~1us
real after inflation. The dearest entry, `bevy_ecs::apply_deferred` at 14.78us
(~6us real), is Bevy's own sync-point machinery, not ours. ⇒ **Tracy's leads are
exhausted: egui and avian were the two real ones**, and both are fixed.

⚠⚠ **THE INSTRUMENT IS THE FOURTH MOST EXPENSIVE CONDITION IN THE FRAME.** It is
registered only when the census is switched on, so a normal run does not pay it —
but it is a reminder in the campaign's own data that measuring is not free, and
that this is precisely why every census row here is gated at build time.

⛔⛔ **AND A TOOLING LIMIT WORTH KNOWING BEFORE SOMEBODY PLANS AROUND TRACY:
THERE ARE NO PER-SYSTEM EXECUTION SPANS.** The export holds 1452
`check_conditions`, 34 `schedule`, 15 `par_for_each` and egui's own
`function_scope` zones — and NO `system{name=…}` entries. That is why egui was
the only named per-system cost: its spans come from egui's instrumentation, not
Bevy's. ⇒ **`Update`'s 1.42ms cannot be attributed per system from this bundle**,
and anyone expecting Tracy to rank Ambition's systems will find only their
condition checks. ⛔⛔ **AND IT IS NOT A CONFIGURATION MISTAKE — BEVY 0.18 EMITS NO PER-SYSTEM
EXECUTION SPAN IN EITHER EXECUTOR.** Read from the source, not inferred:
`bevy_ecs/schedule/executor/single_threaded.rs` has exactly ONE `info_span!`,
`check_conditions`; `multi_threaded.rs` has only `calculate conflicting systems`
and an executor-wide span. `profile` already enables BOTH `bevy/trace` and
`bevy/trace_tracy`, so nothing is switched off. And Ambition serializes `First`,
`PreUpdate`, `Update`, `PostUpdate` and `GgrsSchedule` to `SingleThreaded`
anyway.

⇒ **"use Tracy to find the expensive system" IS NOT ACHIEVABLE IN THIS ENGINE AS
IT STANDS.** The three real routes, in ascending cost: (a) the `check_conditions`
proxy, which measures a system's CONDITION rather than its body; (b) hand-rolled
brackets like this campaign's `sim_phases` boundaries, around a set you already
suspect; (c) a patched Bevy that spans system execution. ⛔ Nobody should plan a
session around (c) without pricing it first.

**⚠ THE COMPILE RATCHET HAS THREE FINDINGS, AND THEY ARE MOSTLY NOT THIS
CAMPAIGN'S.** `compile_ratchet.py --report-only` against the baseline frozen at
`11ef33c5b5a5` (2026-08-27) reports two `REGRESSED` budgets (+36.6s against a
+34.0s allowance) and `critical_path_crates 14 → 15`. The `--diff` traces the
regressed pair to `ambition_platformer2d_core +713 lines`; the biggest movers
overall are `ambition_combat` (+2,521), `ambition_boss_encounter` (+1,615) and
`ambition_input` (+52,111 edit cost from fan-in). None of those is this campaign.

⛔ **DELIBERATELY NOT RE-FROZEN.** `--update` would launder three days of other
agents' growth under a performance commit, and a per-crate ledger that absorbs
somebody else's debt stops being evidence. Whoever lands the `platformer2d_core`
growth should re-freeze and say which change did it.

⚠ **WHAT IS OURS, stated because measuring is not free:** `ambition_dev_tools
+1,388 lines` and `ambition_render +1,121`, of which roughly 600 are this
campaign's censuses — and `ambition_dev_tools` has 18 dependents, so that is real
edit cost paid by every one of them. The censuses earn it (they found egui,
avian, the 494-vs-822 correction and twelve rejections) but the trade should be
visible, and it is the same lesson as the instrument being the fourth most
expensive condition in the frame.

**WHERE TO GO NEXT**, in order:
1. ▢ take a real-hardware Smash profile: `./scripts/profile_desktop.sh --smash`
   on a GPU machine — every number here is from a GPU-less host;
2. ✅ **CLOSED — it is `hall_of_characters`** (130 bodies, `WorldPrep` 2.373ms,
   ~16us of sim per body; a GALLERY, not a gameplay room). ⛔ ~~name the room that
   actually chugs; four sampled rooms do not~~ — the four-room sample it cited is
   itself retired; all 72 rooms were swept;
3. ✅ **CLOSED — `Update` (1.23–1.42ms over 494–497 systems) is BREADTH**, ~2.4–2.9us
   per system, and `PreUpdate`'s non-driver 0.93ms is 6.9us per system: no group
   there hides a millisecond. ⛔ **AND NEITHER TOOL CAN SPLIT IT FURTHER TODAY.** Tracy emits no per-system execution span (above), and a
   hand-rolled bracket needs a SET to stand beside: `ambition_render` spreads its
   99 systems over nine different sets (`SpriteVisualSync`, `ActorOverlaySet`,
   `ActorNameplateSet`, `DialogPresentationSet`, `WorldLabelLayoutSet`, …) with
   no umbrella. ⇒ attributing `Update` means FIRST giving presentation an
   umbrella set, which is an architectural change made for a measurement.
   ⚠ Price it against the prior: every other phase came out diffuse, and 494
   systems at 1.42ms is ~2.9us each. The expected finding is "nothing to find",
   and that is worth knowing before the umbrella is built;
4. ⛔ do NOT fund capability composition as a performance migration. It is
   architecture and startup work; both were measured and neither moved.

**⭐ A NAMED DEFECT CLASS WORTH GREPPING FOR: *work done before the check that
would have avoided it.*** Three fixed this campaign, all found by READING rather
than profiling, none individually measurable:
- `puppy_slug_seed` lowercased a name per candidate actor per frame — BEFORE the
  `dream_seed.is_some()` short circuit that already settled it;
- `cut_rope/victory` collected a `Vec` every frame in every room for one
  `contains` that only matters in one room;
- `publish_bevy_ui_menu_actions` scanned every `PointerLocation` above its rows
  loop, for a question only the pressed-arm ever asks.

⛔⛔ **AND IT DOES NOT GREP.** A heuristic sweep for "an allocation or scan
before a `return` in the same function" over the whole workspace returned **125
candidates**, and the tightest of them —
`ambition_conversation::break_dialogue_on_hit_or_separation` — is CORRECT code:
its `if !conversation.is_live() { return; }` comes FIRST and the `collect()`
after, which is precisely the shape the class prescribes. ⇒ **the class is real
but needs a READING pass with judgment, not a regex.** All three instances were
found by a human-quality survey reading system bodies; none would have been
picked out of a 125-row list. ⛔ Do not build a lint for this.

⛔ AND THE TEMPTING FIX IS USUALLY THE WRONG ONE. In all three the obvious move
is an early return, and in all three the path that looks like a no-op is doing
teardown — the menu's `None` arm releases the arm, `victory`'s drain is cursor
hygiene, `deep_dream`'s attach is spawn-when-live. ⇒ **defer or amortise the
WORK, do not skip the SYSTEM.**

**METHOD, dearly bought — five instruments lied and each was caught by a
SECONDARY number, never by the primary one:** a census must report POPULATION
beside TIMING; check the thing you removed actually left; check the sample window
is steady state; and an instrument that can mislead must SAY SO in its own output.



Jon armed a 24-hour goal: *"make this game run faster, more efficiently, and
elegantly"*, on evidence, with BOTH deliverables required — measurements
preserved as history, and landed work with before/after numbers. His constraint
on method: be cheap with the machine (6-CPU VM, no GPU, shared target dir — one
cargo invocation at a time, no `--workspace --tests`). Build times are fair game
opportunistically but do not outrank the frame.

### Open work, in leverage order

Each row is a lever already MEASURED in the baseline above. Do not re-derive the
baseline; extend it.

- ▢ **D-PERF-1 — hoist `gameplay_allowed` off 83 systems onto a set.** (⚠ "89"
  appears below as the pre-work ESTIMATE; the measured count is **83** — 78
  per-system plus 5 tuple-level — and the arithmetic closes twice later in this
  document.) The
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
  them execute while the mode is inactive. ⚠ **THIS COUNT IS THE SANDBOX COMPOSITION, NOT "the whole app"** — a live Smash
  match measures **196 per-system and 70 set conditions** across 67 sets. Two
  compositions, two populations; say which one a condition count describes. The
  sandbox composition carries 61
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
  which together outweigh `tick_actor_brains` ⚠ **in the 2026-08-28 SANDBOX trace
  — NOT in a Smash match, where the whole `Trace` phase is 15-16us; do not carry
  these into a Smash budget**. Both recorders are per-frame and
  the oob one takes a `CollisionWorld` plus a body query — the budget question is
  what `--ship` carries. ⛔ **"can the cost scale with BODIES rather than frames"
  is ANSWERED and the premise was wrong:** the `Trace` phase scales with
  `players`, NOT `bodies` — ~0.13ms whenever a primary player exists, plus
  ~1.16us per body — and a Smash match (`players=0`) pays 0.015ms.

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

### ⭐⭐ ANSWERED 2026-08-29 — HEADLESS OMITS `LdtkPlugin` BY DESIGN, SO NO ROOM CONTENT EVER SPAWNS

The section above left two possibilities open — "either the headless host
deliberately loads a minimal scene and never populates the room, or room content
fails to load headlessly and nothing reports it." **It is the first, and it is
documented in the code:**

- `crates/ambition_platformer2d_actor_monolith/src/session/setup.rs:128` —
  *"Headless builds skip LdtkPlugin (its tile pipeline needs RenderApp)"*
- `game/ambition_app/src/app/plugins.rs:257` — *"`LdtkPlugin` panics in headless
  because its tile pipeline expects a RenderApp"*

⇒ Room GEOMETRY loads through the repo's own runtime spine (spine and solid
revisions both reach 1), but room CONTENT is spawned by `bevy_ecs_ldtk`, which
is not present. Hence `ldtk entities : 0` and 64 entities in every room. Not a
silent failure — an architectural constraint, correctly commented.

⛔ **AND `NoWindow` CANNOT SUBSTITUTE.** It sets `backends: None` and has no
render app at all, so it cannot host `LdtkPlugin` either. The smash profile's
2048 entities come from its own roster composition, not from LDtk, so they do
NOT show that NoWindow populates a room. ⇒ **the only composition that populates
a ROOM is `OffscreenGpu`**, which is what `capture_scene` already builds.

**A sweep of `headless --start-room` over 66 rooms was run and DISCARDED.**
Differencing 100 vs 1100 ticks gave 0.55–0.71 ms/tick and room load 0.63–0.86s,
with no outlier — a narrow spread that is exactly what an unpopulated world
produces. ⛔ Those numbers describe fixed overhead in 66 rooms and are recorded
here only so nobody re-runs them.

⭐ **THE INSTRUMENT THAT WORKS, AND WHY IT IS VALID ON THIS HOST.** `capture_scene
<room> player <out> 320x240 --warmup 180` with `AMBITION_PROFILE_CENSUS=1` boots
the room populated — `goblin_encounter` reports entities=2048, archetypes=293,
616 tile components, sprites=53, sprites_visible=33. Its timings are
software-rasterized and untrustworthy per the phase-census rule, **but a COUNT
cannot be contaminated by rasterization**, so the population census is valid here
even though the frame time is not. That is what makes the brief's headline
question — *is there a sprite-heavy room at all?* — answerable without a GPU.

### ⭐⭐⭐ ALL 72 ROOMS MEASURED, 2026-08-29 — THE SPRITE-HEAVY ROOM IS REAL, AND IT IS A ONE-SECOND BURST

Every shipped room booted through `capture_scene <room> player <out> 320x240`
with the census on. 72 of 72 succeeded. ⛔ **This retires the recorded claim that
"no shipped room exceeds 46–87 visible sprites" — that was a four-room sample,
and it was wrong.**

| room | visible sprites | total sprites | bodies | archetypes |
|---|---|---|---|---|
| **`mockingbird_arena`** | **277** | **283** | 2 | 339 |
| `sanic_sandbox` | 91 | 156 | 2 | 330 |
| `duel_arena` | 90 | 102 | 5 | 363 |
| `gnu_ton_arena` | 81 | 145 | 5 | 392 |
| `hall_of_characters` | 52 | 237 | **130** | 452 |

⭐⭐ **AND THE PEAK IS AN EVENT, NOT A ROOM.** Sampling `mockingbird_arena` at
20Hz shows the population is a BURST: steady at 34–35 visible from t=0.75 to
t=2.01, ramping 125 → 228 → 278, peaking at **295 visible**, then collapsing back
to 35 by t=3.03. ⇒ **the whole campaign's sprite sampling missed it because every
sample photographed rooms AT REST.** A steady-state census cannot see a
one-second barrage.

**What the burst costs** — same run, same resolution, arms straddling the event,
rows with `frames>=5` only:

| | visible | mean | p95 | worst frame |
|---|---|---|---|---|
| baseline | 35 | 7.42ms | 8.46ms | 11.66ms |
| burst | 277–295 | 7.78ms | 9.50ms | **17.30ms** |

⇒ **8x the sprites costs +0.36ms of mean — 4.9%.** That is **1.40us per visible
sprite**, the same order as the campaign's independent +36-sprite probe
(3.89us/sprite), so two unrelated measurements agree on the slope.

⭐ **THE COST IS IN THE TAIL, NOT THE MEAN.** The worst frame goes 11.66 → 17.30ms,
past the 16.67ms budget at 60Hz. The sprite-heavy moment is real and can drop a
frame, but as a HITCH, not as throughput. ⚠ On a software-rasterizing host; real
GPU hardware should shrink the raster share and leave the CPU-side extraction.

### ⭐⭐⭐ THE ROOM THAT CHUGS IS `hall_of_characters`, AND THE COST IS SIM, NOT RENDER

Measured on `[census] sim_phases`, which is inside the sim tick and therefore the
one timing the GPU-contamination rule does NOT invalidate. 46–71 ticks each.

| room | bodies | live entities | WorldPrep | frame mean |
|---|---|---|---|---|
| `goblin_encounter` | 1 | 1919 | 0.269ms | 7.08ms |
| `basement_npcs` | 4 | 2238 | 0.353ms | — |
| `duel_arena` | 4 | **1732** | 0.354ms | — |
| `basement_enemies` | 9 | 2524 | 0.431ms | — |
| **`hall_of_characters`** | **130** | 3858 | **2.373ms** | **10.55ms** |

⭐⭐ **`WorldPrep` SCALES WITH BODIES, NOT WITH ENTITIES, AND THE PAIR PROVES IT:**
`duel_arena` and `basement_npcs` both have 4 bodies, differ by 30% in live
entities (1732 vs 2238), and their `WorldPrep` agrees to **1 microsecond**. That
is the FOURTH independent probe in this campaign in which entity population
failed to predict cost, and the first one with a matched control.

⇒ **The engine's per-body simulation price is ~16us** ((2.373-0.269)/129). At 130
bodies `WorldPrep` alone is 2.3ms — more than double the ENTIRE gameplay sim of
an ordinary room (0.93–1.37ms).

⛔ **BUT THIS DOES NOT FUND OPTIMISATION WORK, AND SHOULD NOT.** `hall_of_characters`
is a GALLERY that displays every character at once. At 16us/body a Smash match costs a small
fraction of a millisecond here — ⛔ measured `bodies=2` for two fighters; an
earlier "8 bodies" figure in this document had no source. No gameplay room in the game comes near 130 bodies — the
next highest is 9. ⇒ the item "name the room that chugs" is CLOSED: the room
exists, it is not a gameplay room, and its cost is explained and proportional.
The useful residue is the CONSTANT: **budget ~16us of sim per body.**

### ⭐⭐⭐ SMASH'S FRAME SPIKES ARE NOT IN THE SIMULATION — 2026-08-29

The campaign measured MEANS throughout. Responsiveness lives in the TAIL, so
this asks a different question: a 4000-tick match, census at 20Hz, 340 intervals
with `frames>=5`, split into quartiles by WORST FRAME in the interval.

| | calm quartile | spiky quartile | delta |
|---|---|---|---|
| every individual sim phase | — | — | **<= +0.004ms** |
| **sim total** | **0.837ms** | **0.849ms** | **+0.012ms (1.4%)** |
| frame mean | 4.204ms | 4.924ms | +0.72ms |
| frame max | 4.640ms | 8.275ms | **+3.6ms** |

⇒ **THE SIMULATION IS FLAT ACROSS INTERVALS WHOSE WORST FRAME DIFFERS BY 3.6ms.**
Every gameplay phase moves by at most 4 MICROseconds while the tail moves by
milliseconds. ⛔ **Optimising gameplay systems cannot remove Smash's frame
spikes**, which prices the whole "make the sim cheaper" direction for
RESPONSIVENESS at approximately zero. (It remains the right direction for
THROUGHPUT — the sim is 0.84ms of a 4.2ms frame.)

⚠ **WHAT THE SPIKE IS, IS NOT ANSWERED HERE, AND THIS HOST CANNOT ANSWER IT.**
The remaining candidates are the renderer, asset streaming, the allocator, and
OS scheduling — and on a GPU-less VM under variable load, host scheduling alone
can produce them. One interval reached a 172ms max, which is a one-off, not a
pattern. ⇒ this folds into the open real-hardware item rather than standing as
its own lead.

⛔⛔ **AND THE PHASE-LEVEL VERSION OF THIS ANALYSIS WAS ATTEMPTED FIRST AND
ABANDONED, CORRECTLY.** `[census] phases_warning` fired
`untrustworthy=render_blocking world_rendering=1` on the windowless smash run —
`NoWindow` still reports a rendering world camera, so whole-frame phase splits
are contaminated there exactly as the retracted `StateTransition` finding was.
⭐ The guard added earlier in this campaign caught its own author about to repeat
the mistake it was written for. `sim_phases` is inside the gameplay tick and is
the instrument that survives.

### ⭐⭐⭐ THE SMASH FRAME, FULLY ATTRIBUTED AT LAST — 2026-08-29

Unblocked by fixing the phase-census guard — described in the `phases_warning`
material later in this document, NOT the `GameplayGated` guard section above.
⛔ An earlier passage praises that guard for "catching its own author"; it was in
fact a FALSE POSITIVE and the praise is superseded here. `NoWindow` sets
`backends: None` and omits
the RenderApp, so a windowless smash run draws NOTHING and its phase splits ARE
valid. The old guard warned on the CAMERA COUNT and condemned exactly the runs
that were sound.

3000 ticks, `dev`, 2 fighters, windowless. Frame mean **4.45ms**.

⚠ **AN EARLIER SECTION ATTRIBUTES THE SAME SCENARIO AT 4.7ms** (sim 0.93 · driver
0.26 · outside-driver 0.95 · `PostUpdate` 0.65). **Both are real runs and neither
supersedes the other** — this is dev-profile run-to-run spread of roughly 5–10%,
the same spread that made a cross-arm DELTA unusable elsewhere in this document.
⇒ **read every number below as ±10%, and read the SHARES rather than the
milliseconds.** The shares agree between the two runs to within a couple of
points.

| block | ms | share | how it was measured |
|---|---|---|---|
| marked gameplay sim | 0.83 | 19% | `[census] sim_phases`, 17 phases |
| GGRS driver overhead | **0.21** | 5% | `ggrs_driver inside=1.047` minus the sim it contains |
| **`PreUpdate` outside the driver** | **0.93** | **21%** | `PreUpdate` 1.98 minus `ggrs_driver` 1.047 |
| `Update` | 1.23 | 28% | phases |
| `PostUpdate` | 0.51 | 11% | phases |
| `RunFixedMainLoop` | 0.31 | 7% | phases |
| First / StateTransition / SpawnScene / Last / outside | 0.31 | 7% | phases |

⭐ **THE ROLLBACK DRIVER IS CHEAP: 0.21ms**, which is what `check_distance: 0`
predicted — it skips saving entirely. ⛔ Do not go looking for rollback cost here.

⭐⭐ **AND `PreUpdate` IS NOT "THE SIMULATION TICK", which an earlier correction in
this document implies.** The sim is 0.83ms of a 1.98ms `PreUpdate`. **0.93ms — a
FIFTH of the whole frame — is PreUpdate work that is neither the gameplay sim nor
the rollback driver**, and prior probes accounted for only ~0.145ms of it.

**The named candidates**, from `[census] membership schedule=PreUpdate` (137
systems): **31 `Assets` trackers**; ~15 PARTICLE systems (`msgr_spawn_particle`,
`msgr_sync_particle`, `msgr_despawn_*`, `despawn_orphaned_particles`,
`register_transform_particles`, `sync_particle_type_registry`); ~12 PICKING
systems (`ui_picking`, `lunex_2d_picking`, `cube_3d_picking`, `generate_hovermap`,
`update_window_hits`, `update_pointer_map`); the egui pass; and the input writers.
⛔ **NOT A FINDING, checked and explained:** `release_confirmed_effects` appears
**7 times** and `tick_action_state` / `update_action_state` twice each, which
reads as duplicate registration. It is not —
`release_confirmed_effects::<M: Message>` is GENERIC over the effect message
type, so seven rows mean seven effect types, one registration each
(`external_effects.rs:224`). ⇒ a repeated name in a membership census is a
MONOMORPHISATION before it is a bug; check the generic parameter before
reporting it.

⭐⭐⭐ **AND THE 0.93ms IS NOT A HIDDEN HOT SPOT — IT IS THE BROAD FRAME AGAIN.
The arithmetic settles it without a probe:**

| block | ms | systems | us/system |
|---|---|---|---|
| `PreUpdate` minus the driver | 0.93 | ~135 | **6.9** |
| `Update` | 1.23 | 521 | **2.4** |

Both land inside the campaign's independently measured **2.9–15.6us** per-system
band. ⇒ there is no group in `PreUpdate` hiding a millisecond; there are ~135
systems each costing a few microseconds, which is the SAME conclusion the whole
campaign reached from the other direction. ⛔ **A per-group poison probe would
recover TENS of microseconds each**, and an earlier probe already priced ui-focus
+ picking at **0.00** and the 31 asset trackers at **<=0.145ms**.

⇒ **THE ONLY LEVER THAT MOVES A BROAD FRAME IS RETIRING A WHOLE CLASS AT ONCE**,
which is exactly why `gameplay_allowed` (83 evaluations per run → 1) is the one
change in this campaign with a measured win. Hunting the next 6.9us system is not
that lever.

⛔ **STOPPED HERE DELIBERATELY, AND THE STAGE AGREES.** At 4.45ms the frame is a
QUARTER of the 60Hz budget, the spikes are proven NOT to be sim, and the
remaining breadth is priced. ⇒ what is left needs the real-hardware profile —
where the RENDER cost this host cannot produce finally joins the frame — not more
CPU archaeology here.

### ⭐⭐ A BLOCKING `fs::metadata` WAS RUNNING INSIDE THE SIMULATION — FIXED 2026-08-29

`poll_world_source_changes`, the LDtk hot-reload watcher, was registered in the
sim schedule `.in_set(WorldPrep)` — **the sim's largest phase**. It does a
blocking `fs::metadata`, measured at up to **3.9ms on virtiofs**.

⛔ **AND IT WAS UNFIT FOR THAT SCHEDULE ON DETERMINISM GROUNDS TOO:** it keeps its
debounce in a `Local<f32>`, which does NOT rewind, so a session that actually
rolled back would re-stat the file once per RE-SIMULATED tick. `check_distance: 0`
makes that latent rather than live — which is exactly when it is cheap to fix.

⛔⛔ **CORRECTION, same day: I ALSO BLAMED `Res<Time>`, AND THAT WAS WRONG.**
`bevy_ggrs` swaps `Time<()>` for the rolled-back `Time<GgrsTime>` for the duration
of `GgrsSchedule`, and ADR 0023 rule 2 states it outright — *"under fixed tick,
`Res<Time>` inside the tick is Bevy's fixed clock and is therefore deterministic;
this rule is about `std::time`"*. ⇒ **plain `Res<Time>` in a sim system is not a
defect.** The defects here were the blocking syscall and the `Local`. Do not go
hunting `Res<Time>` in the sim schedule on the strength of this entry.

⇒ Moved to `Update`. **Every reader of `WorldSourceHotReload` is a menu system in
`Update` already**, so this puts the writer in its readers' schedule, and the
watcher now also polls while the simulation is paused — which is what a
hot-reload watcher should do.

⛔⛔ **AND A VERIFICATION CLAIM I MADE HERE WAS NOT EVIDENCE.** I wrote *"verified
by `[census] membership`: 0 occurrences in `GgrsSchedule`, 1 in `Update`"*. The
`1 in Update` is real. The **`0 in GgrsSchedule` is absence of the ROW, not of the
system**: `report_schedule_membership` runs at `PreStartup` and filters to named
main-schedule labels, so it emits no `GgrsSchedule` row at all — a grep for one
can only ever return zero. Same family as the absence-grep trap: **a count of
zero from an instrument that never reports that category is not a measurement.**

⭐⭐ **SO THE INSTRUMENT WAS BUILT, AND THE CLAIM NOW HOLDS.**
`report_sim_schedule_membership` samples the sim schedule ONCE, after a session
has activated (it does not exist at `PreStartup`) and through the executable
fallback (its graph is drained by then). It matches BY NAME so `ambition_dev_tools`
takes no `bevy_ggrs` dependency. `[census] membership t=0.313 schedule=GgrsSchedule
systems=545` — the engine's most important schedule is enumerable for the first
time.

Re-verified against it, **with positive controls, which is the part the original
claim lacked**:

| system | in sim | expected |
|---|---|---|
| `poll_world_source_changes` | 0 | 0 — moved |
| `request_player_clone_on_key` | 0 | 0 — moved |
| `spawn_requested_player_clone` | **1** | 1 — correctly stayed |
| `derive_boss_sprite_metrics` | **1** | 1 — untouched control |

⇒ the two ones prove the instrument CAN report a non-zero for this category, so
the two zeros mean something. ⛔ Before believing a zero, ask the instrument for
a case you KNOW is present.

⛔⛔ **NO SPEED CLAIM, AND n=1 WOULD HAVE SUPPORTED A FALSE ONE.** `WorldPrep` in
`goblin_encounter` read **0.248ms** after the move against **0.269ms** before —
a tidy "7.8% win" from a single sample. Three samples each say otherwise:

| | samples | mean |
|---|---|---|
| before | 0.269, 0.273, 0.277 | 0.273ms |
| after | 0.248, 0.274, 0.258 | 0.260ms |

The ranges OVERLAP. ⇒ **not separable from noise on this host**, which is
expected: the stat is debounced to ~3Hz and this filesystem is fast. The payoff
is for slow mounts, Android storage and network shares — and for keeping blocking
IO off the deterministic tick. ⭐ This change is justified by ARCHITECTURE, not by
a measurement, and saying so is the point.

### ⭐⭐ FIGHTER COUNT IS A REAL COST DRIVER — the first product dimension that moves the frame

Every Smash number in this campaign until now used TWO fighters. Smash is a party
game; four is the real case. Measured 2026-08-29, 2500 ticks, interleaved reps:

| fighters | mean | p99 | cast held? |
|---|---|---|---|
| 2 | **4.51 / 4.50ms** | 5.72 / 5.74 | yes — reproducible to 0.01ms |
| 4 | **4.79 / 5.01ms** | 6.11 / 6.40 | ⚠ NO — 4 seats at start, 3 at end |

⇒ **+2 fighters costs AT LEAST +0.3ms (~7%)**, and it is a LOWER BOUND: the
4-fighter arm lost a fighter to a knockout partway through, so its mean averages
a cast that shrank. ⭐ This is the first thing in the whole campaign that scales
with something a PLAYER chooses.

⚠ **and a fighter is not "a body".** The room sweep priced `WorldPrep` at ~16us
per body; two extra fighters costing 0.3ms is roughly 10x that, because a fighter
carries a brain, a sprite rig and combat state, not just kinematics. ⛔ Do not
price a fighter with the per-body constant.

⭐⭐ **THE GUARD THAT CAUGHT THIS DID NOT EXIST AN HOUR EARLIER.**
`smash_match_profile` verified that seats EXISTED at the end, never that the
roster it was ASKED for actually seated, nor that the cast survived the measured
window. It now checks both, and the very next measurement tripped
`POPULATION CHANGED DURING MEASUREMENT: 4 seats at the start, 3 at the end`. ⇒ a
four-arm scaling table would otherwise have been published with one arm quietly
averaging two different matches. (An earlier reading of `bodies=3` for
`--fighters 4` is the same knockout seen through a different column — ⛔ `bodies`
counts `BodyKinematics`, and is NOT the fighter count.)

### ⭐⭐⭐ WHERE A FIGHTER'S COST GOES — and the engine's cost model in four numbers

1200-tick arms, both keeping their cast (`seats_at_end` 2 and 4, no population
warning), so this comparison is clean. Frame **4.58 → 4.83ms**, +0.25ms for two
fighters:

| phase | 2 fighters | 4 fighters | delta | share of the delta |
|---|---|---|---|---|
| `PreUpdate` | 2.033 | 2.137 | +0.104 | 42% |
| `PostUpdate` | 0.529 | 0.607 | **+0.078** | 31% |
| `Update` | 1.307 | 1.373 | +0.066 | 26% |
| `RunFixedMainLoop` | 0.348 | 0.363 | +0.015 | 6% |
| `StateTransition` | 0.169 | 0.154 | −0.015 | — |

⛔⛔ **A SECOND REP HALVED THE CONFIDENCE IN THE TOTAL, AND IT WAS PUBLISHED
BEFORE THE REP.** Repeating both arms (casts held, `seats_at_end` 2 and 4) gave
`PreUpdate` 1.963→2.095, `PostUpdate` 0.535→**0.693**, `Update` 1.246→1.375 — a
total delta of **+0.485ms** against rep 1's +0.25ms.

| | rep 1 | rep 2 |
|---|---|---|
| per-fighter cost | ~125us | **~240us** |
| `PostUpdate` share of the delta | 31% | 33% |
| `PreUpdate` share | 42% | 27% |

⭐⭐ **AND THE REASON IS GENERAL, WORTH KEEPING: ABSOLUTE PHASE COSTS HERE ARE
REPRODUCIBLE; DIFFERENCES BETWEEN ARMS ARE NOT.** Across every 2-fighter run
today `PreUpdate` landed in **1.96–2.03ms**, `Update` in **1.23–1.31**,
`PostUpdate` in **0.50–0.54** — tight. But the 2→4 DELTA of those same tight
numbers swung 2x, because **subtracting two noisy quantities amplifies the
relative error**: a ±0.04ms wobble on a 0.53ms phase is 8%, and on the 0.08ms
DIFFERENCE it is 50%.

⇒ **an absolute measurement here needs one careful run; a DELTA needs several.**
That is why the campaign's absolute findings (frame attribution, phase splits,
per-room `WorldPrep`) held up on re-measurement while this cross-arm delta did
not, and it is the rule to apply to any future A/B on this host.

⇒ **THE SHARES ARE STABLE; THE MAGNITUDE IS NOT.** A fighter costs **~125–240us**,
not the precise 125us this section first claimed, and `PostUpdate` reliably takes
about a THIRD of it. ⭐ The conclusion that survives is the SHAPE — a fighter is
paid roughly a third in presentation, which we do not author — not the headline
number.

The gameplay sim accounts for **+0.086ms** of it (`WorldPrep` +0.048, `Combat`
+0.015, `PlayerSimulation` +0.008). ⇒ **a fighter costs about a THIRD in
simulation, a THIRD in presentation (`PostUpdate`), and a QUARTER in `Update`** —
which is what a fighter IS: a multi-part sprite rig plus a brain plus combat
state. ⛔ Optimising only the sim addresses a third of it.

⭐⭐ **THE ENGINE'S COST MODEL, as measured by this campaign:**

| quantity | price | where |
|---|---|---|
| baseline frame, 2-fighter match | **~4.5ms** | ~630 systems, 2.9–15.6us each, NO hot one |
| **per FIGHTER** | **~125–240us** ⚠ 2 reps | ~1/3 `PostUpdate`; shares stabler than the total |
| per BODY | **~16us** | `WorldPrep` |
| per VISIBLE SPRITE | **~1.4us** | render extraction; 295 sprites = +0.36ms |
| a frame SPIKE | **+3.6ms** at the tail | ⛔ NOT the sim (+1.4% across quartiles) |

⇒ this is the sheet to price a feature against, and the reason the campaign
recommends retiring whole CLASSES rather than tuning systems: nothing on it is
dominated by any single system.

### ⭐⭐ SWEEPING THE SIM SCHEDULE FOR MORE OF THE SAME — 283 systems, 2026-08-29

The hot-reload fix was found BY ACCIDENT, so the sim schedule was swept
systematically (52 registration sites, 283 distinct systems) for blocking IO,
wall-clock time, state-carrying `Local`s, dev tooling, and non-deterministic
randomness.

⭐ **THE REASSURING HALF: BLOCKING IO — ZERO OTHER HITS.** `fs::`, `File::`,
`std::io`, `Command::`, sockets: none, directly or two calls deep.
`poll_world_source_changes` was the only one in the whole simulation.
**Non-deterministic randomness — ZERO hits** (`thread_rng`, `SystemTime::now`,
unseeded `Rng`); `Instant::now` appears exactly once. The trace recorder's disk
writes are already correctly parked in `PostUpdate`.

⛔ **FIXED: `request_player_clone_on_key`** (`app/plugins.rs`) — the structural
twin, a dev hotkey reading `ButtonInput<KeyCode>` in `WorldPrep` and chained into
a spawn. `ButtonInput` is winit frame state that is neither rollback-registered
nor rewound, so `just_pressed` reads true once per SIM RUN rather than once per
press: a frame stepping the sim twice spawns two clones. The key read moved to
`Update`; the SPAWN stayed in the sim, where it belongs.
`SpawnPlayerCloneRequest` was already the seam (tests poke it directly).

▢ **RECORDED, NOT FIXED — a latent class worth a decision.** Several systems pair
a non-rewinding `Local` edge-detector with state that IS rollback-registered, so a
rewind erases the effect while the stale `Local` prevents it being re-applied:
`track_room_visits` (`menu/map/systems.rs:13`) writing save flags;
`push_room_entered_quest_events` (`quest/mod.rs:13`) pushing `RoomEntered` into
the rollback-registered `QuestRegistry`; `despawn_bfs_particles_when_the_room_changes`
(`falling_sand.rs:265`); `advance_room_transition_content_epoch_system`
(`room_transition/loading.rs:102`); `sync_developer_body_profile`
(`dev_tools/editable.rs:555`) which mutates `BodyKinematics`. Also
`commit_ready_room_transition_system` (`room_transition/commit.rs:562`), the one
true `Instant::now()` on the tick — already guarded by an `is_rollback()` early
return, but by a runtime check rather than a structural one.

⚠ **ALL OF IT IS LATENT UNDER `check_distance: 0`**, which never rewinds. ⛔ These
are CORRECTNESS items for the day rollback goes live, not performance items, and
they touch quest and save logic — they want a deliberate decision, not a
drive-by fix from a performance campaign.

### ⭐ STARTUP, RE-MEASURED 2026-08-29 — 608ms, not 2.6s, for the windowless composition

Direction 6's 2.6s is a WINDOWED figure and carries window creation, render
pipeline init and shader compilation. The shipped windowless composition
(`smash_match_profile`) reports:

| phase | ms | share |
|---|---|---|
| app construction (plugin registration) | **377.1** | 62% |
| `after_load_data_handle` ⚠ see below | **169.2** | 28% |
| `startup_begin` | 47.2 | 8% |
| `before_audio_init` | 13.7 | 2% |
| **total before first frame** | **607.7** | |

⇒ plugin registration is still the majority, and at ~0.43ms per registration over
~876 systems it is Bevy's schedule-graph construction, not our code. ⛔ Reducing it
means having FEWER SYSTEMS, which is a capability decision, not an optimisation.

⛔⛔ **AND `after_load_data_handle` IS NOT "169ms OF LOADING" — I WROTE THAT AND IT
IS THE PHASE-ATTRIBUTION TRAP AGAIN, in the STARTUP profiler this time.** The
`Startup` chain between the two marks contains exactly ONE system,
`data::load_data_asset_handle`, and it is **one line**: `asset_server.load(path)`,
which returns a handle immediately, plus a `insert_resource`. It cannot cost
169ms. ⇒ the interval is real; the ATTRIBUTION is not.

⭐⭐ **AND THE MECHANISM IS STRUCTURAL, NOT MYSTERIOUS: THE MARKS ORDER THEMSELVES,
NOT THE SCHEDULE.** `Startup` holds **43 systems** (`[census] schedules`), and the
profiler chains only THREE of them — `startup_begin` → `load_data_asset_handle` →
`after_load_data_handle`. The other ~40 carry no ordering constraint against those
marks, so the single-threaded executor is free to run them BETWEEN the marks, and
whatever it runs there is billed to the interval. ⇒ **a `[startup]` phase measures
its unordered CO-RESIDENTS as much as the system it names.**

⛔ This is the same defect as bracketing an unordered set in `PreUpdate` — noted
earlier in this document as the reason NOT to bracket `PreUpdate` groups — and the
startup profiler has had it all along. A mark chain gives a well-defined interval
only where the schedule is `.chain()`ed end to end, which `Startup` is not.

⚠ These marks measure WALL TIME BETWEEN TWO POINTS, exactly like `[census] phases`.
The same rule applies: a phase name is where the time was BILLED, not what spent
it. I applied that scepticism to the frame census all campaign and then dropped
it for the startup profiler within the hour.

**Fixed while here: `run_headless` built the ENTIRE ROOM SET TWICE** — once to
check for an error, once to count the rooms — on every headless boot, which every
test and the RL harness pays. The `Result` already carries what the count needs.

⛔ **The measured win is small and is not the reason.** Steady-state headless boot
went **0.65-0.66s → 0.63s** (4 runs, all 0.63): about **20-25ms, ~3.5%**. The
room-set build is cheaper than the duplication suggested. ⇒ this change is
justified because building the same thing twice is WRONG, not because 25ms was
worth chasing — the same standing as the hot-reload move.

### ⭐⭐ INPUT LATENCY — the question the brief actually asked, and it is already right

This campaign measured frame TIME and frame TAILS throughout. **Responsiveness to a
fighting-game player is INPUT LATENCY** — press to visible response — which is a
different quantity, and nothing here had measured it.

**The architectural half is settled, and needs no change.** The hazard would be
the sim consuming LAST frame's input: if `run_ggrs_schedules` (in `PreUpdate`) ran
before Bevy's input systems, every press would cost a full extra frame. ⛔ A grep
finds no ordering constraint in THIS repo — which is true and misleading. The
constraint is declared **upstream, where it belongs**: `bevy_ggrs-0.21.0/src/lib.rs:255`
adds `RunGgrsSystems` `.after(InputSystems)` with the comment *"if we are in
PreUpdate, run after input is read"*, and documents it at line 155. ⇒ **input is
read and consumed in the SAME frame, guaranteed by construction.** ⛔ Do not add a
duplicate ordering here; it would be noise.

⛔⛔ **AND THE EMPIRICAL HALF CANNOT BE MEASURED ON THIS HOST — a real boundary,
not a gap in effort.** A headless probe cannot press a button through the DEVICE
path: the input latch is consulted only `if latch.is_device_authority()`, which is
false with no real device wired, so a synthesised `ButtonInput` press is dropped
by design (it protects harnesses that drive `PendingLocalInput` directly).
`drive_control_frame` injects input BELOW that path, so it measures the sim's
response and NOT device-to-sim latency. ⇒ **true input latency needs a real device
on real hardware**, and it joins the real-hardware item rather than standing as
its own lead.

### ⭐⭐⭐ WHO OWNS THE FRAME — the whole map, and it says where optimisation CAN'T help

`[census] owners_in` now runs for `PreUpdate`, `Update`, `PostUpdate` and the sim.

**`PreUpdate` — 137 systems, 23 crates** (this is the 0.93ms that is neither the
sim nor the rollback driver): `bevy_asset` 33 · `bevy_falling_sand` 19 ·
`bevy_egui` 17 · `<unnamed>` (closures) 9 · `bevy_picking` 9 ·
`ambition_platformer2d_runtime` 7 · `bevy_input` 7 · `leafwing_input_manager` 6 ·
`ambition_platformer2d_rollback_ggrs` 5. ⇒ **our own crates own roughly 20 of
137.** The 0.93ms is overwhelmingly THIRD-PARTY, which is consistent with the
earlier probe finding its two largest groups (asset trackers, ui-focus+picking)
worth 0.00 and <=0.145ms.

**`PostUpdate` — 169 systems, 29 crates**: `bevy_asset` 31 · `bevy_lunex` 24 ·
`bevy_ui` 12 · `bevy_falling_sand` 11 · `bevy_light` 10 · `bevy_app` 8 ·
`bevy_sprite_render` 8 · `bevy_camera` 7 · … and
**`ambition_platformer2d_actor_monolith` 3**. ⇒ **our code owns about FIVE of
169.**

⭐⭐ **AND THAT IS THE LOAD-BEARING FACT: `PostUpdate` IS 31% OF WHAT AN ADDED
FIGHTER COSTS, AND IT IS ALMOST ENTIRELY BEVY'S.** Transform propagation,
visibility, sprite extraction, UI layout — a fighter's presentation cost is paid
inside the engine's pipeline, doing work proportional to the entities we hand it.
⇒ **that third of a fighter cannot be optimised by editing our own code.** It can
only be reduced by handing the pipeline FEWER OR SIMPLER ENTITIES, which is a
content and rig decision, not an engine one.

⚠ **THE OBVIOUS NEXT QUESTION — "how many entities IS a fighter?" — DOES NOT
SURVIVE ITS OWN NOISE FLOOR, and nearly got published anyway.** Total live
entities read **1297 at 2 fighters and 1337 at 4**, both casts held: +40 for +2,
a tidy "20 entities per fighter". ⛔ But `live` was earlier observed fluctuating
**1297–1336 WITHIN A SINGLE 2-fighter run** as VFX spawn and despawn. **The
between-arm delta is smaller than one arm's own variance**, so the number means
nothing.

⛔ And `[census] populations` cannot rescue it: it ranks by count, and a 2–4
fighter rig is nowhere near the top 20 in a world holding 1024 sand chunks and 96
UI nodes. **A census ranked by population is blind to the few entities that matter
most in a fighting game.**

⭐⭐ **ANSWERED BY MOVING THE SAMPLE, NOT BY ADDING A QUERY: A FIGHTER IS 8
ENTITIES.** The noise is COMBAT VFX, so the measurement was moved to the quiet
moment the tool already detects — the instant the round goes live, before any
combat. `smash_match_profile` now reports `entities_at_go_live`:

| fighters | entities at go-live | reps |
|---|---|---|
| 2 | **1297** | 1297, 1297 |
| 4 | **1313** | 1313, 1313 |

**Zero variance across reps**, against ±40 later in the same run. ⇒ +16 entities
for +2 fighters = **8 entities per fighter**.

**A fighter is also exactly ONE SPRITE** — `sprites_at_go_live` reads 25 at two
fighters and 27 at four. So a fighter is **8 entities and 1 sprite**.

⛔ **A CORRECTION TO MY OWN INFERENCE, MADE AN HOUR EARLIER IN THIS SECTION.** I
wrote that 8 entities "cannot account for the ~39us of `PostUpdate` a fighter
adds". **They can**: 39us over 8 entities is **~4.9us per fighter entity**, which
is an ordinary per-entity cost for a pipeline of 169 systems. The refutation was
too strong and is withdrawn.

⭐ **THE CORRECT READING: FIGHTER ENTITIES ARE EXPENSIVE, NOT NUMEROUS.** Eight
entities and one sprite is already a lean rig, so:
- ⛔ "hand the pipeline fewer entities" has almost no room — 8 is small;
- ⛔ "draw fewer sprites" has none at all — 1 is the floor;
- ⇒ the only presentation lever left is making EACH fighter entity cheaper —
  shallower transform hierarchy, fewer components, less change-detection churn —
  and that is Bevy's per-entity pipeline cost, which the ownership map already
  showed we do not author.

⚠ **A comparison NOT worth making, and why:** the world averages 0.51ms of
`PostUpdate` over ~1297 entities = 0.39us each, which would make a fighter entity
look 12x average. ⛔ That ratio is junk — ~1024 of those entities are falling-sand
chunks carrying no transform or visibility, so they dilute the denominator without
ever entering the pipeline. Comparing against a world average is only valid when
the population is homogeneous, and this one is not.

⭐ Method worth keeping: **when the noise is caused by an ACTIVITY, sample before
the activity starts** — that beat both a bigger sample and a new query.

⇒ combined with the sim table below, the frame's ownership is now fully mapped:
**the SIM is ours (545 systems, monolith 30%); `PreUpdate` and `PostUpdate` are
mostly the engine's.** Optimisation effort should go where authorship is.

### ⭐⭐ WHO OWNS THE SIMULATION — 545 systems, 29 crates, first measured 2026-08-29

Possible only once the sim schedule became enumerable. `[census] owners_in
schedule=GgrsSchedule`:

| crate | systems | share |
|---|---|---|
| **`ambition_platformer2d_actor_monolith`** | **162** | **30%** |
| `ambition_content` | 62 | 11% |
| `ambition_combat` | 52 | 10% |
| `ambition_demo_mary_o` | 33 | 6% |
| `ambition_sim_view` | 30 | 6% |
| `ambition_demo_sanic` | 25 | 5% |
| `ambition_platformer2d_runtime` | 25 | 5% |
| `ambition_dev_tools` | 22 | 4% |
| `ambition_demo_smash` | 20 | 4% |
| `ambition_boss_encounter` | 18 | 3% |

⭐ **THE MONOLITH IS 30% OF THE SIMULATION**, which turns "it is big" into a
number the decomposition can be planned against. (For contrast, `Update`'s 497
systems spread over 46 crates, led by `ambition_render` at 99.)

⚠ **63 sim systems — 12% — belong to experiences a Smash match is not**
(`mary_o` 33, `sanic` 25, `twintrack` 5). ⛔ **This is NOT a performance finding
and must not be sold as one:** removing four whole experiences was measured
earlier in this campaign and moved neither frame time NOR startup registration.
⇒ they already pay proportionally to what they do, which is what the architecture
promises. It is an OWNERSHIP datum, not a cost one.

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

### ⭐⭐ THE SIM TICK, SPLIT — and the gameplay phases are not the problem

`[census] sim_phases` (new) stands a boundary after each phase of the sim chain,
which `[census] phases` cannot do because in this app the whole sim is ONE
exclusive system inside `PreUpdate`. Smash match, 2 fighters, this host, `dev`:

```text
[census] sim_phases ticks=41 WorldPrep=0.222 Combat=0.204 PlayerInput=0.140
    PlayerSimulation=0.118 Trace=0.090 Progression=0.060 RoomTransition=0.031
    GameplayEffects=0.018 FeatureInteraction=0.013 PresentationSync=0.008
    ResetProcessing=0.007 FeatureCollection=0.005 EncounterSimulation=0.005
    PresentationVisualSync=0.003 Cutscene=0.002 LdtkRuntimeSpine=0.001
    FeatureViewSync=0.000
[census] phases     PreUpdate=2.119 Update=1.399 PostUpdate=0.649
                    RunFixedMainLoop=0.427 StateTransition=0.202
```

⭐⭐⭐ **THE SEVENTEEN GAMEPLAY PHASES SUM TO ~0.93ms OF A 2.12ms `PreUpdate`.
ROUGHLY 1.2ms — MORE THAN HALF — IS IN NO SIM PHASE AT ALL.**

⇒ **the gameplay simulation is not what makes a Smash frame expensive.** The
biggest gameplay phase, `WorldPrep`, is 0.22ms; `Combat` is 0.20ms. Optimizing
any of them chases a tenth of the frame at best.

⚠ WHAT THE MISSING 1.2ms COULD BE, none of it yet measured — do not pick one and
believe it:
- the `ReadInputs` schedule, which `run_ggrs_schedules` runs BEFORE the advance;
- ggrs's own `advance_frame` bookkeeping and request dispatch;
- systems registered into the sim schedule OUTSIDE the phase chain, which no
  boundary brackets and whose time therefore lands nowhere;
- the DefaultPlugins `PreUpdate` population — `bevy_ui::ui_focus_system`,
  leafwing input, picking, asset events — which sits in `PreUpdate` but OUTSIDE
  `run_ggrs_schedules` entirely.

⭐ The next instrument is a boundary immediately inside and immediately outside
`run_ggrs_schedules`, which splits that 1.2ms between "the GGRS driver" and "the
rest of PreUpdate" in one measurement. That is the question worth answering next.

⛔ TWO INSTRUMENT FAILURES ON THE WAY TO THIS NUMBER, both caught by
implausibility rather than by a test:
1. the first `sim_phases` reading said `PlayerInput=3.96ms` inside a sim tick the
   main census put at 1.98ms TOTAL. A closing boundary attributes "now minus the
   previous boundary", and with no OPENING mark the first bucket absorbed
   everything between the previous tick's last phase and this tick's first — the
   whole rest of the frame. ⇒ `open_sim_phase_window` runs `.before(PlayerInput)`
   and attributes nothing.
2. the condition census first reported `system_conditions=0` beside
   `systems=886` (see I3).
⇒ **a boundary instrument's first output must be checked arithmetically against
a coarser measurement that already exists.** Both lies were caught only because
one did.

### ⭐⭐⭐ THE FRAME, FULLY ATTRIBUTED — and the answer is HOST SERVICES

`[census] ggrs_driver` brackets `RunGgrsSystems` from outside, which closes the
last gap. Smash match, 2 fighters, this host, `dev`, steady state:

```text
PreUpdate                2.14 ms
  ggrs driver            1.19 ms
    sim phases (17)      0.93 ms
    driver overhead      0.26 ms   ReadInputs + ggrs bookkeeping + out-of-chain sim systems
  OUTSIDE the driver     0.95 ms   DefaultPlugins: ui focus, leafwing input, picking, asset events
```

⭐⭐ **ABOUT 0.95ms OF A 2.14ms `PreUpdate` — ROUGHLY 20% OF THE WHOLE 4.7ms
FRAME — IS NOT THE SIMULATION.** It is the `DefaultPlugins` `PreUpdate`
population running every frame of a HEADLESS match with no window, no pointer
and no gamepad.

⇒ **this is the brief's direction 5 (host services vs game runtime), and it is
the largest attributable cost the campaign has found.** It is also the shape the
brief predicted: machinery that is installed rather than active.

⚠ CAVEATS, because this number will be quoted: the first 300-frame bucket reads
1.92ms because it includes startup and the opening ceremony — 1.19ms is the
steady-state figure. `dev` profile on this host. And a HEADLESS run is exactly
where this population should cost LEAST; a windowed run has real pointers and
real focus work, so the desktop figure could be larger, not smaller.

⛔ WHAT THIS DOES NOT SAY: which of ui focus / leafwing / picking / asset events
owns the 0.95ms. That needs one more bracket, or Tracy for attribution only.
⛔ And it does NOT say the work is removable — `bevy_ui` focus and input are load
bearing when a window exists. The question direction 5 asks is whether a
composition that has no window should be installing them at all.

### `PreUpdate` HAS 137 SYSTEMS, AND MOST OF THEM ARE FOR THINGS A MATCH IS NOT DOING

`[census] membership` names every system in a schedule. It prints names rather
than timing them on purpose: timing would need this crate to depend on
`bevy_ui`, `bevy_picking` and leafwing purely to name their sets, which is an
instrument joining the population it measures.

⛔ **AND IT CORRECTS THE "NINE SYSTEMS" FIGURE.** That came from the MinimalPlugins
sandbox. The SHIPPED app's `PreUpdate` holds **137**:

| group | count | examples |
|---|---|---|
| `Assets::<A>::track_assets` | **31** | one per registered asset type |
| falling-sand particles | ~20 | `msgr_spawn_particle`, `advance_chunk_dirty_state`, `update_chunk_loading`, `sync_particle_type_registry` |
| picking backends | ~14 | `cube_3d_picking`, `lunex_2d_picking`, `ui_picking`, `generate_hovermap`, `update_window_hits`, `system_cursor_*` |
| raw input | ~15 | keyboard, mouse, gamepad, gilrs, touch, IME, file-drag-drop |
| leafwing | 7 | `tick_action_state` x2, `update_action_state` x2, `update_input` |
| `release_confirmed_effects` | 7 | one per message type |
| egui | 4 | `setup_primary_egui_context_system`, `write_egui_input_system` |
| bevy_ui | 3 | `ui_focus_system`, `update_ui_size_and_scale_system` |

⭐⭐ **egui AND THREE PICKING BACKENDS RUN EVERY FRAME OF A HEADLESS MATCH** with
no window and no pointer. That is directions 1 and 5 of the brief with names
attached, and it is where the 0.95ms lives.

⛔⛔ **AND IT CORRECTS I10, WHICH WAS MINE.** I recorded falling sand as "not a
performance lever" because shrinking `with_map_size(32)` to `2` removed 1536
chunk entities and changed nothing. But that removed the CHUNKS, not the ~20
falling-sand systems in `PreUpdate` — which kept running over an empty particle
set. ⇒ the chunk experiment tested the wrong half, and whether those systems cost
anything is STILL OPEN. ⚠ this is the same error twice in one campaign: assuming
the population is the cost when the SYSTEMS are the thing that runs.

⚠ NOT YET MEASURED: which group owns the 0.95ms. The list makes a targeted
bracket cheap, and the honest order is largest-group-first — but 31 `track_assets`
systems doing nothing may cost less than 4 egui systems doing something.

### ⭐ DEVELOPER TOOLING IS OPTIMIZED, NOT STRIPPED

Jon, 2026-08-29: *"developer tooling should be optimized too"*. That settles a
question this campaign had been circling and rules out a whole class of
"solution":

⛔ **NOT** "measure a runtime without dev tooling and call that the real number".
⛔ **NOT** "move it behind a ship gate and stop caring what it costs".
⭐ **YES** "these systems stay available and get cheaper" — the same standard the
brief sets for every other capability: absent costs nothing, dormant costs
little, active costs something attributable.

⚠ AND THE MEASUREMENT PROBLEM IS REAL (see I12): a cargo feature is NOT a clean
A/B for runtime cost, because toggling one changes feature unification across
the dependency graph. Pricing any of this honestly means disabling the systems
AT RUNTIME inside ONE build.

Known dev-host costs, none yet priced this way:
- `record_actor_oob_frame_system` (40.5us/frame in the 08-28 trace) and
  `record_frame_system` (37.9us) — together they outweighed `tick_actor_brains`;
- `poll_world_source_changes` — its per-frame change announcement is FIXED
  (`6e6a5ce12`), but its `fs::metadata` is still a blocking main-thread syscall,
  3.9ms worst case on virtiofs, now debounced to ~3Hz;
- four egui systems and the inspector plugins in `PreUpdate`;
- the census family itself, which is why every one of its rows is registered
  only when the census is switched on.

⭐ The recorders are the interesting ones. ⛔ **BUT "their cost should scale with
BODIES, not frames" IS SUPERSEDED** — measured later the same day, the `Trace`
phase scales with `players` rather than `bodies`: ~0.13ms whenever a primary
player exists plus ~1.16us per body, so a Smash match pays 0.015ms and a 130-body
gallery pays 0.281ms. The instinct was right — it already scales with something —
but the quantity named here is the wrong one.

### ▢ D-PERF-6 — the flight recorder should cost BODIES, not SOLIDS

Designed 2026-08-29, deliberately NOT taken, with the blocker named so the next
session starts from the analysis rather than the symptom.

`record_actor_oob_frame_system` builds, EVERY FRAME, up to 64
`CollisionTraceShape` from the room's solid geometry — each with a `name.clone()`
and an aabb conversion — plus per-body string clones. Its own doc comment states
the rule it violates: *"The same set every frame for a static room, so the
markdown only renders the latest."* The code knows the set is identical and
rebuilds it anyway, and the only reader takes `latest.solids`.

⛔ **CACHING IT DOES NOT WORK, which is why this is deferred rather than done.**
A `Local` cache rebuilt on room change still has to put a `Vec` on each frame,
and `Vec<CollisionTraceShape>::clone()` allocates the same ~128 `String`s that
building it did. The win requires SHARING, not caching.

Two roads, both real work:
- `Arc<Vec<CollisionTraceShape>>` on the frame — ⛔ blocked: serde is declared
  workspace-wide as `features = ["derive"]` with no `"rc"`, so `Arc` will not
  serialize, and adding it is a workspace-wide dependency change for a dev-tool
  optimization;
- move `solids` off the FRAME and onto the BUFFER, captured when the room
  changes — the honest fix, since the geometry is a property of the room and not
  of the frame, and it matches what the reader already does. ⚠ it changes the
  dump format, so it needs the dump reader updated in the same slice.

⚠ SIZE IT BEFORE BUILDING IT. The 08-28 trace put this recorder at 40.5us/frame
and its sibling at 37.9us — together ~78us, about 2.4% of a 3.2ms `profiling`
frame, and BELOW this host's ~15% run-to-run noise. ⇒ it will not show up in a
frame-time A/B, and the case for doing it is allocation pressure and elegance,
not a millisecond. ⭐ Jon's standard applies: the recorder stays fully
available, it just stops paying per frame for something that changes per room.

### ⭐⭐⭐ THE CAMPAIGN'S CONCLUSION: THE FRAME IS BROAD, NOT DEEP

The shipped app's schedule shape, measured 2026-08-29 — ⛔ note `Update` is
**494**, not the 822 every planning doc quotes. That figure came from the SANDBOX
composition, where `SimulationHost` defaults to `RenderFrame` and the sim itself
runs in `Update`. In the shipped app the sim is `GgrsSchedule`.

| schedule | systems | cost | per system |
|---|---|---|---|
| `PreUpdate` | 137 | 2.14ms | ~15.6us |
| `Update` | **494** | 1.42ms | ~2.9us |
| `GgrsSchedule` (the sim) | 236 registration sites | 0.93ms over 17 phases | — |

**What each holds:**

- `PreUpdate` — the GGRS driver (containing the WHOLE sim), plus 31
  `track_assets`, ~20 falling-sand, ~14 picking across THREE backends, ~15 raw
  input, 7 leafwing, 4 egui, 3 bevy_ui;
- `Update` — a long FLAT tail of presentation, almost all one instance each:
  `update_player_hud`, `update_quest_panel`, `update_speech_bubbles`,
  `update_worldlines`, `update_spacetime_camera`, `upgrade_actor_sprites`,
  `update_rollback_proof_hud`, `update_split_observer_panes`. Only four names
  repeat at all (`prune_narrative_inputs` x9, `install` x7,
  `system_state_pipe_into_manager` x5, `publish_bevy_ui_menu_previews` x4).

⭐⭐ **THERE IS NO HOT SYSTEM. THERE IS A BROAD POPULATION.** ~630 systems across
two schedules, at 2.9-15.6us each, most of them belonging to capabilities a Smash
match is not using — quest panels, speech bubbles, worldlines, spacetime
cameras, split observer panes, three picking backends, egui, a sand grid.

⇒ **THIS IS WHY CAPABILITY ACTIVATION IS THE RIGHT ARCHITECTURAL ANSWER, and the
reason is not the one the briefs gave.** Not because any system is slow — none
is. Because the population is broad and mostly dormant-but-running, so the only
lever with leverage is one that removes WHOLE GROUPS from the frame rather than
making any member faster.

⛔ AND IT IS WHY THE MICRO-OPTIMIZATION DIRECTIONS ALL FAILED HERE. Nine
hypotheses rejected this campaign (⚠ the authoritative tally is the Investigations
table at the end — this sentence's count went stale as the table grew), and the
pattern behind every one is the same:
each targeted a single expensive thing, and there is no single expensive thing.
Entity populations (three times), archetype fragmentation, rollback snapshots,
multi-render, falling sand — all null. The one change that DID move a number
(`gameplay_allowed`, 83 evaluations to 1) worked because it retired a whole
class at once.

⭐⭐ **THE LIMIT IS NOW TESTED, AND THE MECHANISM HOLDS.** Gating
`GameplaySimulationRoot` to `run_if(|| false)` — 17 phases measured at 0.93ms —
took `[census] sim_phases` to 0.06ms and `[census] ggrs_driver` from 1.19ms to
**0.240ms**. The driver shed 0.95ms against the 0.93ms the phases were
independently measured at. ⇒ **a false set condition reclaims the systems' own
work**, so capability gating buys what the systems cost, not merely their
scheduling.

⭐ AND TWO INSTRUMENTS AGREED FOR THE FIRST TIME: the residual driver cost with
the sim gated off, 0.240ms, matches the 0.26ms computed as "driver overhead in no
sim phase" from the entirely separate `PreUpdate − driver − sim_phases`
arithmetic. That is `ReadInputs` plus ggrs bookkeeping, measured twice by
different means.

⛔ THE FRAME NUMBER FROM THAT PROBE IS CONFOUNDED AND IS NOT USED. Gating the sim
means no match ever starts — the scenario aborted with *"the opening ceremony
never released the cast"*, which is the premise check doing its job — so its
3.17ms is "no match at all", not "this match minus the sim". Only the driver and
phase numbers are clean, because those measure the gated region itself.

⚠ REMAINING LIMIT: this calibrates the MECHANISM on one large set. It does not
say what any particular dormant capability costs — the systems in the groups
named above still have to be measured before anyone gates them. Bevy skips a set whose condition is false cheaply, so the
saving is the systems' own work, not their scheduling — and this campaign has
been wrong three times about what a population costs. ⇒ the first capability
gate should be measured on ONE group before the pattern is generalized.

### THE WHOLE FRAME, ACCOUNTED — and only ONE phase is substantially OURS

| phase | ms | share | whose, and what |
|---|---|---|---|
| `PreUpdate` | 2.14 | 45% | the GGRS driver 1.19 (**our sim** 0.93 + `ReadInputs`/ggrs bookkeeping 0.26) + **0.95 diffuse Bevy** (asset trackers, raw input, leafwing, egui — two gates proved no group in it is a lever) |
| `Update` | 1.42 | 30% | **OURS** — 494 systems, a flat tail of presentation |
| `PostUpdate` | 0.65 | 14% | **BEVY'S RENDER PIPELINE** — see below |
| `RunFixedMainLoop` | 0.40 | 8% | not yet examined |
| `StateTransition` | 0.14 | 3% | Bevy's per-state machinery, already ruled |

⭐ `PostUpdate` NAMED: 31 `Assets` event systems, 8
`check_entities_needing_specialization`, four `system_fetch_dimension_from_camera`
+ four `system_touch_camera_if_fetch_added`, then `visibility_propagate_system`,
`update_ui`, `update_text2d_layout`,
`mark_meshes_as_changed_if_their_materials_changed`, `detect_text_needs_rerender`
— and `update_spot_light_frusta`, `update_point_light_frusta`,
`validate_shadow_map_size`, which are 3D LIGHTING SYSTEMS IN A 2D FIGHTING GAME.

⇒ ⛔ **`PostUpdate` IS NOT OUR LEVER EITHER.** It is Bevy's render/UI pipeline
almost end to end, the same verdict the DefaultPlugins block got in `PreUpdate`.
⚠ And it relocates the brief's direction 4: if our presentation projection
rewrites unchanged state, it shows in **`Update`** where our `rebuild_*` systems
live, NOT in `PostUpdate` where the render extraction runs.

⭐⭐ **SO THE ONE PHASE THAT IS BOTH SUBSTANTIALLY OURS AND STILL UNATTRIBUTED IS
`Update`: 1.42ms over 494 presentation systems.** That is the campaign's next
measurement and the last place a lever can be hiding.

### ⛔⛔⛔ CAPABILITY ACTIVATION WILL NOT MAKE SMASH FASTER — measured

The decisive probe for the direction both briefs rank FIRST. Rather than gate
seven capabilities one at a time and collect seven nulls, remove **every
non-Smash experience at once** — Sanic, Mary-O, Pocket and Twintrack, their
whole registration, far more than the ~77 `Update` systems they contribute:

| | baseline | all four experiences removed |
|---|---|---|
| frame mean | 4.5–5.0ms | **4.40–4.67ms** |
| `ggrs_driver` | 1.19ms | 1.085–1.104ms |
| `seats_at_end` | 2 | **2** — a real match, not confounded |

**Within noise.** ⇒ **RUNTIME CAPABILITY ACTIVATION IS ARCHITECTURE-ONLY.** It is
worth doing for composition clarity, ownership and startup cost. It is worth
approximately NOTHING for frame time, and nobody should fund it as a performance
migration.

⛔ THIS RETIRES THE BRIEFS' TOP-RANKED ITEM ON EVIDENCE. The GPT review called
runtime capability composition *"the biggest one"*, and the second brief made it
direction 1. Both rested on the reading that inactive Sanic/Smash/Mary-O
machinery *participates in* a frame. It does not, twice over: their gameplay
rules are TUPLE-GATED (collective in Bevy 0.18, ~4 evaluations each, corrected
earlier), and their presentation systems — measured here, in aggregate, at the
upper bound — cost nothing recoverable.

⭐ THE ARITHMETIC SAID SO IN ADVANCE, and it is the general lesson: ~77 systems
at `Update`'s measured ~2.9us each is ~0.22ms, about 5% of frame, BELOW this
machine's ~15% noise floor. ⇒ **on a frame this diffuse, no group of fewer than
~500 systems can produce a measurable win.** Size the group against the noise
floor BEFORE building the gate, every time.

⭐⭐ **AND STARTUP HAS NOW BEEN PRICED TOO — IT DOES NOT MOVE EITHER.** The same
removal, measured against a startup baseline (the profile binary now calls
`note_process_start()`, which it never did, so its report used to begin
mid-plugin-build and SAID SO):

| | baseline | four experiences removed |
|---|---|---|
| plugin registration | 372.3ms | **380.8ms** |
| asset load | 184.4ms | 178.8ms |
| `Update` systems | 494 | **433** (−61, crates 46 → 38) |

The removal certainly took — 61 systems and 8 crates gone, twintrack and mary_o
absent from `[census] owners_in`. Plugin registration did not drop.

⇒ ⛔⛔ **THE DIRECTION IS CLOSED ON BOTH QUANTITIES.** Capability composition
buys nothing for frame time and nothing for startup registration. It remains
worth doing for composition clarity, cost ownership and dependency hygiene —
which are real goods — but it must be argued on those grounds, with NO
performance claim attached.

⚠ The startup shape is still worth knowing: registration is **372ms, 60% of a
~619ms startup**, against per-frame costs in microseconds. Installing ~630
systems is genuinely expensive ONCE. But it is dominated by the engine core and
the content plugins every composition needs — not by the experiences, which is
what removing four of them just demonstrated.

### WHO OWNS `Update` — ⛔ NOT a target list; see the sizing rule below

⛔⛔ **THIS SECTION WAS WRITTEN AS "the target list for capability gating" AND THAT
CONTRADICTS THIS DOCUMENT'S OWN ARITHMETIC.** The groups it names are 27, 25, 10,
8 and 7 systems, while the measured rule is that **on a frame this diffuse no
group of fewer than ~500 systems can produce a measurable win** — the noise floor
is ~10–15%. ⇒ read the list as OWNERSHIP (who is asking the frame for work), not
as a work queue. ⭐ If one is gated anyway, size it against the noise floor FIRST
and expect a null.

#### The ownership breakdown

`[census] owners_in` breaks one schedule down by the crate owning each system.
Smash match, shipped composition:

```text
schedule=Update systems=494 crates=46
  ambition_render=99  ambition_app=53  actor_monolith=42  demo_twintrack=39
  game_shell=30  content=20  demo_smash=17  touch_input=16  menu=15
  platformer2d_host=15  load_presentation=14  conversation=12  bevy_lunex=11
  demo_mary_o=10  portal2d_presentation=8  bevy_yarnspinner=7
  menu_kaleidoscope=7  platformer2d_provider=7  audio=6  dialog=6
```

⭐⭐ **`ambition_demo_twintrack` OWNS 39 SYSTEMS IN `Update` DURING A SMASH
MATCH.** Twelve are now gated (`spacetime_3d`); 27 remain. And it is not alone —
the population belonging to capabilities this match provably is not using:

| dormant here | systems | how we know it is dormant |
|---|---|---|
| twintrack, after the `spacetime_3d` gate | 27 | no `TwinTrackExperiment` exists |
| conversation + dialog + yarnspinner | 25 | no conversation is running |
| demo_mary_o | 10 | not the active mode |
| portal2d_presentation | 8 | `[census] portal` measured `rigs=0 active=0` |
| menu_kaleidoscope | 7 | not on a menu |
| **≈** | **~77** | ~16% of `Update`'s population |

⛔ **AND A COUNT IS STILL NOT A COST** — this is the target list for measurement,
not a claim. The two largest groups in `PreUpdate` were worth nothing between
them, and three separate entity-population leads came back null. Each of these
gets gated and measured on its own; the ones that early-return cheaply will show
nothing and are architectural tidying, and any that does REAL work while dormant
is the measurable win.

⚠ `ambition_render=99` is the largest owner and is NOT dormant — a match does
render. If our presentation projection rewrites semantically unchanged state
(the brief's direction 4), those 99 are where it lives, and that is a different
investigation from gating.

### ▢ THE SPRITE SCALING CURVE — the knob exists, the CULLED path is free, the VISIBLE one is still unmeasured

`smash_match_profile --sprites N` spawns N plain sprites into a live match on the
real stack: same app, same schedules, same render path, one dimension varied.
Deliberately plain (`Sprite` + `Transform` + `Visibility`, no gameplay
components), spawned AFTER the round goes live (earlier and the session teardown
between lobby and stage sweeps them), on a deterministic grid, one shared colour
and no texture — so it measures the per-sprite path and explicitly NOT batch
breaking, which needs its own knob.

```text
sprites=0     mean 4.52ms   total 32     visible 5
sprites=250   mean 4.58ms   total 281    visible 7
sprites=1000  mean 4.70ms   total 1025   visible 7
```

⭐ **A THOUSAND CULLED SPRITES COST NOTHING MEASURABLE** — 4.52 → 4.70ms across a
32× population increase, inside the 13% noise band. Bevy's culling is doing its
job and non-visible sprites are not a cost in this engine.

⛔⛔ **BUT `sprites_visible` NEVER LEAVES 5–7, SO THE QUESTION THE CAMPAIGN WAS
OPENED ABOUT IS STILL UNANSWERED.** The first version grid-placed around the
world origin and culled all thousand; the second anchored on a sprite the camera
already draws and STILL culled them — the camera census reports `Main Camera
layers=0+2+5` beside a `Front HUD Camera layers=1`, so the anchor almost
certainly landed on a HUD sprite in screen space rather than a world one.

⛔⛔ **AND IT IS NOT A PLACEMENT PROBLEM. FOUR ATTEMPTS SETTLED THAT.** Origin
grid: culled. Anchored on the first entity with `ViewVisibility`: culled — that
matched a HUD sprite in SCREEN space, because presentation entities are the only
ones carrying visibility at all. Anchored on `(&MatchSeat, &GlobalTransform)`:
matched NOTHING and tripped the abort, because **this engine splits simulation
from presentation** — the sim body carries `BodyKinematics` and the `Transform`
lives on a separate projected entity. Anchored on the fighter's actual
`BodyKinematics.pos`: **still `sprites_visible=5` with 1025 sprites present.**

⇒ ⭐⭐ **THE FINDING IS ARCHITECTURAL: A RAW SPRITE IS NOT DRAWABLE IN THIS
COMPOSITION.** The census reports `per_view_projections=6`; the engine's sprites
are PROJECTED PER VIEW, not rendered from world entities directly. A sprite that
is not part of that projection never becomes visible no matter where it is put.

⇒ ▢ **A synthetic sprite benchmark here must go THROUGH the presentation
projection, not around it** — spawn whatever the projection consumes and let it
produce the render entities, or drive a real room that already contains hundreds.
That is a different piece of work from a `--sprites N` knob, and it is the honest
next step for the hundreds-of-sprites question.

⭐ The abort guard earned its place twice: it turned a silent flat curve into an
explicit failure naming exactly what was missing, which made the third diagnosis
a two-minute read instead of another speculative run.

⭐⭐ **THE METHOD LESSON, THIRD INSTANCE TODAY.** Every one of these was caught by
a SECONDARY number in the same census row while the primary looked fine:
`--no-default-features` silently not disabling falling sand (caught by
`ChunkRegion=1024` still present); a probe sampling 0.72s of startup (caught by
`max=40.66ms` and a missing driver row); a thousand sprites spawned and culled
(caught by `sprites_visible`). ⇒ **a census must report POPULATION beside
TIMING.** Timing alone is exactly as convincing when it is wrong.

### ⭐⭐ A REAL ROOM IS REACHABLE AFTER ALL — via `capture_scene`, not the headless path

The "every headless room is a two-body world" finding was about ONE CODE PATH.
`run_game.sh sandbox --headless` goes through `cli_direct_entry` → `run_headless`,
which builds `MinimalPlugins` with the sim in `Update`. `capture_scene` uses the
PRODUCTION composition and camera policy — and it loads real content:

```text
capture_scene central_hub_complex player
  draws   sprites=151  sprites_visible=46  text2d=46  per_view_projections=18
  churn   transforms=2515  transforms_changed=55  sprites=151  sprites_changed=32
  ecs     entities=4096  archetypes=350  bodies=2  players=1
  frame   mean=19.88ms  p50=16.80  p95=31.89
```

Against a Smash match: `sprites_visible=5`, `transforms=59`,
`per_view_projections=6`, `entities=2048`. ⇒ **a real room is 42x the transforms
and 3x the per-view projections**, and it is the first populated, actually-drawing
scene this campaign has measured.

⭐⭐ **AND THE CHURN QUESTION IS ANSWERED ON REAL CONTENT: 55 of 2515 transforms
changed, 32 of 151 sprites.** The presentation projection is NOT rewriting
semantically unchanged state. The brief's direction 4 is acquitted on a workload
that can actually test it — ⚠ noting `Changed<T>` is set by any `DerefMut`, so a
LOW number is a real acquittal (a projection writing identical values would show
as changed).

⛔⛔ **DO NOT COMPARE THE 19.88ms TO THE SMASH FRAME.** This host has NO GPU, so
`capture_scene` rasterizes in SOFTWARE, while the Smash runs are `NoWindow`.
Different rendering modes are not comparable — that is the campaign's own rule
and the reason the history's comparability key separates them. The 19.88ms says
what software rasterization of this room costs; it says nothing about what a
player's machine does.

⇒ ▢ **THE VEHICLE FOR THE SPRITE QUESTION IS `capture_scene`, NOT A SYNTHETIC
KNOB.** It already produces a populated visible scene through the real
projection, which four attempts at spawning raw sprites could not. Whoever picks
this up should vary ROOMS (or content within one) under `capture_scene` on a GPU
machine and read `sprites_visible` beside the frame — the population is real, the
projection is real, and the census rows already exist.

### ⭐⭐ NO SHIPPED ROOM HAS HUNDREDS OF VISIBLE SPRITES — the founding premise, measured

Four real rooms through `capture_scene`:

| room | sprites | **visible** | text2d | per-view projections | entities |
|---|---|---|---|---|---|
| `central_hub_complex` | 151 | **46** | 46 | 18 | 4096 |
| `you_have_to_cut_the_rope` | 79 | **37** | 12 | 8 | 2048 |
| `goblin_encounter` | 53 | **29** | 7 | 7 | 2048 |
| `sanic_sandbox` | 95 | **27** | 5 | 5 | 2048 |

⛔ **THIS FOUR-ROOM TABLE IS SUPERSEDED — all 72 rooms were later swept and
`mockingbird_arena` bursts to 295 visible. See "ALL 72 ROOMS MEASURED".** The
`entities` column is also wrong here: it reports ALLOCATED slots, not live
entities, and the census now prints `live=` beside it.

⭐⭐ **THE MOST VISIBLE SPRITES IN ANY OF THEM IS 46.** The campaign was opened on
*"a room with hundreds of sprites can visibly chug"* — and on this evidence no
shipped room HAS hundreds of visible sprites. ⇒ either the chugging room is one
not sampled here, or the sprite count was never the cause. ⛔ Somebody should
name the actual room before more work is spent on sprite scaling; four of them
say the premise does not hold.

⚠ NOTE the strong correlate that is NOT sprites: `central_hub_complex` has 4096
entities and **18 per-view projections** against 5–8 elsewhere. If a room is
slow, per-view projection count is the dimension that actually varies between
these rooms.

### ⭐⭐⭐ VISIBLE SPRITE COUNT DOES NOT DRIVE FRAME COST — measured on real rooms

Re-run with `--warmup 900` so the sample is steady state (68–73 frames, not the
2–12 the default gives):

| room | sprites | **visible** | projections | mean |
|---|---|---|---|---|
| `central_hub_complex` | 139 | 34 | **18** | 14.27, 14.74ms |
| `sanic_sandbox` | 119 → 155 | **51 → 87** | 5 | 13.75, 13.89ms |

⭐⭐ **THE TWO ROOMS COST THE SAME ~14ms** despite `sanic_sandbox` carrying 2.5x
the visible sprites and `central_hub_complex` carrying 3.6x the per-view
projections. NEITHER dimension predicts cost.

⭐⭐⭐ **AND THE WITHIN-RUN EVIDENCE IS SHARPER: `sanic_sandbox` went 51 → 87
VISIBLE SPRITES between two consecutive samples while its frame moved 13.75 →
13.89ms. THIRTY-SIX ADDITIONAL VISIBLE SPRITES COST 0.14ms.** Same process, same
room, seconds apart — no cross-run noise to explain it away.

⚠ **THE SLOPE HELD, THE POPULATION CLAIM DID NOT.** The 0.14ms/36-sprite slope
here (3.89us/sprite) was later confirmed at 8x the scale on `mockingbird_arena`'s
295-sprite burst (1.40us/sprite). What was wrong was the assumption that no room
goes higher — four rooms at rest are not the population.

⇒ **THE CAMPAIGN'S FOUNDING PREMISE IS ANSWERED: sprite count is not why a room
would chug.** ⛔⛔ ~~Combined with the population table above — no shipped room exceeds 46–87
visible sprites~~ — **RETIRED, and this sentence sat five lines after its own
retraction.** That was a FOUR-ROOM sample of rooms AT REST; `mockingbird_arena`
bursts to **295 visible**. ⭐ What survives is the second half: **sprite count does
not describe this engine's cost** — the slope held at 8x the scale (1.40us/sprite
against an independent 3.89us probe).

⛔⛔⛔ **AND THE ~14ms IS *NOT* SOFTWARE RASTERIZATION. I WROTE THAT AND IT WAS
WRONG.** The phase split settles it:

```text
[census] phases frames=71 First=0.089 PreUpdate=4.871 StateTransition=2.056
    RunFixedMainLoop=2.419 Update=2.931 SpawnScene=0.120 PostUpdate=1.171
    Last=0.111 outside=0.458
[census] render_pass main_opaque_pass_2d/fragment_shader_invocations = 0
```

Those phases sum to **14.226ms — the whole frame** — and the renderer reports
**ZERO fragment shader invocations**. ⇒ **the floor is CPU work in the app
schedules**, and rasterization is not in it at all. A plausible story that fitted
the facts, believed for an hour because nothing had contradicted it yet.

⭐⭐ **AND THE PHASES INFLATE UNEVENLY AGAINST A SMASH MATCH, which is the lead:**

| phase | Smash | real room | ratio |
|---|---|---|---|
| **StateTransition** | 0.14ms | **2.056ms** | **15x** |
| RunFixedMainLoop | 0.40 | 2.419 | 6x |
| PreUpdate | 2.14 | 4.871 | 2.3x |
| Update | 1.42 | 2.931 | 2x |
| PostUpdate | 0.65 | 1.171 | 1.8x |

⛔⛔ **AND MY OWN INSTRUMENT LIED ABOUT WHAT IS IN IT — same root cause as I3,
walked into again.** `[census] membership` reported `StateTransition systems=0`,
which would mean 2ms of PURE Bevy machinery with no systems at all, and I began
counting registered state types on that basis (there is exactly one, `GameMode`)
before checking it against the other census. `[census] schedules` reports
**`StateTransition=8`**. The membership census reads `schedule.graph().systems`
and `Schedule::initialize` DRAINS the graph; `StateTransition` runs during
startup, so by `PreStartup` its graph is empty while `systems_len()` still finds
eight in the executable.

⇒ it prints `unavailable=graph_already_initialized` now instead of a zero,
because *"zero systems costing 2ms"* is a conclusion somebody will draw — I drew
it. ⭐ **FIXED, AND THEY ARE NAMED NOW.** The graph and the executable are
COMPLEMENTARY, not alternatives: `initialize` moves systems from one to the
other, so the graph answers before first run and `Schedule::systems()` answers
after. The census reads both, and `StateTransition` resolves to **21 systems**:

```text
last_transition x10   despawn_entities_on_enter_state x3
apply_state_transition x2   despawn_entities_on_exit_state x3   apply_deferred x3
```

⭐⭐ **`last_transition` x10 MEANS TEN REGISTERED STATE TYPES — and this workspace
declares exactly ONE.** `init_state::<GameMode>` is the only one in `crates/` or
`game/`; the other nine belong to Bevy's own plugins. So the per-frame state
machinery is mostly not ours and mostly not about our states.

⭐⭐⭐ **AND THREE OF THE TWENTY-ONE ARE `apply_deferred` — COMMAND-FLUSH SYNC
POINTS.** In a room with 2048–4096 entities a flush is not cheap, and three of
them inside one phase is a far better hypothesis for 2.06ms than transition logic
over a state that rarely changes. ⛔ And it argues AGAINST the reflex of replacing Bevy's state
machinery — if the cost is command flushing, the states are not the problem.

⛔⛔ **TWO EXPLANATIONS TESTED, BOTH DEAD.** (a) *"flushes get expensive with
population"* — `sanic_sandbox` reports `entities=2048`, IDENTICAL to a Smash
match, and still costs 2.06ms against 0.14ms. (b) *"the compositions register
different state machinery"* — `[census] membership` returns **the same 21
systems, same names, same counts** in both the `capture_scene` production path
and the `NoWindow` smash path. ⇒ same systems, same entity count, FIFTEEN TIMES
the cost.

⚠ THREE HYPOTHESES REMAIN AND THIS CAMPAIGN CANNOT SEPARATE THEM:
1. **commands queued per frame** — a flush costs what it flushes, which tracks
   spawn/despawn CHURN rather than resident population, and nothing here measures
   it;
2. **`StateScoped` population** — the six `despawn_entities_on_*_state` systems
   iterate state-scoped entities, and a real room may hold far more than a Smash
   stage;
3. ⭐ **MEASUREMENT ARTIFACT** — the phase census attributes WALL TIME between
   markers, and `capture_scene` renders offscreen. If GPU submission or readback
   blocks the main thread, whichever phase brackets that moment absorbs it.
   ⛔ THIS ONE MUST BE RULED OUT FIRST: it would make the whole 2.06ms a property
   of the VEHICLE rather than of the engine.

⛔⛔⛔ **HYPOTHESIS 3 IS CONFIRMED. THE `StateTransition` FINDING IS RETRACTED,
AND SO IS EVERY PHASE NUMBER FROM A RENDERING VEHICLE.**

The discriminator was render resolution — a phase full of state machinery has no
business caring how many pixels exist. Same room, same warmup, 16x the pixels:

| | 320x240 | 1280x960 | ratio |
|---|---|---|---|
| frame mean | 7.33ms | 16.19ms | 2.2x |
| **StateTransition** | **0.169ms** | **1.822ms** | **10.8x** |
| PreUpdate | 3.165 | 4.980 | 1.6x |
| RunFixedMainLoop | 0.736 | 2.592 | 3.5x |
| Update | 1.933 | 3.902 | 2.0x |

⇒ **`StateTransition` SCALES WITH RESOLUTION**, and so does every other phase.
The phase census attributes WALL TIME between schedule markers, so when the
render path blocks the main thread — submission, readback, or a software
rasterizer — whichever phase brackets that moment absorbs it. The "15x
inflation" was the vehicle, not the engine.

⛔⛔ **WHAT THIS INVALIDATES, stated plainly because these are recorded above:**
- the `StateTransition = 2.06ms, 14% of a real room's frame` finding — RETRACTED;
- the recommendation to reopen the brief's direction 8 on that evidence — WITHDRAWN;
- the `apply_deferred`-flush hypothesis built on it — it was explaining a number
  that is not real;
- the claim that a room's ~14ms floor is *"CPU work in the app schedules"* —
  the phases sum to the frame because the BLOCKING IS INSIDE THEM, which is not
  the same thing as the app doing that work.

⭐⭐⭐ **THE DURABLE LESSON, AND IT IS THE MOST IMPORTANT INSTRUMENT CAVEAT OF THIS
CAMPAIGN: `[census] phases` IS ONLY MEANINGFUL WHERE NOTHING BLOCKS ON A GPU.**
It is trustworthy in the `NoWindow` smash path, which is where the sim-tick
split and the driver bracket were measured — those stand. It is NOT trustworthy
in `capture_scene` or any windowed run, and the census should say so at the point
of use. ⚠ Note `fragment_shader_invocations=0` did NOT protect against this:
there is real GPU work in submission and upscaling even when the opaque pass
shades nothing.

⇒ ▢ what survives from the room measurements: the POPULATIONS (sprites, visible,
projections, entities) and the CHURN ratios, none of which are wall-clock. The
phase splits from those runs should be treated as void.

⛔⛔ **SUPERSEDED — this rests on the same voided runs.** The resolution A/B showed
`StateTransition` and *every other phase* scale with pixels, which voids the 6x
inflation this paragraph explains. It is left as a record. ~~`RunFixedMainLoop` IS
EXPLAINED AND IS NOT A DEFECT.~~ Its 17 systems are
`run_fixed_main_schedule` — which runs the WHOLE fixed-timestep sim — plus
`swap_to_fixed_update`/`swap_to_update`, transform easing, three gizmo context
pairs and `update_action_state`. Its 6x inflation is the sim taking more fixed
steps in a heavier room: the work the room asked for, not overhead.

⛔⛔⛔ **SUPERSEDED AND RETRACTED — DO NOT USE THIS SECTION.** The resolution
A/B (320x240 vs 1280x960) showed `StateTransition` scales with PIXELS, so this
number is GPU blocking billed to a state phase. The retraction is above; it is
repeated here because this passage sits AFTER its own retraction and kept three
stars for hours. The original text follows only as a record of the error.

~~`StateTransition` IS 2.06ms — 14% OF A REAL ROOM'S FRAME.~~ The 2026-08-28
baseline measured it at 0.15ms in an empty sandbox, correctly identified it as
Bevy's per-state machinery rather than our code, and concluded it was NOT where
to start. On real content it is fifteen times larger and the second-worst
per-phase inflation in the engine. ⇒ ▢ **the brief's direction 8 deserves
reopening on THIS evidence** — investigate what state machinery a real room runs
that a Smash stage does not, and ⛔ still do not replace Bevy's states before
knowing.

⛔⛔ **THE FRAME TIMES IN THE PRECEDING TABLE ARE NOT USABLE AND ARE OMITTED ON PURPOSE.**
`capture_scene` is a SCREENSHOT tool: the whole run is ~1.3s and the census
sampled `frames=2` to `frames=12`, so every mean is startup-contaminated.
`goblin_encounter` reported 187.90ms over TWO frames — that is app construction,
not a slow room. ⇒ using it as a profiling vehicle needs its `--warmup N` raised
until the sample is steady state, which is the same lesson as the 0.72s probe
window earlier.

### THE CAMPAIGN'S BEFORE/AFTER — and why it is NOT a 10% win

Two measured Smash-match rows now sit in `runtime_frame_cost.jsonl`, same
comparability group, with the campaign's changes between them:

| metric | before | after | Δ% |
|---|---|---|---|
| frame mean | 3.185ms | 2.866ms | −10.0% |
| frame p50 | 2.777ms | 2.561ms | −7.8% |
| frame p95 | 4.219ms | 3.608ms | −14.5% |
| **frame p99** | **6.100ms** | **12.417ms** | **+103.6%** |
| frame max | 213.06ms | 164.61ms | −22.7% |

⛔⛔ **DO NOT QUOTE THE −10%.** Three independent reasons it is not a result:

1. it sits AT the noise floor this campaign measured — the same binary and
   scenario produced means of 4.41, 4.51, 4.82 and 4.84ms, a ~10% spread, and
   the recorded rule (I6) is that nothing under ~15% is signal here;
2. **p99 DOUBLED while max improved.** A real speedup shifts a distribution; it
   does not halve the mean and double the 99th percentile. That is outlier
   behaviour;
3. the ingest flagged the before-row as taken on a DIRTY TREE — *"its binary is
   not that commit alone"* — so the two rows are not cleanly attributable to the
   commits between them.

⭐⭐ **A THIRD SAMPLE SETTLED IT: THE p99 "REGRESSION" WAS NOISE.** Three rows in
the group now, identical scenario/profile/instruments/machine:

| run | mean | p99 |
|---|---|---|
| before (dirty tree) | 3.185ms | 6.100ms |
| after | 2.866ms | **12.417ms** |
| after, 3rd sample | 2.816ms | **7.713ms** |

⛔⛔ **p99 RANGES 6.1–12.4ms ON IDENTICAL RUNS — A 2× SPREAD.** It must not be
used as a regression signal on this host, and the ledger's 5% default threshold
will cry wolf on it every time. **Mean is the usable metric**: 2.816–3.185ms
across all three, a 13% total range, which is consistent with the ~10–15% floor
measured from dev-profile runs and is why nothing under ~15% has been claimed
during this campaign.

⚠ The two post-campaign runs agree closely (2.816, 2.866) and both sit below the
single pre-campaign run. **Suggestive and NOT a claim** — the before side is n=1
and was taken on a dirty tree. Settling it needs repeated runs on a clean
before-commit, which is its own piece of work.

⭐ The tool caught all three by itself, including the metrics only one side
recorded. That is the provenance machinery earning its place: the easiest thing
in the world here would have been to report a 10% improvement and be wrong.

⇒ The expected result was no material change and that is what this is. The value
is the RECORD — two comparable rows spanning the campaign, showing honestly that
the frame is where it was, which is what makes "the frame is broad, not deep"
checkable by somebody who was not here.

### WHAT LANDED, 2026-08-29 — and what each is honestly worth

⛔ ONLY THE FIRST ROW HAS A MEASURED SPEED CLAIM. Everything else is correctness
or composition, and is recorded that way ON PURPOSE: this campaign spent eleven
probes learning that a plausible improvement here is usually worth nothing, and
a landed change with an invented justification is worse than no change.

| change | what it fixes | measured? |
|---|---|---|
| `gameplay_allowed` hoisted onto `GameplayGated` | 83 evaluations per schedule run → **1** | ⭐ YES, structurally: `system_conditions` 139 → 61 |
| `AttackVfxView` query filtered | a presentation fact stamped on **1297 of 2048 entities** — sand chunks and UI nodes | ⛔ no frame change; kept on correctness |
| confirmed-frame boundary published from the session | `fully_confirmed()` was false forever, silently vetoing the winner card, the return to select, AND the autosave | ⭐ fixes a SOFTLOCK Jon reported |
| twintrack `spacetime_3d` / `observatory` / `split_screen` gated | 30 of 33 systems dormant outside twintrack | ⛔ microseconds; architectural |
| 9 census reporters gated at build time | the instrument was the sharpest instance of the antipattern it was built to find | ⛔ architectural |
| Yarn mirror + cut-rope mirror gated on conversation liveness | `RwLock` write + 3 collections rebuilt per frame, one **growing with playtime** | ⚠ tracks save size, not measurable here |
| `puppy_slug_seed` de-allocated | a `String` per candidate actor per frame, BEFORE the short circuit that made it pointless | ⚠ under the noise floor |
| `cut_rope/victory` buffer → `Local` | a `Vec` per frame in every room for one `contains` | ⚠ under the noise floor |
| `emit_intro_flag_chains` → change-driven | re-derived a save table every frame forever; `flag()` is a linear scan that **lengthens with progress** | ⚠ grows with save |

⭐ **THE RECURRING DEFECT, three times in one day:** *"cheap because the data is
small"* written down as a comment, over a collection that grows with playtime —
`dialog_visits`, the save's flag vector, and the mirror's extras. A claim about
size is not a claim that survives a save file ageing.

⭐ **AND THIS ONE *DOES* GREP — the search is written down and currently returns
EMPTY.** Look for `fn name(...) -> bool` whose parameters mention `Query<` or
`Res<`, then compare how often the name appears in `run_if(...)` against how
often it is CALLED. ⛔ Two filters do all the work: exclude std method names
(an unfiltered pass returns 1577 hits for `is_empty`), and exclude any predicate
taking a non-system argument — `opposed(a: Entity, b: Entity, ...)` in
`ambition_combat::clank` is a per-PAIR test and can never be a run condition,
because a condition takes only system params.

Filtered, the whole workspace yields THREE names: `twintrack_is_active`,
`world_inspector_visible` and that false positive. ⇒ **both real ones are now
gated, so this search is exhausted here** — but it is cheap, it independently
rediscovered what a reading survey found, and it is worth re-running after any
large content landing.

⭐ **THE RECURRING TELL:** a predicate hand-copied instead of gated —
`twintrack_is_active` in THREE files consulted by 4 of 33 systems,
`portals.is_empty()` in FIVE system bodies, `FallingSandRoomState::active_room`
in NINE. Every one of them is the gate somebody wanted and nobody wrote.

⛔ **DELIBERATELY NOT DONE:** `setup_cut_rope_encounter`'s boss scan — its
queries are EMPTY in a Smash match (`Without<ReleaseOnDeath>` drains the tagged
boss, and a Smash stage has no `BossConfig`), so its cost is in boss rooms, and
a room gate risks moving where the boss gets tagged. No measured cost, real
risk, left alone.

### ⛔⛔ WHERE THE LEVERAGE IS NOT — read this before opening a perf campaign here

Eleven hypotheses tested this campaign, ten rejected. Grouped by what they rule
out, because the negatives are the durable part:

**Not entity populations** (three separate probes): 1297 spurious
`AttackVfxView`, 64 archetypes, 1536 falling-sand chunks — none bought a
millisecond. ⛔ Entity count is not a proxy for frame cost in this engine.

**Not a dormant capability's presence**: falling sand's whole plugin — 21
`PreUpdate` systems AND its 1024 chunks — costs nothing measurable.

**Not the rendering path**: `world_rendering=1`, `offscreen=0`, portal
`rigs=0`. Smash draws the world exactly once.

**Not rollback snapshots**: the shipped session is `check_distance: 0` and ggrs
skips saving entirely at zero, so `SaveWorld`/`LoadWorld` never run and ~19
rollback registrations cost zero per frame.

**Not the DefaultPlugins block** (two runtime gates): UI focus + picking 0.00ms;
31 asset trackers ≤0.145ms. The 0.95ms outside the driver is DIFFUSE.

**Not the gameplay simulation**: 17 phases, 0.93ms total, largest `WorldPrep` at
0.22ms. Optimizing any single one chases a tenth of the frame.

⭐ **WHAT DID WORK, and it is the only thing that did:** retiring a whole class
at once — `gameplay_allowed` from 83 evaluations per run to 1. And the mechanism
is calibrated: gating a set off reclaims its systems' actual work (0.95ms shed
against 0.93ms measured).

⇒ **THE SHAPE OF THE ANSWER IS ARCHITECTURAL, NOT LOCAL.** A 4.7ms frame spread
over ~630 systems at 2.9–15.6us each has no hot spot to find. Anyone opening a
performance campaign here should start from composition — what is installed, and
what a title actually needs — and should NOT start by profiling for an expensive
system. There isn't one, and this campaign spent eleven probes proving it so the
next one does not have to.

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
| I16 | The portal subsystem does REAL work while dormant (three claims from the same survey) | ⛔ **ALL THREE REJECTED ON INSPECTION.** (a) `bridge_portal_carves` takes `ResMut<FeatureEcsWorldOverlay>` every tick — but `rebuild_feature_ecs_world_overlay` ALREADY takes `ResMut` on the same resource and rebuilds it every frame, so the deref adds no change-detection harm and the clear-and-extend is on an empty vec. (b) `sync_transitable_to_ground_items` marks every `GroundItem` changed every tick — but **no `Changed<GroundItem>` reader exists anywhere in the workspace**, so nothing pays for it. (c) same for `Changed<PortalTransitable>` | ⇒ (b) and (c) are LATENT, not present: the day somebody adds a `Changed<GroundItem>` reader they will get everything every tick. ⛔ Not fixed, because this repo does not add machinery without a customer, and a `set_if_neq` guard today would be speculative. ⭐ Recorded so the next agent finds the analysis rather than the symptom — and so that whoever adds that reader knows to fix this first. |
| I15 | `reduce_encounter_lifecycles` allocates a `BTreeMap` and an `EncounterParticipants` every sim tick with zero encounters alive (from a source survey) | ⛔ **REJECTED — VERIFIED FALSE.** `BTreeMap::new()` is documented as *"does not allocate anything on its own"* and allocates only on first insert; with no commands there are none. `EncounterParticipants { members: Vec<_> }` defaults to `Vec::new()`, also allocation-free | ⇒ that system is already clean when idle, and "fixing" it would have been churn on a wrong premise. ⭐ **A SURVEY IS A LIST OF CANDIDATES, NOT FINDINGS** — this one was checked against the standard library's documented behaviour before being believed, and it did not survive. The other four Tier-2 items in the same survey DID survive the same check and were fixed. |
| I14 | The 0.95ms outside the GGRS driver has a GROUP in it worth gating | ⛔ **REJECTED — IT IS DIFFUSE.** Two runtime gates, both clean (`seats_at_end=2`, driver unmoved): UI focus + picking cost **0.00ms**; all 31 `Assets::track_assets` cost at most **0.145ms**, ~7% and below the noise floor | ⇒ **the DefaultPlugins block is NOT A LEVER.** The two largest groups in it are worth nothing between them, and what remains (~15 raw input, 7 leafwing, 7 `release_confirmed_effects`, 4 egui) is smaller still. The 0.95ms is tens of microseconds each across a hundred systems. ⭐ **THIS IS THE MOST USEFUL NEGATIVE OF THE CAMPAIGN**: it says the remaining leverage is in AMBITION'S OWN composition, not in what Bevy installs — and it says so before anyone spent a migration finding out. |
| I13 | UI focus and picking are a meaningful share of the 0.95ms outside the driver | ⛔ **REJECTED, and cleanly this time.** `UiSystems::Focus` + `PickingSystems::{ProcessInput,Backend,Hover}` gated to `run_if(\|\| false)` at RUNTIME inside one build: `PreUpdate` 2.14 -> 2.19ms, `ggrs_driver` 1.19 -> 1.196ms, `seats_at_end=2` so the match really ran | ⭐ The source read predicted this: `PickingSettings::{input,hover}_should_run` already gates picking, and a windowless host has no pointers — so most of that population was ALREADY skipping. ⇒ **the 0.95ms is elsewhere**: 31 `track_assets`, ~15 raw input, 7 leafwing, 7 `release_confirmed_effects`, 4 egui. ⭐ This is the first probe run the RIGHT way per I12 — runtime gate, one build — and the driver figure matching to three decimals is evidence it touched only what it meant to. |
| I12 | A runtime measured WITHOUT `dev_tools` prices what a shipped game would pay | ⛔ **THE PROBE IS VOID, and it failed in the most tempting direction.** With `dev_tools` off the frame got 2-3x SLOWER — 9.2-13.7ms against a 4.5-5.0ms baseline, `ggrs_driver` 3.44ms against 1.19-1.83ms | ⛔ Do NOT read this as "developer tooling makes the game faster". Turning one feature off `ambition_app` changes FEATURE UNIFICATION across the whole dependency graph, so the two builds differ in more than the thing being varied — the same trap that flips `cargo test -p` results in this repo. ⇒ **a cargo feature is not a clean A/B for runtime cost.** Pricing dev tooling honestly needs the systems disabled AT RUNTIME inside one build, not a second build. ⚠ Jon's steer, same day: **developer tooling should be OPTIMIZED, not removed** — so the useful question was never "what would a shipped game pay" but "what do these systems cost, and can they cost less while staying available". |
| I11 | Falling sand's ~20 `PreUpdate` SYSTEMS cost the Smash frame something (I10 tested only the chunks) | ⛔ **REJECTED, and this time the experiment was the right one.** Removing `FallingSandRoomPlugin` took `PreUpdate` from 137 to 116 systems — `msgr_spawn_particle` and the rest confirmed gone from `[census] membership` — and the frame did not move: 4.50–4.76ms over six steady-state samples against a 4.41–4.99ms baseline, `seats_at_end=2` | ⇒ **falling sand costs nothing measurable in a Smash frame — plugin, systems AND chunks.** It still deserves dormancy on DESIGN grounds (a fighting game should not install a sand grid) but it is not a performance lever and never was. ⚠ two failed attempts preceded this: one where `--no-default-features` silently did not disable the feature (I9), and one where the removal worked but the window was 0.72s of startup-contaminated samples with `max=40.66ms`. ⛔ A probe needs BOTH a check that the thing left AND a steady-state window. |
| I10 | The 1024 `bevy_falling_sand` chunk entities cost the Smash frame something | ⛔ **REJECTED.** Shrinking `with_map_size(32)` to `2` took the world from 2048 entities to 512 — every chunk gone — and the frame got **SLOWER**: 5.24ms against the 4.33–4.93ms band | ⇒ **entity population is NOT a proxy for frame cost in this engine, and that is now three-for-three** (1297 `AttackVfxView`, 64 archetypes, 1536 chunk entities — none of them bought a millisecond). ⛔ Stop reaching for the biggest number in `[census] populations`; it has never once been the answer. Falling sand still deserves dormancy on DESIGN grounds — a fighting game should not install a sand grid — but it is not a performance lever and must not be sold as one. |
| I9 | `--no-default-features` on `ambition_app_tools` disables the `falling_sand` feature | ⛔ REJECTED, AND THE EXPERIMENT SILENTLY DID NOT RUN — the census still reported `ChunkRegion=1024` and an identical archetype count | A dependency's own `default = ["desktop_dev"]` is not disabled by `--no-default-features` on the DEPENDENT; that needs `default-features = false` on the dependency declaration. ⛔ The run LOOKED like a clean negative result. Always check that the thing you removed actually left — `[census] populations` was the only reason this was caught rather than published as "falling sand costs nothing". |
| I8 | Smash renders the world more than once (the brief's "fix that immediately") | ⛔ REJECTED — `[census] views` reports `cameras=4 active=3 world_rendering=1 offscreen=0`, and `[census] portal` reports `rigs=0 active=0` | Smash draws the world EXACTLY ONCE. The whole render-view direction closes with no work. ⚠ one loose end worth its own row: the `Cube scrim display camera` (menu kaleidoscope) sits ACTIVE at `order=7` with `layers=none` throughout a match — it draws nothing but still occupies an active camera slot. Same class as the rest: a dormant capability that never quite went dormant. |
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
