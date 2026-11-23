# Tracks — standing backlog and work reservoir

**Role:** preserve worthwhile unresolved work that is not the current executable
queue. [`queue.md`](queue.md) owns execution order. Focused plans own design and
measurement detail.

A track row should state only the enduring claim, the trigger for promotion and
its owner. Revalidate all three against HEAD before moving a row to `queue.md`.
Do not copy campaign history, volatile counts or closed-work receipts here.

**Selected work moved out of the reservoir (2026-11-06):** the Radiant NPC planner, per-game network-authority experiment, boss cinematic census, GNU2 finale and GNU motion diagnosis now have explicit [`queue.md`](queue.md) entries and owner plans. Do not maintain parallel task checklists here. Room-level rollback (SIMULATION-ISLANDS) was selected on 2026-11-10 under the hybrid direction.

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

[`engine/architecture.md`](engine/architecture.md) supplies the current
ownership model. The
[actor-monolith frontier](engine/actor-monolith-work-frontier.md) owns active
architecture packet gates. Work already selected in `queue.md` is not repeated
here.

The [audit frontier](engine/architectural-audit-frontier.md) records which
boundaries have received a deep source review and ranks the next research
questions. Its candidates are not selected implementation work. Use it to
choose an audit before adding another architecture packet.

The 2026-11-22 [endpoint audit](engine/relationship-endpoints-and-transitions.md)
refines selected ARCHITECTURE-COMPOSITION and A7 work. Its current-residency
versus prospective-transfer distinction is shared, while capture, ride,
possession and item disposition remain native. Broader durable relationships,
cross-room summon delivery and generic graph/index work stay unselected;
the live cohort fixes do not establish their contracts.

The following [observation audit](engine/world-facts-observations-and-memory.md)
selects OBS1–OBS3 under ARCHITECTURE-COMPOSITION. It repairs native direct
evidence admission, source-aware tactical memory/crew reports and module
observation demand. It does not select a durable social-memory store, theft
law, recognition system or model-backed dialogue. The next research candidate
was shared observers and control ownership changes; the follow-up below
has now audited that boundary on the same runtime source.

The shared-observer/control pass selects
[IN4/IN5](engine/participant-action-system.md#input-continuity-across-routing-and-control-changes)
and [MV1](engine/multiplayer-and-multiview.md#mv1--resolve-the-selected-view-before-its-framing-and-continuity)
under ARCHITECTURE-COMPOSITION. Source recognition, admitted interruption
and body-action continuation keep separate owners; each view derives its
framing and continuity from its selected policy. P2's provider-action final
hop remains gated on an actual gameplay consumer, and M3 still owns the
shared orientation/input-reference product choice. The audit frontier now
also treats static comprehension as an audit customer; a fresh build is
not a prerequisite for further source reasoning.

The static-contract pass selects [room scope SC1](engine/capability-and-runtime-composition.md#sc1--select-rule-subjects-inside-the-work-gate)
and [complete strike origin](engine/combat-reception-and-injury.md#carry-attack-origin-through-strike-creation-and-contact)
under ARCHITECTURE-COMPOSITION, and
[named animation binding](engine/runtime-rigged-sprite-animation.md#bind-semantic-roles-to-named-source-rows)
under E2/PVS-1. The smaller
[A2 coverage correction](engine/authoring-and-tools.md#static-source-coverage)
keeps source navigation honest about submodule roots and missing corpora.
These improve local native contracts without a new execution framework,
registry or collection library. Subsequent passes covered authored
omission/override and moving supports. The current
[audit frontier](engine/architectural-audit-frontier.md) retains the next
research questions. Cross-version saved-content compatibility is deferred
to 2027. Actual performance ranking still needs measurements.

### Curved ground and local gravity

- ▢ [Curved surfaces and local gravity](engine/curved-surfaces-and-local-gravity.md)
  is tentative. CV1 states the existing hill-picture goal's missing
  support-to-pose handoff. CV2 prepares authored curves with bounded native
  geometry. CV3 first makes continuous down an honest ordinary-walker
  capability; CV4 can then prove one static circular world. CV5 adds a
  named surface field only if a scene needs it. No CV card is selected by
  this audit, and SUP1's current moving-support work is independent.
  Decide attraction versus attachment, airborne behavior and physical
  posture before generalizing. Exact analytic curves, broad field
  composition, terrain feet and planetary AI are separate choices.

### Persistent systemic world

- ▢ **Open-world residency beyond live rooms.** Several live rooms exist.
  Preserve the distinction among world existence, room residency, simulation
  activity and local visibility. Promote background simulation only when a
  mechanic needs it; today the session clock covers dormant elapsed time.
  Conditional leftovers include sole-room reader customers, twin-instance
  identity, view/prefetch claim budgets and restocking only with actual stock.
  Keep broader network/island and local-seat acceptance with their owners.
  Owner: [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md).
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

The 2026-11-21 review selected bounded work under
[ARCHITECTURE-COMPOSITION](queue.md#architecture-composition--link-owned-facts-and-preserve-causal-boundaries).
Its [linking owner](engine/capability-linking-and-prepared-facets.md) joins
prepared facts, native interpretation and dependency inversion. Further facet
moves follow a demonstrated repeated interpretation or independent consumer;
there is no all-crates extraction program.

- ▢ **Shared gameplay bindings.** Select two-ended
  [body anchor alignment](engine/body-anchor-bindings.md),
  [scene/conversation occurrence roles and skip settlement](engine/story-sequences.md),
  and [domain interaction opportunities](engine/interaction-opportunities-and-activation.md)
  under ARCHITECTURE-COMPOSITION. Share the relation each consumer needs;
  keep movement, lifetime, sequencing and acceptance with their owners.

- ▢ **Capability/runtime composition beyond SC1.** The selected room-subject
  correction uses current room access and does not reopen completed profile
  or closure proofs. Continue broader cuts when a minimal consumer inherits
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

- ▢ **N-view production composition.** MV1's selected target/framing/
  continuity correction is in the queue, with specified native
  independent-view witnesses. Broader adaptive layout, HUD/prompt scope and controller/view
  presentation follow actual Ambition or TwinTrack requirements; preserve
  the built join and room-split roads.
  Owner:
  [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md).
- ▢ **Shared-view reference and input policy.** MV1 selects explicit
  preference inheritance. Broader M3 chooses shared orientation and each
  participant's logical input reference when body frames disagree; preserve
  body-relative control and held-direction semantics. Promote from the
  actual shared/split customer. Owner:
  [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md).
- ▢ **Real network transport / Matchbox signaling.** Trigger only with an actual
  online Ambition or Smash customer. Owner: [`engine/netcode.md`](engine/netcode.md).

### Conditional GGRS-island rollback

- ▶ **Selected 2026-11-10.** The maintainer chose a hybrid direction for the metroidvania: rollback kept inside a room while cheap, hosted authority around it. The work is the queue row [SIMULATION-ISLANDS](queue.md#simulation-islands--room-level-rollback-inside-a-hybrid-world); this heading stays for inbound links. Owner: [island rollback and compact input](engine/island-rollback-and-compact-peer-input.md#simulation-islands-independently-rewindable-active-areas).

### World facts, orchestration and agentic characters

- ▢ **Deterministic world facts + observations/memory.** Keep simulation truth
  separate from what a character observed or believes. OBS1–OBS3 are selected
  bounded corrections, not a new reservoir checklist; OBS1 and OBS2 landed
  2026-11-23, OBS3 is open. Promote durable social
  knowledge only with a concrete recognized-incident/dialogue customer and
  stated room/death/checkpoint/replay/save horizons. Preserve existing narrow
  Player-hostility and per-seat dialogue-visit policy. Use
  `scripts/measure/authored_route_gates.py` for route-condition measurements instead of
  copying counts into planning. Owner:
  [`engine/world-facts-observations-and-memory.md`](engine/world-facts-observations-and-memory.md).
- ▢ **Agentic character runtime.** Policies consume the knowledge projection
  their role permits and propose typed intent through ordinary action admission.
  Module-backed brains already exist; OBS3 supplies the shared observation
  demand boundary. Remote/model acceptance, persistent goals and generated
  dialogue remain separate customer-gated work. Owner:
  [`engine/agentic-character-runtime.md`](engine/agentic-character-runtime.md).
- ▢ **Authored gameplay orchestration.** Generalize beyond prepared calls only
  when a real customer needs a reusable rule representation. Owner:
  [`engine/authored-gameplay-logic-and-orchestration.md`](engine/authored-gameplay-logic-and-orchestration.md).

### Presentation and observability

- ▢ **Render/animation/VFX.** Extend semantic presentation cues, backend
  separation and quality behavior from product pressure. Owner:
  [`engine/render-animation-and-vfx.md`](engine/render-animation-and-vfx.md).
- ▢ **Appearance order and optional 2.5D (selected).** Give the cart named
  interior placement for an entire appearance, then share sampled art between
  current 2D and optional Bevy 3D views. Mechanical depth remains independent.
  Owner: [`engine/presentation-order-and-25d-rendering.md`](engine/presentation-order-and-25d-rendering.md).
- ▢ **Inspection/diagnostics/workbench.** Build read-only discoverability from
  domain-contributed descriptors rather than a new simulation authority. Owner:
  [`engine/inspection-diagnostics-and-workbench.md`](engine/inspection-diagnostics-and-workbench.md). Completed effective-geometry/F1 coverage is recorded in [`GEOMETRY-OBSERVABILITY`](queue.md#geometry-observability--render-the-effective-geometry-behind-gameplay---done-2026-11-17), not as another independent backlog program.
- ✓ **Art inventory and coverage.** The shared CLI and offline page report
  missing or stale bundles, identities, lineage, and animation coverage.
  Owner: [`engine/art-inspector-and-character-coverage.md`](engine/art-inspector-and-character-coverage.md).
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
- **Focused spatial query contracts.** The `00bf92c22142` audit supplies
  concrete material-overlap and partial-publication customers under
  PORTAL-SIMILARITY, and static support/occupancy coverage under E4. Keep their
  existing execution lanes. Broad surface/zone unification remains deferred
  until a mechanic requires it. Owner:
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
- ▢ **Asset library and editor lifecycle.** Measure the real time for focused
  publication or startup, or promote a real asset-browser operation. Keep
  staleness advisory and preserve the native no-rebuild path. Owner:
  [`engine/asset-library-editor-and-delivery.md`](engine/asset-library-editor-and-delivery.md).

## Unselected residuals found in the planning cleanup

These were explicit unfinished items inside dated owner diaries. This section
is a **reservoir**, not a second execution queue; verify each against current
source before promotion.

- ▢ **Parts-only artifact distribution.** P5c is still open: exclude baked
  `_spritesheet.png` for parts-only characters from shipped products while
  retaining it as an offline oracle. P6c target-GPU cost/parity evidence is
  also not established by the offline checks. Promote for a real package
  build or device validation. Owner:
  [`engine/mary-o-part-realization.md`](engine/mary-o-part-realization.md).
- ▢ **Parts-versus-baked compatibility cleanup.** The older part-parity run
  found a `feet_pixel` versus `feet_anchor_norm` inconsistency for director,
  officer and medic. It also reported baked fallback half-pixel blur and frame
  clipping. Do not reopen a parts migration: first confirm a current asset
  reader or remaining baked character still exhibits the discrepancy. Owner:
  [`engine/mary-o-part-realization.md`](engine/mary-o-part-realization.md).
- ▢ **Foreign/UI optimizer follow-ups.** The October performance audits
  observed `virtual_joystick::update_ui` false writes, Bevy light/PBR/OIT
  work without the corresponding scene, and zero-intensity hit-flash material
  refresh. These are historical candidates, not proven current defects.
  Reprofile on a target before selecting upstream or fork work; prove portal
  rendering when changing hit-flash behavior. Owner:
  [`engine/performance-and-composition-findings.md`](engine/performance-and-composition-findings.md).
- ▢ **Per-duty held-item capability attachment.** S3 finished the supported
  profile migration but did not validate every shrine, whereabouts,
  checkpoint and match-item step nested under `CoreHeldItems`. Promote
  when a real profile reveals an incorrect lifetime. Owner:
  [`engine/system-composition-audit.md`](engine/system-composition-audit.md).
- ▢ **October 30 consignment-errand report.** The dated world-detail log
  reported a test expecting an old Georg Canter dialogue choice. Re-run
  `the_consignment_errand` before classifying it as a live regression.
  Owner: [`game/world-detail-24h.md`](game/world-detail-24h.md).

## Build, platform and performance reservoir

- ▢ **Project build/iteration architecture.** Measure profiles, resource-aware
  test lanes, feature combinations and bootstrap cost. Owner:
  [`engine/project-build-and-distribution.md`](engine/project-build-and-distribution.md).
- ▢ **Published asset cleanup and content-only delivery.** Promote GC when
  old bundles cause a storage problem or a release needs pinned versions.
  Promote rsync updates when a second machine or external game needs them.
  Owner: [`engine/asset-library-editor-and-delivery.md`](engine/asset-library-editor-and-delivery.md).
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
  reproduced hotspot and target profile. Dated collision-world, read-model,
  identity, navigation and exact-idle-body proposals are catalogued in
  [`engine/performance-and-composition-findings.md`](engine/performance-and-composition-findings.md),
  not automatically selected.

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

- ▢ **Dormant NPC completion.** Before extending authored timed lives or
  implementing real world-object use, admit only the elapsed plan prefix
  supported by total actor time, object capacity and destination validity.
  State the source of historical observations. Keep the completed Radiant
  slices closed and hourly cancellation/restart valid; no general activity
  framework is selected. Owner:
  [LIFE1/LIFE2](engine/radiant-npc-planning.md#dormant-completion--execute-a-supported-prefix-of-the-plan).

- ▢ **Strike pulse contact memory.** Prepare pulse membership and keep one
  history per occurrence/pulse instead of copying per-volume victim sets.
  The deeper audit found supported overlapping/skipped-window counterexamples,
  but no current roster reproduction or measured speedup. This remains
  separate from ACT2 and demo acceptance. Owner:
  [combat pulse contract](engine/combat-model.md#a-strike-pulse-owns-its-contact-memory).

- ▢ **Boss animation vocabulary fold.** Converge remaining boss animation/frame
  vocabulary when a current boss/product customer needs it.
- ▢ **Dialogue continuity.** Body-generic interruption, separation and
  station-keeping. Owner: [`engine/dialogue-continuity.md`](engine/dialogue-continuity.md).
- ▢ **Listener-side dialogue adaptation.** Promote when content must adapt to
  listener context.
- ▢ **Character dialogue from suggestions/barks.** Shelved product design; keep
  dormant until a customer exists.
- ✖ **Falling sand.** Code removed 2026-10-11 (Jon); not a work item. A
  reimplementation is a new design. Owner: [`engine/falling-sand.md`](engine/falling-sand.md).
- ▢ **Per-route music.** Preserve as a product capability; do not revive a closed
  audio campaign as its execution authority. Use the
  [music program/lifecycle owner](engine/music-programs-and-playback.md)
  behind each selected frontend route.
- ▢ **Further musical transition capabilities.** The
  [selected music row](queue.md#music-playback--owned-handoffs-and-prepared-entrances)
  addresses handoff ownership, candidate arbitration, prepared entrances,
  and common recipes. Phase-preserving cross-product entry, independent loop
  release tails, scheduled pickups, and layered stingers need the product
  facts, bounded voice budget, and native support stated in
  [MUS3/MUS4](engine/music-programs-and-playback.md#authoring-interface-and-rejection).
  Select each with an authored customer. Do not silently approximate an
  unsupported request, require middleware parity before fixing ownership,
  or add a generic transition graph/VM.

## Boss-encounter content acceptance reservoir

- ▢ **Encounter-specific review, not another boss-system rewrite.** The
  [`game/bosses.md`](game/bosses.md#current-state-and-outstanding-acceptance-2026-11-06-audit)
  owner distinguishes implemented fights from remaining playtest, tune,
  art, audio, validator and arena follow-ups for PCA, T-rex, Hunny Horror,
  Wolfenstein/Inugami, Mockingbird, GNU-ton and the Hall. Select a bounded
  encounter task after reproducing its dated findings or accepting the
  relevant artistic target. Preserve boss-specific telegraph, punish and
  stage identity; do not impose a universal body-relative attack reach.
  GNU-ton mechanical debts are owned separately by
  [`game/gnu-ton-fight-debts.md`](game/gnu-ton-fight-debts.md).

## Game/customer reservoir

- ▢ **Ambition open-world production.** Primary product driver. Owners:
  [`game/open-world-roadmap.md`](game/open-world-roadmap.md) and
  [`game/systemic-progression.md`](game/systemic-progression.md).
- ▢ **Super Smash Siblings demo quality.** The S0 feature gate is closed.
  The next product pass follows the main Ambition improvements: one coherent
  hit/recovery/landing sequence, actual CPU return to support, then cast
  choreography and tuning. Shared action/time and clip fixes with a current
  Ambition customer remain eligible in their existing engine lanes.
  [Inventory §17](demos/smash-parity-inventory.md#17-current-implementation-order)
  owns order; [feel acceptance](demos/smash-feel-audit-2026-10-29.md)
  owns the bounded review. Promote a Smash-only slice when D72 is selected,
  not because shared engine code now exists.
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
- ▢ **Character visual-era audit and curated skins (Q170 ruled).** The current
  design-history era index overcounts actual looks by conflating Git revisions
  with visual redesigns. Audit rendered holistic appearances per lineage,
  coalesce minor revisions without deleting provenance, and report corrected
  candidate eras; only then curate which distinct designs deserve shipped
  `WornSkin` catalog/publish entries. Promote the index audit before skin
  publication; selection UI, unlocks and Android tier policy remain
  owner-plan choices. Owner:
  [`game/character-generations.md`](game/character-generations.md).

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
- **Gravity pressure plate:** a real authored customer. `GravityFlipSwitch` was <!-- cite-ok: a deleted name -->
  deleted under `Q137` (2026-09-19); a new plate is an input into
  `BaseGravity`, not a second implementation.
- **Route-keyed music inside one experience:** a product need for, for example,
  a stage theme and a winner-card theme. Declare them as two frontend-audio
  routes (`FrontendAudioProfile`, `FrontendAudioRegistry` in
  `crates/ambition_audio/src/selection.rs`) instead of a process-global music
  switch through `MusicDirectorState`. The route supplies authority and a
  goal to the [common music lifecycle](engine/music-programs-and-playback.md);
  it does not install a second track executor. `provider` and `experience` share string
  vocabulary by maintainer decision; do not start a rename from this item.
- **Dormant `GatePortalRegistry` cluster:** a real authored gate-portal customer
  or maintainer decision that the feature is dead. Revalidate production
  producers and rollback-schema consequences before deletion.
- **Provider-owned persistence/item identities:** a second provider that needs to
  own the domain.
- **Test execution parallelism:** measurements showing execution rather than
  compile/link or memory pressure dominates.

## Queue cleanup: unselected or conditional residuals (2026-11-06)

These entries moved out of `queue.md` because they have no current executable
slice, await a product/renderer prerequisite, or duplicate another active row.
They are **not evidence of completed implementation**. Validate against HEAD
before promotion.

- ▢ **NPC combat-kit census and bounded repairs (`NPC-KIT-1/2`).** Select when
  expanding the NPC combat roster; inventory executable kits deterministically,
  then fix uncovered NPCs in bounded batches. Owner: [`game/ambition.md`](game/ambition.md).
- ▢ **Launcher vanity fade (`VC5`).** The reusable launcher content fade is
  explicitly open but is cosmetic work, not a correctness interrupt. Owner:
  [`engine/shell-vanity-sequence.md`](engine/shell-vanity-sequence.md).
- ▢ **Actor effects beyond the selected ownership slice.** Numeric phases 0–5
  are complete; phase 6's control/pose and native-interval customers are selected
  under ARCHITECTURE-COMPOSITION. Later periodic/status behavior remains a
  domain customer of the same ownership protocol. Owner:
  [`engine/actor-effects-and-modifier-composition.md`](engine/actor-effects-and-modifier-composition.md).
- ▢ **Simulation tick-rate compatibility.** Promote with a target game/profile
  requiring a 30/40/60/120 Hz selection and measure motion, collision, AI,
  hitstop and replay across the supported rates. Owner:
  [`engine/simulation-tick-rate-compatibility.md`](engine/simulation-tick-rate-compatibility.md).
- ▢ **Controller routing/connection view.** Developer read-only view of
  devices, participants, unassigned/disconnected claims and active action
  contexts. Deferred until the multiplayer menu-region and gate work is
  settled; reuse existing routing authorities. Owner:
  [`engine/participant-action-system.md`](engine/participant-action-system.md#controller-routing-debug-view--open).
- ▢ **Architecture/capability census residuals (A9, D166,
  CONTENT-AUTHORITY-DUPLICATES).** A9 profiles and the external capability
  witness are built; D166 reports no known residual. Reopen only with a
  demonstrated duplicate reader, authored/runtime divergence or unsupported
  profile customer. Owners:
  [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md),
  [`engine/character-authoring-package.md`](engine/character-authoring-package.md),
  [`engine/extension-model.md`](engine/extension-model.md).
- ▢ **Attack reach and brain engagement (`REACH-VIEW`, Q168 ruled).** Do not
  shorten the robot-family slash on body-width or duel-test grounds: other
  attacks may be too short, and a large blade arc does not make the robot a
  long-range duelist. Re-measure representative rendered strike regions and
  their actual hit geometry; repair mismatches or undersized authored attacks
  based on play feel. Revisit the reverted reach projection with a same-hitbox
  visual/strike witness and matchup tests that allow asymmetrical ranges but
  reject indefinite CPU non-engagement. Preserve rider-without-a-move and
  dash/charge travel reach as separate possible residuals. Owner:
  [`engine/combat-model.md`](engine/combat-model.md#authored-attack-reach-and-matchup-behavior-q168-2026-11-06).
- ▢ **Landmark clip loop authoring and shark saddle.** Package landmark time
  matches looping/one-shot animation, but authored rig loop rows disagree with
  animator policy; publish one loop bit per row. The shark saddle additionally
  waits for a published pose/rig attachment. Owners:
  [`engine/mary-o-part-realization.md`](engine/mary-o-part-realization.md)
  and [`engine/runtime-rigged-sprite-animation.md`](engine/runtime-rigged-sprite-animation.md).
- **Portal spatial contracts (selected via PORTAL-SIMILARITY).** Keep the
  implemented canonical material, unequal apertures and independent/chained
  limbs. Next complete occurrence-scoped topology edits, event-time root
  motion with retained history, conservative candidate propagation and
  complete observer charts with demand/ready capture sources. The
  [delivery plan](engine/portal-parts-similarity-and-chirality.md) owns P1–P6;
  [the queue](queue.md#portal-similarity--canonical-parts-and-scaled-mirror-portals)
  selects work. Do not revive a one-pair host workaround or the old claim
  that retained portal rendering pools are missing.
- **Scale-relative gameplay (selected via SCALE-FEEL).** Keep ordinary
  movement and action timing at different material scales. Convert canonical
  gravity response with controls, then contact bands and existing action/
  relation quantities. World fields and already resolved values keep their
  meaning. [The owner](engine/scale-relative-gameplay.md) defines SR1–SR3;
  the portal motion/contact authorities remain in P2/P3.
- **Execution cost (selected via EXECUTION-COST).** Full/reference semantics
  remain available while gameplay selects local specializations. Prepare
  immutable clip and dispatch relationships; retain correct occurrence views;
  process active journal work; retain arrival state for navigation cost.
  [The owner](engine/execution-specialization-and-work-selection.md) links
  bounded domain packets and the [queue](queue.md#execution-cost--specialize-gameplay-work-at-its-actual-inputs)
  selects them. No always-on reference solver or generic cache framework.
- **Data-structure kernels (within EXECUTION-COST).** Shared bounds queries
  and strict unique lookup become independently measured operations in the
  geometry and binding foundations. [The owner](engine/data-structure-kernels.md)
  specifies interchangeable results, direct/reference implementations and
  measured selection before optional preparation. Preserve domain lifetimes.
- ▢ **Review follow-ups requiring a fresh measurement.** Cold/warm rig table
  preparation memory and frame stalls; human acceptance for robot
  crouch/turn/double jump (REVIEW-1014); rendered portal-piece frame cost
  (IN6); feature-index instance key only when one definition is live twice
  (IN1); profile large-room terrain skin setup only if still slow;
  `StageView::room_toward` on rotated gravity (REVIEW-1013); a held
  continuous portal pair crossing rooms (REVIEW-1012) only when a real
  customer needs it; human front-idle/combat and fighter-pad acceptance.
  Ordinary room travel must already retire or explicitly rebind a held
  portal lease; that source-confirmed lifetime work is selected in P1. The
  second-instance door-routing policy (REVIEW-1011) also needs a live
  instance-creating customer. Owner:
  [`triage/2026-10-11-integration-source-audit.md`](triage/2026-10-11-integration-source-audit.md)
  and relevant renderer/asset documents. Sprite refresh (IN7) was built.
- ▢ **Readiness/residency transition measurements.** The main room-commit
  correctness slice remains MULTIPLAYER-SEAMS. For later performance work,
  measure GPU upload cost, byte-based residency, retained CPU images and the
  remaining loading-timer regression on a real target profile. Owner:
  [`engine/readiness-driven-room-transitions.md`](engine/readiness-driven-room-transitions.md).
- ▢ **Portal observer and capture work (P5/P6).** Same-room multiview,
  rolled viewport clipping, ordered crossing samples, scoped parallax and
  admitted ready images are explicit customers. Preserve current window
  pixel density/freshness and retained pools; measure CPU/GPU work separately.
  Detail: [view/capture contract](engine/portals-one-transit-for-every-mover.md).
- ▢ **Movement/portal policy witness.** Finish the portal/gravity-zone
  behavior witness when that movement slice is selected. Block-to-chain
  crawl transfer remains customer-gated, not a default engine expansion.
  Owner: [`engine/unified-movement-kernel.md`](engine/unified-movement-kernel.md).
- ▢ **Noodly Temple content polish.** Door feedback, key HUD, weapon pose,
  meatball presentation and biome placement are authored content follow-ups,
  not engine correctness blockers. Promote a bounded region slice when
  selected. Owner: [`game/noodly-temple.md`](game/noodly-temple.md).
- ▢ **World/playthrough acceptance.** The headless persistent-world path and
  independent SDK/navigation acceptance are built. Its remaining death/fact
  horizon work belongs to the active DIALOGUE-CHECKPOINT row; do not restart
  WORLD-ACCEPTANCE or SDK-GAME as separate campaigns.

## Standing execution rule

Before promoting a card:

1. inspect HEAD and confirm the missing thing still exists;
2. prefer the focused plan that already owns the design;
3. state the product, authority or dependency payoff;
4. give one inspectable acceptance criterion;
5. promote only executable work to `queue.md`.

## Deferred Smash semantics

- ▢ **Knockback-threshold armor variant.** Damage-threshold and super armor are implemented (`S0.5`); a separate knockback threshold has no selected authored customer or resolved physical quantity. Do **not** invent a unitless threshold or block the S0 parity gate waiting for a ruling on an unselected extension. Promote when a real move needs it; choose a typed `HitKnockbackMagnitude` interpretation and validate where launch magnitude becomes available. Owner: [Smash parity inventory](demos/smash-parity-inventory.md#smash-local-p0-raw-two-fighter-combat-semantics).

## Fighter evaluation residuals

- ▢ **Mirror/seat evaluation semantics.** `--paired` currently exchanges
  noise streams rather than seats, so it cannot remove seat-keyed bias.
  Separate that rig correction from the policy question of ordinary
  per-seat cognition streams versus character-keyed symmetry. Promote when
  representative fighter-ladder evaluation or the Q49 mirror witness needs
  unbiased pairing. Owner: [fighter brain](engine/fighter-brain.md#mirror-symmetry-is-a-correctness-property-q49).
- ▢ **Fixed-tick match-report pacing.** The October 30 run-to-run variance
  investigation found a per-host-frame skipped-simulation-tick cadence and
  phase-dependent gravity-modifier divergence even while sampled `sim_dt`
  stayed fixed. The root cause is **not established**. Promote with a
  reproducible current-HEAD two-process trace before modifying the clock or
  fighter scoring. Owner: [fighter brain timing investigation](engine/fighter-brain.md#modeling-rules-for-the-option-layer)
  and the simulation/rollback scheduler.

## Deferred experiment from the recent plan triage

- **Fighter tactical intent:** the first measured gate failed and the maintainer
  paused the experiment. Keep `tactics` off in shipped profiles. A later
  selection starts with the failed punish trace, not a production rollout.
  Owner: [fighter tactical intent](engine/fighter-tactical-intent.md#5b-first-gate-run-2026-10-31-not-passed).
