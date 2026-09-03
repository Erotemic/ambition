# Performance and iteration — current measured model

## ⭐⭐ ANSWERED 2026-09-02: the shipped program runs at 250-310 fps

Jon's `desktop-timeline-run-20260902T015909Z`: `profiling` optimisation, NO
`profile` feature, V-Sync Off (`present_mode=Immediate`), frame cap Off,
Ultra, RTX 3090, 79 s through eight rooms including a pirate fight.

```text
room                      frame ms (1 s windows)     fps
central_hub_complex       3.2-3.4                    ~300
hall_of_characters        3.9 once the art landed    ~250
pirate_cove / sky_lookout 3.2-3.6                    ~290
mockingbird_arena         3.1-3.4                    ~310
ninja_dojo                3.2-3.7                    ~290
gravity_lab               3.1-4.6                    220-320
portal_lab                3.9-4.2                    ~250
steady-state phase split  PreUpdate 0.70  Update 1.0  PostUpdate 0.8  RunFixedMainLoop 0.33  outside 0.36
```

The prediction was 120-200 fps and 5-8 ms; it was **pessimistic** — the
windowed additions cost ~0 over the 3.4 ms headless floor, which is now the
same number on the 3090 and the VM. So "the game runs under 100 fps" was
three things stacked: the dev build (opt-level 1), `Fifo` v-sync at 144 Hz
turning any frame over 6.9 ms into 72 fps, and — when profiling — the Tracy
build's 2.5x. None of them is the shipped program. **There is no frame-rate
campaign.** The floor is ~3.4 ms of system count (above), which is half a
144 Hz budget and nothing on the CPU main thread needs attention for it.

**What IS user-visible, in the same run:** 23 frames over 33 ms in 79 s, and
nine of them — 355, 207, 199, 155, 137, 107, 97, 94, 89 ms — inside the hall's
first two seconds, while 126 images / 434 MP arrived (2.15 GB resident after).
The remaining ones are the boot (504 ms) and portal_lab's entry (123 ms). That
is the asset campaign, and it is now the ONLY performance campaign:
[`asset-preparation-and-residency.md`](asset-preparation-and-residency.md).

## WHERE THIS STANDS — 2026-09-01, end of the perception campaign

**Where the day ended (2026-09-01, late).** Three findings changed the frame:
the Tracy build costs ~2.5x per frame (every absolute schedule time in the
ledger's Tracy rows is inflated); the game ran under `Fifo` v-sync with nothing
recording it (now a Video setting, and on the census row); and the shipped
host's headless frame in the hall is 4.5 ms of which **3.7 ms is a floor that
does not depend on the cast** — `bevy_ecs` 34%, our crates ~6%, ~2400 system
runs per frame, no hotspot, not the executor. The decision pipeline is closed
(1.9 ms/tick, linear). Next number wanted, and the only one that decides the
next campaign: `profile_desktop.sh --no-tracy` with V-Sync Off, in the hall, on
Jon's hardware; prediction 120-200 fps. Scripts: `sim_scaling_curve.py`,
`instrumentation_tax.sh`, `tracy_self_time.py`, `headless_room_frame.sh`.

**Closed.** The many-actor decision campaign. `Decide` at 130 bodies went
0.328 → 0.023 ms/tick (−93%) when ADR 0034 increment 1 landed; the hall frame
went 1.94 → 1.56 ms in one ledger group. The sim profile is now FLAT — nothing
above 3.3%, ~10% spread over nine collision symbols — which is the real reason
to stop, more than any single number.

**Open, needs one capture from Jon.** A windowed run with `FramePaceCap::Off`.
Prediction on record: it will NOT materially change the frame, because the
limiter was already sleeping only ~4 us/frame. If it does, that reasoning is
wrong.

**Open, needs a decision from Jon.** Whether to author a DENSE melee room.
Bounded attention cannot be validated without one: `Perception::Sighted`'s
viewport already caps kept peers at ~14 (max 21) and holds there when the room
doubles, so the written acceptance criterion passes on current code. See
`bounded-perception-and-attention.md`.

**Next by measurement**, once the uncapped capture exists: the steady-state
main-thread ranking is `run_ggrs_schedules` 320 ms/s, `render_system` 136,
`camera_schedule` 76. The simulation is the largest main-thread consumer in the
SHIPPED host — which simulates in `GgrsSchedule` inside `PreUpdate`, NOT in
`RunFixedMainLoop`, and not in the same schedule as the headless sandbox.

⚠ **Read every number here against its host and its phase.** Two hosts
(`--start-room` picks the sandbox, no rollback), two clocks (wall absorbs GPU
blocking, `phases_cpu` does not — and it is a PROCESS clock summing all threads,
so compare the two as a RATIO, never as a difference), and Tracy session totals are bimodal across
load — each of those produced a published-then-withdrawn conclusion today.


## ⛔⛔ MEASURED 2026-09-01 (late): the `--features profile` build is a DIFFERENT PROGRAM, ~2.5x per frame

Every Tracy capture in the ledger was taken from a build whose per-system cost
is not the shipped one. Headless production host, startup route, 4000 ticks,
two reps interleaved, same `profiling` cargo profile, on the VM:

```text
arm                                           frame ms   PreUpdate ms
release optimisation, NO `profile` feature    1.45-1.56    0.27-0.29
`profile`, spans filtered (RUST_LOG=error)    1.78-1.81    0.32
`profile`, spans on, no tracy-capture         3.64-3.81    0.59-0.61
`profile`, spans on, tracy-capture attached   4.11-4.33    0.67-0.70   (4.2M zones / 4000 ticks)
```

~1050 zones per tick at this route, ~2.5 us each: the `tracing` -> `tracing-tracy`
path formats every `system{name=..}` span's fields into a zone name on every
enter. It is not Tracy's own cost (tens of ns); it is the subscriber's. The
`RUST_LOG=error` arm shows it: same binary, spans filtered at the layer, and the
frame is back within 20% of the shipped build.

**What this does to the record.** `tracy-csvexport -e` (self time) on Jon's
Ultra hall capture: `schedule{GgrsSchedule}` is 54% SELF time — 2.6 of 4.9 ms
per tick lies inside the schedule but inside no system zone; `Update` 52%,
`Render` 47%. That self time is where the per-zone tax lands. At the ~240k
zones/s that capture carried (50 fps), the tax is of the order of 10 ms of a
19.8 ms frame. ⇒ **every absolute schedule time from a Tracy capture is
inflated, and inflated MORE for schedules made of many tiny systems** (the sim:
434 systems ran per tick, largest 0.32 ms, mean 6 us). Zone-vs-zone rankings
of real work (`integrate_sim_bodies`, `prepare_assets`) stand; "the sim is X ms"
does not.

**And the number Jon plays is a third program.** `./run_game.sh` with no
profile word is the DEV build (workspace crates at opt-level 1). "50-60 fps in
the hall on Ultra" is that build under `Fifo` v-sync at 144 Hz, where a frame
over 6.9 ms shows as 72 and one over 13.9 ms as 48. There is no measurement yet
of the shipped program, in the hall, with v-sync off.

**The measurement that answers it** (Jon's hardware): pull, Video > V-Sync Off,
then `scripts/profile_desktop.sh --no-tracy` — `profiling` optimisation, census
on, no spans — and walk into the hall. The ledger label carries `no-features`
and `+present:Immediate`, so it cannot be confused with the Tracy rows. The FPS
overlay in that run is the honest number; the `[census] phases` row (with
`phases_cpu`) is the honest phase split.

**Reproduce:** `scripts/instrumentation_tax.sh` (the four arms, interleaved) and
`scripts/tracy_self_time.py <bundle>` (self vs total per schedule zone).

⚠ The headless numbers were never inflated: the no-window recipe drops
`LogPlugin`, and with it the Tracy layer, unless the binary carries `profile`
(then `run_shared_host_headless` re-adds it). The 1.56 ms hall tick was a build
without the feature. Two binaries at `target/profiling/ambition_game_bin` on two
machines (the VM's target is a bind mount, not Jon's directory) are not one
binary — check `metadata.json:cargo_features` before comparing.

## ⭐ THE CURVE ON THE PRODUCTION HOST (2026-09-01, late) — item (2) done on the right program

`examples/hall_bench` drives the shipped local session (sync test,
`check_distance: 0`) through the sim harness in `hall_of_characters`, headless,
built WITHOUT the `profile` feature. `scripts/sim_scaling_curve.py` runs it at
each cap, interleaved, first repetition discarded (a 1.5 GB binary's first run
pays the page cache: rep 1 read 2.0 ms where reps 2-3 read 1.0). 3000 ticks,
median of 1 s windows, VM. The `!! NEVER CLOSED` warning appeared zero times.

```text
cap                                 2      16      64     130     200
actors (AbilityBase)                0      17      65     130     130   <- the room authors 129; 200 = uncapped
WHOLE TICK (wall, harness)      1.02    1.19    1.58    1.95    1.87   ms
sum of measured sim phases      0.43    0.55    0.79    1.07    1.05
WorldPrep.Integrate            0.021   0.043   0.109   0.196   0.193
FeatureViewSync                0.059   0.079   0.123   0.173   0.167
Combat                         0.078   0.095   0.123   0.153   0.150
Progression                    0.027   0.038   0.058   0.080   0.077
Decision.* (7 marks)           0.038   0.056   0.090   0.137   0.133
WorldPrep.AfterIntegrate       0.015   0.021   0.033   0.048   0.048
ContactDamage                  0.002   0.003   0.008   0.013   0.012
```

**What it says.** The shipped simulation of the full hall is ~1.9 ms per tick,
of which ~0.9 ms follows the actor count: **~7 us per actor per tick, linear.**
Nothing bends upward — `Decision.Targeting` goes 0.007 → 0.042 for 65x the
actors, so the documented O(n²) in `select_actor_targets` is not what runs.
The largest single phase in the hall is `Integrate` at 0.196 ms. At 60 Hz the
whole actor-dependent cost is ~1 ms of a 16.7 ms budget.

**The actor-dependent slice, by symbol** (perf on the whole shipped host,
130 vs 2 actors, added samples; the run gained 20% samples for 128 actors):
allocator 3.3%, libc memcpy (three unresolved libc addresses) 6.5% ≈ 0.05 ms
per frame — the upper bound on EVERY clone the perception path does —
`select_actor_targets` 2.2% ≈ 0.02 ms, `tick_actor_brains` 1.7%,
`first_body_sweep` 1.7%, `integrate_sim_bodies` 1.2%, `resolve_axis_repair`
1.2%. The two standing hypotheses of this goal are answered with numbers:
the documented O(n²) in targeting costs 0.02 ms/frame at full cast, and the
`PerceptionPeer` string clones are inside a 0.05 ms memcpy budget.

**⇒ Item (3) has nothing to optimise that would move a frame.** The decision
pipeline is closed by measurement on the program that ships. Whatever makes the
hall 50-60 fps on Jon's machine is not in `GgrsSchedule`'s work; the candidates
left are windowed-only — the per-system executor overhead of 3234 systems
(2400 runs per frame; 500 in `Update`, 217 in `PostUpdate`), the render app,
and the dev build — and every one of them needs the `--no-tracy`, V-Sync-Off
capture named above before another line is changed.

Two host numbers to keep apart: the direct sandbox (`--start-room`) ticked the
hall in 1.56 ms; the production host ticks it in 1.9 ms. Both are the
uninflated program; the difference is the rollback host's own machinery.

## ⭐ THE WHOLE SHIPPED HOST, headless, in the hall (2026-09-01, late)

`AMBITION_HEADLESS_GAMEPLAY_ROOM=<room>` now makes `--headless` drive the
launcher into the Ambition route and play in that room — every schedule the
windowed binary runs, minus the render app. `scripts/headless_room_frame.sh`
runs it on the no-`profile` build and prints the census phase split. 3000
ticks, median of 1 s windows, reps 2-3, VM:

```text
cap             frame   PreUpdate  Update  PostUpdate  RunFixedMainLoop  StateTransition
  2 actors      3.7 ms    1.20      0.87     0.78         0.48              0.12
 64             3.9-4.2   1.42-1.55 0.90-0.99 0.76-0.81   0.47-0.48         0.13
130 (full)      4.3-4.7   1.68-1.88 0.97-1.11 0.77-0.84   0.48-0.51         0.13
```

### ⚠ RE-RUN 2026-09-02, after headless started decoding art

The row above was measured on a composition that **decoded no file-backed art at
all**: `ImagePlugin` registers the image loader in `Plugin::finish`, which never
ran under the `app.update()` loop `--headless` uses, so every asset stage after
"demanded" was measuring an empty population. `124684f56` fixed that. Same
script, same room, 3000 ticks, uncapped, reps 2-3 (the script's own rep 1 is the
page-cache warm-up and is shown for completeness):

```text
                frame   PreUpdate  StateTrans  RunFixedMainLoop  Update  PostUpdate
2026-09-01 130  4.3-4.7  1.68-1.88    0.13          0.48-0.51    0.97-1.11  0.77-0.84
2026-09-02 unc  4.7-5.8  1.83-2.15    0.13-0.18     0.53-0.65    1.13-1.33  0.87-1.06
  (rep1 4.961 / rep2 5.765 / rep3 4.726)
```

Every phase moved UP. The qualitative reason is not a regression: the binary is
now doing asset work it previously skipped entirely — decode, the insertion
stamp, residency accounting and extraction all act on a real population (the
same hall entry now shows ~201 routed images resident where it showed 0).

⛔⛔ **THIS IS NOT A CONTROLLED A/B AND NOTHING SHOULD BE CREDITED THE DELTA.**
**156 commits** separate the two rows, 37 of them asset/image-shaped. The
composition fix is the only change that alters what the program *does* rather
than how fast it does it, which is why it is named first — but a number produced
across 156 commits attributes to none of them. ⚠ And the spread is wide:
4.726-5.765 ms across three reps, 22%, so a difference under ~1 ms between these
two rows is not a difference at all.

⇒ Recorded to keep the table truthful, NOT as a tuning signal. The measurement
that would answer "what did decoding cost" is one binary, two arms, interleaved.

### ⭐ A SECOND MACHINE, 2026-09-02: the calculex VM, and the spread collapses to 2%

Same script, same commit (`6162b3e88`), 3000 ticks, uncapped, three reps. ⛔ **A
DIFFERENT MACHINE from every row above** — the calculex laptop's VM: 6 vCPU on an
**i7-7700HQ**, 15 GB, **no `/dev/dri` at all**, sole tenant. Nothing here may be
differenced against the aivm rows; a machine is not an arm.

```text
room                    frame (3 reps)          median  PreUpdate  StateTrans  RunFixedMainLoop  Update  PostUpdate
hall_of_characters      6.955 / 7.107 / 7.058    7.058    2.70        0.20          0.91         1.53      1.19
central_hub_complex     5.765 / 5.595 / 5.736    5.736    1.81        0.19          0.90         1.24      1.14
```

⭐ **THE SPREAD IS THE RESULT, not the frame time.** 2.2% across the hall's three
reps and 3.0% across the hub's, against the **22%** (4.726-5.765 ms) recorded on
the aivm box directly above. Same script, same rep count, same kind of VM. The
difference is tenancy: this box had one job and its load average sat pinned at
6.5 on 6 cores for its own builds and nothing else.

⇒ **So the "wide spread" caveat attached to headless numbers is a property of a
SHARED box, not of the harness or of `--headless`.** A sibling session reported
five identical runs on a shared machine giving frame-spike totals of 61, 4, 9, 6,
52 and nearly published a 15x improvement that was load. On an untenanted machine
the same instrument is stable to 2%. ⛔ Which does not retire the caveat — it
relocates it. Read it as "counts not clocks **on a shared box**", and prefer an
untenanted machine when the clock is the measurement.

⭐ **AND THE HALL-MINUS-HUB DIFFERENCE IS ONE PHASE.** 7.058 - 5.736 = 1.32 ms,
of which `PreUpdate` is 0.89 and `Update` is 0.29; `RunFixedMainLoop`,
`StateTransition`, `PostUpdate`, `First` and `Last` are identical between the two
rooms to within 0.05 ms. Whatever the hall costs over the hub on this machine, it
is not the fixed simulation step.

⚠ What this row does NOT say: anything about GPU cost (there is no GPU), and
anything about the aivm rows (different silicon). `[census] phases_trust` reports
`trustworthy=no_render_backend … Phase splits from this run are usable` on every
rep, which is the harness certifying this configuration rather than me asserting
it.

#### The population-cap curve, re-taken untenanted

Same box, same commit, `CAPS="2 64 130"`, three reps interleaved across caps by
the script. ⛔ Again a DIFFERENT MACHINE from the 2026-09-01 cap curve above —
compare the SHAPE, never the absolute values.

```text
cap        reps (ms)                 median   spread   PreUpdate   Update   PostUpdate
  2        5.589 / 5.571 / 5.522     5.571     1.2%      1.72       1.27       1.10
 64        6.253 / 6.331 / 6.542     6.331     4.6%      2.27       1.43       1.17
130        7.053 / 6.842 / 6.871     6.871     3.1%      2.67       1.53       1.18
```

⭐ **THE COST OF THE ACTOR POPULATION IS `PreUpdate`, and this curve is quiet
enough to say so.** From 2 actors to 130 the frame grows **1.30 ms**, of which
`PreUpdate` is **0.95** and `Update` is 0.26. `PostUpdate` moves 0.08,
`RunFixedMainLoop` and `StateTransition` do not move at all. On the 2026-09-01
aivm curve the same three rows overlap within their own noise, so the attribution
could not be made there — it can be made here because every arm's spread (1.2%,
4.6%, 3.1%) is smaller than the gaps between arms.

⚠ Two honest limits. This is a no-GPU software-rasterizer box, so nothing here
speaks to what a GPU would add. And "PreUpdate carries it" localises the cost to
a schedule, not to a system — naming the system needs a per-system census, not
this instrument.

**Two readings.** First: the shipped program's main-world frame in the full
hall is ~4.5 ms on this VM — against Jon's Tracy capture's 19.8 ms (PreUpdate
8.5, Update 5.4, PostUpdate 3.2, RunFixedMainLoop 1.2), a uniform ~4x across
every phase, which is the instrument tax times the windowed additions. Second,
and the architectural one: **~3.7 ms of it is a FLOOR that does not depend on
the cast.** Two actors cost 3.7 ms; 130 add 0.8. `Update` is 0.87 ms and
`PostUpdate` 0.78 ms with nothing to present; `RunFixedMainLoop` is 0.48 ms
(avian's physics schedules, 50+22 systems, for debris); `StateTransition` is
0.13 ms for 16 systems.

⭐ **AND THE HOST AGREES, which is what makes this more than a headless
curiosity (2026-09-02, `desktop-timeline-run-20260902T215256Z`, 3090, windowed,
the shipped `run_game.sh profiling` walk).** The native profile there is flat in
the same shape: the top symbol is `leafwing_input_manager`'s `InputMap` lookup at
**1.38%**, the allocator cluster (`_mi_page_malloc_zero` ×2, `mi_free`,
`_mi_page_free_collect`, `mi_theap_malloc_aligned`) sums to about **4%**, and the
ECS executor is third. ⇒ Two different programs — a 2-actor headless run and the
windowed game on real hardware — agree that there is no hot spot to attack and
that the largest attributable cluster is allocation. ⚠ 70.5% of the capture is
the game binary and 22.7% the kernel, so the ranking is of the right layer.

`perf` on the 2-actor run (flat, `-F 999`, 4000 ticks) names no hotspot — the
top symbol is the allocator at 3% — but it names FAMILIES:

```text
allocator (mimalloc malloc/free/zero)          7.9%
task pool / executor handoff (user side)       8.2%   + kernel futex/scheduler 7.8%
leafwing input maps (process_actions, clash)   3.6%   two seats, no window
QueryState::new_archetype                      3.1%   a QueryState is being BUILT per frame (archetype count is stable)
UI layout + text (taffy, parley)               2.6%   with no window
main thread 83%, compute pool 15%
```

(The `new_archetype` caller is not found by grep — every `world.query` /
`SystemState::new` in the runtime crates is inside a test — so it is in a
dependency or reached through a lens/join. A DWARF call graph on the 1.6 GB
binary exceeded its report budget again; naming it needs a frame-pointer build.
At 3% of a 3.7 ms floor it is 0.1 ms: a curiosity, not a lead.)

One guess killed before anyone chases it: `RunFixedMainLoop` at 0.48 ms/frame
holds avian's six fixed schedules (PhysicsSchedule 50 systems, SubstepSchedule
20, FixedFirst/FixedPostUpdate/FixedLast), and avian's OWN code is 0.12% of the
profile — the bucket is ~120 near-empty systems being scheduled, plus
leafwing's `update_action_state` (input symbols total 4.7%). An idle physics
pipeline costs its system count, not its math.

And the executor is not the floor either — measured, not read off the perf
buckets. `AMBITION_MAIN_SCHEDULES_SINGLE_THREADED=1` puts First/PreUpdate/
Update/PostUpdate/Last and the five Fixed schedules on Bevy's single-threaded
executor; same run, interleaved, 2 actors:

```text
executor        frame    PreUpdate  Update  PostUpdate  RunFixedMainLoop
multi (default) 3.40     1.10       0.80    0.71        0.45
single          3.21     1.08       0.79    0.71        0.32
```

−5% on the frame, −30% on the fixed bucket (~120 near-empty systems, where
dispatch IS the cost), nothing on Update/PostUpdate. So the 8% "executor" and
8% kernel in the perf buckets are mostly the compute pool existing (workers
parking and waking) rather than per-system dispatch, and the floor is what the
systems DO — ~2400 small bodies of real work at ~1.3 us each. ⇒ reducing it
means running fewer of them, not scheduling them differently.

By crate, the same profile: **`bevy_ecs` 34%**, core/alloc/std 10%, kernel 8%,
mimalloc 7%, leafwing 3.3%, and every `ambition_*` crate together ~6%. Inside
`bevy_ecs`: `QueryState::update_archetypes_unsafe_world_cell` 9.4% +
`new_archetype` 3.1% — the per-system, per-query "did new archetypes appear"
check, ~70 ns a call times several thousand calls a frame — then
`System::run_without_applying_deferred` 4%, `apply_deferred` 1.2%. With two
actors, a third of the frame is the ECS's own per-system bookkeeping, and the
game's code is a rounding error. That is the number to hold up against the
system count.

⇒ The shipped frame is not one slow system; it is **~2400 system runs per
frame at ~1.5 us each with allocation on the way** — 3234 systems in 33
schedules, of which the harness composition needs 2373 and the direct sandbox
fewer. Halving it is a composition question (whole plugin groups that could
carry a run condition when nothing they own exists; systems that allocate per
frame; a query rebuilt per frame), not a hotspot fix, and it is outside the
decision pipeline this goal was armed for. It is the next campaign if Jon's
capture confirms the windowed frame is CPU-main-thread-bound.

The render thread, from the same Ultra capture (ESTIMATE, derived, not
measured): `sub app{RenderApp}` 6.7 ms/frame carrying ~1500 of the frame's
5838 zones; at ~2.5 us a zone that is ~3.8 ms of instrument, leaving ~3 ms
of render work (`render_system` 2.7 of which `RenderGraph` 2.5, `queue_submit`
0.6). The main thread's 18.5 ms carried ~4300 zones ≈ 10.7 ms of instrument,
leaving ~8 ms — which is the 4.5 ms headless floor-plus-cast, the dozen
windowed-only systems, and the 0.9 ms extract. Under `Immediate` the frame is
the longer thread, so main-bound.

**Prediction on record for that capture** (`--no-tracy`, V-Sync Off, hall,
3090): the main-thread frame lands between 5 and 8 ms — this 4.5 ms plus the
dozen windowed-only systems and the render extract — so the overlay reads
120-200 fps unless the RENDER thread is the longer one. If it reads under 100
with `outside` small, the render app is the bottleneck and the transparent
overdraw campaign is the one to open. If it reads under 100 with a fat
`PreUpdate`/`Update`, this section is wrong about the windowed additions.

**State:** OPEN, but narrow. Optimize measured user-visible or developer costs;
do not maintain a speculative micro-optimization backlog.

Raw measurement authority lives in the `dev/ambition_dev_measurements`
submodule. This file owns **current interpretation and next decisions**, not the
multi-week experiment diary.

Related focused work:

- [`asset-preparation-and-residency.md`](asset-preparation-and-residency.md)
- [`project-build-and-distribution.md`](project-build-and-distribution.md)
- [`capability-and-runtime-composition.md`](capability-and-runtime-composition.md)

## ⛔⛔ WITHDRAWN: `run_tests.sh` propagates its exit code correctly — I PIPED IT

An earlier version of this section claimed the runner exits 0 both when it
refuses to run and when a job fails, and told the next reader to distrust it.
**That is false and the section is withdrawn.**

```bash
run_tests.sh:   "$repo_root/scripts/setup/target_bindmount.sh" --check   # returns 2
                exec python3 "$repo_root/scripts/run_tests.py" "$@"
run_tests.py:   return 1 if failed else 0
```

The gate is correct in both states. **I invoked it as
`./run_tests.sh --rust 2>&1 | tail -20`, and a pipeline reports the exit status
of its LAST command:**

```bash
$ false | tail -1 ; echo $?
0
```

So the 0 I read was `tail`'s, twice. The repo's own note about `| grep` voiding
an exit code says exactly this, and I wrote the warning anyway.

**Confirmed unpiped**, with a lane that had one failing job:

```text
$ ./run_tests.sh --rust > lane.log 2>&1 ; echo "EXIT: $?"
EXIT: 1
passed: 3 | failed: ['workspace (default features)']
```

⇒ **The rule is about the invocation, not the runner.** Do not pipe a gate whose
exit code you intend to read; if you must, check `${PIPESTATUS[0]}`. Reading
`passed`/`failed` out of `target/run_tests_status.json` is still the most
informative check, and an absent status file still means the lane never started —
but the exit code was never lying.

### The red it surfaced anyway

`ambition_demo_mary_o::power_loop::every_tier_change_holds_its_arriving_sheets_transition_clip`
— *"the eight-frame fire transformation is the clip a flat 0.5s cut off"*
(`power_loop.rs:358`). **Pre-existing**, confirmed by running it at `82cda301f`
in a clean baseline worktree with submodules initialised. Not caused by the
perception, census, geometry or knob work. ✔ CLOSED 2026-09-02: the test's own
diagnosis ("the fire form alone fails to join") was wrong — the fixture had NO
`CharacterCatalog` or `AuthoredSheets`, so EVERY beat came from the 0.45 s
fallback, and the grow arm passed only because 0.45 ≥ its 0.28 s clip. The
fixture now carries the demo catalog, and a premise guard pins the grow beat to
its clip's length (red at 0.450 vs 0.280 when the catalog is removed). The
shipped composition always has both resources; nothing changed in the game.
⭐ **AND RE-CONFIRMED UNDER THE FEATURE UNION 2026-09-02**, which is the run that
matters and is not the one a `cargo test -p` gives:

```text
cargo test -p ambition_demo_mary_o --test power_loop       11 passed
cargo nextest run --workspace -E 'test(every_tier_change…)'  1 passed
```

⛔ Both arms, because this repo has already had a test that was green per-crate
and RED EVERY TIME under the gate's feature union — the jab-string test, called
"flaky" for a day on the strength of the per-crate reading. A closed row that
only ever ran the narrow arm is not closed.

## Measurement rules

A number is actionable only with enough context to know what it measured:

- source commit;
- host/hardware;
- scenario and whether gameplay was actually live;
- build profile/features and relevant instrumentation;
- rendered versus headless;
- ⛔⛔ **which simulation host** — the direct sandbox and the production shared
  host are different programs, and only one of them rolls back (see below);
- the exact changed variable(s).

For small A/B effects, interleave arms when practical. Recent repeated headless
runs showed block-to-block drift large enough that a single assumed global
"noise floor" is not trustworthy.

Prefer exact counters for structural claims when available. On weak GPU work,
fragment counts established the framebuffer/MSAA changes even when timing noise
and profiler configuration were still being reconciled.

When later evidence corrects a comparison, replace the old headline instead of
preserving both as current guidance.

## ⛔⛔ MEASURED 2026-09-01: every hall number in this campaign profiled a host that does not roll back

`--start-room` is not a room selector. `cli_direct_entry()` (`cli.rs:920`)
returns true for `--direct`, `--start-room` **and** `--room`, and `--headless`
branches on it:

```text
cli_direct_entry()  ->  headless::run_headless      the explicit direct sandbox
otherwise           ->  run_shared_host_headless    the production shared host
```

So `--headless -- -- --start-room hall_of_characters` — the command this whole
campaign measured the hall with — silently selected the sandbox. The bundle says
so itself, and nobody read it:

```text
desktop-timeline-run-20260901T072436Z, game-stderr-stamped.txt:24
  ambition_app: running the explicit direct sandbox headlessly (--headless flag)
```

**The two hosts differ in how they host the simulation.** `headless.rs`
mentions rollback nowhere, and the schedule census of that bundle confirms the
consequence — 20 schedules, 932 systems, **858 of them in `Update`, and no
`GgrsSchedule` at all**. The shipped host does not work that way:

> Developer-visible builds run their authoritative simulation through
> [`rollback::GgrsSchedule`]. During ordinary local play this plugin owns a
> zero-distance local `SyncTestSession`.
> — `dev/rollback_observatory.rs:3`

### ⛔ AND THE COST I FIRST ATTRIBUTED TO IT IS NOT THERE — ggrs says so

The first version of this section claimed the zero-distance SyncTest still
**saves and checksums** every registered component every frame, and costed it at
130 canonical encodes per frame. That is **false**, and the refutation is four
lines into the dependency:

```rust
// ggrs-0.13.0/src/sessions/sync_test_session.rs:155
// we can skip all the saving if the check_distance is 0
if self.check_distance > 0 {
```

`local_session.rs:40` sets `check_distance: 0` for the local session, so
ordinary play issues **no save requests at all**. The composition comment is
exact where I read past it: *"rollback stays dormant."* F9 raises the distance
for one bounded pulse, and only then is anything saved.

⇒ There is no per-frame rollback wire cost in ordinary play, at any population.
I inferred one from the phrase "SyncTestSession" without reading what the
session does at the distance it actually runs at. A dependency's own source is
cheap to check and settles this class of question outright.

### What survives, and what is still open

Established:

- `--start-room` selects a **different program**, and the campaign measured it;
- that program installs no rollback host — no `GgrsSchedule` among 20 schedules;
- the shipped one does, on **every** build and platform. `visible_composition.rs:110`
  is explicit: *"NOT GATED ON `dev_tools`, AND THE SAME HOST ON EVERY PLATFORM."*
  ⇒ `rollback_observatory.rs:7`, *"Non-developer release compositions keep their
  existing simulation host"*, is **stale** — there is no such composition now.

Refuted: the per-frame save cost, above.

✔ MEASURED 2026-09-02 (this paragraph was written before the instrument
existed): `AMBITION_HEADLESS_GAMEPLAY_ROOM=<room> --headless` runs the
PRODUCTION shared host in a room, and `scripts/headless_room_frame.sh` censuses
it — see "the shipped frame floor" above: 4.5 ms in the hall of which 3.7 ms is
a cast-independent floor, ~2400 system runs per frame, `bevy_ecs` 34% and our
crates ~6% of the process. What the shared host composes around the sim is
that floor, and it is system COUNT, not a hotspot. Nothing remains to compare
against `--start-room`.

## ⭐ MEASURED 2026-09-01: the hall's decision cost is supplied to brains that provably cannot read it

Counted from the authored spec, `tools/.../specs/hall_of_characters_area.ron`:

```text
129  type: "NpcSpawn"
129  brain_override: "stand_still"     <- every one of them
```

Zero tactical brains. And `stand_still` does not merely ignore the world view —
it is never handed one. `brain_tick.rs:49`:

```rust
// The nine ordinary arms answer for themselves
if tick_simple_state_machine(sm, snapshot, out) {
    return;
}
```

`tick_simple_state_machine` takes **no `perception` argument**. Only the
`Smash` and `Fighter` arms below it receive a `WorldView`. So the type system,
not a profile, establishes that all 129 hall bodies have a view built and a
`WorldMemory` updated for them each tick, and the function that ticks them
cannot read either.

⇒ Of `Decide`'s 0.234 ms/tick at the hall, the peer-independent remainder is
0.039. **The other ~0.195 ms is supplied to brains whose tick function never
receives it.** Not "mostly wasted" — unreadable by construction.

⛔ **"The brains cannot read it" is not "nothing reads it."** The view is also
consumed by `believed_target`, which maintains `PerceptionMemory` and the
snapshot target for every body whatever its brain. That is why the skip is not
free — see *there is no free version of the skip* below before acting on this
section.

This confirms the standing hypothesis exactly, and it scopes what this campaign
measured. The decision-pipeline work landed here (the `peers()` borrow, the
`WorldMemory` sort) optimized the **supply** side, which is real and correct.
The **demand** side — what a room of genuinely tactical brains costs — has never
been measured, because no such room exists. The acceptance criterion in
`bounded-perception-and-attention.md` needs one built.

## ⭐ MEASURED 2026-09-01: 88% of the hall's decision phase builds views nobody reads

Three arms, same room, same build, same host; 3 interleaved reps each, medians,
3000 ticks. The only variable is which brain all 129 authored NPCs get.

```text
phase                          statues   brutes    smash    smash Δ
WorldPrep.Decision.Decide        0.340    0.730    0.377      +11%
WorldPrep.Integrate              0.253    0.332    0.252        0%
Combat                           0.115    0.232    0.117       +2%
WorldPrep.Decision.Targeting     0.034    0.036    0.033       -3%
WorldPrep.Decision.Observe       0.027    0.031    0.027        0%
```

- **statues** — the authored cast, all `stand_still`, answered by
  `tick_simple_state_machine`, which takes no `WorldView`.
- **brutes** — `ambition::melee_brute_striker`. Also takes no `WorldView`, but
  it ACTS.
- **smash** — the `ambition::medium_striker` autonomous profile, whose
  `template: Smash` is one of the only two arms that CONSUME a `WorldView`.

⭐ **THE THREE NUMBERS ARE THE WHOLE CAMPAIGN.**

**Building the views is the cost.** The peer-independent remainder of `Decide`
is 0.039, so ~0.30 of the statues' 0.340 — **88%** — is constructing peer lists,
world views and memory for 129 bodies that cannot read any of it.

**Reading them is nearly free.** Swapping in 129 brains that genuinely consume a
129-actor view costs **+11%**, and moves nothing else: `Integrate` 0%, `Combat`
+2%. Those bodies do not act on what they read, so this is the read alone.

**Acting is what actually costs.** The brutes read nothing at all and cost
**+115%**, with the downstream to match — `Integrate` +31%, `Combat` +102%.

⇒ Bounded perception is aimed at the right term: the 88%, which is paid whatever
the brains do. It will not touch the +115%, and nothing in the campaign should
claim it does. And `Targeting` — the sole quadratic — is 0.033-0.036 across all
three arms: **a busy room does not spend its time searching.** Fourth
independent reason not to build the spatial index.

### ⛔⛔ AND THERE IS NO FREE VERSION OF THE SKIP — the view has TWO consumers

The obvious move from the numbers above is "skip `build_world_view` for a brain
that cannot receive one; `world_view` is a local, not a component, so it changes
no checksummed state." **That is wrong, and `actors/update.rs:551-577` is where
it fails.** The view is built once and consumed twice:

```rust
let world_view = build_world_view(...);
// 1. maintains ROLLBACK STATE for every body, whatever its brain
believed_target(perception_policy, &world_view, perception_memory.as_deref_mut(), dt)
// 2. the brain, which for 129 of 129 hall bodies cannot receive it
tick_brain_with_actions(..., Some(&world_view), ...)
```

`believed_target` writes `PerceptionMemory` — `rollback_component_canonical`,
`"actor.perception_memory"` — and sets `snapshot.target_pos`. So the 88% is not
work the engine does *only* for the brain: it is work the engine does on the
body's behalf, whose results are then read by nobody in the authored hall.

⇒ Skipping it necessarily changes a canonically checksummed value, which is
exactly what ADR 0034 says and exactly why that ADR exists. There is no
performance-only version of this change. Do not go looking for one.

### ⭐ MEASURED: what the gate would actually buy — `Decide` 0.340 → 0.024

A throwaway probe (applied, measured, reverted; never committed) skipped view
construction and belief maintenance for brains that cannot consume a view. 3
reps, and the perception-reading arm is the control, because the probe leaves it
untouched — so it also tests whether the two binaries are comparable at all:

```text
Decide, statues   0.340 -> 0.024 ms/tick   -93%
Decide, smash     0.377 -> 0.387           +3%   (control)
```

**0.316 ms/tick saved**, ~17% of the ~1.8 ms headless hall frame, turning the
largest sim phase into a rounding error. This also supersedes the estimated
0.039 "peer-independent remainder": with construction AND belief maintenance
gone, the floor measures **0.024**.

⛔ The probe is the checksummed-state change, not a cheaper cousin of it. It
skips `believed_target`, which is exactly why ADR 0034 exists. The number is
here so that decision is made against a measurement instead of an estimate.

⚠ The smash arm is idle by construction, not by design: flat `Integrate` and
`Combat` say those bodies never engaged. It prices the READ, not a fight. A room
of 129 mutually hostile fighters would move all three numbers at once and is a
different, harder experiment.

## ⭐ MEASURED 2026-09-01: waking the hall doubles `Decide`, and NONE of it is perception

The first demand-side measurement this campaign has. Same room, same build, same
host; 3 interleaved reps per arm, medians, 3000 ticks each. The only variable is
`AMBITION_ACTOR_BRAIN_OVERRIDE=ambition::melee_brute_striker`, which replaces all
129 authored `stand_still` brains with an active one.

```text
phase                          statues   brutes    delta
WorldPrep.Decision.Decide        0.349    0.715    +105%
Combat                           0.123    0.224     +82%
WorldPrep.Integrate              0.263    0.333     +27%
WorldPrep.Decision.Targeting     0.037    0.037       0%   <- the O(n²) scan
WorldPrep.Decision.Observe       0.029    0.028      -3%
WorldPrep.Decision.Prepare       0.026    0.028      +8%
WorldPrep.ContactDamage          0.012    0.012       0%
```

⭐ **THE SPLIT IS THE RESULT.** Every perception-SUPPLY phase is flat —
`Targeting`, `Observe`, `Prepare`, `Publish`, `StateMaintenance` — because they
do the same work whatever the brain then does with it. The whole +105% of
`Decide` is `tick_actor_brains` itself, and the downstream consequences of bodies
that now move and fight (`Combat` +82%, `Integrate` +27%).

Two things follow.

**Supply is invariant to demand, which is the argument for the declaration
gate.** ~0.09 ms/tick of peers, views and targeting is paid identically whether
129 brains read it or none do. Gating it is not a trade against cognition
quality; it is subtraction. See ADR 0034.

**The quadratic scan is not what a busy room costs.** `Targeting` did not move at
all — 0.037 both ways. A room where every actor is actively fighting spends its
time thinking and colliding, not searching. This is the third independent reason
not to build the spatial index yet.

⚠ **STILL UNMEASURED, and it is the interesting half.** `melee_brute_striker` is
answered by `tick_simple_state_machine`, which takes no `WorldView` — so this
measures brains that ACT, not brains that PERCEIVE. Only `Fighter` and `Smash`
consume a view, and no authored catalog preset names either; `Fighter` is
reachable only through an `autonomous_profile`. A perception-reading arm needs
one of those wired to the hall, and it is the arm that would finally price the
attention work.

✔ **RE-CHECKED 2026-09-02, and the first half HOLDS: no authored catalog preset
names `Fighter` or `Smash`.** The catalog's `brain:` values are
`patrol_peaceful` (43), `stand_still` (37), `melee_brute_striker` (20),
`melee_brute_brute` (4), `skirmisher_ranger` (2), `wanderer_puppy_slug` (1) and
7 empty; `melee_brute_striker` binds to `MeleeBrute`, not `Fighter`.

⭐ **BUT THE SECOND HALF IS CLOSER THAN IT READS.** Eight `autonomous_profiles`
are authored in the catalog — `medium_striker`, `cellular_duelist`, `patroller`,
`robot_duelist`, `pirate_boarder`, `pirate_boarder_heavy`, `skirmisher`,
`door_guard` — and they are WIRED, through `authored/*.rs` and encounter RONs
rather than a catalog field. **And the hall places three of their customers**:
`goblin`, `npc_lab_raider` and `npc_pirate_raider` all appear as `NpcSpawn`
values in `hall_of_characters`, and `authored/goblin.rs` /
`authored/npc_lab_raider.rs` both name `medium_striker`.

▢ **The one link I could not close by reading**: whether that path actually
yields a `Fighter` brain at runtime, or whether the catalog's own `brain:` field
wins first. That needs a run, not a grep. ⇒ If it does yield one, the arm this
row is waiting for may already be reachable in the hall and only needs
measuring.

⚠ Method note, because the cheap check misleads twice: characters do NOT name a
profile through an `autonomous_profile:` field — grepping for one reports ZERO
users and reads as "nothing is wired". They reference it by local name through
`BrainProfileRef`, from authored Rust and encounter data.

## ⛔⛔ WITHDRAWN, SAME DAY: the mark repair changed NOTHING measurable

Earlier today this section claimed the one-sided sim-phase marks had been
under-billing, and put the correction at +19% on `Decide` and +18% on
`Integrate`. **That was block drift, and the claim is withdrawn.**

The two numbers came from different blocks with different binaries — the very
comparison this file's own measurement rules forbid. Re-run as an interleaved
A/B, both binaries alternating in one session, 5 pairs with the first dropped:

```text
@130 statues                     pre     post    shift
WorldPrep.Decision.Decide      0.335    0.336      +0%
WorldPrep.Integrate            0.249    0.251      +1%
Combat                         0.113    0.114      +2%
Decision.Targeting             0.034    0.034      +1%
Decision.Observe               0.026    0.026      +4%
```

The raw samples say where the fiction came from — one high outlier in a block:

```text
Decide  pre  [0.332, 0.332, 0.338, 0.394]
        post [0.330, 0.334, 0.337, 0.353]
```

### ⛔⛔ AND THE FIRST REPAIR MISREAD THE GRAPH: nine phases ARE chained

Worse than the withdrawn number: the repair declared indices `10..20` unordered
because I read ONE `configure_sets` block in `schedule.rs`, saw no `.chain()`,
and generalised. A **second** block chains the whole post-Core run, and the
file's own doc comment says so:

```text
CoreSimulation -> FeatureCollection -> FeatureInteraction -> LdtkRuntimeSpine
  -> EncounterSimulation -> Cutscene -> GameplayEffects -> Progression
  -> ResetProcessing -> FeatureViewSync,   then PresentationVisualSync after it
```

So nine of those eleven now carry both edges. **`Trace` is the only genuinely
unordered phase** (`.after(CoreSimulation)` and nothing else).

⭐ And `Trace` could not be fixed by labelling it. `close(index)` bills
now-minus-`last` and then ADVANCES `last`, so a serial mark on an unordered phase
does not merely mislabel its own bucket — it steals from whichever chain bucket
closes next, wherever the scheduler happened to run it. `Trace` now has an
independent start/end clock that never touches `last`. Its number is that
phase's own wall time and **overlaps** a chain bucket rather than partitioning
with it; the census row labels it `overlapping=`.

⚠ The schedule file's doc comment is itself stale in the other direction: it says
`ResetProcessing` and `Trace` are both tail consumers outside the chain.
`ResetProcessing` joined the chain (with a comment explaining why); only `Trace`
did not.

### ⭐ RE-MEASURED after the graph repair: the tail was misattributed by a whole phase

Interleaved, both binaries alternating in one session, 4 pairs after dropping the
first:

```text
phase                        1-sided tail   bracketed    shift
WorldPrep.Decision.Decide           0.460       0.459      -0%
WorldPrep.Integrate                 0.348       0.347      -0%
Decision.Targeting                  0.065       0.066      +1%
Decision.Observe                    0.060       0.060       0%
--- and then the post-Core tail ---
PresentationVisualSync              0.230       0.000    -100%
FeatureViewSync                     0.000       0.237       new
FeatureInteraction                  0.003       0.024    +700%
FeatureCollection                   0.035       0.090    +157%
Trace                               0.074       0.000   unmeasured
```

**The decision pipeline is untouched** — every phase this campaign's conclusions
rest on moves 0-1%. That is the second independent confirmation that the marker
work does not disturb them (they were already bracketed on both edges).

**The tail was wrong by a whole phase.** `FeatureViewSync` read 0.000 and its
0.237 ms was billed to `PresentationVisualSync` — which in a HEADLESS run should
be near zero, because there is no presentation to sync. It now is. That is the
third-largest sim phase in the room and it was invisible.

`Trace` now reads 0.000 honestly instead of 0.074 dishonestly.

⇒ **The repair is structural hygiene for the decision pipeline, and a real
correction for everything after `CoreSimulation`.** A one-sided
mark genuinely CAN bill a successor's work to the previous bucket, and it is
still worth having both edges — but at this workload the scheduler was not
actually doing so, and no earlier number needs the 18% correction I published.

⭐ The lesson is one I had already written down and did not apply: compare arms
in ONE session, interleaved, and drop the first pair. A rebuilt binary measured
an hour later is a different block, and this workload drifts by ~18% between
blocks — which is larger than most effects worth chasing here.

## ⭐ THE CURVE, on the repaired instrument (2026-09-01)

Same room, headless, no Tracy, 3 reps per point, medians, 3000 ticks. Population
set with `AMBITION_ACTOR_POPULATION_CAP`; the mark closes on every run and the
`!! NEVER CLOSED` warning appears zero times.

```text
bodies                         2       16       64      130
WorldPrep.Decision.Decide  0.005    0.022    0.152    0.331
WorldPrep.Integrate        0.014    0.041    0.128    0.249
Combat                     0.045    0.057    0.074    0.104
Decision.Targeting         0.003    0.007    0.017    0.032
Decision.Observe           0.004    0.007    0.014    0.026
Decision.Prepare           0.010    0.013    0.016    0.021
WorldPrep.ContactDamage    0.001    0.002    0.006    0.011
```

**`Decide` is superlinear in the middle and linear at the top.** 8x the bodies
from 2 to 16 costs 4.4x; 4x from 16 to 64 costs 6.9x; 2.03x from 64 to 130 costs
2.18x. That is the `kept`-peer saturation the earlier model named — once a
viewer's kept set stops growing, each new body adds a constant.

⛔⛔ **AND THE SELF-DOCUMENTED O(n²) SCAN MEASURES LINEAR.**
`select_actor_targets` documents itself quadratic and its PAIR COUNT is, but
`Decision.Targeting` grows 0.003 → 0.007 → 0.017 → 0.032: **2.4x for 4x bodies**
from 16 to 64, and **1.9x for 2.03x** from 64 to 130. Sub-linear, then linear.
The count is quadratic and the time is not, at every population this room can
reach.

⇒ Fifth independent reason not to build the spatial index. It would attack
0.032 ms/tick of a phase that is 0.331, on a term that is not even growing
quadratically in the range that exists.

⚠ **200 IS NOT REACHABLE IN THIS ROOM.** The hall authors 129 NPCs and the cap
only removes; there is no knob that adds. A 200-body point needs a room authored
for it, and the acceptance criterion in
`bounded-perception-and-attention.md` wants those bodies genuinely tactical
anyway — so that experiment is owed as a room, not as a flag.

## ⛔ AN INVALID PROBE, recorded because its number looked plausible

To price the per-viewer `String` clone in `build_world_view`
(`id: p.id.clone()`, paid once per viewer per kept peer — ~1800 allocations a
tick at 130 bodies), a throwaway probe replaced it with `String::new()` and
measured interleaved against the real binary.

```text
Decide   with clone 0.476   without 0.601   +26%
```

**Removing work made it 26% SLOWER, which is the tell.** Empty ids collapse every
actor onto one `WorldMemory` key, so the belief store, the membership scan and
`believed_target` all run a different program. The probe measured a different
game, not this game minus an allocation. No number from it is usable.

A valid probe has to keep the ids distinct and make the clone cheap — `Arc<str>`
on **both** `PerceivedActor.id` and `PerceptionPeer.id`, so the per-pair clone
becomes a refcount bump and the allocation happens once per body per tick
instead of once per pair. Four production construction sites; the rest are tests.

⇒ **The clone remains unpriced.** Its ceiling is ~1800 → ~130 allocations a
tick, and the gate in ADR 0034 removes ~93% of the phase it sits in, so it is
not the next thing to build.

⚠ Note the absolutes: this block read `Decide` 0.476 where an earlier block read
0.336 on the same binary — **42% block drift**. Interleaving controls for it
(both arms in one block) and is the only reason the +26% was legible at all.

## ⭐ LANDED 2026-09-01: ADR 0034 increment 1, measured with its own control

The gate is in. Interleaved against the pre-gate binary, both alternating in one
session, 4 pairs after dropping the first:

```text
authored hall cast -- 129 of 129 declare None
phase                            no gate     gate    shift
WorldPrep.Decision.Decide          0.353    0.026     -93%
WorldPrep.Integrate                0.264    0.264      -0%
WorldPrep.ContactDamage            0.012    0.012      +0%
census window wall time            6.291    4.287     -32%
```

**It matches the throwaway probe's ceiling exactly** (0.340 → 0.024 predicted,
0.353 → 0.026 delivered), which is the outcome that licenses trusting the probe
method for the next one.

⭐ **AND THE WALL TIME MOVED.** The census window shrank 32%, so this is work
that LEFT THE PROCESS rather than moving between buckets — the failure mode a
boundary instrument is most prone to, and the reason to look at a second
instrument before believing a phase number.

### The control: a cast that NEEDS the belief is untouched

The same room with `AMBITION_ACTOR_BRAIN_OVERRIDE=ambition::melee_brute_striker`,
whose brains classify `TargetBelief`:

```text
WorldPrep.Decision.Decide          0.735    0.748      +2%
Combat                             0.240    0.239      -0%
```

⇒ The gate DISCRIMINATES; it does not merely delete work. Brains that need a
belief still get one and still pay for it, and that arm is what separates
"correctly selective" from "broke perception".

⚠ Several unrelated phases also fell 12-25% on the `None` arm (`PlayerSimulation`,
`Progression`, `FeatureCollection`). `Integrate` and `ContactDamage` did not move
at all, so it is not a uniform scaling artefact. Removing 129 world-view
constructions a tick plausibly relieves allocator and cache pressure elsewhere —
**plausibly** is the operative word; that mechanism is not measured and should
not be quoted as one.

## ⭐ THE CURVE AFTER THE GATE: the decision pipeline is no longer the many-actor cost

Same room, headless, no Tracy, 3 reps per point, medians, 3000 ticks, on the
build with ADR 0034 increment 1 landed.

```text
bodies                         2       16       64      130
WorldPrep.Integrate        0.014    0.039    0.129    0.249   <- now the largest
Combat                     0.045    0.051    0.071    0.093
Decision.Targeting         0.004    0.006    0.015    0.030
WorldPrep.Decision.Decide  0.003    0.005    0.012    0.023   <- was 0.331
Decision.Observe           0.004    0.007    0.014    0.023
WorldPrep.ContactDamage    0.001    0.002    0.006    0.011
```

**`Decide` fell out of the top of the table.** It was 0.331 at 130 bodies and is
0.023 — and its slope collapsed with it: 65x the bodies from 2 to 130 now costs
7.7x, where before the same span was steeply superlinear. The campaign that
opened on "the decision pipeline is the many-actor cost" has closed it.

⭐ **`Integrate` IS THE NEW HEADLINE, AND IT IS LINEAR.** 0.249 ms/tick at 130,
roughly 10x `Decide`, and it grows 2.8x / 3.3x / 1.93x against population steps
of 8x / 4x / 2.03x — linear in bodies, slightly sublinear at the top. There is no
quadratic term in it to attack; it is a per-body constant, which is a
constant-factor problem rather than an architectural one.

⚠ `Combat` is 0.093 at 130 and 0.045 at TWO — nearly half of it is fixed cost
that does not care how many bodies exist. A population sweep cannot tell you what
that fixed half is; that needs a different instrument.

⇒ **What the curve now names, in order:** `Integrate`'s per-body constant, then
`Combat`'s fixed floor. Neither is the perception architecture, and neither wants
a spatial index — `Targeting` is 0.030 at 130 and still linear.

## ⭐ MEASURED 2026-09-01 ON REAL HARDWARE (RTX 3090): the sim is not the frame

Jon's windowed capture, `desktop-timeline-run-20260901T172520Z`. i9-11900K, RTX
3090, Vulkan, 1600x900, quality **Ultra**, 287 sprites (86 visible), ONE world
camera, 6031 frames.

```text
frame p50            9.77 ms   (~102 FPS)
frame p99           17.02 ms
worst frame        468.75 ms
```

Against the same production host run headless, with no render backend:

```text
phase                windowed   headless
whole frame              9.66       2.40
RunFixedMainLoop         0.558      0.515   <- the simulation
PreUpdate                3.154      0.408
Update                   2.936      0.572
PostUpdate               1.805      0.513
```

⭐ **THE SIMULATION IS 0.55 ms — under 6% of the frame — AND IT IS THE SAME
NUMBER HEADLESS.** It is real work and it is not the problem. Roughly 7.3 ms
appears only when rendering, on a scene of 287 sprites.

### ⛔⛔ AND THE PHASE SPLIT CANNOT SAY WHERE, WHICH I MISSED FIRST

I read `PreUpdate 3.15 ms` as CPU work and said the GPU was idle. **The game had
already refused that reading**, in stderr, on every window:

```text
[census] phases_warning untrustworthy=render_blocking — `[census] phases`
attributes wall time between markers, so GPU blocking lands in whichever phase
brackets it. Trust phase splits only from a run with no rendering.
```

The summary printed the table with no trace of that warning, which is how I read
past it. Both are fixed: the summary now carries the caveat, and
`[census] phases_cpu` reports the same split on a CPU clock, so a phase whose
wall time far exceeds its CPU time was WAITING.
⛔⛔ **CORRECTED 2026-09-02: `wall - cpu` is NOT the stall, and this sentence
said it was.** The census reads `CLOCK_PROCESS_CPUTIME_ID`, which SUMS EVERY
THREAD — a phase keeping four cores busy for 2 ms reports 8 ms of CPU against
2 ms of wall, and the subtraction goes NEGATIVE. **Read the RATIO**: cpu/wall is
roughly how many cores the phase kept busy, and a ratio near zero is a stall.
The subtraction is only the stall where the ratio cannot exceed one. The
function's own doc said so from the start; three comments and this line
propagated the thread-clock reading it had already rejected. **Neither number below is attributed until a capture with the
CPU column exists.**

### What is structurally true regardless (counts, not timings)

`PreUpdate` runs **144 systems** windowed against 126 headless. Of them:

```text
37 x Assets                  one asset_events per registered asset type
15 x update_instance_states  bevy_kira, one per audio CHANNEL
 7 x release_confirmed_effects
```

We register **14 audio channels, 12 of them music crossfade layers**
(`MusicLayer0A..5B`), each adding a per-frame system whether music plays or not.
Candidates, not causes.

### The hitches are the asset campaign, confirmed on real hardware

The 468/466/373 ms frames at t≈19-21s line up with
`perfect_cellular_automaton_spritesheet` materializing — 7 pages, ~108 MP. That
is one of the two characters already measured as owning 43% of the session's
decode work.

### ⛔ THE DECISIVE INSTRUMENT WAS BROKEN, AND SO WAS THE CHECK FOR IT

The capture lost its per-system zones. The bundle said *"the game never
connected"*; `tracy-capture.log` said `The client you are trying to connect to
uses incompatible protocol version`. It CONNECTED. The game links Tracy
**0.14.0** (`tracy-client-sys 0.29.0`, protocol 82); the built server is
**0.13.1**.

`profile_deps.sh` has a check for exactly this, and it **could only ever report
MISMATCH** — its parse read the word `Major` instead of the number, so it printed
"the game links Major.Minor.Patch" and compared that to a real version. A warning
that fires on every correctly-installed machine is one a reader learns to skip.
Both fixed.

⇒ **Next capture wants:** `./run_developer_setup.sh --profile` (aligns Tracy),
then a windowed run. It will carry `phases_cpu`, and Tracy's zones will name the
systems. Until then, "7.3 ms of rendering" is a total, not an attribution.

## ⭐ MEASURED 2026-09-01, REAL HARDWARE: the frame is CPU-BOUND, not GPU-bound

Two windowed captures on the RTX 3090 carrying the new `[census] phases_cpu`
row. `CLOCK_PROCESS_CPUTIME_ID` sums every thread, so `cpu/wall` is roughly how
many cores a phase kept busy — a stall reads near ZERO, not negative.

```text
phase                wall      cpu   cpu/wall
PreUpdate           2.676    6.550     2.45
Update              2.664    3.621     1.36
PostUpdate          1.636    2.251     1.38
outside             0.574    1.657     2.89   <- present/vsync WAIT
RunFixedMainLoop    0.493    1.287     2.61   <- the whole simulation
TOTAL               8.589   16.303     1.90
```

⭐ **`outside` IS THE TELL.** That phase is the present/vsync wait — the one
place a GPU-bound frame parks. It shows **2.89 cores busy**. The machine is never
idle anywhere in the frame.

⇒ **16.3 ms of CPU is burned per 8.6 ms frame, across ~1.9 cores**, to draw 287
sprites with one camera. This settles the question the wall-clock split could not
and that I wrongly answered from it earlier: the GPU is not the constraint.

⚠ **AND IT STILL DOES NOT ATTRIBUTE BY PHASE.** A process clock counts every
thread, including render work running concurrently with whatever main-world
phase happens to be open. `PreUpdate 6.55 ms` is 6.55 ms of PROCESS cpu during
PreUpdate's window, not 6.55 ms of PreUpdate's own systems. Naming the systems
still needs Tracy zones.

⛔ Tracy still refused both captures — `PROTOCOL MISMATCH`, now named on the
bundle instead of blamed on the game. Client 0.14.0, server 0.13.1;
`./run_developer_setup.sh --profile` fixes it.

## ⭐ The shipped build lost four first-party crates

`cargo tree -p ambition_app`, measured against the manifests at `98a9cb015`:

```text
crates in the shipped app build   547 -> 539
first-party crates                 62 -> 58
```

Gone: `ambition_demo_pocket`, `ambition_demo_twintrack`, `ambition_relativity`,
`ambition_relativity2d`, and `bevy_falling_sand`. None of them was reachable from
the launcher; relativity arrived through `platformer2d`'s `all_capabilities`,
which is the default feature set, and pocket was a registered-but-unlisted
provider.

## ⭐⭐ MEASURED 2026-09-01 WITH TRACY: the frame is PACED, and the sim is not where I said

First capture with Tracy actually connected (`desktop-timeline-run-20260901T220143Z`,
7,887 zones). Shares of a Tracy-inflated frame — read the ratios, not the
absolutes.

```text
system                                          tot%   mean us
bevy_render::run_render_schedule                35.54    6316
bevy_ggrs::schedule_systems::run_ggrs_schedules 23.89    4245
bevy_render::renderer::render_system            11.77    2092
bevy_render::render_asset::extract_render_asset  7.67     1363
bevy_core_pipeline::schedule::camera_driver      7.06     1255
bevy_framepace::framerate_limiter                4.61      819
bevy_time::fixed::run_fixed_main_schedule        3.11      554
```

### ⚠ THE CAP WAS ON — AND MEASURABLY NOT BINDING

Jon confirmed the cap was enabled and has since turned it off. But the
steady-state window table says it was doing almost nothing:

```text
framerate_limiter   0.26 ms/s   (~4 us/frame at 60fps)
```

`framerate_limiter` sleeps `limit - frame_time`. Four microseconds means the
frame was ALREADY at or past the cap's target, so there was nothing to wait for.

⇒ **PREDICTION, recorded before the uncapped capture so it can be wrong:**
turning the cap off will NOT materially change the frame. The limiter was
returning nearly immediately. If the uncapped run is much faster, this reasoning
is wrong and the 4 us/frame needs explaining.

⚠ My earlier "4.61% of the frame" for the limiter was the session total again —
the same trap as `extract_render_asset` above, in the same capture.

### THE CAP, as originally read

`bevy_framepace::framerate_limiter` is 4.61% of the frame, and
`FramePaceCap::Auto` is the DEFAULT: *"`Auto` caps to the display refresh
(battery saver); `Off` renders unthrottled."*

⇒ "Why is this not at 200 FPS on a 3090" has an answer before any optimisation:
**because it is configured not to be.** Every frame number taken so far — 9.77 ms
p50 uninstrumented, 16.18 ms with Tracy — was taken under a limiter.

⛔ **AND THAT RETRACTS MY "CPU-BOUND" READING.** I concluded from `phases_cpu`
that the machine is never idle and the frame is CPU-bound. Under a PACER that is
not a safe inference: a limiter that spins rather than blocks keeps cores busy by
construction, which is exactly the signature I read as "busy". The honest state is
**unknown until a capture with `FramePaceCap::Off`**, and that is the next
measurement, not more analysis.

### And the simulation is ~23%, not 6%

`bevy_ggrs 0.22` runs its update loop in **`PreUpdate`**
(`schedule: PreUpdate.intern()`), so `schedule{PreUpdate}` at 31.92% CONTAINS
`GgrsSchedule` at 23.02%. PreUpdate's own non-sim work is the remaining ~8%.

⇒ My earlier "the simulation is under 6% of your frame" was wrong. It read
`RunFixedMainLoop` (0.558 ms), which is Bevy's fixed-timestep loop — Tracy puts
`run_fixed_main_schedule` at 3.11% — and **that is not where this game
simulates.** The production host simulates in `GgrsSchedule`, inside PreUpdate.

⚠ The headless numbers are unaffected: `--start-room` selects the direct sandbox,
which installs no rollback host at all, so there the sim really is in the phases I
was reading. The two hosts simulate in different schedules, which is the same trap
recorded above under `--start-room` selecting a different program.

### ⭐ STEADY STATE (t>14s), which is a different ranking from the session totals

```text
zone                    ms/second   ~us/frame
run_ggrs_schedules         319.91        5332   <- the simulation
render_system              135.91        2265
camera_schedule (x4)        75.96        1266
extract_render_asset         8.02         134
framerate_limiter            0.26           4
```

⇒ **On the main thread, the SIMULATION is the largest steady-state consumer**,
ahead of render submission. The render THREAD runs concurrently (its own zone is
96.66% of wall), so this ranks main-thread work against itself and does not say
rendering is cheap — it says the sim is not the small slice the session totals
made it look.

### Rendering is about half of the session, and here is where it goes

```text
sub app{RenderApp}                              35.82
  schedule{Render}                              35.38
    system{render_system}                       11.77
      RenderGraph                               10.73
    system{camera_driver}                        7.06
    system{prepare_assets<sprite>}               1.77
    system{prepare_assets<texture>}              1.74
sub app{RenderExtractApp}                       16.06
  schedule{ExtractSchedule}                     12.25
    system{extract_render_asset<GpuImage>}       7.67
camera_schedule, all four cameras                6.75
```

⛔⛔ **WITHDRAWN, WITHIN THE HOUR: `extract_render_asset` IS NOT A STEADY-STATE
COST.** I read 7.67% off the session total and called it a per-frame cost. The
per-window table refuses it — the number is entirely the load phase:

```text
window        total_ms   window        total_ms
 9.0-10.0       739.72    14.0-15.0        0.34
13.0-14.0       804.53    20.0-21.0        0.35
                          27.0-28.0        0.30
```

Steady state is **8.02 ms per SECOND — about 134 us a frame.** A session mean
over 1,785 calls hid a bimodal load/steady split, which is the summary-statistic
trap this file already records twice. Those 739 ms and 804 ms windows ARE the
asset hitch, exactly where the campaign says it is, and nowhere else.

⚠ **THREE CAMERAS RUN EVERY FRAME** (1,784 invocations each) though the camera
census reports one WORLD camera. Cameras 0, 7 and 9 cost 478 + 274 + 336 µs a
frame between them; camera 8 is intermittent (185 invocations) but expensive when
it runs (1,082 µs). 6.75% of the frame across four cameras in a 2D game with one
world view is worth a question, though not yet a defect: a HUD and a UI camera
are legitimate, and nothing here says they are not.



`sub app{RenderApp}` 35.8% + `sub app{RenderExtractApp}` 16.1%, with
`extract_render_asset` at 7.67% — the asset-materialization campaign, visible in
a steady-state frame rather than only in the load hitch.

## Current runtime model

### Simulation CPU: linear-ish at two fighters, superlinear in a full room

At the two-fighter populations this section was written for, a normal headless
frame is **4.3–4.5 ms** with ~0.83 ms of marked gameplay simulation and ~0.21 ms
of GGRS driver overhead, spread across hundreds of small systems rather than one
hot one. That reading still holds **for two fighters**.

**It does not describe a populated room, and as of 2026-09-01 it is measured.**
`hall_of_characters` at 130 bodies, headless and without Tracy, varying
population inside one room:

| | slope, 17 → 130 bodies | at n=130 |
|---|---:|---:|
| `WorldPrep.Integrate` | 0.86 (after) | 0.252 ms/tick |
| `ActorDecision.Decide` | 1.27 (after) | 0.341 |
| `ActorDecision.Targeting` | 1.03 | 0.053 |

Cost per body nearly quadruples across that range. The dominant term was
per-actor perception CONSTRUCTION, not cognition — 130 brains decide in 0.098 ms
while building what they decide about cost 0.76 ms. Borrowing the shared peer
snapshot instead of cloning it per actor halved `Decide` and raised headless tick
throughput 24%; the remainder is bounded only by a bounded representation, which
is `bounded-perception-and-attention.md`.

⚠ **The shape is superlinear but not n²**, and the reason is a design constraint
rather than a constant: the actor channel is viewport-clipped, so the cost is
O(n × visible) and *visible* is set by spatial density. Count is not the
independent variable.

System count is useful for architecture/composition census. It is not a cost
model by itself.

### Weak-GPU rendering: framebuffer/raster scale is material

The current feature-matched laptop comparison is:

| | baseline | treated |
|---|---:|---:|
| median p50 | **51.045 ms** | **20.101 ms** |
| approximate rate | 19.6 FPS | 49.7 FPS |
| speedup | | **2.54×** |

The treated build capped the effective framebuffer scale and removed 4× MSAA.
The exact fragment count moved from **5,760,000** (3200×1800) to **1,440,000**
(1600×900) before overdraw, and the MSAA writeback pass disappeared.

A separate 18.467 ms treated run was built without Tracy support and is **not**
feature-matched to the baseline. Do not use it as the current 2.76× headline.

The 2.54× result still changes two raster knobs together. The next useful A/B is
to separate framebuffer/DPI scaling from MSAA before assigning the gain to one
mechanism.

Transparent overdraw is large enough to measure. One capture saw roughly 41.5M
transparent fragment invocations over a 7.8M-pixel framebuffer. Attribute the
responsible layers/draw area before designing a new rendering architecture.

### Asset/device materialization: demonstrated hitch source

A rendered desktop run had healthy steady state (p50 about **7.54 ms**, p99 about
**12.50 ms**) but rare catastrophic hitches, with a worst frame around **516
ms**.

The principal measured spike was downstream of asynchronous decode:
`extract_render_asset<GpuImage>` reached about **454.9 ms**. Large bursts tracked
image megapixels arriving together. Loaded image residency also grew throughout
the run.

Several changes reduced avoidable burst work: prewarming lazy registries, raising
roster demand before bodies spawn, bounding character materialization, retaining
HUD images, avoiding unconditional material mutation, and memoizing repeated
schema work. A follow-up run observed a worst in-play frame around **78.4 ms**,
but it was not an identical-scene controlled A/B. Do not quote that as a precise
percentage win.

The funded architecture is explicit demand/preparation/device materialization and
residency ownership. See the focused asset plan.

⛔ **BEFORE ANYONE ADDS AN IO-POOL KNOB FOR THIS: the measured spike is not in
the IO pool.** PNG decode runs on Bevy's IO task pool, and that pool is small by
default — `bevy_app-0.19.1/src/task_pool_plugin.rs:73-127`, "25% of cores for
IO, at least 1, no more than 4", where the count truncates and then rounds up at
a fraction of 0.5 and clamps to `[1, 4]`:

```text
 8-9   logical CPUs -> 2 IO threads
10-13  logical CPUs -> 3 IO threads
14+    logical CPUs -> 4 IO threads
```

⚠ **So two machines comparing the same scene can be comparing two pool sizes** —
13 cores gets three threads and 14 gets four, which is a boundary neither box
advertises. Record `nproc` beside any decode-sensitive capture.

⭐ But the row above already says the spike is DOWNSTREAM of decode:
`extract_render_asset<GpuImage>` is render-world extraction/upload, not the
decode itself, and the 468/466/373 ms frames were pinned to one sheet's
megapixels ARRIVING together. Widening the decode pool makes them arrive faster
at the step that is already the bottleneck. Separate the extract cost from the
decode cost in one capture before spending a knob on either.

### Startup: important, but the capability hypothesis did not survive

Removing four experiences and roughly 61 `Update` systems from the tested
composition did not improve plugin registration: the measured values were about
**372.3 ms → 380.8 ms**, inside noise and in the wrong direction.

This does not prove startup is irrelevant. It proves that generic capability
removal has not earned a startup-performance claim. Measure startup work by
actual attributed cost before restructuring for it.

## Current developer-iteration model

Build/test iteration is independently valuable even while simulation CPU is
healthy.

### Development optimization level

A measured comparison of three first-party crates at dev `opt-level = 0` versus
`1` moved the representative runtime from about **5.12 ms → 2.96 ms** while the
measured one-file rebuild penalty was only about **1–2%**.

Preserve that result when revisiting dev profile policy. Do not trade a large
runtime distortion for a marginal rebuild change without evidence.

### Optimized incremental builds

Optimized incremental compilation produced invalid/corrupt link/runtime behavior
in the observed workflow. Current launch tooling disables incremental for those
optimized profiles. Treat that as a correctness constraint until a controlled
Rust/toolchain change demonstrates otherwise.

### Test resource shape

The full `app_it` target can exhaust machine memory at default concurrency while
passing with lower test concurrency. Test policy therefore needs resource-aware
lanes/presets rather than treating maximum parallelism as universally faster.

⛔ **AND MEMORY IS NO LONGER THE BINDING RESOURCE — DISK IS, measured
2026-09-03.** `target/debug/deps` alone reached **141 GB** and this box could not
run its own suite: the runner's floor is 40 GB and one `cargo test --workspace`
spends **14 GB in under three minutes**. The mechanism is the decomposition
campaign's own side effect — every feature job builds its own variant of the
graph, cargo never prunes the last one, and five crates were carved out of the
actor monolith in a single day, each multiplying the variants a feature job
resolves. ⇒ "Resource-aware lanes" now has to mean disk as well as concurrency,
and the cheap lever is an mtime prune rather than `cargo clean`;
[`pickup-carve-checklist.md`](pickup-carve-checklist.md) carries the recipe and
the caveat that its cost depends entirely on build cadence.

Feature-combination checks are also valuable: broad combination sweeps have
found real integration failures that crate-local/default-only tests miss.

Detailed build-policy work belongs in
[`project-build-and-distribution.md`](project-build-and-distribution.md).

## Closed or low-leverage generic optimization directions

Do not reopen these as architecture campaigns without new measurements.

### Generic capability removal for frame time

Removing several whole experiences did not materially move the measured frame.
Capability composition remains valuable for ownership, dependency closure,
compile/test isolation and SDK quality.

### Generic change-driven projection

Measured projection candidates were too small or already gated/change-driven to
justify a repository-wide conversion. Use change detection where it improves
semantics or a local measured cost, not as a blanket performance doctrine.

### Parallelizing the current simulation schedule

The experiment produced roughly **1.5 million voluntary context switches over
3,600 ticks** while gameplay systems were individually tiny. Thread dispatch,
parking and synchronization overwhelmed the work available to parallelize.
Single-threaded deterministic simulation remains a reasonable current policy.

### Run-condition micro-optimization

Conditions are frequent but individually cheap in the measured workload.
Collective capability/run conditions can improve semantic activation boundaries;
do not sell them as a major frame-time program.

### Entity/system count and broad physics rewrites

Current fighter/body-count and fight/idle experiments did not show enough cost to
fund general entity-count reduction or a physics rewrite. Reopen only with a
representative workload that demonstrates the scale problem.

## Open measurements and work

### P1 — separate the weak-GPU raster knobs

Run an interleaved rendered A/B that varies framebuffer/DPI scale and MSAA
independently on the weak GPU. Retain exact fragment/pass counters beside timing.

### P1 — asset preparation/materialization/residency

Follow
[`asset-preparation-and-residency.md`](asset-preparation-and-residency.md): stage
specific telemetry, demand before first use, rendered pacing validation, explicit
residency owners/budgets, and elimination of measured re-preparation/re-loads.

### P1 — transparent draw attribution — ✔ ANSWERED 2026-09-01

Measured with `report_draw_census` (`crates/ambition_render/src/runtime_census.rs`):
four backdrop sprites hold 96% of all drawn sprite AREA and fifty-seven gameplay
sprites hold 4% — the lever is the backdrop's layer count and blending, not the
actors (`dev/ambition_dev_measurements/journal/2026-09-02-the-overdraw-is-the-backdrop.md`,
queue row D-RASTER-3). What remains is the weak-GPU measurement itself, which
needs that machine.

### P1 — build/test iteration

Resolve dev profile policy, optimized-incremental policy, resource-aware test
lanes, clean-checkout/generated-artifact expectations and supported feature
combinations in the build plan.

### P2 — startup attribution

Only after a current startup trace shows a material user-facing cost, identify
which preparation/plugin/assets dominate it. Do not infer the answer from plugin
count.

### ~~P2~~ P1 — throughput scaling: the threshold has been crossed

The condition this row waited on — "a real product scenario materially exceeds
the current fighter/body/room populations" — **happened**. `hall_of_characters`
is a player-accessible room with 130 authored actors and it is a deliberate
stress workload. The curve above is the re-measurement this row asked for.

**Closed 2026-09-01.** Four changes, all "stop paying a general-purpose price for
a special case":

- **Borrow the peer snapshot** rather than clone the room per actor per tick.
- **Sweep axis-aligned boxes with the closed form**, not parry's generic GJK —
  which was 10.7% of the whole process, the largest single cost in the profile.
- **Test view membership against sorted keys** in `WorldMemory::update`, not a
  linear scan of the view per remembered actor — 12.89% of the process at crowd
  density.
- **Look up before inserting** in the same function, so a peer already
  remembered costs no `String` clone.

The curve, 6000-tick runs with the startup census window excluded, two reps per
point agreeing within 3%:

```text
bodies   Decide  Integrate  frame p50
     9   0.0113     0.0254      0.578
    18   0.0251     0.0446      0.670
    34   0.0630     0.0733      0.795
    66   0.1583     0.1310      1.055
   130   0.3410     0.2521      1.662

slopes 9 -> 130:   Decide 1.27    Integrate 0.86    frame 0.40
```

⛔ **THE FIGURES PUBLISHED EARLIER IN THIS ROW WERE MEASURED WRONG.** They
averaged the census's one-tick startup window, whose every phase reads 0.000, into
short runs — `(0.000 + 0.341 + 0.332) / 3 = 0.224` was published as `Decide` at
130 bodies against a true 0.341. The bias was worst at low populations, where
runs are shortest, so every slope was too STEEP. The row no longer carries them;
the frame column was never affected (`[census] frame` has no startup window) and
its headline stands: **3.07 -> 1.66 ms p50 at 130 bodies.**

⭐ `Integrate` is **sublinear** at 0.86 — cost per body falls as the room fills,
which is what per-tick amortisation looks like: the collision world is rebuilt
once per tick however many bodies then sweep against it.

`Integrate`'s superlinearity was never a missing broadphase; it was a per-sweep
constant large enough to look like one. The simulation profile is now flat,
nothing above 2.4%.

Open, in priority order:

1. ~~Windowed `Update` and `PostUpdate` are unattributed.~~ **Measured
   2026-09-01, and the framing was wrong.** `capture_scene --fit-room` runs the
   room through the real render stack offscreen, so this needs no display.

   ```text
   fixed cost at 3 bodies    6.93 ms      presentation owns nearly all of it
   marginal for 127 actors   2.43 ms      sim 66%, presentation 34%
   ```

   ⛔ **"The sim is only ~25% of a windowed frame" is a share of the ABSOLUTE
   frame**, most of which is fixed and does not change with population. Of what
   130 actors ADD, the simulation is two thirds — and that is on a software
   rasteriser, so the render share is an upper bound.

   Two separate campaigns fall out, and they were being conflated:
   - **the marginal ~2.4 ms** caps POPULATION and is the half already cut in
     half today.

     ⚠ **Its simulation share is NOT settled.** The phase census says 66%;
     `perf`, on the same two runs, says the game's own code is 34% and rendering
     44%. The `perf` number is the better one — the census attributes wall time
     between markers, so a software rasteriser's CPU work lands inside
     `PreUpdate` and reads as simulation. On real hardware the rendering share
     largely disappears and the game's share rises, but that is an inference
     from a bound, not a measurement, and it needs a display.
   - **the fixed ~6.9 ms** caps the baseline frame rate — and **has no hot
     spot.** Measured: 55.5% of that run is this host's software rasteriser,
     and inside the game's own code it takes **197 symbols to reach half**, with
     the largest at **0.99%**. ⛔ Do not open a "make the baseline faster"
     campaign expecting something to optimise; halving the biggest symbol buys
     0.5% of a frame. The only levers on a diffuse cost are structural — fewer
     systems, fewer entities, less per frame — which is a composition question,
     not a profiling one.

     ⚠ Diffuse at the SYMBOL level is not diffuse at the SYSTEM level, and Bevy
     0.19 has no per-system profiler to tell them apart. And none of this
     touches real-GPU rendering: the weak-GPU transparent-overdraw lead (~5.3x)
     is separate and still live.
2. **Bounded perception** (`bounded-perception-and-attention.md`) — re-read its
   measured section first. Bounding the COUNT of perceived actors is worth ~8%;
   `kept` already saturates at ~14 and the cost is per-item construction. The
   design is right for density; it is not the next millisecond.
3. **`getenv` at 1.36% of the process**, unexplained and **not cheap** — do not
   pick this up expecting a quick win. The profiling profile has no frame
   pointers (`force-frame-pointers` is not a cargo profile key; see the comment
   in `Cargo.toml`), and adding them via `RUSTFLAGS` would not help: the
   unresolved caller sits above `getenv` inside precompiled `std` and glibc,
   which have none regardless. `-Z build-std` is the actual price.

⭐ **AND BOTH HALVES ARE NOW DIFFUSE.** After the two fixes above, the top game
symbol at 130 actors is the allocator at 1.21% and nothing else clears 0.4%.
There is no third structural win visible to `perf` on this workload. Further
simulation progress needs either a per-system profiler — which Bevy 0.19 does not
have — or a composition change: fewer systems, fewer entities, less per frame.
Treat that as a reason to stop measuring this workload, not to measure it harder.

⛔ Do NOT reopen: the O(n²) body-contact pairing (dormant, `contact_empty=true`),
`select_actor_targets` (measured slope 1.03), or `Arc<str>` actor identity
(measured 0.04 ms). All three were named by review, all three measured
negligible.

⚠ **The hall-dormancy decision's condition now has its number.** The 2026-08-08
row authorises dormancy *"especially if that reduces lag"* and states that the
condition must be measured. Measured 2026-09-01: the whole 127-actor cast costs
**+2.98 ms** of a ~10 ms offscreen frame, of which the simulation — the only part
dormancy removes, since a dormant statue still draws — is **8–18%** depending on
which instrument attributes it. See
`journal/2026-09-02-what-hall-dormancy-would-actually-buy.md` in the measurements
repo; the decision is not this document's to make.

⭐ Two facts belong beside it. The cast is roughly **half** as expensive as it was
before this campaign, so the condition is being weighed against a moving number.
And every simulation defect fixed here — the peer clone, the GJK sweep,
`WorldMemory`'s quadratic — was found by profiling 130 **awake** actors, so a
dormancy policy that keeps an all-awake mode for measurement costs nothing and
one that does not deletes the workload that finds these.

## The measured series

[`runtime-frame-history.md`](runtime-frame-history.md) is generated from
`dev/ambition_dev_measurements/runtime_frame_cost.jsonl` and is the only place
frame times may be compared ACROSS runs: it groups by everything that changes a
frame time without the engine changing — scenario, content version, build
features, machine, renderer, resolution, instruments — and refuses to subtract
across groups.

⛔ Do not quote a frame time from a journal entry as a baseline. A journal records
what one run measured; the ledger records what may be compared to what.
`scripts/lib/profile_bundle_to_history.py <bundle>` appends, and
`scripts/perf_history.py report -o docs/planning/engine/runtime-frame-history.md`
regenerates — a test fails if the committed report has drifted from the ledger.

## Standing prohibitions

- Do not compare headless simulation timing to rendered weak-GPU timing as though
  they describe the same bottleneck.
- Do not call asynchronous decode completion "ready to draw."
- Do not copy mutable benchmark headlines into several planning files.
- Do not optimize by theoretical operation count when the measured ceiling is
  below the noise/drift of the experiment.
- Do not keep an old theory beside its correction in current guidance. Git and
  the measurement journal own the investigation history.
