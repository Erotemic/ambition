# Roadmap — Ambition and Engine 1.0

Current facts are in [`status.md`](status.md). Immediate execution is in the
self-replenishing [`queue.md`](queue.md).
[`tracks.md`](tracks.md) is the standing reservoir.

## North star

Ambition is the flagship game. The product thesis is an unusually systemic 2D
platforming world: persistent actors and objects, embodied capability/item
progression, open-world traversal, reactive characters, multiplayer residency
and agent-native authoring.

The engine becomes a credible Godot/Unity-class 2D engine by making those
capabilities reusable through idiomatic Bevy plugins/crates and a semantic SDK.

## Priority order

### P0 — controlled-character actor kernel

Finish the post-D73 runtime refactor before widening the world model. Generic
actor simulation must stop depending on a privileged primary-player frame.

Owner: [`engine/controlled-character-actor-kernel.md`](engine/controlled-character-actor-kernel.md).

### P1 — world mechanics and authoring

Continue LDtk + kinematic world objects, especially moving platforms, against the
cleaner actor/body boundary.

Owners: [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md),
[`engine/kinematic-world-objects.md`](engine/kinematic-world-objects.md).

### P2 — persistent systemic world foundation

Build world residency, instance lifetime/provenance, item custody, body/item
capability gating and platformer reachability as one compatible family of engine
semantics.

Owners:
[`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md),
[`engine/instance-lifetime-provenance-and-persistence.md`](engine/instance-lifetime-provenance-and-persistence.md),
[`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md),
[`engine/capability-progression-and-world-gating.md`](engine/capability-progression-and-world-gating.md),
[`engine/platformer-navigation-and-reachability.md`](engine/platformer-navigation-and-reachability.md),
[`engine/persistent-actors-and-population.md`](engine/persistent-actors-and-population.md).

### P3 — multiplayer and multiview

Apply the same actor/world semantics to local, online and mixed participants,
shared/fixed/adaptive split presentation and eventually different-room play.

Owners: [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md),
[`game/multiplayer.md`](game/multiplayer.md).

### P4 — reactive world and characters

Expose authoritative world facts/observations and typed agent action seams.
Layer more ambitious reactive dialogue/AI only after world truth, navigation and
persistent actor semantics are trustworthy.

Owners: [`engine/world-facts-observations-and-memory.md`](engine/world-facts-observations-and-memory.md),
[`engine/agentic-character-runtime.md`](engine/agentic-character-runtime.md),
[`game/reactive-characters-and-dialogue.md`](game/reactive-characters-and-dialogue.md).

### P5 — presentation, SDK and productization

Rendering/animation/VFX, participant/view-aware UI, inspection/workbench,
capability composition, SDK, performance and project/distribution workflows grow
as real Ambition slices expose pressure.

## Ambition build order

Use [`game/open-world-roadmap.md`](game/open-world-roadmap.md) and
[`game/systemic-progression.md`](game/systemic-progression.md).

The build order is **world first, story over reality**. Prove a large persistent
world, traversal/capabilities, items/mechanisms, persistent/spawned actors and
save/load coherence before relying on a linear story spine to provide meaning.
The Fia/Alice/Bob and other story arcs remain desired content, but they should
inhabit a world whose state already matters.

## Bevy plugin/crate direction

Use [`engine/bevy-plugin-and-crate-strategy.md`](engine/bevy-plugin-and-crate-strategy.md).

A reusable domain should increasingly own its Bevy plugin registration and
semantic vocabulary, depend downward, and be testable in a small `App`/harness.
Independent publication is a later maturity step, not a reason to pre-generalize.

## Ambiguity policy

The roadmap intentionally contains unresolved questions. Every focused plan must
state them explicitly under **Open design questions — deliberately unresolved**.
An agent may investigate and propose an answer when a concrete implementation
slice requires it, but should not silently convert an under-specified question
into durable architecture merely to continue execution.
