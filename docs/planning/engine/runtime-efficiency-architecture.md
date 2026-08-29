# Runtime efficiency architecture

**State:** OPEN engine-product program. Synthesis, not a new campaign — every
item here has a home document, and this file exists to say how they RANK against
each other and why.

Source: a GPT architecture review, 2026-08-29, commissioned while the runtime
efficiency campaign was running. Its framing is kept because it is good; ⭐ the
measured annotations are this repository's, and where a measurement contradicts
the review THE MEASUREMENT WINS and says so.

## The comparison that makes this concrete

Godot separates a high-level `SceneTree` from lower-level rendering, physics and
audio servers, and its own documentation tells you to bypass the node layer for
very large workloads: high-level nodes cost more CPU and memory, and large
numbers of them get expensive.

⭐ Ambition does not need to copy that design. It needs the architectural
PROPERTY that makes it possible: **the authoring model must not dictate the cost
of the hot representation.**

⛔ And it should not try to beat Godot everywhere. Godot has years of work in
rendering backends, asset import, editor tooling, platform support, physics and
navigation. Ambition can plausibly become extremely efficient for the games it
is designed around long before it matches Godot feature-for-feature.

## The ten directions, ranked, with what is actually measured

### 1. Runtime capability composition has to become real

⛔⛔⛔ **MEASURED 2026-08-29: THIS WILL NOT MAKE THE GAME FASTER.** Removing every
non-Smash experience from a live Smash match — Sanic, Mary-O, Pocket, Twintrack,
their entire registration — moved the frame from 4.5–5.0ms to 4.40–4.67ms, which
is inside the noise floor, with `seats_at_end=2` so the match really ran. The
review calls this *"the biggest one"* and it is the wrong size: **it is an
architecture and startup item, not a frame-time one.** Do not fund it as a
performance migration. Details and the arithmetic that predicted it in
[`performance-and-iteration.md`](performance-and-iteration.md).

**The claim:** the model is too close to *install the universe, then ask hundreds
of conditions every frame whether each piece should do anything.* The wanted
model is hierarchical — an executable installs experiences, the current
experience activates capabilities, the current world activates instances —
so portals, conversations, mounts, boss machinery and specialized menus have
essentially zero hot-loop cost when absent.

Home: [`capability-and-runtime-composition.md`](capability-and-runtime-composition.md).

⚠⚠ **THE REVIEW'S SUPPORTING EVIDENCE IS PARTLY WRONG, and the correction
changes the size of this job.** It says inactive Sanic, Smash and Mary-O
machinery *participates in* tiny unrelated workloads, resting on an earlier
reading that a sandbox run "ticked their systems 1802 times each". Measured on
2026-08-29:

- the demos ARE gated, and gated the right way — `SanicRulesPlugin::hosted()`
  applies `run_if(in_mode(SANIC_MODE))` to whole TUPLES (`rules`,
  `milestone_sfx`, `badniks`, `ring_loss`), and Bevy 0.18 makes a tuple-level
  `run_if` COLLECTIVE: one anonymous set, evaluated at most once per schedule
  run. Sanic's 28 registered systems cost about four condition evaluations, not
  twenty-eight, and none of them execute while the mode is inactive;
- across the whole app after the `gameplay_allowed` hoist there are **61**
  per-system conditions and **29** set conditions in total — the "hundreds of
  conditions every frame" figure does not survive contact with a structural
  count;
- ⛔ what IS measured: **154 of 780 registered systems (19.7%) in a sandbox run
  belong to four experiences that run never enters** — `ambition_demo_mary_o=44`,
  `ambition_demo_twintrack=44`, `ambition_demo_smash=38`,
  `ambition_demo_sanic=28`.

⇒ **the cost is REGISTRATION, not per-frame execution**: plugin build time (the
number a phone player feels at startup), schedule graph size, and memory — not a
hot loop. That makes this a startup-and-composition problem rather than a
frame-time one, and it is still worth doing; it is not the frame emergency the
earlier reading implied. ⛔ Anyone opening this should re-measure before
believing either number.

### 2. Authoritative simulation and derived presentation need a harder split

Gameplay authority in a compact deterministic model; render/UI/audio as
PROJECTIONS of it. A renderer should not require every gameplay fact to exist as
a continuously maintained, feature-rich presentation entity. Rollback gives
Ambition a second, independent reason to want a clean authoritative boundary.

Ambition has moved a long way here already; the work is making it pervasive.

### 3. Derived state needs to become change-driven by default

*Authoritative data changes → a generation/event/change marker advances →
dependents update once*, replacing *recompute every frame just in case* as the
DEFAULT projection model.

⛔ This does not mean converting the engine to events. Determine the complete
authoritative input set first; a missed invalidation path is a correctness bug,
so each conversion owes a regression test covering every path.

Measured candidates: `rebuild_control_prompt` (31.8us/frame),
`rebuild_feature_view_index`, `rebuild_attack_vfx_views`,
`sync_ecs_actors_with_save`. See D-PERF-3 in
[`performance-and-iteration.md`](performance-and-iteration.md).

### 4. Render views need explicit ownership, lifecycle and cost accounting

Cameras cannot stay arbitrary entities any feature happens to spawn. A gameplay
camera, split-screen view, portal capture, minimap, HUD view, cutscene view or
offscreen effect is potentially large render work. A semantic view layer should
know who requested each view, what world it sees, its resolution, refresh
policy, render target, and whether it is active.

⭐ This ADDS expressiveness rather than removing it: a dormant portal capability
has no capture cameras, an occluded portal can refresh infrequently, and a view
about to become visible can prewarm.

Home: [`multiplayer-and-multiview.md`](multiplayer-and-multiview.md).

### 5. World residency must become separate from world existence

A large game cannot keep every authored thing participating in queries, physics,
presentation, AI and rendering merely because it exists. Godot recommends tiling
large worlds and notes that REMOVING nodes from the active tree beats hiding or
pausing them. Ambition needs active / prewarmed / dormant residency, with
dormant actors still existing semantically and persistently.

Home: [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md).

### 6. A bulk-instance lane, eventually

ECS entities stay the normal expressive representation, but leaves, sparks,
background decoration, debris, crowd visuals and other massive homogeneous sets
do not each need a boss's worth of scheduling and projection machinery.

⛔ NOT YET, and ⛔ NOT A CUSTOM RENDERER. Bevy already provides the batching and
instancing infrastructure. The scaling benchmarks have to say where this lane
starts paying before anything is built.

### 7. The schedule should converge on few barriers and many parallel jobs

⭐ **THE RAW SYSTEM COUNT IS NOT THE PROBLEM.** 876 systems is fine if Bevy can
parallelize them. The concerning signals are repeated run-condition evaluations,
excessive command flushes, unnecessary exclusivity, and cross-domain scheduling
constraints. Capability activation belongs at coarse boundaries, not in hundreds
of independent predicates.

⚠ One measured caveat on the current shape: **822 of 887 systems sit in
`Update`**, so whatever per-system overhead exists is paid 822 times in one
schedule.

### 8. Resource residency and rendering state need an engine-level policy

Textures, atlases, materials, shaders, pipelines, offscreen targets and audio
banks need explicit ownership and lifetimes. A room transition should be able to
say what is active, what is prewarmed and what can be released. Repeated content
should share materials and atlases rather than accidentally breaking batching.

⚠ OPEN QUESTION, NOT A FINDING: whether Ambition's presentation representation
is already defeating Bevy's 2D batching is UNMEASURED. That is a profiler
question and it should be answered before any of this is designed.

### 9. Developer-host machinery must become orthogonal to the runtime

Hot reload, forensic recording, inspectors, overlays, profiling and editor
integration are valuable and stay. They should be HOST capabilities with
controlled cost, not inseparable from a production runtime.

⚠ Partly already addressed: the hot-reload watcher's per-frame change
announcement was fixed in `6e6a5ce12`. What remains is the blocking
`fs::metadata` on the main thread and the two per-frame trace recorders. See
D-PERF-4.

### 10. Performance contracts as part of the architecture

The measurement repository should hold CURVES AND BUDGETS, not point
measurements: frame cost against entities, sprites, moving sprites, cameras,
portal views, collision bodies and active capabilities — so that "this refactor
doubled run-condition evaluations" is caught automatically.

⭐ The machine-readable half of this now exists: `runtime_frame_cost.jsonl` plus
`scripts/perf_history.py`, with a comparability key that refuses to subtract a
software-rendered run from a hardware-GPU one, or a Tracy frame time from an
unprofiled one. What is missing is the CURVES — the scaling scenarios.

## The architecture this is heading toward

```text
Game / Experience
        |
        v
Capability Composition
        |
        +-----------------------+
        |                       |
        v                       v
Authoritative Runtime       Host Services
        |                   hot reload / editor
        |                   forensic / profiler
        v
Deterministic Phase Kernel
        |
  +-----+------+---------+
  |            |         |
Combat       Portals     AI        ...
active only  active only active only
  |            |         |
  +-----+------+---------+
        |
        v
Change-driven projections
        |
        v
Presentation / Audio / UI
        |
        v
Semantic render views
        |
        v
Bevy rendering backend
```

And orthogonally:

```text
World state
   |
   +-- active residency
   +-- prewarmed residency
   +-- dormant / persistent state
```

## ⛔ What NOT to do

Stated as prohibitions because each is a plausible-sounding move that would cost
a campaign:

- ⛔ do not replace Bevy ECS;
- ⛔ do not build a custom renderer yet;
- ⛔ do not dynamically uninstall Bevy plugins as the primary mechanism;
- ⛔ do not merge hundreds of systems just to lower the count;
- ⛔ do not weaken portals, rollback, multiple views or rich gameplay to hit a
  performance target;
- ⛔ do not put a general `shared_tangle` cleanup near the top unless it enables
  one of the boundaries above;
- ⛔ do not try to beat Godot everywhere.

## The order to execute in

1. **runtime capability activation and composition** — ⛔ RE-SCOPED TWICE AND NOW
   MEASURED: removing four whole experiences from a Smash match changed the frame
   by nothing outside noise. Pursue it for composition clarity, cost ownership and
   STARTUP (plugin build scales with registered systems, and startup is what a
   phone player feels). ⛔ Do not pursue it for frame time;
2. **change-driven projection architecture** — the one with measured per-frame
   cost behind it today;
3. **render-view and residency ownership**.

Those three address what has actually been observed and lay the foundation for
the later bulk-instance and streaming work. Done well, the engine stops
behaving like a giant Bevy app containing every game, and starts behaving like a
runtime whose work follows the game being played.

⚠ On the evidence available on 2026-08-29, item 2 has more measured per-frame
cost behind it than item 1 does. The ordering above is the review's; the
measurement suggests 2 may deserve to go first, and whoever picks this up should
settle that with a measurement rather than by preference.
