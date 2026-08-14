# Tracks — standing backlog and work reservoir

**Role:** preserve worthwhile unresolved work across runs. This file does **not**
own execution order; [`queue-72h-2026-08-08.md`](queue-72h-2026-08-08.md) does.
When the queue needs more work, select from this reservoir, re-measure the claim
against HEAD, promote the chosen work to the queue, and continue.

A focused plan owns technical design. This file should carry compact cards and
links, not duplicate implementation diaries.

`▢` means unresolved reservoir work. Completed execution history belongs in git
or `docs/archive/`.

## Replenishment order

Use this order unless Jon or the live queue says otherwise:

1. direct new maintainer direction and reproducible observations;
2. **Ambition flagship needs that expose reusable engine capability**;
3. architecture work that removes duplicate authority or major change/dependency
   amplification;
4. serious secondary game/acceptance customers that exercise reusable seams;
5. preserved product features with settled intent; and
6. trigger-based work only when its trigger actually exists.

D73 is closed. Do not promote its deleted archetype/roster/mirror work again.

## Engine 1.0 successor reservoir

The umbrella is
[`engine/engine-1.0-architecture-program.md`](engine/engine-1.0-architecture-program.md).
These cards are capability fronts, not a serial mega-campaign.

- ▢ **Ambition-first authoring/LDtk + kinematic world objects.** Moving platforms
  already author through LDtk, but their path references, motion-mode shape,
  diagnostics, dynamic-geometry ownership and contact/crush semantics are not
  yet Engine-1.0 quality. Use
  [`engine/authoring-and-tools.md`](engine/authoring-and-tools.md),
  [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md)
  and [`engine/kinematic-world-objects.md`](engine/kinematic-world-objects.md).

- ▢ **Ambition multiplayer + multi-view presentation.** Build one participant
  model that supports local, online and mixed parties independently of shared,
  fixed-split or adaptive split-screen. Grow toward two resident rooms when
  participants separate. Use
  [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md)
  and [`game/multiplayer.md`](game/multiplayer.md).

- ▢ **Simulation authority and deterministic phase structure.** Decompose
  parameter-ceiling systems such as actor-brain ticks by semantic phase and
  ownership; invert rollback declaration ownership so the generic runtime is not
  a census of every gameplay domain. Use
  [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md)
  and [`triage/bevy-system-parameter-architecture.md`](triage/bevy-system-parameter-architecture.md).

- ▢ **Drain the actor monolith by coherent ownership.** Choose carves from
  current dependency/authority evidence, especially boundaries that improve
  capability closure or iteration cost. Do not carve by line count. Use
  [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md).

- ▢ **Capability/runtime composition.** Make optional capabilities honest in
  Cargo dependency closure and runtime/plugin assembly; a minimal consumer
  should not silently inherit unrelated domains. Use
  [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).

- ▢ **Public SDK 1.0.** Continue hiding implementation topology behind semantic
  game concepts and close real provider ergonomics gaps through real consumers,
  not permanent blind-agent/source-scan ceremony. Use
  [`engine/public-sdk-1.0.md`](engine/public-sdk-1.0.md).

- ▢ **Performance and iteration as engine ergonomics.** Re-measure compile fanout,
  runtime/mobile budgets, multi-view cost, asset residency and authoring latency
  when a concrete slice makes them actionable. Use
  [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md).

- ▢ **Room transition / multi-room transactionality.** Keep real
  movement-kernel → loading-zone → readiness/commit behavior on one transaction,
  especially under rollback. This also becomes prerequisite evidence for
  Ambition participants occupying different rooms. Use
  [`engine/room-transition-loading.md`](engine/room-transition-loading.md).

- ▢ **Rollback scope provenance correction.** `RoomScopedEntity` provenance still
  needs a behavioral proof across possession/release scope changes before its
  rollback waiver/registration shape is trusted. Do not replace that proof with
  a source-only policy test.

- ▢ **Stable identity where lifetimes differ.** Keep participant, seat, input
  channel, simulation slot, placement identity and display labels semantically
  distinct. Materialize stronger ID types when multiplayer/topology work makes
  the distinction pay for itself. Use
  [`engine/participant-action-system.md`](engine/participant-action-system.md)
  and [`triage/stable-identifier-centralization.md`](triage/stable-identifier-centralization.md).

## Authoring and content reservoir

- ▢ **Provider-defined actions through the full physical/UI seam.** Semantic
  action registration exists. Finish provider-defined actions through physical
  binding, cues/touch and remaining participant contexts without restoring a
  seat-0 special path. Use
  [`engine/participant-action-system.md`](engine/participant-action-system.md).

- ▢ **Authoring diagnostics for declared IDs.** Resolve bad authored references
  during preparation/compiler/schema validation with provenance and useful
  errors. Do not grow a permanent runtime census. Use
  [`triage/declared-id-resolution-checks.md`](triage/declared-id-resolution-checks.md).

- ▢ **Remaining named-content evictions.** When reusable engine crates still own
  a closed Ambition-specific family, migrate one structurally complete family at
  a time after verifying it is still present.

- ▢ **Editable SVG/component character authoring.** Continue the procedural
  sprite workflow toward editable component/paper-doll authoring where it
  improves iteration without changing runtime identity. See
  [`engine/svg-component-character-migration.md`](engine/svg-component-character-migration.md).

- ▢ **Sprite residency and live quality.** Finish useful residency cohorts,
  packaging and live Apply behavior. See
  [`sprite-residency-and-live-quality.md`](sprite-residency-and-live-quality.md).

## Combat, AI and actor-behavior reservoir

- ▢ **Smash body-generic residuals.** The old migration diary is closed. Current
  reusable gaps include grab/hold/throw, optional richer shield semantics,
  body-scale equipment resolution if still desired, and local-N acceptance.
  Use [`smash-body-generic-combat-2026-08-09.md`](smash-body-generic-combat-2026-08-09.md).

- ▢ **Fighter-brain evaluation/calibration.** The brain stack exists; build the
  scenario outcome runner and calibrate the difficulty ladder through measured
  survival/damage/recovery evidence. Use
  [`engine/fighter-brain.md`](engine/fighter-brain.md).

- ▢ **Projectile contact against published body geometry.** Correctness for
  intangible/no-hurtbox bodies exists; ordinary projectile feel still needs the
  maintainer decision on authored silhouette/parts versus coarse body AABB.
  Keep it in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md)
  until that decision is made.

- ▢ **Boss animation vocabulary fold.** Converge remaining boss animation/frame
  projection onto shared character semantics without reopening boss body
  ontology. See [`engine/boss-system.md`](engine/boss-system.md).

- ▢ **Dialogue continuity in a running world.** Support body-generic interruption,
  separation and station-keeping semantics. See
  [`engine/dialogue-continuity.md`](engine/dialogue-continuity.md).

- ▢ **Listener-side dialogue adaptation.** Speaker/listener identity is already
  possession-aware; content adaptation to the listener remains a product
  capability.

## Game/customer reservoir

- ▢ **Ambition content and story.** Keep building the main game through
  [`game/vision.md`](game/vision.md), [`game/bosses.md`](game/bosses.md), direct
  maintainer observations and the new [`game/multiplayer.md`](game/multiplayer.md).
  Product work should expose reusable engine gaps without becoming subordinate
  to the acceptance suite.

- ▢ **Super Smash Siblings.** Serious platform-fighter customer and possible
  future first-class game. Use
  [`demos/super-smash-siblings.md`](demos/super-smash-siblings.md) and the concise
  Smash successor plan; Ambition remains the flagship.

- ▢ **TwinTrack.** Use it to force independent observer/reference-frame views,
  split-screen and relativity presentation through the same multi-view engine
  that Ambition needs. See [`demos/twintrack.md`](demos/twintrack.md).

- ▢ **Sanic / Super Mary-O / Hollow Lite.** Retain their focused acceptance lists
  as movement/collision, classic-world-authoring, and boss/encounter customers.
  Do not copy those lists back here.

## Product/features deliberately kept alive

- ▢ **Character dialogue from suggestions/barks.** Design is settled and
  intentionally shelved. See
  [`triage/character-dialogue-from-suggestions.md`](triage/character-dialogue-from-suggestions.md).
- ▢ **Falling-sand extensions.** Water/oil and Oiler-related mechanics remain
  desired deferred work. See [`engine/falling-sand.md`](engine/falling-sand.md).
- ▢ **Per-route music within one experience.** Route-specific music remains a
  useful capability. See
  [`frontend-audio-is-per-experience.md`](frontend-audio-is-per-experience.md).
- ▢ **Player-facing art/content repairs.** Morph-ball presentation, shrine/glider
  presentation and similar authored repairs stay product work unless a reproduced
  reusable-engine defect appears.

## Deferred / trigger-based work

- **Production network transport / Matchbox signaling** — start with the first
  actual online Ambition or Smash slice; the transport must feed the same
  participant/control model as local input. See [`engine/netcode.md`](engine/netcode.md).
- **Slower Light** — future 3D game; wait for a 3D runtime;
  [`engine/slower-light.md`](engine/slower-light.md).
- **Leafwing clash-scan optimization** — only when its dependency/version trigger
  becomes relevant.
- **Broader stable-ID centralization** — do not invent one universal ID framework
  ahead of concrete identity families.
- **Provider-owned placement families** — open a typed extension seam only when a
  real provider requires a family outside the common Tier-0 vocabulary.
- **Reusable menu-host extraction** — draw the reusable/product boundary from a
  real second consumer.
- **Boss crate extraction** — reassess after remaining boss vocabulary converges;
  do not preserve a legacy ontology by extracting it.
- **Body-generic NPC world interaction/economy** — when NPC agency or multiplayer
  currency requires it, reuse body interaction intent and body-owned wallet
  semantics rather than player/NPC-specific paths.
- **Test execution parallelism** — re-measure only if execution again dominates
  compile/link cost before adding runner machinery.

## Standing execution rule

The reservoir exists so the continuation queue does not run out of valuable
work. It is intentionally **not** an execution diary.

Before promoting a card:

1. inspect HEAD and verify the missing thing is still missing;
2. prefer the focused plan that already owns the design;
3. state the product/authority/dependency payoff rather than a process step;
4. keep tests proportional to the invariant; and
5. when a card closes, archive its campaign history instead of leaving another
   permanent completion narrative here.
