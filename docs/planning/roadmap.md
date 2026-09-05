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

⭐⭐ **THE PRIORITIES BELOW ARE SCORED ON TWO AXES, NOT ONE.** Authority
decomposition — which crate owns the fact, what may mutate it, one lifecycle,
dependency direction — and **capability composability**: can this capability be
ABSENT, does the rest still form a coherent application, does it declare only its
real prerequisites. **The second does not follow from the first.** A repository
can satisfy every ownership rule on every page here and still ship an
effectively indivisible engine.

⇒ Authority comes FIRST and sequencing is permitted; a slice need not deliver
both. What is not permitted is a run of slices that all advance the first axis
being read as progress on the engine's decomposition. **A landed slice says which
axis it moved.**

ⓘ The rule, the ordering, the absence criterion and the minimum-host tests that
would prove it live in
[`engine/decomposition.md`](engine/decomposition.md) under "Decomposition has two
dimensions", with the durable statement in
[`../architecture/package-and-capability-boundaries.md`](../architecture/package-and-capability-boundaries.md).
Deliberately not restated here: that page says in its own words that it is the
single home, and a second copy is how the two vocabularies drifted apart in the
first place. ⚠ **This roadmap named neither axis until 2026-09-04** — a search
for the concept as well as the spelling (*optional*, *install*, *minimal
consumer*, *inherit*) returned nothing on this page, while
`engine/actor-monolith-decomposition.md` had been carrying it as exit criteria 2
and 4 all along.

### P0 — authoritative-state correctness and lifetime boundaries

The immediate correctness program is broader than rollback registration. An
authoritative population needs the right rewind codec and participation, stable
semantic identity where reconstruction or peer selection depends on it,
deterministic composition when multiple entities affect one result, and the
correct gameplay-session/timeline owner.

`26ec7b19` closed the demonstrated cross-game rollback-health leak by making
rollback authority gameplay-session-owned while preserving same-session health
across timeline rebases. Remaining work includes runtime-created populations,
deterministic selection/composition, and related structural tests
(non-rewinding memory closed 2026-09-02, S2).

Owner: [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md).

### P1 — canonical construction and reconstitution

Fresh room construction, confirmed room transitions, same-room replay,
checkpoint/save restoration, and persistent occurrence reconstruction should
consume one semantic construction model rather than maintain independent reset
or reconstruction ledgers.

Prepared transactional construction, the transition readiness/authorization
transaction, the same-room replay, and — since 2026-08-31 (`758e9df37`) — the
durable-restore leg all run one constructor: a save load prepares its first room
against the saved occurrence facts at the activation edge rather than building
the room and correcting it. This item is closed; the owner doc's C3 records the
proof.

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

⭐ **WHERE P2 STANDS, 2026-09-04 — three of these four moved and the fourth is
deliberately parked.** Recorded here because the tier's own sentence ("build
world residency, occurrence lifetime/provenance, item custody, body/item
capability gating…") reads as five unbuilt things and three of them are done:

- **Body/item capability gating: the ENGINE publishes TEN conditions and every
  one is reachable from an authored route** — `gated_by` is a condition LINE, so
  a wall reads any of them. Re-derived 2026-09-04 evening by
  `scripts/authored_route_gates.py`, which reads the ids out of the source
  rather than keeping a list: `body.can`, `body.fits`, `boss.cleared`,
  `custody.is_held`, `encounter.cleared`, `inventory.holds`, `quest.active`,
  `wallet.can_afford`, `world.flag_set`, `world.switch_on`. ⚠ **RUN IT rather
  than quoting this** — the figure moved 6 → 9 → 10 in a single day, and the
  phrasing this replaced ("five of the seven gate families", itself corrected
  from "all seven") had already been wrong twice.
  ⛔ **What is left is FACTS, not predicates**, and that is unchanged: soft
  systemic pressure and social/knowledge have nothing route-facing to read,
  because no durable fact records them —
  [`engine/world-facts-observations-and-memory.md`](engine/world-facts-observations-and-memory.md)
  measures the save's fourteen families and neither is among them.
  ⚠ **And the world does not USE the vocabulary**: the whole authored corpus of
  route gates is THREE walls, two gated, both on the same story flag, and five
  of the ten conditions are authored nowhere at all. Whether that is a content
  gap worth closing is question 55, not an engine deficit.
- **Item custody: every migration row's exploration half is closed.** The
  remainder is two maintainer decisions and one fighter-side call site.
- **Occurrence lifetime: an occurrence enters the whereabouts ledger through
  custody and the ledger enforces it**, so a room unload cannot silently erase a
  persistent instance — the three classes it can destroy are carried, remembered
  or as-authored, and the fourth (`SpawnOrigin::Dynamic`) is not a persistent
  instance by definition.
- ⛔ **World residency is UNBUILT and should stay so until a customer needs two
  resident rooms.** `RoomSet.active` is a `usize`, singular by type. Its own
  page says this is a sequencing dependency and not a licence to build a
  universal world scheduler, and with one resident room every residency query
  has a trivial answer. Building the vocabulary before the customer would be the
  speculative work three of these four pages each forbid in their own words.

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
