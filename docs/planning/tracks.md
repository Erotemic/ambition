# Tracks — standing backlog and work reservoir

**Role:** preserve worthwhile unresolved work that is not the current executable
queue. [`queue.md`](queue.md) owns execution order. Focused plans own design and
measurement detail.

A track row should state only the enduring claim, the trigger for promotion and
its owner. Revalidate all three against HEAD before moving a row to `queue.md`.
Do not copy campaign history, volatile counts or closed-work receipts here.

## Replenishment order

Unless Jon or a reproducible report changes the order:

1. direct maintainer observations;
2. Ambition flagship needs that expose reusable engine capability;
3. authoritative-state, lifetime and reconstitution correctness;
4. measured product performance or build/iteration blockers;
5. ownership, dependency and SDK improvements with a real consumer;
6. serious secondary-game customers;
7. trigger-based work only after its trigger exists.

## Competitive 2D engine bar

Use [`engine/godot-class-2d-capability.md`](engine/godot-class-2d-capability.md)
as a gap detector, not a second queue or an editor-parity checklist. Promote a
capability only when a real Ambition or secondary-game customer lacks a supported
path. Prefer Bevy/ecosystem machinery for generic concerns and add Ambition policy
only where stronger semantics are required.

Standing pressure worth rechecking includes:

- external/minimal game proof that public capabilities compose into a buildable,
  testable and packageable 2D project;
- LLM-first discovery, inspection, mutation and validation across engine and
  authored vocabulary;
- structured diagnostics, provenance and why-not answers without requiring a GUI
  inspector or implementation grep;
- real-customer audits of rendering, animation/VFX, audio, UI, input, assets and
  platform/export completeness;
- measured runtime, hitch/memory and build/test behavior on declared target
  profiles.

Do not promote scene-editor cloning, visual scripting, GDScript parity, general
3D breadth, a plugin marketplace or a general rigid-body layer without a product
requirement.

## Engine architecture reservoir

The [architecture reassessment](engine/architecture-reassessment.md) supplies the
current ownership model. The
[actor-monolith frontier](engine/actor-monolith-work-frontier.md) owns active
architecture packet gates. Work already selected in `queue.md` is not repeated
here.

### Persistent systemic world

- ▢ **Open-world residency.** Preserve the distinction among world existence,
  room residency, simulation activity and local visibility. Promote when actual
  Ambition or multiplayer pressure requires a residency/background-simulation
  policy. Owner:
  [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md).
- ▢ **Persistent occurrence/reconstitution semantics.** Terminal versus resettable
  occurrences, foreign-room relocation, unloaded-room items and durable
  relationships must consume the canonical construction/reconstitution model.
  Revalidate the remaining terminal-consumption and rollback obligations before
  promotion. Owner:
  [`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md).
- ▢ **Item custody/accounting residual.** Finish body-owned instance/count
  semantics for held weapons/abilities and any remaining unloaded-room occurrence
  behavior. Product policy questions belong in the decision ledger, not here.
  Owner: [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md).
- ▢ **Capability progression/world gating.** Physical verbs remain body-owned;
  participant-owned knowledge/keys/theorems need a real participant authority
  before they can be claimed as implemented. Extend route-facing facts and
  capability vocabulary only from concrete progression needs. Owner:
  [`engine/capability-progression-and-world-gating.md`](engine/capability-progression-and-world-gating.md).
- ▢ **Platformer navigation/reachability.** Build the navigation program from a
  real route-planning customer. Do not treat fighter rollout vocabulary defects
  as evidence about navigation architecture. Owner:
  [`engine/platformer-navigation-and-reachability.md`](engine/platformer-navigation-and-reachability.md).

### Capability, package and SDK boundaries

- ▢ **Capability/runtime composition.** Continue when a minimal consumer inherits
  a capability it did not request or a real host needs substitution. Owner:
  [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).
- ▢ **Public SDK 1.0.** Hide implementation topology behind semantic game APIs and
  prove them with external/minimal consumers. Owner:
  [`engine/public-sdk-1.0.md`](engine/public-sdk-1.0.md).
- ▢ **Reusable Bevy-domain extraction.** Extract or publish only where a mature
  domain has coherent ownership, plugin registration and a second useful
  consumer. Doctrine:
  [`../architecture/package-and-capability-boundaries.md`](../architecture/package-and-capability-boundaries.md).

### Multiplayer and multiview

- ▢ **N-view production composition.** Per-view projection foundations exist;
  promote broader layout, HUD ownership and input routing when Ambition or
  TwinTrack needs them. Owner:
  [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md).
- ▢ **Per-view camera/reference-frame policy.** Extend shared/split-view policy
  from real multiview requirements. Owner:
  [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md).
- ▢ **Real network transport / Matchbox signaling.** Trigger only with an actual
  online Ambition or Smash customer. Owner: [`engine/netcode.md`](engine/netcode.md).

### World facts, orchestration and agentic characters

- ▢ **Deterministic world facts + observations/memory.** Keep simulation truth
  separate from what a character observed or believes. Use
  `scripts/authored_route_gates.py` for route-condition measurements instead of
  copying counts into planning. Owner:
  [`engine/world-facts-observations-and-memory.md`](engine/world-facts-observations-and-memory.md).
- ▢ **Agentic character runtime.** Typed actions/dialogue should consume world
  truth; model-backed realtime characters must enter through participant seams
  rather than nondeterministic tick mutation. Owner:
  [`engine/agentic-character-runtime.md`](engine/agentic-character-runtime.md).
- ▢ **Authored gameplay orchestration.** Generalize beyond prepared calls only
  when a real customer needs a reusable rule representation. Owner:
  [`engine/authored-gameplay-logic-and-orchestration.md`](engine/authored-gameplay-logic-and-orchestration.md).

### Presentation and observability

- ▢ **Render/animation/VFX.** Extend semantic presentation cues, backend
  separation and quality behavior from product pressure. Owner:
  [`engine/render-animation-and-vfx.md`](engine/render-animation-and-vfx.md).
- ▢ **Inspection/diagnostics/workbench.** Build read-only discoverability from
  domain-contributed descriptors rather than a new simulation authority. Owner:
  [`engine/inspection-diagnostics-and-workbench.md`](engine/inspection-diagnostics-and-workbench.md).
- ▢ **Moveset observatory M3 — art/geometry agreement.** Do not build a parallel
  projection helper: measure the rendered frame through the camera that drew it.
  The remaining product definition is which anchor, tolerance and zoom constitute
  agreement. Owner: [`moveset-inspector.md`](moveset-inspector.md).
- ▢ **Localization/accessibility.** Grow when actual translation or accessibility
  requirements exist. Owner:
  [`engine/ui-localization-and-accessibility.md`](engine/ui-localization-and-accessibility.md).

### Content and procedural iteration

The [extension model](engine/extension-model.md) owns independent authoring,
loadable artifacts and procedural SDK architecture. Its
[packet catalog](engine/fast-iteration-implementation.md) owns implementation
slices. Only `queue.md` promotes them.

## Authoring and content reservoir

- ▢ **LDtk/world tools.** Keep authoring semantics provider/domain-owned and
  validate against the same model runtime construction consumes. Owner:
  [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md).
- ▢ **Kinematic-world second customer.** Reopen moving/dynamic-geometry work only
  when a new customer requires the unresolved geometric-displacement versus
  surface-drag distinction.
- ▢ **Compositional spatial semantics.** Reopen when a real customer crosses
  existing surface-semantic axes, repeats shared zone identity/provenance
  plumbing or is blocked by a closed engine spatial switch. Owner:
  [`engine/world-geometry-and-spatial-semantics.md`](engine/world-geometry-and-spatial-semantics.md).
- ▢ **Provider-defined actions through the physical/UI seam.** The producer side
  exists; promote when a customer needs a semantic-action reader plus finite
  controller/touch presentation and multi-map policy. Owner:
  [`engine/participant-action-system.md`](engine/participant-action-system.md).
- ▢ **Declared-ID/binding diagnostics.** Extend source-qualified failures where a
  real authored reference still bypasses the binding boundary. Do not create a
  universal asset census. Owner:
  [`engine/binding-resolution-boundary.md`](engine/binding-resolution-boundary.md).
- ▢ **Semantic dependency/reference graph.** Add cross-authoring reference,
  unresolved-reference and transactional rename/delete support from a real
  authoring customer. Reuse the inspection/binding model rather than creating a
  second runtime registry.
- ▢ **Remaining named-content evictions.** Move named game content out of reusable
  engine crates only when a current dependency/provider boundary still owns it.
- ▢ **Editable SVG/component character authoring.** Continue from concrete
  character-production pressure, not an editor checklist.

## Build, platform and performance reservoir

- ▢ **Project build/iteration architecture.** Measure profiles, resource-aware
  test lanes, feature combinations and bootstrap cost. Owner:
  [`engine/project-build-and-distribution.md`](engine/project-build-and-distribution.md).
- ▢ **Rendered external-consumer proof.** Headless consumption exists; run the
  visible consumer on suitable hardware when available.
- ▢ **Cross-compile persona audit.** Check one host-buildable target/persona at a
  time. Missing platform toolchains are prerequisites, not product defects.
- ▢ **Android font path.** The Bevy 0.19 path is typechecked but still requires a
  real Android runtime/glyph-resolution witness. Use
  `scripts/setup/android_prereqs.sh` to establish the toolchain first. Owner:
  [`engine/project-build-and-distribution.md`](engine/project-build-and-distribution.md).
- ▢ **Asset residency/materialization followups.** Add budget/eviction policy only
  after a target profile or maintainer decision supplies a real budget. Keep
  residency/materialization ownership with the asset architecture rather than a
  generic cache layer.
- ▢ **Generic CPU optimization.** No standing campaign. Reopen only with a
  reproduced hotspot and target profile.

## Bevy 0.19 follow-ups

These are opportunities, not migration work.

- ▢ **Text gizmos for developer overlays.** `debug_overlay` already demonstrates
  the model. Reassess remaining dev overlays only where it removes a real font or
  entity-lifecycle burden; preserve product UI and fixed-width requirements.
- ▢ **`Rem` sizing for UI accessibility scaling.** Use when a concrete
  accessibility or device-legibility requirement needs global text scaling.
- ▢ **Resources-as-components for singleton authorities.** Review only where
  observers/hooks solve an ownership invariant. Rollback authority and broad
  `EntityMut` exposure outrank ergonomic savings.
- ▢ **Upstream text entry.** Prefer Bevy's first-party editing when Ambition needs
  an input field; no standalone migration campaign.
- ▢ **BSN for declarative spawn trees.** Consider for presentation/menu trees
  when a real tree benefits; do not migrate runtime content merely to use it.

## Combat, AI and behavior reservoir

- ▢ **Boss animation vocabulary fold.** Converge remaining boss animation/frame
  vocabulary when a current boss/product customer needs it.
- ▢ **Dialogue continuity.** Body-generic interruption, separation and
  station-keeping. Owner: [`engine/dialogue-continuity.md`](engine/dialogue-continuity.md).
- ▢ **Listener-side dialogue adaptation.** Promote when content must adapt to
  listener context.
- ▢ **Character dialogue from suggestions/barks.** Shelved product design; keep
  dormant until a customer exists.
- ▢ **Falling-sand extensions.** Water/oil/Oiler mechanics remain deferred work.
  Owner: [`engine/falling-sand.md`](engine/falling-sand.md).
- ▢ **Per-route music.** Preserve as a product capability; do not revive a closed
  audio campaign as its execution authority.

## Game/customer reservoir

- ▢ **Ambition open-world production.** Primary product driver. Owners:
  [`game/open-world-roadmap.md`](game/open-world-roadmap.md) and
  [`game/systemic-progression.md`](game/systemic-progression.md).
- ▢ **Super Smash Siblings.** Current executable work belongs in `queue.md`;
  standing feature truth belongs in
  [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).
- ▢ **TwinTrack.** Future multiview/reference-frame customer; currently lower
  priority than the main games.
- ▢ **Sanic / Super Mary-O / Hollow Lite.** Keep their focused acceptance lists as
  movement, collision, world-authoring and encounter customers. Owners:
  [`demos/sanic.md`](demos/sanic.md),
  [`demos/super-mary-o.md`](demos/super-mary-o.md) and
  [`demos/hollow-lite.md`](demos/hollow-lite.md).
- ▢ **Player-facing authored-art repairs.** Treat morph-ball, shrine/glider and
  similar fixes as product work unless a reproduced defect demonstrates a
  reusable renderer or authoring-system problem.

## Trigger-based work

Do not promote these until the trigger exists:

- **Slower Light:** a real 3D runtime/customer.
- **Leafwing clash-scan optimization:** a dependency/version change or measured
  CPU cost that matters to a declared target profile.
- **Broader stable-ID centralization:** concrete identity families that share
  actual operations.
- **Provider-owned placement-family extension:** a provider outside the common
  authored vocabulary.
- **Reusable menu-host extraction:** a real second consumer.
- **Boss crate extraction:** coherent boss vocabulary/ownership that benefits
  from an independent package boundary.
- **Body-generic NPC economy/world interaction:** NPC agency or multiplayer
  currency pressure.
- ✅ **Dormant `GravityFlipSwitch` cluster — RULED AND DELETED 2026-09-19.**
  `Q137` kept gravity switching and retired the unreachable overlap plate; the
  component, its system, both rollback registrations, the view facts, the
  visual and the exit-oracle row went together. A later pressure plate is an
  INPUT into `BaseGravity`, not a revived parallel implementation.
- **Dormant `GatePortalRegistry` cluster:** a real authored gate-portal customer
  or maintainer decision that the feature is dead. Revalidate production
  producers and rollback-schema consequences before deletion.
- **Provider-owned persistence/item identities:** a second provider that needs to
  own the domain.
- **Test execution parallelism:** measurements showing execution rather than
  compile/link or memory pressure dominates.

## Standing execution rule

Before promoting a card:

1. inspect HEAD and confirm the missing thing still exists;
2. prefer the focused plan that already owns the design;
3. state the product, authority or dependency payoff;
4. give one inspectable acceptance criterion;
5. promote only executable work to `queue.md`.
