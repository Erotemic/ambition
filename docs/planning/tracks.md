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

[`engine/architecture.md`](engine/architecture.md) supplies the current
ownership model. The
[actor-monolith frontier](engine/actor-monolith-work-frontier.md) owns active
architecture packet gates. Work already selected in `queue.md` is not repeated
here.

### Persistent systemic world

- ▢ **Open-world residency beyond live rooms.** Several live rooms exist.
  Preserve the distinction among world existence, room residency, simulation
  activity and local visibility. Promote background simulation of rooms that are
  not live only when a real mechanic needs it; today the session clock covers
  time that passes while a room is not live. Owner:
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

- ▢ **N-view production composition.** Each view frames its own player and
  draws its own live room. Promote broader layout, HUD ownership, input routing
  and an Ambition join road (`Q153`, ruled 2026-10-19) when Ambition or TwinTrack needs them.
  Owner:
  [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md).
- ▢ **Per-view camera/reference-frame policy.** Extend shared/split-view policy
  from real multiview requirements. Owner:
  [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md).
- ▢ **Real network transport / Matchbox signaling.** Trigger only with an actual
  online Ambition or Smash customer. Owner: [`engine/netcode.md`](engine/netcode.md).

### World facts, orchestration and agentic characters

- ▢ **Deterministic world facts + observations/memory.** Keep simulation truth
  separate from what a character observed or believes. Use
  `scripts/measure/authored_route_gates.py` for route-condition measurements instead of
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
- ▢ **Asset library and editor lifecycle.** Measure the real time for focused
  publication or startup, or promote a real asset-browser operation. Keep
  staleness advisory and preserve the native no-rebuild path. Owner:
  [`engine/asset-library-editor-and-delivery.md`](engine/asset-library-editor-and-delivery.md).

## Unselected residuals found in the planning cleanup

These were explicit unfinished items inside dated owner diaries. This section
is a **reservoir**, not a second execution queue; verify each against current
source before promotion.

- ▢ **Multiroom moving-platform visuals.** A 2026-10-24 ignored witness
  `the_visual_follows_its_set_while_a_second_room_is_live` reported that
  `sync_moving_platform_visuals` used `Single<RoomInstanceRoot>` and stopped
  when two room roots were live. Promote if reproducible: resolve each
  platform's room, unignore the test, and verify two live rooms. Owner:
  [`engine/performance-and-composition-findings.md`](engine/performance-and-composition-findings.md#f48--after-the-content-gates-what-is-left-is-bevys-and-jons-2026-10-24).
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
- ▢ **Foreign/UI optimizer follow-ups.** The 2026-10-17 performance audit
  mentioned `virtual_joystick::update_ui` compare-before-write and an open
  `bevy_light` analysis. Reprofile on a current target first and obtain a
  concrete cost/ownership boundary before selecting upstream or fork work.
  Owner: [`engine/performance-and-composition-findings.md`](engine/performance-and-composition-findings.md).
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
- ✖ **Falling sand.** Code removed 2026-10-11 (Jon); not a work item. A
  reimplementation is a new design. Owner: [`engine/falling-sand.md`](engine/falling-sand.md).
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
  switch through `MusicDirectorState`. `provider` and `experience` share string
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
- ▢ **Actor effects/modifier composition.** Seven-phase plan needs a selected
  concrete modifier consumer before implementation; current source may already
  cover individual effects. Owner:
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
- ▢ **Portal host with limbs in multiple simultaneous links.** Existing host
  transit selects one pair; extend shared partitioning only when a shipped
  host or test scene requires two links at once. Owner:
  [`engine/render-animation-and-vfx.md`](engine/render-animation-and-vfx.md).
- ▢ **Review follow-ups requiring a fresh measurement.** Cold/warm rig table
  preparation memory and frame stalls; human acceptance for robot
  crouch/turn/double jump (REVIEW-1014); rendered portal-piece frame cost
  (IN6); feature-index instance key only when one definition is live twice
  (IN1); profile large-room terrain skin setup only if still slow;
  `StageView::room_toward` on rotated gravity (REVIEW-1013); a held
  portal pair crossing rooms (REVIEW-1012) only when a real customer
  needs it; human front-idle/combat and fighter-pad acceptance. The
  second-instance door-routing policy (REVIEW-1011) also needs a live
  instance-creating customer. Owner:
  [`triage/2026-10-11-integration-source-audit.md`](triage/2026-10-11-integration-source-audit.md)
  and relevant renderer/asset documents. Sprite refresh (IN7) was built.
- ▢ **Readiness/residency transition measurements.** The main room-commit
  correctness slice remains MULTIPLAYER-SEAMS. For later performance work,
  measure GPU upload cost, byte-based residency, retained CPU images and the
  remaining loading-timer regression on a real target profile. Owner:
  [`engine/readiness-driven-room-transitions.md`](engine/readiness-driven-room-transitions.md).
- ▢ **Portal presentation seams (S4).** Recheck portal edge strips,
  parallax continuity and mobile frame cost after the mechanical transit
  tests, rather than repeating completed body transport. Owner:
  [`engine/portals-one-transit-for-every-mover.md`](engine/portals-one-transit-for-every-mover.md).
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
