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
