# Performance and iteration — current measured model

**State:** ACTIVE MEASUREMENT — the broad runtime bottleneck classes are known.
Current work is target-specific raster/materialization cost, simulation
throughput at meaningful population, and build/test iteration cost.

This page records the **measurement contract, current conclusions and next
measurements**. It does not retain the chronology of every invalid probe,
retraction or benchmark table. Use git history and
`dev/ambition_dev_measurements` for that evidence.

## What this page owns

Four distinct performance questions must stay separate:

1. **shipped-frame performance** — the program a player actually runs;
2. **simulation throughput** — deterministic sim cost independent of rendering;
3. **asset/materialization latency** — visible/startup hitches caused by preparing
   and realizing assets;
4. **developer iteration** — compile, link, test and artifact/disk cost.

Do not use a number from one class as evidence for another.

Asset lifecycle details are owned by
[`asset-preparation-and-residency.md`](asset-preparation-and-residency.md).

## Measurement contract

A performance claim is valid only when the receipt identifies enough context to
reproduce the same program.

### Always record

- exact Git HEAD;
- command/configuration;
- target machine/hardware;
- build profile/features;
- whether the visible host/rendering/netcode are present;
- stage/fixture/population;
- warmup/steady-state window;
- clock/frame cap/vsync state where relevant.

### One HEAD per long run

Do not edit, merge or otherwise change the tested tree while a long suite or
benchmark is running. A receipt that names one SHA while different jobs saw
different trees is not a single-head receipt.

### Measure the shipped program before alternate profiles

A profiling feature, headless harness or special diagnostic build is a different
program unless proven equivalent for the claim being made.

Alternate builds are useful for attribution. They are not a replacement for a
shipped-host measurement.

### Use controls for optimization claims

An optimization needs:

1. baseline;
2. changed arm;
3. a control that still exercises the work when appropriate;
4. enough repetitions to distinguish the effect from run spread.

If the instrument or attribution model changes, re-take the relevant baseline.

### Moving counts come from tools

Do not copy current workspace/test/compile counts into several planning files.
Point to the checker/measurement that computes them.

## Current runtime conclusions

These are the conclusions that still direct work; their historical tables have
been removed from the live page.

### The ordinary two-fighter simulation is not the dominant shipped-frame cost

Repeated shipped-host and instrumented runs found enough headroom that generic
simulation micro-optimization is not the first performance lever for ordinary
matches.

Simulation cost becomes important as actor/body population grows. Continue to
measure population curves rather than extrapolating from two fighters.

### Rendering/raster cost is material on weak-GPU targets

The framebuffer/raster scale and visual quality tier can move frame cost
substantially on weak/software raster targets. This is a target-profile problem,
not evidence for deleting simulation capabilities.

### Asset materialization is a demonstrated hitch source

First visible use can be delayed by CPU/GPU materialization even when the asset
bytes are already available. Treat preparation/materialization/residency as a
separate latency program.

### Decision/perception work was a real high-population cost and has been gated

The earlier hall measurements found brains building/processing views that were
not consumed. The bounded-attention/perception work removed a large part of that
cost at the correct semantic gate.

Do not reopen generic “skip the whole decision pipeline” work without a new trace
showing the current gate is insufficient.

### Parallelizing the current schedule is not a first-line optimization

The simulation schedule has deterministic dependencies and prior timing
attribution was sensitive to phase instrumentation. Establish a current hotspot
whose work can actually overlap before introducing parallel scheduling
complexity.

### Developer iteration remains an architecture constraint

Compile/link/test cost must remain visible during crate decomposition. Moving
code can improve ownership while increasing critical-path crate count or link
work; both facts belong in the receipt.

The compile-cost ratchet should distinguish actual coupling regressions from
workspace growth and stale/unpriced weights. Re-freeze a baseline only after the
measurement model is current; do not bank regressions simply to make the gate
green.

## Current work

### P1 — weak-GPU raster/quality attribution

Maintain representative weak-GPU/software-raster measurements that separately
vary:

- framebuffer/raster scale;
- texture/art tier;
- expensive presentation features.

Acceptance: a quality recommendation must name which knob changed the measured
cost rather than bundling all “Potato” behavior into one arm.

### P1 — asset preparation/materialization/residency

Owned by [`asset-preparation-and-residency.md`](asset-preparation-and-residency.md).
Performance work here should consume that page's stage-specific measurements
rather than maintaining a second lifecycle model.

### P1 — build/test iteration

Keep the repository's target-bind/headroom policy in every repeated-Cargo entry
point.

Measure and improve:

- common edit-to-test paths;
- expensive crate/link critical paths;
- redundant feature-union work;
- target/artifact growth;
- agent-safe disk usage.

A new test/measurement script that invokes Cargo repeatedly must use the shared
preflight rather than becoming a second unguarded build road.

### P1 — population scaling

Re-run throughput curves when architecture or brain/perception work changes the
per-actor cost materially.

Report both total step time and the responsible phase/domain. Do not infer
superlinear behavior from a single before/after population point.

### P1 — Android CPU (Mary O, measured 2026-10-13)

Three device profiles (`dev/ambition_dev_measurements/profiles/android-*`,
read with `scripts/symbolize_android_profile.py`). Shares are of all CPU of
the app:

- The simulation (`GgrsSchedule`) is 26-35%, rendering about 10%. The
  rest is spread over about 1000 systems, scheduler overhead and malloc.
- Rollback itself is near zero. The local session has `check_distance: 0`,
  so GGRS never saves, rewinds or checksums; rollback systems are 0.1-0.4%.
- A new query state (`World::query*`, `SystemState::new`) in a system that
  runs each frame is matched against every archetype (about 850). Ours are
  fixed. These are in dependencies, and each upstream fix is a few lines
  (keep the state as a `&mut QueryState`/`&mut SystemState` param, or skip
  the work when there is none):
  - `virtual_joystick` 2.8: four exclusive systems, about 1.2%;
  - `bevy_ecs_ldtk` 0.15 `clean_respawn_entities`: about 0.8%. A set
    condition from our side would delay an LDtk respawn by one frame;
  - `bevy_yarnspinner` 0.9 `fetch_resources`: about 0.6%;
  - `bevy_light` 0.19 `check_dir_light_mesh_visibility`: queues its command
    each frame with no directional light in the world, about 0.5%;
  - `bevy_ggrs` `handle_requests`: 0.2-0.4%.
- The sim must do 60 ticks for each second of real time, whatever the frame
  rate. So it takes its CPU first, and rendering gets what is left. If the
  device slows by a factor (heat), the frame rate falls by more than that
  factor. `profile_android.sh` now records the clocks and the thermal state
  to show when this happens.

The archetype count was audited on 2026-10-14: it is flat in play, and the
matching is mostly each system's per-run check, so the lever is the number of
systems that run each frame, not the archetypes. That audit and its plan are
in [system-composition-audit.md](system-composition-audit.md).

### P2 — startup attribution

Measure startup only when a current user-visible/startup budget is threatened.
Separate:

- process/plugin initialization;
- asset declaration;
- asset preparation/materialization;
- first room/session activation.

Do not optimize generic plugin count based on startup intuition without an
attributed trace.

## Developer-iteration policy

### Development optimization level

Use the repository's configured development profiles rather than inventing a
special benchmark profile that invalidates edit/test comparison.

### Optimized incremental builds

Keep incremental artifacts on the verified target volume. Long-running mutation
or sweep tools must re-check the hard free-space floor during the run.

### Tests are part of iteration cost

Prefer narrowly targeted tests first, then the declared lane/union gates needed
for the changed surface. A broad test plan is useful only when its jobs are
attributable to one tree.

## Closed / low-leverage generic directions

Do not reopen these without new contrary measurement:

- deleting arbitrary capabilities for generic frame-time improvement;
- blanket change-driven projection rewrites;
- schedule parallelization before a parallelizable hotspot is shown;
- run-condition micro-optimization as a primary lever;
- entity/system-count reduction without an attributed cost;
- broad physics rewrites justified only by aggregate frame rate.

### New 2026-10-11 terrain-skin cost witness

`crates/ambition_render/src/rendering/chain_skin.rs` adds reusable dressing
for surface chains. Preserve the **correct** separation: simulation publishes
surface geometry; the renderer decorates it from a prepared room theme.
This is a useful engine capability, not a Sanic-only special case.

Three costs require measurement before the next large surface-chain room:

1. `earth_mesh` clips each polygon against every skin grid cell in its
   axis-aligned bounds. The iteration count rises with bounding-box area,
   not only occupied polygon area. Measure steep/large polygons, narrow
   diagonals, and room unload/reload. Record cell visits, generated vertices,
   mesh build time and allocated bytes. Consider clipping only intersecting
   cells, chunking, or caching *only* after an attributed cost is observed.
2. `dress_chain_turf` spawns a Sprite child for each pattern piece on every
   chain segment. Measure the count, draw calls and frame cost in a long
   Sanic-style route. Do not add batching or a custom material before a
   target-profile trace shows a limit.
3. `ChainEarthSettled` and `ChainTurfDressed` mark one-time preparation.
   Verify that a changed prepared theme, quality tier or reloaded asset
   invalidates derived meshes/children once and frees old asset handles.
   This is a **correctness** guard, independent of measured speed.

Use the existing quality/residency, source-generation and room teardown
contracts. Present the same topology through normal and Potato art tiers.
Compare cold realization, second entry and an edit while the room is live.
No game crate should maintain a parallel skin/tessellation implementation.

### Portal fragments create entities and materials on every presentation frame

The new portal compositor uses a deliberately rebuilt representation. That
simplicity is reasonable for correctness, but it has a measurable cost to
investigate, not an assumed performance emergency:
`crates/ambition_portal2d_presentation/src/far_side.rs` and
`src/visuals.rs` despawn and spawn derived pieces every update and add
new `PortalClipMaterial` assets when clipped pieces are generated. A still
source can thus allocate even when its transform and sheet are unchanged.

Benchmark stable and animated portals under the same graphics settings,
visible cameras, and fixed simulation steps. Collect frame-time distribution,
entity spawn/despawn, distinct and retained material handles, mesh allocations,
GPU draw calls, and peak resident bytes over a long run. Compare no panes,
one pane, and two panes; then room unload, camera switch, rollback and
quality downgrade. Disable candidate/pane work separately to attribute the
cost. A drop in CPU time with no material gain is not proof that a retained
reconciler is worthwhile; a material residency increase without a live
reference leak is not yet proof of a leak.

If the traced cost is significant, hand the lifetime fix to
[render/VFX R8](render-animation-and-vfx.md#r8---portal-fragment-lifetime-and-measured-render-churn),
not to each portal-using game. Preserve the presentation-only nature of the
pieces and the portal-visibility arbitration while optimizing.

## Acceptance

The performance/iteration program is healthy when:

1. representative shipped-host budgets are measured on the target classes that
   need them;
2. simulation population curves identify the responsible phases;
3. asset hitches are attributed to explicit lifecycle stages;
4. build/test cost is measured without unsafe target/disk behavior;
5. architecture changes can bank measured wins without also accepting unrelated
   regressions;
6. every quoted benchmark can be reproduced from its recorded configuration.

## Standing prohibitions

- no benchmark receipt across a moving Git tree;
- no performance conclusion from a different program unless equivalence is part
  of the experiment;
- no copied moving counts when a tool computes them;
- no “optimization” without a control and attribution;
- no repeated Cargo loop that bypasses target/headroom checks;
- no dated benchmark diary appended to this live page.

## Architecture measurements need separate hypotheses

A9 separates Cargo closure, compilation fanout, linked bytes, runtime installation
and runtime cost. A smaller SCC is none of those measurements. Report the target,
features, cold/warm state, scenario identity and hardware with each claim.

Recheck the current prepared flow execution before reusing an earlier authored-
flow clone diagnosis. A12's representation changed; historical source observations
are not current measurements. Measure a live flow customer's allocations/copies
before adding an arena, cache or custom interpreter runtime.

A residency budget, broadphase index and parallel schedule change each need their
own measured customer. A new shared abstraction that increases rebuild reach or
per-tick indirection has a cost even when source lines decrease.

## Work-set invariants versus storage experiments

Fast iteration DO work includes explicit preparation dependencies, immutable shared
metadata, bounded invocation write sets and separation of active from unrelated
dormant state. It excludes rebuilding every content section for a scalar edit or
copying the whole world for every callback. Those ownership/work-set requirements
do not depend on a timing threshold.

M0-M3 in [extension evidence](extension-iteration-evidence.md) choose physical
caches, batching, snapshot chunks/COW/deltas and executable deployment. Keep a
full logical reference for restore/hash comparison. Record visited records,
changed bytes, retained history and boundary crossings as well as time. A simple
clone-backed store may remain if real costs support it; neither 'clone is always
cheap' nor 'COW is always faster' is an architectural fact.
