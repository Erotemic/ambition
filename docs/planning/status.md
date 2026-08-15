# HEAD orientation

**Snapshot:** `834407e0abf1` (2026-08-14 local project date).

This page is a cold-start map, not an execution queue and not a completion
diary. [`queue-72h-2026-08-08.md`](queue-72h-2026-08-08.md) is the continuing
execution authority. [`tracks.md`](tracks.md) is the standing reservoir used to
replenish it. Focused plans own technical design.

If this page disagrees with current source or a focused open plan, update this
page rather than appending an archaeological correction.

## Major closure: D73 is finished

The authority-convergence campaign closed on 2026-08-13. The live architecture
no longer has an enemy `ArchetypeSpec` / `CharacterRoster` body authority or a
build-legacy-body-then-patch character road. Intrinsic body/capability facts come
from authored/prepared `CharacterDefinition`; placement, disposition,
controller, participant and ruleset facts remain contextual.

The migration working memory is archived under
[`../archive/planning-superseded/2026-08-13/`](../archive/planning-superseded/2026-08-13/).
Do not reconstruct deleted D73 representations because an archived review names
them.

## Current architectural direction

The successor umbrella is
[`engine/engine-1.0-architecture-program.md`](engine/engine-1.0-architecture-program.md).
The goal is a credible Godot/Unity-class 2D engine on Bevy while **Ambition
remains the flagship game and primary product driver**.

The highest-value successor fronts are:

1. **Ambition authoring + kinematic world objects.** Treat authoring/tooling as
   an engine product, improve LDtk as a first-class spatial compiler surface,
   and use moving platforms as the first vertical slice. See
   [`engine/authoring-and-tools.md`](engine/authoring-and-tools.md) and
   [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md)
   and [`engine/kinematic-world-objects.md`](engine/kinematic-world-objects.md).
2. **Ambition multiplayer + multi-view presentation.** Support local, online and
   mixed participants independently of shared/fixed/adaptive split-screen; grow
   toward multiple resident rooms when participants separate. See
   [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md)
   and [`game/multiplayer.md`](game/multiplayer.md).
3. **⭐ THE SYSTEMIC WORLD SUBSTRATE — the next major frontier** (D125). What a
   thing IS, which runtime occurrence it is, why it exists and how long it lasts;
   then item custody as the first demanding consumer, then capability-driven
   gating and reachability, then residency and persistent populations. Its seven
   focused plans were all written and reachable only from
   [`tracks.md`](tracks.md) until 2026-08-14 — the design was never the gap.
4. **Simulation authority and determinism.** Decompose parameter-ceiling systems
   by phase/authority and invert rollback declaration ownership. See
   [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md).
5. **Capability/runtime composition.** Make optional capabilities honest in
   dependency and composition topology. See
   [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).
6. **Public SDK, authoring ergonomics, performance and iteration.** See
   [`engine/public-sdk-1.0.md`](engine/public-sdk-1.0.md) and
   [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md).

⚠ **the browser is a TEST FIXTURE, not a front** (Jon, 2026-08-14). It is a
powerful architecture probe while the engine is decomposed — it found a shipped
composition that differed from desktop's and a developer instrument that was
load-bearing for gameplay input — but it does not decide which subsystem gets
built next. ⭐ **the test for any tempting performance task: would we want this
abstraction if the web target disappeared tomorrow?** Semantic asset readiness,
cross-platform phase telemetry, canonical asset publication, host-owned input and
an explainable load barrier all pass it. Brotli, wasm audio scheduling, Hall
streaming, a generic residency scheduler and byte shaving do not.

## Product and engine customers

- **Ambition:** flagship game. Its real content, authoring, multiplayer,
  persistence and presentation needs have first claim on product value.
- **Super Smash Siblings:** serious platform-fighter customer and possible future
  first-class game, but not the project focus. Its remaining body-generic work is
  in [`smash-body-generic-combat-2026-08-09.md`](smash-body-generic-combat-2026-08-09.md).
- **TwinTrack:** strongest current pressure test for independent views and
  observer/reference-frame presentation; split-screen should exercise the same
  multi-view model Ambition uses.
- **Sanic / Super Mary-O / Hollow Lite:** retained acceptance customers for
  movement, classic platforming/content, and encounters/boss authoring.

An acceptance customer may eventually become a first-class game. That changes
its product investment, not the engine ownership rules.

## Durable architecture to remember

- one body, one path;
- character definitions own intrinsic reusable body composition;
- controllers provide intent rather than defining a body species;
- construction/preparation fails before partial mutation;
- deterministic simulation authority is explicit and snapshotable;
- views are local presentation over one simulation, not duplicate worlds;
- transport, control assignment, world residency and view layout are independent
  axes;
- LDtk is Ambition's preferred spatial authoring surface and should improve when
  real Ambition content outgrows it;
- the actor monolith is drained by coherent ownership, not line-count quotas;
- public APIs should expose game concepts rather than historical crate topology.

## Explicitly deferred, not abandoned

- production online transport/Matchbox work should grow from an actual
  multiplayer slice rather than be built speculatively;
- Slower Light remains a future 3D relativity game;
- water/oil extensions to falling-sand remain desired deferred product ideas;
- the Leafwing clash-scan optimization remains trigger-based maintenance.

## Where to look next

1. [`queue-72h-2026-08-08.md`](queue-72h-2026-08-08.md) for execution order.
2. The focused plan named by the selected row.
3. [`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md)
   for direct maintainer observations.
4. [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) only when
   an actual product/feel decision is required.
5. [`tracks.md`](tracks.md) when replenishing the queue.
6. `docs/concepts/`, `docs/systems/`, `docs/architecture/` and `docs/adr/` for
   settled truth; `docs/archive/` for history.
