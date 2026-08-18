# Engine 1.0 architecture program

**State:** OPEN — Ambition-first successor program after D73.

This is the long-horizon architecture program for turning the engine under
Ambition into a credible Godot/Unity-class 2D engine on top of Bevy, with a more
specific product thesis: **systemic 2D worlds plus agent-native authoring**.

The live 72-hour queue chooses execution slices. This document explains the
capability fronts those slices are trying to converge toward.

## Product order

**Ambition is the flagship game and primary product driver.** The near-term
product target is not editor parity for its own sake. It is a large, persistent,
reactive 2D platforming world with embodied capability progression, meaningful
objects and actors, multiplayer residency, and strong LLM-native authoring.

The governing oracle is:

> Can Ambition use the capability deeply while another game can opt into the
> same capability through supported Bevy/plugin/provider seams without editing
> Ambition-specific engine code?

## Immediate priority — controlled-character actor kernel

Before starting large new world or multiview implementations, finish the runtime
actor/control boundary far enough that generic simulation no longer means
"PrimaryPlayer".

Use [`controlled-character-actor-kernel.md`](controlled-character-actor-kernel.md),
[`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md)
and [`actor-monolith-decomposition.md`](actor-monolith-decomposition.md).

This is the highest-leverage prerequisite for multiplayer, persistent actors,
navigation, possession, item custody and future crate extraction.

## Capability programs

### E1 — agent-native authoring, LDtk and kinematic world objects

Use [`authoring-and-tools.md`](authoring-and-tools.md),
[`ldtk-authoring-and-world-tools.md`](ldtk-authoring-and-world-tools.md) and
[`kinematic-world-objects.md`](kinematic-world-objects.md).

The authoring product is semantic discovery/inspection/mutation/validation for
agents first. Human graphical frontends can sit over the same source semantics
later. Moving platforms are the first spatial/dynamic-world vertical slice.

### E2 — simulation authority and determinism

Use [`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md).

Make deterministic behavior emerge from explicit ownership and phase structure,
not schedule-topology accidents, tuple-packed god systems, mirrored authority or
a generic rollback census of every domain.

### E3 — multiplayer, multiview and room residency

Use [`multiplayer-and-multiview.md`](multiplayer-and-multiview.md),
[`camera-reference-frame-policy.md`](camera-reference-frame-policy.md) and
[`../game/multiplayer.md`](../game/multiplayer.md).

Transport, control assignment, world residency and presentation layout remain
separate axes. Ambition should support local/remote/mixed participants, shared,
fixed-split and adaptive split views, and different-room exploration.

### E4 — capability and runtime composition

Use [`capability-and-runtime-composition.md`](capability-and-runtime-composition.md).

Consumers should opt into coherent capabilities without silently inheriting
unrelated bosses, portals, persistence, rollback adapters, UI or Ambition policy.

### E5 — public SDK 1.0

Use [`public-sdk-1.0.md`](public-sdk-1.0.md).

Expose semantic game concepts rather than historical crate topology.

### E6 — performance and iteration

Use [`performance-and-iteration.md`](performance-and-iteration.md).

Compile fanout, runtime/mobile budgets, asset residency, multiview cost,
headless throughput and agent iteration latency are engine ergonomics.

### E7 — persistent systemic open world

Use:

- [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md)
- [`instance-lifetime-provenance-and-persistence.md`](instance-lifetime-provenance-and-persistence.md)
- [`item-custody-and-accounting.md`](item-custody-and-accounting.md)
- [`capability-progression-and-world-gating.md`](capability-progression-and-world-gating.md)
- [`platformer-navigation-and-reachability.md`](platformer-navigation-and-reachability.md)
- [`persistent-actors-and-population.md`](persistent-actors-and-population.md)

The world should remain coherent when rooms unload, important actors/items move,
spawned mobs come and go, participants separate, and traversal changes because
of actual capabilities/items/world mechanisms rather than story-stage switches.

### E8 — world facts and agentic characters

Use [`world-facts-observations-and-memory.md`](world-facts-observations-and-memory.md)
and [`agentic-character-runtime.md`](agentic-character-runtime.md).

The simulation owns reality. Character AI consumes observations/facts/memory and
selects typed actions/dialogue without inventing authoritative world state.

### E9 — presentation and observability

Use:

- [`render-animation-and-vfx.md`](render-animation-and-vfx.md)
- [`ui-localization-and-accessibility.md`](ui-localization-and-accessibility.md)
- [`inspection-diagnostics-and-workbench.md`](inspection-diagnostics-and-workbench.md)

Multiview, HUD/focus, animation/VFX and machine-readable inspection should form
coherent downstream Bevy plugins rather than another application-shaped tangle.

### E10 — project lifecycle and extension

Use [`project-build-and-distribution.md`](project-build-and-distribution.md),
[`extension-model.md`](extension-model.md) and
[`reusable-authored-world-composition.md`](reusable-authored-world-composition.md).

Make the project lifecycle explicit while keeping scripting/prefab-like
abstractions evidence-driven rather than feature-matrix obligations.

### E11 — Bevy-native plugin and reusable crate decomposition

Use [`bevy-plugin-and-crate-strategy.md`](bevy-plugin-and-crate-strategy.md)
and [`decomposition.md`](decomposition.md).

Reusable components should increasingly look like idiomatic Bevy domains:
components/resources/messages/schedule sets owned by a domain plugin, with
registration moving with the crate. Extract or publish independently only after the API is
actually game-independent and usable through ordinary Bevy plugin/system composition.

## Cross-program rules

1. **Ambition first, reusable second, neither sacrificed.**
2. **One authoritative representation.** New abstractions remove or make
   unreachable the authority they replace.
3. **World facts precede story interpretation.** Persistent state, custody,
   capability and actor location are simulation truth; dialogue/AI may interpret
   them but not invent them.
4. **Composition over taxonomy.** Bodies, capabilities, participants, views,
   world objects and services compose.
5. **Agent-native authoring is part of the engine product.** A capability is not
   finished if an agent cannot discover, inspect, author/operate and validate it
   without archaeology.
6. **Headless and visible hosts share simulation contracts.**
7. **Transport is not gameplay ontology; views are presentation.**
8. **Prefer structural guarantees over source-policy ceremony.**
9. **Do not pre-generalize without multiple real customers.**
10. **Every focused plan names its uncertainties.** Open questions are allowed;
    implicit ambiguity that future agents mistake for doctrine is not.
11. **Crates follow ownership and registration.** A file move is not a Bevy
    boundary if the old owner still imports/registers the domain.

## Program-level exit shape

Engine 1.0 does not mean every conceivable feature exists. It means common paths
are coherent enough that a substantially different 2D game can consume them
without learning Ambition's migration history.

A credible 1.0 should make these statements unsurprising:

- Ambition is a deep open-world/systemic flagship built on supported surfaces;
- the controlled protagonist is an ordinary actor body under control authority;
- persistent actors/items/world changes survive residency transitions coherently;
- capability/item/world-state progression is queryable and navigation-aware;
- one/many participants and views can inhabit one simulation, including multiple
  rooms;
- agent-authored worlds/content prepare through semantic validated tooling;
- dynamic world geometry is ordinary authored engine data;
- optional capabilities and rollback participation follow domain ownership;
- reusable domains can become standalone Bevy plugins without dragging Ambition
  policy with them;
- public SDK, diagnostics and project workflows are usable outside Ambition.

## Open design questions — deliberately unresolved

The overall direction is intentional, but several high-level choices remain open:

- how large-world residency/background simulation should be partitioned;
- the navigation representation for dynamic platformer worlds;
- which presentation/tooling domains deserve independent ecosystem crates;
- which capabilities should remain internal plugins versus independently consumable
  Bevy crates.

**Three of these were RULED on 2026-08-17** and are recorded in
[`../maintainer-decisions.md`](../maintainer-decisions.md) rather than left here:

- **the instance-ID/persistence model** — Morrowind-style: an item in the WORLD is
  an **occurrence with identity**; an inventory **entry carries a count, and the
  count is usually 1**. The shape is uniform — twenty arrows are ONE entry with
  count 20, and uniqueness decides whether two entries may MERGE rather than how
  either is stored. ⛔ *"dict"* was vocabulary for that shape, **not** a mandate for
  a particular container; the Rust representation is the implementer's call.
  Crossing the boundary is the rule to design: a pickup merges or adds, a drop
  MINTS an occurrence, and a unique item's identity survives the round trip.
  ⛔ the general form is deliberately not built yet.
- **capability progression, body versus participant** — it **splits by what the
  verb IS**: physical verbs are body-owned because they are facts about anatomy;
  knowledge, keys and theorems are participant-owned because they are facts about
  the mind. ⛔ the open cost is a vocabulary that classifies a NEW verb at the
  verb, not at the call site.
- **deterministic versus model-backed character AI** — ⭐⭐ the question dissolves:
  **a real model-backed character is a REMOTE PLAYER**, joining through the
  participant/netcode seam, playing in realtime and taking rollback exactly as a
  human peer does. There is no non-deterministic system inside the tick to draw a
  boundary around. ⚠ RL agents and simpler models may be modelled differently, and
  that half stays open. ⛔ **DEFERRED** — Jon: *"this will not be important"* — and
  models writing dialogue is the near-term use, unaffected.

Focused plans own these questions. Do not resolve them by inference merely to
make the roadmap look complete.
