# Engine 1.0 architecture program

**State:** OPEN — Ambition-first architecture program.

This document is the capability map. It does not own execution order; use
[`../queue.md`](../queue.md). It does not duplicate current repository status;
use [`../status.md`](../status.md).

## Product order

Ambition is the flagship and primary architecture driver. The near-term product
target is a large, persistent, reactive 2D platforming world with embodied
capability progression, meaningful objects and actors, multiplayer-compatible
residency, and strong agent-native authoring.

The governing oracle remains:

> Can Ambition use the capability deeply while another game can opt into the same
> capability through supported Bevy/plugin/provider seams without editing
> Ambition-specific engine code?

Engine architecture is only one part of the 1.0 bar. A credible engine product
also needs ordinary 2D capability completeness, measured runtime/build
efficiency, a coherent public extension surface, structured diagnostics, and an
agent-operable path from intent to validated/packageable game change. The
cross-program acceptance map is
[`godot-class-2d-capability.md`](godot-class-2d-capability.md).

That comparison is about **engine capability and expressiveness**, not editor
parity. Visual/manual tools are optional frontends; LLM-first semantic operation
is the preferred authoring model.

## Current architecture gate

The immediate gate is **authoritative-state correctness across reconstruction and
lifetime boundaries**.

The last rollback campaign established that these are independent properties:

- rewind codec;
- rollback entity participation;
- stable semantic identity;
- deterministic selection/composition;
- gameplay-session/timeline ownership.

`26ec7b19` added the missing gameplay-session ownership to rollback authority:
health carries across timeline generations only for the same `SessionScopeId`, a
foreign session reads no health from it, and session-mirrored process resources
are re-established at activation. ADR 0027 owns the durable rule.

The next architectural convergence is canonical **construction and
reconstitution**: new room, transition, replay, restore, and persistent
occurrence reconstruction should share semantic construction rather than grow
parallel reset/restore ledgers.

Use [`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md)
and [`construction-and-reconstitution.md`](construction-and-reconstitution.md).

### Re-measured 2026-09-03 — the convergence is narrower than the sentence above

*"New room, transition, replay, restore, and persistent occurrence
reconstruction should share semantic construction"* reads as one program over
five operations. Measured against the kernel, those operations already go
through **three different canonical authorities**, and only one of them is
construction:

| authority | what it owns | example |
|---|---|---|
| **construction** | building an authoritative occurrence | `ConstructionDomain` — 3 production implementors: `ActorConstruction`, `GravityZoneConstruction`, `PortalGunConstruction` (the last outside the kernel entirely) |
| **transit** | relocating a body that already exists | `restore_checkpoint_on_session_start` moves through `ae::movement::transit_body`, *"the ONE transit authority (ADR 0024)"* — not a raw position write |
| **resource hydration** | save → resources, no entity involved | `restore_inventory_from_save` writes `OwnedItems`, `BodyWallet` and the minted baselines |

The kernel has **12** `reset_*`/`restore_*` entry points. They distribute across
those three, plus in-place baseline restores that argue their own case:
`EnemyState::reset_to_spawn` deliberately re-projects nothing, because
*"`tuning`/`brain_profile` are projected once at spawn and never mutate at
runtime … they already hold the baseline"*.

⇒ **The paths that actually RECONSTRUCT already reach the construction road** —
`items/pickup/mod.rs`, `items/pickup/minted_horizon.rs` and `session/reset/mod.rs`
all name it. The ones that do not are not parallel ledgers; they are a different
authority doing a different job.
⚠ **So the risk this gate names is real but should be stated as one question, not
five:** *does a new ENTITY-reconstruction path grow its own ledger instead of
naming a `ConstructionDomain`?* A reader who takes the sentence at face value
will go looking for convergence between checkpoint transit and inventory
hydration, which should never converge — they share a trigger (a restore) and
nothing else.

## Capability programs

### E1 — simulation authority, determinism and lifetime

Make deterministic behavior emerge from explicit ownership, semantic phases,
stable identities and defined composition rather than query order, non-rewinding
scratch state, mirrored authority, or process-global state with ambiguous
lifetime.

Owner: [`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md).

### E2 — construction and reconstitution

Prepared immutable content lowers into typed construction lanes under one
transaction. Fresh construction, confirmed lifecycle transitions, replay and
restore should converge on that model. Persistence stores durable facts rather
than becoming another ephemeral rollback engine.

Owner: [`construction-and-reconstitution.md`](construction-and-reconstitution.md).

### E3 — persistent systemic open world

Use [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md),
[`item-custody-and-accounting.md`](item-custody-and-accounting.md),
[`capability-progression-and-world-gating.md`](capability-progression-and-world-gating.md)
and [`platformer-navigation-and-reachability.md`](platformer-navigation-and-reachability.md).

The world should remain coherent when rooms unload, important actors/items move,
spawned populations come and go, participants separate, and traversal changes
because of actual capabilities/items/world mechanisms.

### E4 — actor kernel and domain ownership

The target is a small coherent actor/body simulation kernel, not a small file.
Move boss, encounter, item, portal, persistence, presentation, developer and
other domain authority when a real ownership boundary exists. Carves should
reduce dependency/change fanout or improve capability/test/package isolation.

Owners:

- [`actor-monolith-decomposition.md`](actor-monolith-decomposition.md)
- [`controlled-character-actor-kernel.md`](controlled-character-actor-kernel.md)

### E5 — capability and runtime composition

Consumers should opt into coherent semantic capabilities without silently
inheriting unrelated domains. Capability work is justified by dependency
closure, host composition, testing and SDK quality; measured experience removal
did not materially improve frame time or plugin-registration startup.

Owner: [`capability-and-runtime-composition.md`](capability-and-runtime-composition.md).

### E6 — public SDK 1.0

Expose semantic game concepts rather than internal crate topology. Keep
implementation carves behind the facade and prove APIs through minimal/external
consumers.

Owner: [`public-sdk-1.0.md`](public-sdk-1.0.md).

### E7 — performance, assets and iteration

Treat performance as measured domains:

- simulation CPU;
- rendered weak-GPU raster cost;
- asset preparation/device materialization/residency;
- startup where measured;
- build/test/profile iteration.

Current evidence says representative simulation CPU is healthy and broad generic
CPU campaigns are low priority. Raster scale and asset materialization have
shown direct user-visible cost.

Owners:

- [`performance-and-iteration.md`](performance-and-iteration.md)
- [`asset-preparation-and-residency.md`](asset-preparation-and-residency.md)
- [`project-build-and-distribution.md`](project-build-and-distribution.md)

### E8 — multiplayer, multiview and multi-room residency

Transport, participant/control assignment, world residency and presentation
layout are separate axes. Build local/remote/mixed participants and N-view
presentation from the same session/actor/world semantics rather than introducing
multiplayer-only ontology.

Owners: [`multiplayer-and-multiview.md`](multiplayer-and-multiview.md),
[`../../systems/camera-reference-frames.md`](../../systems/camera-reference-frames.md), and
[`../game/multiplayer.md`](../game/multiplayer.md).

### E9 — agent-native authoring and world tools

Agent-native authoring means semantic discovery, inspection, mutation and
validation against the same content model the runtime consumes. LDtk and dynamic
world geometry should use typed provider/domain boundaries rather than app-owned
special cases.

Owners: [`authoring-and-tools.md`](authoring-and-tools.md) and
[`ldtk-authoring-and-world-tools.md`](ldtk-authoring-and-world-tools.md).

### E10 — world facts, orchestration and agentic characters

The simulation owns reality. World-fact/observation systems, authored
orchestration, deterministic AI and future model-backed participants consume
that truth through typed actions. They do not create a second authoritative
world model.

Owners:

- [`world-facts-observations-and-memory.md`](world-facts-observations-and-memory.md)
- [`authored-gameplay-logic-and-orchestration.md`](authored-gameplay-logic-and-orchestration.md)
- [`agentic-character-runtime.md`](agentic-character-runtime.md)

### E11 — presentation and observability

Rendering/animation/VFX, participant/view-aware UI and machine-readable
inspection remain downstream of authoritative simulation and should compose as
focused Bevy domains rather than another application-shaped tangle.

Owners:

- [`render-animation-and-vfx.md`](render-animation-and-vfx.md)
- [`ui-localization-and-accessibility.md`](ui-localization-and-accessibility.md)
- [`inspection-diagnostics-and-workbench.md`](inspection-diagnostics-and-workbench.md)

### E12 — Godot-class 2D capability and expressiveness

Continuously test whether the collection of programs above adds up to a usable
engine rather than a set of elegant subsystems. The competitive bar covers
ordinary 2D rendering/presentation, movement/collision, animation/VFX, audio,
UI, input, assets/readiness, persistence, diagnostics, headless execution,
platform/build/package, extension and SDK capability.

The bar does **not** require Godot-style editor workflows. Generic capability may
come directly from Bevy or an ecosystem plugin; Ambition owns the semantic
contracts and specializations that its games actually need.

Owner: [`godot-class-2d-capability.md`](godot-class-2d-capability.md).

## Cross-program rules

1. **Ambition first, reusable second, neither sacrificed.**
2. **One authoritative representation.** New abstractions delete, isolate or
   make unreachable the authority they replace.
3. **Lifetime is part of authority.** A fact owned by one gameplay session or
   timeline cannot become another session's current truth merely because the
   allocation survived.
4. **Rollback state, participation, identity and composition are distinct.** Do
   not use success in one as proof of the others.
5. **Construction is semantic, not snapshot-shaped.** Room start, transition,
   replay and durable restore should converge on prepared content plus durable
   facts rather than parallel constructors.
6. **World facts precede story interpretation.** Dialogue/AI may interpret state;
   they do not invent it.
7. **Composition over taxonomy.** Bodies, capabilities, participants, views,
   world objects and services compose.
8. **Headless and visible hosts share simulation contracts, but tests must use
   the host topology required by the property.** Render cost cannot be inferred
   from headless timing; shell/session composition cannot be proven by a toy
   host.
9. **Crates follow ownership and registration.** A file move is not a boundary if
   the old owner still imports/registers the domain.
10. **Measure before optimizing.** Preserve concise warnings for attractive
    directions that measured low leverage; do not retain the full investigation
    in active planning.
11. **Do not pre-generalize without a real customer.**
12. **Every focused plan names genuine unresolved questions.** Product/design
    choices go to `awaiting-maintainer-decision.md` rather than being inferred.
13. **Compete on engine capability, not editor mimicry.** LLM-first semantic
    discovery/mutation/validation is a primary authoring surface; GUI work is
    justified by a visual/manual task, not by another engine having a panel.
14. **Commodity capability may stay Bevy-native.** A complete engine product does
    not imply an Ambition wrapper for rendering, physics, UI, audio, assets, or
    platform services that Bevy already supplies adequately.

## Program-level exit shape

Engine 1.0 does not require every conceivable feature. It requires common paths
to be coherent enough that a substantially different 2D game can consume them
without learning Ambition's migration history.

A credible 1.0 should make these statements unsurprising:

- Ambition is a deep persistent/systemic flagship built on supported surfaces;
- a controlled protagonist is an ordinary actor body under explicit control
  authority;
- rollback/replay correctness does not depend on ECS iteration order or a
  process-global health flag with ambiguous ownership;
- room/session reconstruction follows one inspectable construction model;
- persistent actors/items/world changes survive residency transitions coherently;
- capability/item/world-state progression is queryable and navigation-aware;
- one/many participants and views can inhabit one simulation, including multiple
  rooms when the product needs it;
- optional capabilities and rollback/content declarations follow domain
  ownership;
- asset demand/materialization/residency and weak-GPU quality have explicit
  budgets rather than incidental burst behavior;
- reusable domains can become standalone Bevy plugins without dragging Ambition
  policy with them;
- public SDK, diagnostics and project workflows are usable outside Ambition;
- another substantial 2D game has supported paths for ordinary presentation,
  animation/VFX, audio, UI, input, assets/loading, persistence and packaging
  without building private replacements around the engine;
- a capable LLM agent can discover, inspect, author, validate, test and package a
  representative cross-domain change without operating a monolithic editor or
  learning internal crate topology;
- capability gaps are judged against real game expressiveness and measured
  runtime/build cost, not against another engine's UI feature count.

## Open program questions

Focused plans own the details. The main still-open strategic questions are:

- large-world residency/background simulation granularity;
- navigation representation for dynamic platformer worlds;
- terminal/resettable persistent occurrence semantics;
- which presentation/tooling domains deserve independent ecosystem crates;
- which capabilities remain internal plugins versus standalone packages;
- the first real external/P2P lifecycle barrier and transport customer.
