# Performance and iteration — current measured model

**State:** OPEN, but narrow. Optimize measured user-visible or developer costs;
do not maintain a speculative micro-optimization backlog.

Raw measurement authority lives in the `dev/ambition_dev_measurements`
submodule. This file owns **current interpretation and next decisions**, not the
multi-week experiment diary.

Related focused work:

- [`asset-preparation-and-residency.md`](asset-preparation-and-residency.md)
- [`project-build-and-distribution.md`](project-build-and-distribution.md)
- [`capability-and-runtime-composition.md`](capability-and-runtime-composition.md)

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

Still open, and now the only reason to run the comparison: the two hosts differ
in schedule structure and in what the shared host composes around the sim
(startup, launcher, providers, session bridge, host-relative routing). Whether
that costs anything per frame at hall density is unmeasured. It is a smaller
question than the one I thought I had, and it is not on the decision-pipeline
critical path.

**The measurement, when it is worth taking:** `--headless
--headless-acceptance-cycle` runs the production shared host and enters real
gameplay routes; census it and compare schedule structure. `--start-room` cannot
express it, because that flag is what selects the other host.

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

This confirms the standing hypothesis exactly, and it scopes what this campaign
measured. The decision-pipeline work landed here (the `peers()` borrow, the
`WorldMemory` sort) optimized the **supply** side, which is real and correct.
The **demand** side — what a room of genuinely tactical brains costs — has never
been measured, because no such room exists. The acceptance criterion in
`bounded-perception-and-attention.md` needs one built.

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

### P1 — transparent draw attribution

Identify which render layers/material classes own the large transparent fragment
area before changing renderer architecture.

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
