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

- ⏸ **Controlled-character actor kernel — LIVE AS LEDGER ROW D117, and RESTING.**
  ⚠ do not promote again; the row records that control authority converged (one
  `tick_controlled_brains`) and that only the time-integrator fork remains, blocked
  on the hit-emphasis decision. Original card: remove protagonist-special
  assumptions from generic actor decision,
  targeting/crowd arbitration and control flow before widening world/multiplayer
  systems. Use
  [`engine/controlled-character-actor-kernel.md`](engine/controlled-character-actor-kernel.md).

- ◐ **Systemic open-world foundation. PROMOTED TO THE LEDGER 2026-08-14 as D125 —
  do not promote it a second time.** ⚠ all six plans below had been reachable from
  this card and from NOWHERE else, so a fully-designed frontier was invisible to
  the queue, which is the only authority a working session consults. ⭐ **that is
  what this reservoir is for, and the lesson is that a card sitting here
  indefinitely is not "queued", it is stranded** — when the ledger thins, promote
  from here rather than inventing new work. Build compatible semantics for
  room/region residency, instance provenance/lifetime, item custody/accounting,
  embodied capability gates, platformer reachability and persistent actor
  populations.
  Use [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md),
  [`engine/instance-lifetime-provenance-and-persistence.md`](engine/instance-lifetime-provenance-and-persistence.md),
  [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md),
  [`engine/capability-progression-and-world-gating.md`](engine/capability-progression-and-world-gating.md),
  [`engine/platformer-navigation-and-reachability.md`](engine/platformer-navigation-and-reachability.md)
  and [`engine/persistent-actors-and-population.md`](engine/persistent-actors-and-population.md).

- ▢ **World facts + reactive/agentic characters.** Keep deterministic world truth
  separate from observations/beliefs, then let character policies/dialogue choose
  actions against that truth. Use
  [`engine/world-facts-observations-and-memory.md`](engine/world-facts-observations-and-memory.md),
  [`engine/agentic-character-runtime.md`](engine/agentic-character-runtime.md) and
  [`game/reactive-characters-and-dialogue.md`](game/reactive-characters-and-dialogue.md).

- ▢ **Presentation + observability product surface.** Converge render/animation/VFX,
  participant/view-aware UI and machine-readable inspection without building a
  second simulation or monolithic editor. Use
  [`engine/render-animation-and-vfx.md`](engine/render-animation-and-vfx.md),
  [`engine/ui-localization-and-accessibility.md`](engine/ui-localization-and-accessibility.md)
  and [`engine/inspection-diagnostics-and-workbench.md`](engine/inspection-diagnostics-and-workbench.md).

- ▢ **Bevy-native reusable crate extraction.** For each mature reusable domain,
  move registration with its plugin, narrow dependencies, prove a minimal
  consumer, and evaluate independent Bevy-game value before publication. Use
  [`engine/bevy-plugin-and-crate-strategy.md`](engine/bevy-plugin-and-crate-strategy.md).

- ◐ **Ambition-first authoring/LDtk + kinematic world objects — LIVE AS LEDGER ROW
  D115.** ⚠ K2/K3/K4 are CLOSED; K5 and K6 remain. Do not promote again. Card:
  Moving platforms
  already author through LDtk, but their path references, motion-mode shape,
  diagnostics, dynamic-geometry ownership and contact/crush semantics are not
  yet Engine-1.0 quality. Use
  [`engine/authoring-and-tools.md`](engine/authoring-and-tools.md),
  [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md)
  and [`engine/kinematic-world-objects.md`](engine/kinematic-world-objects.md).

- ◐ **Ambition multiplayer + multi-view presentation — LIVE AS LEDGER ROW D116.**
  ⚠ M2 is nearly closed (per-view projections landed); do not promote again. Card:
  Build one participant
  model that supports local, online and mixed parties independently of shared,
  fixed-split or adaptive split-screen. Grow toward two resident rooms when
  participants separate. Use
  [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md)
  and [`game/multiplayer.md`](game/multiplayer.md).

- ⏸ **Per-view camera reference frames — LIVE AS LEDGER ROW D118, a REST ROW whose
  remainder belongs to D116.** ⚠ do not promote again. Card: Preserve the current external/world
  observer camera and add an optional controlled-body/view-subject-relative mode
  so gravity changes can visually rotate the world. Keep it a view policy, pair
  it coherently with body-relative input, and make rotated clamps/portal roll
  composition correct. Use
  [`engine/camera-reference-frame-policy.md`](engine/camera-reference-frame-policy.md).

- ▢ **Simulation authority and deterministic phase structure.** Decompose
  parameter-ceiling systems such as actor-brain ticks by semantic phase and
  ownership; invert rollback declaration ownership so the generic runtime is not
  a census of every gameplay domain. Use
  [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md)
  and [`triage/bevy-system-parameter-architecture.md`](triage/bevy-system-parameter-architecture.md).

- ▢ **Drain the actor monolith by coherent ownership — LIVE AS LEDGER ROW D33, and
  currently the only live row with NO agent on it.** Choose carves from
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

- ◐ **Room transition / multi-room transactionality — LIVE AS LEDGER ROW D71/D92.**
  ⚠ the census is closed and the transition is measured (asset-bound: preflight
  1.7ms, manifest 14.7ms, 0 of 164 assets settled at commit). Do not promote again.
  Card: Keep real
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

- ▢ **Deterministic authored gameplay logic and orchestration.** ⭐ LIVE as
  **D127** in the ledger — do not promote a second copy. Authoring is strong for
  **nouns** and weak for **verbs and relationships over time**; several
  independent partial condition → effect implementations already exist in tree.
  Rust extends the engine's vocabulary; authored content composes vocabulary that
  already exists. ⛔ not scripting, not a god enum, and **M0 (inventory +
  evidence) is the only authorized step** — the campaign starts when its two
  proof customers are ready or when product work repeatedly demands bespoke Rust
  behavior wiring. Use
  [`engine/authored-gameplay-logic-and-orchestration.md`](engine/authored-gameplay-logic-and-orchestration.md).

## Authoring and content reservoir

- ▢ **Binding-resolution residual defects.** ⭐ **dispositioned out of the
  stranded list 2026-08-15 — this is live work with concrete fixes, not spent
  history.** The campaign's mechanism landed (`Ref<N>`, `Resolver<N>`, `Bound<N>`,
  structured unresolved diagnostics, construction-time refusal); what remains is
  (1) per-frame item-art diagnostics passing generic declarers like `"ground
  item"` instead of provider/source identity, so **two providers with the same
  unresolved id suppress one another's diagnostic**, and (2) failed-file detection
  beyond item art where invisibility is real. ⛔ **do not reopen a campaign to
  wrap every string id** — keep a slice only when it removes a real silent-failure
  or duplicate-authority path. ⭐ note the adjacency: this machinery is what A9's
  reverse-reference queries and D127's prepared references would build on. Use
  [`engine/binding-resolution-boundary.md`](engine/binding-resolution-boundary.md).

- ▢ **Closeout review followups (2026-07-20), residual only.** ⭐ **also
  dispositioned out of the stranded list**: three items re-verified against HEAD
  on 2026-08-13 — portal mapping convention as **session authority rather than a
  process global** (⭐ which is the same class as the multiview process-globals
  D116 named), honest shipping/fresh-clone configurations, and a measured
  runtime-scale pass with cheap bounded fixes plus a collision-composition
  measurement. Each carries its own Evidence/Work/Exit. Use
  [`engine/closeout-review-followups-2026-07-20.md`](engine/closeout-review-followups-2026-07-20.md).

- ▢ **Project-wide semantic dependency/reference graph.** *"what references
  character:fia?"*, *"what breaks if I delete this?"* — reverse references,
  structured unresolved-reference diagnostics and transactional rename planning
  across authoring backends. ⛔ an extension of existing authoring/inspection
  work, **not** a new campaign, and ⛔ not a second index that can disagree with
  prepared references. Use `A9` in
  [`engine/authoring-and-tools.md`](engine/authoring-and-tools.md).

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

- ⏸ **Smash body-generic residuals — LIVE AS LEDGER ROW D72, and PAUSED at Jon's
  direction** (do not tune the fighter until higher-leverage architecture is
  exhausted). Card: The old migration diary is closed. Current
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

- ▢ **Ambition open-world game production.** Build the main game through
  [`game/open-world-roadmap.md`](game/open-world-roadmap.md),
  [`game/systemic-progression.md`](game/systemic-progression.md),
  [`game/vision.md`](game/vision.md), [`game/bosses.md`](game/bosses.md), direct
  maintainer observations and [`game/multiplayer.md`](game/multiplayer.md).
  Prove a systemic world before relying on linear story gates; later story work
  should consume the same persistent facts, actors, items and capabilities.

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
  becomes relevant. ⚠ **Jon does not want to carry a leafwing fork**, and the
  Ambition side is already landed and inert until the dependency changes; the
  measured cost is **1–3.1% of frame CPU** in every gameplay chunk. Detail:
  [`triage/leafwing-clash-scan-patch-2026-07-23.md`](triage/leafwing-clash-scan-patch-2026-07-23.md).
- **Localization / translation catalog** — trigger-based. No translation catalog
  or runtime locale system exists; some vocabularies already carry stable ids or
  reserved i18n keys. Detail:
  [`engine/presentation-and-shell-audit.md`](engine/presentation-and-shell-audit.md)
  and [`engine/ui-localization-and-accessibility.md`](engine/ui-localization-and-accessibility.md).
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

## ⚠ Stranded plans — promote or retire (measured 2026-08-15)

⭐ **this section exists to end the structural invisibility, not to judge these
documents.** A planning doc reachable from no ledger row, no reservoir card, no
`status.md` entry and no other document is invisible to the execution authority —
that is exactly how seven fully-written Engine 1.0 plans sat unreachable until
2026-08-14, and the fix then was **promotion, not writing**.

Measured by basename across all of `docs/` (excluding `docs/archive`, which is
evidence rather than authority) and `AGENTS.md`: **13 planning documents, 2,048
lines, referenced by nothing.** Listing them here makes them reachable. ⛔ **a
disposition is still owed for each** — promote to a card or retire to the
archive; being listed here is not a verdict that the work is live.

- `engine_rename_campaign.md` (322)
- `engine/portable-preparation-and-load-explainability.md` (484)
- `engine/sprite-renderer.md` (174)
- ~~`engine/closeout-review-followups-2026-07-20.md` (212)~~ ✔ **PROMOTED** to a card above
- ~~`engine/binding-resolution-boundary.md` (55)~~ ✔ **PROMOTED** to a card above
- ~~`engine/combat-model.md` (60)~~ ✔ **ROUTED** — residual combat work verified
  against `cecd01ca`; CM1–CM5, CM7, CM8 landed. Current body-generic integration
  is owned by [`smash-body-generic-combat-2026-08-09.md`](smash-body-generic-combat-2026-08-09.md),
  which is live as **D72**. ⛔ do not schedule the landed items again.
- ~~`engine/presentation-and-shell-audit.md` (45)~~ ✔ **ROUTED** — the
  thirteen-domain audit is closed; what remains is forward capability gaps, of
  which **localization is trigger-based** (no translation catalog or runtime
  locale system exists). See the deferred/trigger list below.
- ~~`engine/shell-vanity-sequence.md` (44)~~ ✔ **ROUTED** — **VC5 only**, the
  title launcher fade-in. VC1–VC4 and VC6 are implemented. A single small
  presentation item, not a campaign.
- `triage/ambition-test-support.md` (306)
- `triage/ambition-registry-core.md` (231)
- ~~`triage/leafwing-clash-scan-patch-2026-07-23.md` (43)~~ ✔ **ROUTED** to the
  deferred/trigger list below, where its entry already existed without a link.
- ~~`triage/gameplay-presentation-profiles.md` (32)~~ ✔ **ROUTED** — GP1–GP5 are
  implemented (profile resolution, fixed/aspect viewport policy, surround layout,
  provider profile declaration, occupancy/control regions, touch placement, the
  HUD's first surround-region consumer). ⭐ **its remainder is adjacent to D116's
  deferred half** — viewport/layout policy is exactly where a second view's
  rectangle would have to come from, so read it before reopening that.
- ~~`game/ambition.md` (40)~~ ✔ **ROUTED from `status.md`** — ⚠ it is not residual
  work at all but a **structural hub** for the flagship customer (it links
  `vision.md`, `open-world-roadmap.md`, `systemic-progression.md`,
  `multiplayer.md`), and nothing pointed at it. That was a pure routing gap, and
  it is the reason this list exists.

⚠ **two of these are surprising and should be checked before anyone retires
them:** `game/ambition.md` describes the flagship game, and
`engine/combat-model.md` is live design vocabulary — that neither is linked from
the ledger, `tracks.md`, `status.md`, `README.md`, `roadmap.md` or `AGENTS.md` is
more likely a routing gap than a sign the content is dead.

⛔ **do not bulk-archive this list.** Two already declare themselves as
*residual work re-verified against HEAD*, which is live work with no route to it.

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
