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
- the exact changed variable(s).

For small A/B effects, interleave arms when practical. Recent repeated headless
runs showed block-to-block drift large enough that a single assumed global
"noise floor" is not trustworthy.

Prefer exact counters for structural claims when available. On weak GPU work,
fragment counts established the framebuffer/MSAA changes even when timing noise
and profiler configuration were still being reconciled.

When later evidence corrects a comparison, replace the old headline instead of
preserving both as current guidance.

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
| `WorldPrep.Integrate` | 1.32 | 0.637 ms/tick |
| `ActorDecision.Decide` | 1.64 | 0.415 |
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

Open, in priority order:

1. **Bounded perception** (`bounded-perception-and-attention.md`). The remaining
   per-actor construction is superlinear and no local change touches it.
2. **`WorldPrep.Integrate` at slope 1.32**, now the largest sim phase at 130
   bodies. One attribution pass, not a campaign — and *not* an argument for a
   physics engine until something names the term.
3. **Windowed `Update` (2.59 ms) and `PostUpdate` (1.71 ms) are unattributed.**
   The sim is only ~25% of a windowed hall frame, and the capture that showed it
   is 88.5% CPU in the game binary — so presentation, not the GPU, owns the rest.
   This needs a host with a display; it cannot be measured headless.

⛔ **Keep the hall cast awake.** Making distant actors dormant is a legitimate
game policy and it is not this row: applied to the benchmark it deletes the
workload that finds these defects. See the conflict noted in
`maintainer-decisions.md`.

## Standing prohibitions

- Do not compare headless simulation timing to rendered weak-GPU timing as though
  they describe the same bottleneck.
- Do not call asynchronous decode completion "ready to draw."
- Do not copy mutable benchmark headlines into several planning files.
- Do not optimize by theoretical operation count when the measured ceiling is
  below the noise/drift of the experiment.
- Do not keep an old theory beside its correction in current guidance. Git and
  the measurement journal own the investigation history.
