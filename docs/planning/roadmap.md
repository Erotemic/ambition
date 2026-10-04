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

The priorities below are scored on two axes. The first is authority
decomposition: which crate owns the fact, what may mutate it, one lifecycle,
dependency direction. The second is capability composability: can this capability
be absent, does the rest still form a coherent application, does it declare only
its real prerequisites. The second does not follow from the first. A repository
can satisfy every ownership rule and still ship an effectively indivisible engine.

Authority comes first, and sequencing is permitted; a slice need not deliver both.
Do not read a run of authority slices as progress on decomposition. A landed slice
says which axis it moved.

The rule, the ordering, the absence criterion and the minimum-host tests live in
[`engine/architecture.md`](engine/architecture.md#decomposition-doctrine) under
"Two dimensions", with the durable statement in
[`../architecture/package-and-capability-boundaries.md`](../architecture/package-and-capability-boundaries.md).
They are not restated here.

### P0 — authoritative-state correctness and lifetime boundaries

The immediate correctness program is broader than rollback registration. An
authoritative population needs the right rewind codec and participation, stable
semantic identity where reconstruction or peer selection depends on it,
deterministic composition when multiple entities affect one result, and the
correct gameplay-session/timeline owner.

Rollback authority is gameplay-session-owned. Remaining work includes
runtime-created populations, deterministic selection/composition, and related
structural tests.

Owner: [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md).

### P1 — canonical construction and reconstitution

**Converged.** Fresh construction, room transition, same-room replay, new-game
reset and save restore run one constructor. A save load adopts its occurrence
ledger at activation, before initial construction. Each new session or room is a
hidden candidate that is verified before one publication switch; a failed
candidate leaves the live world unchanged. The shape is stated in
[`../architecture/engine-architecture.md`](../architecture/engine-architecture.md#content-and-construction).
New reset or reconstruction roads must use this model, not a separate ledger.

Owner: [`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md).

### Fast iteration - current cross-program priority

The maintainer's edit-to-play latency is a current architecture requirement, not
only the lower-priority profiling work below.

Where it stands: content packs compile through one path
(`ambition_content_pack::compile`) with no Rust rebuild. A development build
reads content off disk, and a running game plays a saved edit to movement
defaults, combat and time feel, boss tuning, dialogue, items, audio registries,
fighter facets and the character catalog. A session reads the content generation
it was prepared against. Named content keeps moving out of Rust tables into pack
schemas (quests, cutscenes, music cues, room bindings).

Remaining work: one prepare/admit/publish contract across every reloadable
registry, and procedural modules with shared rollback state. The owners are the
I2/I3 row in [the queue](queue.md) and the
[extension model](engine/extension-model.md). Do not wait for a mod marketplace,
every demo, or a whole actor-monolith carve.

### P2 — persistent systemic world foundation

Build world residency, occurrence lifetime/provenance, item custody, body/item
capability gating, persistent actor population, and platformer reachability on
the P0/P1 ownership and reconstruction model.

Owners:

- [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md)
- [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md)
- [`engine/capability-progression-and-world-gating.md`](engine/capability-progression-and-world-gating.md)
- [`engine/platformer-navigation-and-reachability.md`](engine/platformer-navigation-and-reachability.md)

Where P2 stands:

- **Body/item capability gating.** The engine publishes route-facing conditions,
  and a route gate (`gated_by`) can read any of them. Run
  `scripts/authored_route_gates.py` for the current list; do not copy it here.
  What is missing is facts, not predicates: soft systemic pressure,
  social/knowledge state and broken world mechanisms have no durable
  route-facing record. A smashed breakable is session state restored by
  re-authoring, by design (`SpawnOrigin::Dynamic` is not persisted).
  [`engine/world-facts-observations-and-memory.md`](engine/world-facts-observations-and-memory.md)
  owns the durable families. The authored world uses few of the conditions;
  whether that is a content gap is question 55, not an engine deficit.
- **Item custody.** The exploration half of every migration row is closed. The
  remainder is two maintainer decisions and one fighter-side call site.
- **Occurrence lifetime.** An occurrence enters the whereabouts ledger through
  custody, and the ledger enforces it, so a room unload cannot silently erase a
  persistent instance.
- **World residency.** Several rooms can be live at once. Each live room is its
  own root entity, entities carry an `InRoomInstance` stamp, and each view draws
  the room it frames. Separated players each keep their room live. A room that
  is not live does not simulate; a mechanic that must keep time uses the session
  clock. Open work (the OW cuts and the remaining `SoleLiveRoom` readers) is in
  [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md).

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

### P4 - ownership-based engine composition and supported public profiles

Use [`engine/architecture.md`](engine/architecture.md) for the target authorities
and the [bounded packets](engine/actor-monolith-work-frontier.md) for their state.
Landed: A1 checkpoint restoration, A2 projectile contacts, A8 several live rooms,
A10 candidate construction, A11a/A11b installed technique support. Open: A3
acceptance, A4 body-execution regrouping, A5 destructibles, A6/A7 definition and
item separation, A9 minimal profiles, A11c and A12b. These are independent work
streams, not sequential prerequisites for game development. The frontier's A12
(flow bounds) is not `queue.md`'s A12 (move-contact attribution). The
[queue](queue.md) selects current priority.

The outcome is a set of recognizable state/behavior/lifetime authorities and a
public programmatic engine that can be used without accidental flagship
requirements. Preserve the corrected spawn boundary, coherent internal cycles,
explicit composition and normal downward dependencies. Do not optimize for crate
count, zero foreign installs, zero SCCs or a cosmetically renamed runtime.

### P5 — multiplayer and multiview

Apply the same participant, actor, lifetime, world-residency and presentation
semantics to local, online and mixed participants and to shared/fixed/adaptive
split presentation. Local different-room play exists in the engine: separated seats
each keep a live room and each view draws its own room. Ambition has no
production join road for a second seat yet (Q153). Online transport waits for a real
customer.

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

## Cross-cutting Engine 1.0 competitive capability bar

The priority order above is architecture sequencing. Engine 1.0 also has a
**product completeness bar**: it must be able to support serious 2D games across
ordinary rendering, movement/collision, animation/VFX, audio, UI, input, assets,
persistence, diagnostics, headless testing and project build/package concerns.

That bar is owned by
[`engine/godot-class-2d-capability.md`](engine/godot-class-2d-capability.md).
It is deliberately not an editor roadmap. Ambition competes through engine
capability, runtime/build efficiency, semantic expressiveness, public composition,
inspectability and LLM-first operation.

Use the capability map as a **gap detector**, not as a second queue:

1. a real game/customer exposes a missing ordinary engine capability;
2. check whether Bevy or a maintained ecosystem plugin already supplies the
   generic mechanism;
3. identify the semantic/composition/public layer Ambition actually lacks;
4. route the executable slice to the focused plan and queue;
5. verify it through Ambition plus a materially different customer where the
   boundary is supposed to be reusable.

Do not promote visual-editor parity, visual scripting, a scripting-language
clone, a plugin marketplace, general 3D breadth, or generic rigid-body ownership
merely to make the feature list resemble Godot.

The current highest-value competitive gaps line up with the architecture order:
correct deterministic/lifetime semantics; canonical reconstitution; persistent
world behavior; public SDK/capability closure; asset/raster/runtime quality;
project build/package; structured diagnostics; authored orchestration; and
multiplayer/multiview maturity. Presentation/UI/audio gaps should be filled from
real game pressure rather than replacement-framework campaigns.

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
