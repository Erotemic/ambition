# Roadmap — Ambition and Engine 1.0

Current facts are in [`status.md`](status.md). Immediate execution is in
[`queue.md`](queue.md). [`tracks.md`](tracks.md) is the standing reservoir.

## North star

Ambition is the flagship game. The product thesis remains a systemic 2D
platforming world: persistent actors and objects, embodied capability/item
progression, open-world traversal, reactive characters, multiplayer residency,
and agent-native authoring.

The engine becomes reusable by making those capabilities ordinary Bevy
plugins/crates and a semantic SDK rather than by exposing Ambition's historical
crate topology.

## Engineering priority order

### P0 — authoritative-state correctness and lifetime boundaries

The immediate correctness program is broader than rollback registration. An
authoritative population needs the right rewind codec and participation, stable
semantic identity where reconstruction or peer selection depends on it,
deterministic composition when multiple entities affect one result, and the
correct gameplay-session/timeline owner.

`26ec7b19` closed the demonstrated cross-game rollback-health leak by making
rollback authority gameplay-session-owned while preserving same-session health
across timeline rebases. Remaining work includes runtime-created populations,
residual non-rewinding memory, deterministic selection/composition, and related
structural tests.

Owner: [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md).

### P1 — canonical construction and reconstitution

Fresh room construction, confirmed room transitions, same-room replay,
checkpoint/save restoration, and persistent occurrence reconstruction should
consume one semantic construction model rather than maintain independent reset
or reconstruction ledgers.

Prepared transactional construction and the transition readiness/authorization
transaction already exist. The next work is convergence, not another snapshot
engine.

Owner: [`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md).

### P2 — persistent systemic world foundation

Build world residency, occurrence lifetime/provenance, item custody, body/item
capability gating, persistent actor population, and platformer reachability on
the P0/P1 ownership and reconstruction model.

Owners:

- [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md)
- [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md)
- [`engine/capability-progression-and-world-gating.md`](engine/capability-progression-and-world-gating.md)
- [`engine/platformer-navigation-and-reachability.md`](engine/platformer-navigation-and-reachability.md)

### P3 — measured runtime quality and developer iteration

Treat performance as several measured problems rather than one generic ECS
optimization agenda:

- weak-GPU framebuffer/raster cost;
- asset demand, render materialization and residency;
- startup only where measured;
- build/test/profile iteration cost.

Do not revive generic system-count reduction, broad change-driven projection,
parallel `GgrsSchedule`, or capability stripping as CPU work without new
evidence.

Owners:

- [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md)
- [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md)
- [`engine/project-build-and-distribution.md`](engine/project-build-and-distribution.md)

### P4 — residual actor kernel, capability composition and SDK boundaries

Continue actor-monolith decomposition where a carve removes real authority or
dependency coupling. The target is a coherent residual actor/body simulation
kernel, not an arbitrary line count.

Capability composition remains important for dependency closure, test isolation,
platform composition, reusable packages and the public SDK. Current measurement
does not justify it as a frame-time/startup optimization.

Owners:

- [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
- [`engine/controlled-character-actor-kernel.md`](engine/controlled-character-actor-kernel.md)
- [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md)
- [`engine/public-sdk-1.0.md`](engine/public-sdk-1.0.md)

### P5 — multiplayer and multiview

Apply the same participant, actor, lifetime, world-residency and presentation
semantics to local, online and mixed participants, shared/fixed/adaptive split
presentation and eventually different-room play.

Owners: [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md)
and [`game/multiplayer.md`](game/multiplayer.md).

### P6 — reactive world, characters and authored orchestration

Expose deterministic world truth and observations first. Let character
AI/dialogue and authored orchestration consume typed facts/actions without
creating a second source of authoritative state.

Owners:

- [`engine/world-facts-observations-and-memory.md`](engine/world-facts-observations-and-memory.md)
- [`engine/agentic-character-runtime.md`](engine/agentic-character-runtime.md)
- [`engine/authored-gameplay-logic-and-orchestration.md`](engine/authored-gameplay-logic-and-orchestration.md)
- [`game/reactive-characters-and-dialogue.md`](game/reactive-characters-and-dialogue.md)

## Controlled-character work is no longer a roadmap gate

The first major decision-authority convergence has landed. Remaining
controlled-character work is a bounded residual-kernel/control integration
problem and should proceed when it closes real duplicate authority or supports a
customer. It is not a prerequisite for every open-world or architecture slice.

## Ambition build order

Use [`game/open-world-roadmap.md`](game/open-world-roadmap.md) and
[`game/systemic-progression.md`](game/systemic-progression.md).

The build order remains **world first, story over reality**. Prove a large
persistent world, traversal/capabilities, items/mechanisms, persistent/spawned
actors, and save/load coherence before relying on a linear story spine to provide
meaning. Story should consume the same world facts rather than substitute for
them.

## Bevy package direction

Durable package/decomposition doctrine is in
[`../architecture/package-and-capability-boundaries.md`](../architecture/package-and-capability-boundaries.md).
A reusable domain owns its vocabulary and plugin registration, depends downward,
and is testable in a small host. Extract or publish independently only when the
API has a real game-independent customer.

## Ambiguity policy

Focused plans must distinguish settled direction from open design questions. An
agent may investigate an unresolved question when a concrete slice requires it,
but should not turn an under-specified product choice into architecture merely to
continue execution. Genuine maintainer choices go to
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).
