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

Body integration had a measured serial cost with moving crowds. `7a79333f52`
prepares shared room walls, steps independent bodies with Bevy query batches,
and publishes effects in stable session, room, and simulation identity order.
The active-crowd fixture checks repeated body state and effect order. The room
fixture checks permissions, isolation, and changed geometry. Keep all body ticks
and the common movement kernel. See the
[measurement receipt](../../../dev/ambition_dev_measurements/journal/2026-11-19-active-crowd-integration.md).
The [desktop follow-up](../../../dev/ambition_dev_measurements/journal/2026-11-19-desktop-parallel-body-result.md)
measures 6.378 to 2.823 ms per integration tick and 67.230 to 36.240 ms per
frame with the same 538 bodies.

`7d4bc595d9` replaces per-actor candidate scans with exact room queries over a
reusable geometry tree. `1add31e00e` reuses combat-geometry publication storage
and projects hurt volumes through the common material kernel. Moving VM
fixtures preserve target and view hashes. Exhaustive query oracles and material
projection tests cover the changed contracts. See the
[measurement receipt](../../../dev/ambition_dev_measurements/journal/2026-11-20-spatial-targets-and-geometry-publication.md).
Keep all actor ticks and exact room-wide selection. Desktop frame impact still
needs a native profile.

### Developer iteration remains an architecture constraint

Compile/link/test cost must remain visible during crate decomposition. Moving
code can improve ownership while increasing critical-path crate count or link
work; both facts belong in the receipt.

The compile-cost ratchet should distinguish actual coupling regressions from
workspace growth and stale/unpriced weights. Re-freeze a baseline only after the
measurement model is current; do not bank regressions simply to make the gate
green.

## Current work

### Selected — specialize execution and remove inactive work

The maintainer's 2026-11-21 efficiency direction is explicit: preserve full
expressiveness and a reference evaluator, while gameplay uses proven cheap
cases. The [execution contract](execution-specialization-and-work-selection.md)
sets that boundary and selects E1–E4. Reference evaluation can run offline;
it is not a second solver paid on every live operation.

Current source exposes concrete work to remove:

| Customer | Current cost mechanism | Selected owner |
| --- | --- | --- |
| One projectile leg | Source cells built for recipients, then rebuilt for world contact; root admission can search twice | Portal P2/P4 |
| Ordinary part animation | Immutable whole-clip predicates and stable track relationships interpreted at frame time | Runtime rigged animation PVS-1/PVS-2 |
| Whole-body image effects | A finishing-only change can redraw unchanged source parts | Runtime rigged animation PVS-3 |
| One addressed behavior | Entry-by-trigger matching, repeated occurrence-index captures and duplicate/idle mailbox collection | Addressed behavior delivery |
| Navigation | Empty-world envelope measured per room graph; repeated reachability; route state drops arrival position needed by cost | Navigation E4 |

These are source-derived mechanisms, not a new performance baseline. Begin
with sharing successful projectile preparation, compiled clip properties and
the duplicate collector. Broader preparation, active-journal scheduling and
navigation route reuse retain their owning correctness and lifetime work.
Preserve recent portal filtering, refusal recovery, retained image storage,
shared admitted metadata and the November 18–20 fixes above.

Measure preparation cost, steady work and invalidation separately. Extra
unused vocabulary, dormant objects or unrelated geometry are controls for
the claimed work boundary. Cache warmth must not select a different accepted
simulation result, refusal or delivery time. See the execution contract for
operational versus high-precision reference comparison. A whole-room or
whole-population scan hidden in a new preparation step is not a completed cut.

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

The 2026-11-21 [linking plan](capability-linking-and-prepared-facets.md)
selects a structural hypothesis: pure schema and binding work should not
import the native ECS interpreter merely to describe authored content.
That dependency currently reaches lightweight tooling through
`ambition_engine_schemas`. Runtime plugin omission cannot remove it.

Measure the dependency cut at the selected domain boundary. A smaller crate
is not the objective. The objective is that a change in a native system does
not invalidate a source compiler or an independent authoring consumer that
uses only the domain's stable values.

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

### Completed — desktop preparation work (2026-11-18)

- File access restarted sprite scans. `e90558386e` filters access reports
  before the scan queue. The published-library reload test checks idle reads
  and publication replacement.
- Room plans copied and hashed the complete character catalog.
  `6a338f31ef` shares the immutable catalog and its digest. Retained-service
  identity tests cover content replacement.
- Animation bounds sampled total travel. `810cba0e0b` uses a conservative
  acceleration bound. Combined-motion and published-corpus tests check clipping.
- Camera targets allocated and uploaded blank CPU images. `4154964107` uses
  Bevy's uninitialized target images. The capture path retains GPU readback access.
- Quality realization decoded tables that publication had already checked.
  `e86ee7b4b0` retains those tables in the immutable generation and removes the
  mutable decode cache and warm-up workers. A realization test removes the
  source file before selecting a quality tier. `0772afb205` keys landmark reuse
  by the full-resolution set; its test removes a full table while adding a tier.

Measurements and verification:
[`desktop preparation evidence`](../../../dev/ambition_dev_measurements/journal/2026-11-18-desktop-preparation-efficiency.md).
Keep generation data immutable. Do not restore frame-time metadata decoding
or background warm-up after admission. Desktop frame-time gains still require
a comparable capture. A capped spike log cannot prove that later play had no spikes.

### P1 — Android CPU (Mary O, measured 2026-10-13)

Three device profiles (`dev/ambition_dev_measurements/profiles/android-*`,
read with `scripts/measure/symbolize_android_profile.py`). Shares are of all CPU of
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
  - `virtual_joystick` 2.8: four exclusive systems, about 1.2%. Fixed in
    Ambition's fork; `Cargo.toml` pins it in `[patch.crates-io]` until a
    release has it;
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

## Build invalidation follows semantic boundaries

**Planned 2026-11-21; unmeasured in this review.** The compile-time experiment
for capability linking has three different subjects:

| Change | Expected unaffected consumer | Evidence required |
| --- | --- | --- |
| Native domain system body | The migrated domain's independent schema consumer and portable module | The implementation package is outside those consumers' resolved normal dependency closures; name surviving paths in the full CLI separately |
| Pure schema or binding rule | Consumers of unrelated schemas | A narrow consumer imports the selected format/binder; the all-schema CLI may intentionally rebuild |
| Authored data or script | Engine host | Existing preparation/reload road observes the change with no host Cargo invocation |

Start with the bounded pure-format/native-adapter cut in the
[linking packets](capability-linking-and-prepared-facets.md). Move the schema's
actual validation and prepared values together. A copied DTO, a re-export
that still imports the native owner, or two validators does not make the cut.
Keep a module boundary until a real independent compiler/consumer needs a
Cargo boundary; that consumer is the reason for the package.

Use the existing compile telemetry for cold, no-op and representative warm
edits on the same machine/profile. Attribute changed compiler units,
frontend, codegen, linking and the selected behavior check. Report the
feature-resolved closure separately from elapsed time. A closure reduction
can be proven without claiming a speedup; a warm edit can improve without
reducing cold third-party work.

Keep the composition implementation bounded. Ordinary typed owner functions
and monomorphic prepared values are sufficient. A generic type-level list of
every game capability can multiply Rust instantiations and make diagnostics
harder to read; it would work against this goal. Use type erasure only at an
existing opaque artifact/backend boundary, with typed ownership on each side.

The same distinction applies outside Rust. Content reference edges tell us
what a definition uses. Producer input observations tell us what can change
a result, including a newly present file, directory membership, selected
defaults and tool/environment inputs. Use the owners' actual observations
before reusing a prepared result. See
[content generation](content-generation-and-reload.md) and
[asset delivery](asset-library-editor-and-delivery.md). Reuse within a
candidate does not authorize partial publication of mechanical generations.

The explicit hold on [prevalidated content artifacts](prevalidated-content-artifacts.md)
stays in force. This work neither selects a serialized artifact cache nor
opens the held LDtk cleanup packets. Do not turn a useful ownership cut into
an unrelated cache, build-system or test-runner campaign.

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

### Portal work follows visible projections and source images

**Current source, 2026-11-21:** the carry already retains piece entities,
materials and meshes in `clip_material::ClipQuadPool`; unchanged appearance
and geometry do not replace their assets. Same-size targets are reused and
idle targets retire through `carry::resources`. The October far-side churn
measurement concerns a retired compositor. Do not build another retained
fragment framework to solve that historical finding.

The live structural work is in
[portal view/capture delivery](portals-one-transit-for-every-mover.md#delivery-packets):
complete observer/path/room binding, desired versus ready pictures, and image
source reuse. Window rigs currently allocate a target for every paired end,
even shut ones; `max_active_captures` bounds running cameras, not bytes. An
over-budget rig can expose an opaque pane before its first capture or retain
an indefinitely stale image. Admit current-frame open views explicitly and
use the declared non-view fallback for excess demand. An **open** window
refreshes every presented frame; texture age is not a quality tier.

Count source images and their finishing passes, live/held window targets and
carry targets separately before attributing total memory. The November GNU
image receipt records five carry families at 12.125 MiB of target descriptors
plus 51.2578125 MiB of upstream image-pair descriptors outside that budget;
these are dated software-renderer resource facts, not device frame-time
measurements. A completed local image can remove a second family raster pass
only when it includes the whole requested appearance. Keep the universal
capture fallback and independent family roots/depths.

Remaining CPU work includes repeated clipping before pool comparison,
unconditional open-window mesh writes, room-wide occluder copies/ray scans
and family traversal; remaining render work includes source composition and
capture passes. Resolve their ownership and demand first, then compare the
same visible scene, views, quality and motion with the removed work. Preserve
admitted geometry, source-image readiness and the current-frame finalization
boundary. [Render/VFX R8](render-animation-and-vfx.md#r8---portal-fragment-lifetime-and-measured-render-churn)
owns the presentation integration; individual games do not add private
portal optimizations. The
[GNU image receipt](../../../dev/ambition_dev_measurements/summaries/portal-gnu-limb-render-20261111.md)
and [portal review](../../../dev/reviews/portal-correctness-and-cost-review-2026-11-10.md)
state the existing measurement limits; displayed-host/device costs remain open.

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
