# Tracks — standing backlog and work reservoir

**Role:** preserve worthwhile unresolved work across runs. This file does **not**
own execution order; [`queue.md`](queue.md) does.
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

⭐ **`engine/character-authoring-package.md` is NOT in this reservoir — it went
straight to the queue as D165 on 2026-08-17** (maintainer direction), with a
canonical character height in shared world units as its first slice. It is noted
here only because it was invisible from every ledger for weeks, and this is one of
the two files a replenishing agent reads. ⛔ do not copy its content here: the
plan owns the design and D165 owns which slice is live.

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

- ⏸ **Ambition-first authoring/LDtk + kinematic world objects — RESTING as ledger
  row D115.** ✔ **K2–K6 are ALL closed** (2026-08-15). ⛔ do not promote again and
  ⛔ do not manufacture a K6 customer — the second dynamic-geometry customer turned
  out to be the **door**, and the door does not move. ⭐ preserve the discovery it
  left: **geometric displacement ≠ surface drag**, and `Block::velocity` conflates
  them, so a conveyor-like customer needs that split first. Card:
  Moving platforms
  already author through LDtk, but their path references, motion-mode shape,
  diagnostics, dynamic-geometry ownership and contact/crush semantics are not
  yet Engine-1.0 quality. Use
  [`engine/authoring-and-tools.md`](engine/authoring-and-tools.md),
  [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md)
  and [`engine/kinematic-world-objects.md`](engine/kinematic-world-objects.md).

- ⏸ **Ambition multiplayer + multi-view presentation — RESTING as ledger row
  D116.** ✔ M2's **presentation/projection** half is closed; ▢ its **production
  N-view composition/layout, HUD ownership and input routing** remain deferred.
  ⛔ do not promote again and ⛔ do not expand into networking. Card:
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

- ▢ **Capability/runtime composition — PROMOTED TO THE LEDGER 2026-08-16 as D136;
  do not promote it a second time.** Make optional capabilities honest in
  Cargo dependency closure and runtime/plugin assembly; a minimal consumer
  should not silently inherit unrelated domains. Use
  [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).
  ⭐ promoted because five independent slices in one night turned out to be this
  same failure — the engine unable to ship its own art, a persistence authority
  absent from every headless harness, a demo's rules reaching another demo's
  fighter, a laundered crate edge becoming a declared one, and a format-specific
  field in the canonical session world. The card could not have made that
  argument; the instances can.

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

- ✔ **Rollback scope provenance correction — the behavioural proof EXISTS,
  verified 2026-08-22.** Four tests, all green, and none of them source-only:
  `possession_suspends_residency_without_touching_the_lifetime` (the lifetime is
  unchanged by possession), `a_possessed_body_is_carried_through_a_room_transition`
  (against the real transition), `losing_the_target_hands_control_back_to_home`
  (destruction while driven), and
  `an_authored_actor_carried_out_of_its_room_and_back_does_not_meet_a_copy`.
  ⭐ the first one's doc records that it used to assert the OPPOSITE — that
  possession promoted a body out of room scope — and that the promotion was the
  defect, which is the provenance question this row was holding open.

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

- ▢ **Provider-defined actions through the full physical/UI seam — PROMOTED TO
  THE LEDGER 2026-08-26 as D242.** Semantic action registration exists. Finish
  provider-defined actions through physical binding, cues/touch and remaining
  participant contexts without restoring a seat-0 special path. Use
  [`engine/participant-action-system.md`](engine/participant-action-system.md).
  ⚠ that doc's header verifies against `cecd01ca` (2026-08-13); re-grep each of
  its nine items before working it.

- ▢ **Authoring diagnostics for declared IDs.** Resolve bad authored references
  during preparation/compiler/schema validation with provenance and useful
  errors. Do not grow a permanent runtime census. Use
  [`triage/declared-id-resolution-checks.md`](triage/declared-id-resolution-checks.md).

- ▢ **Remaining named-content evictions.** When reusable engine crates still own
  a closed Ambition-specific family, migrate one structurally complete family at
  a time after verifying it is still present.
  ⭐ **VERIFIED 2026-08-26 AND THERE IS ESSENTIALLY ONE LEFT — the rest is prose
  and test data.** Searching engine crates for DEFINITIONS named after Ambition
  content (not mentions) finds: `ambition_combat::moveset::player_robot_slash`
  (57 lines, three cue consts + one overlay fn, ONE production caller —
  `avatar/starting_character.rs:275`) and the `PLAYER_ROBOT_SLASH_*` ids it is
  compile-time pinned to in `ambition_sfx::ids`. Everything else that greps
  (`pirate`, `goblin`, `sanic`, `mary_o`) is comments or test fixtures.
  ⛔ **AND IT IS NOT A FAMILY THAT CAN MOVE ALONE, which is why it is still
  here.** The overlay's whole point is a `const _: () = assert!` pinning each cue
  string to an `ambition_sfx::ids` entry, and that id table is a SHARED
  VOCABULARY of 165 ids (`player` 45, `world` 37, `ui` 25, `portal` 10, `hazard`
  9, `dialogue` 8, `vfx` 5) that the audio pipeline generates against. Evicting
  the robot family means moving the ids, the pin AND keeping the generated
  registry's keys — three coupled things for a purity gain. ⇒ **price it as an
  AUDIO-VOCABULARY question, not a moveset one**, and do not start it as a
  five-minute file move.

- ▢ **Editable SVG/component character authoring.** Continue the procedural
  sprite workflow toward editable component/paper-doll authoring where it
  improves iteration without changing runtime identity. See
  [`engine/svg-component-character-migration.md`](engine/svg-component-character-migration.md).

- ▢ **Sprite residency and live quality.** Finish useful residency cohorts,
  packaging and live Apply behavior. See
  [`sprite-residency-and-live-quality.md`](sprite-residency-and-live-quality.md).

## Combat, AI and actor-behavior reservoir

- ▢ **Smash product push — LIVE AS D72.** The product is active; do not wait on
  unrelated architecture migrations. Use
  [`demos/super-smash-siblings.md`](demos/super-smash-siblings.md) for the
  charter, [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md)
  for current feature truth, and the active campaign linked from the charter for
  execution order. `E1` features may add a small reusable engine semantic;
  `E2` features get focused campaigns; `WAIT` rows stay deferred.

- ▢ **Fighter-brain evaluation/calibration.** The brain stack exists; build the
  scenario outcome runner and calibrate the difficulty ladder through measured
  survival/damage/recovery evidence. Use
  [`engine/fighter-brain.md`](engine/fighter-brain.md).

- ✔ **Projectile contact against published body geometry — CLOSED 2026-08-22.**
  Both halves are in. Intangible/no-hurtbox bodies were already correct; the
  ordinary-feel half needed decision 1, Jon ruled it 2026-08-17 (*"the projectile
  respects the AUTHORED HURT VOLUME — the same geometry melee uses"*), and
  `step_projectiles` now asks `victim.reached_by(&kin.aabb().into())` — the same
  `strike_reaches_victim` rule melee and feature hits ask. Pinned by
  `a_bolt_misses_the_gap_in_an_authored_silhouette`.
  ⛔ it is a real feel change on shipped content and it is the intended one: a
  shot that used to land on a body whose authored volume is tighter than its box
  now misses, and one that grazes an edge now connects.

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
  [`demos/super-smash-siblings.md`](demos/super-smash-siblings.md) as the product
  index and [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md)
  as the one standing feature backlog; Ambition remains the flagship.

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

## ⚠ Stranded plans — promote or retire

⭐ **this section exists to end structural invisibility, not to judge these
documents.** A planning doc reachable from no ledger row, no reservoir card, no
`status.md` entry and no other document is invisible to the execution
authority — that is how seven fully-written Engine 1.0 plans sat unreachable
until 2026-08-14; the fix was **promotion, not writing**.

The 2026-08-15 census (13 docs, 2,048 lines, reachable from nothing) is fully
routed:

- `engine/portable-preparation-and-load-explainability.md` — D124's own plan; the D124 row now links it.
- `engine_rename_campaign.md` — the stale `Sandbox*` rename is complete (`check_retired_crate_names.py` reports none). ⇒ misfiled, not dead: architecture/product work (including couch multiplayer) was bundled into it and should move to the topic plans that own it.
- `engine/sprite-renderer.md` — routed as REFERENCE: **measure-by-default** — a sprite ships the geometry the gameplay layer needs, so a body and its hitbox cannot silently disagree.
- `engine/closeout-review-followups-2026-07-20.md`, `engine/binding-resolution-boundary.md` — promoted to reservoir cards above.
- `engine/combat-model.md` — current body-generic combat ownership contract; the Smash inventory owns product feature status and D72 owns execution selection. The old Smash successor plan is superseded and archived.
- `engine/presentation-and-shell-audit.md` — the thirteen-domain audit is closed; localization remains trigger-based (see deferred list).
- `engine/shell-vanity-sequence.md` — VC5 only (title launcher fade-in) remains; VC1–VC4, VC6 implemented.
- `triage/ambition-test-support.md` — strong candidate, design decisions pending. ⛔ the dependency boundary and fixture API must be piloted before promotion, not adopted wholesale.
- `triage/ambition-registry-core.md` — proposed direction: one dependency-light protocol for the registry pattern repeated across Ambition. ⛔ prove it through one or two migrations before broad adoption.
- `triage/leafwing-clash-scan-patch-2026-07-23.md` — routed to the deferred/trigger list below.
- `triage/gameplay-presentation-profiles.md` — GP1–GP5 implemented; its remainder is adjacent to D116's deferred multi-view half.
- `game/ambition.md` — the flagship's structural hub; was a pure routing gap, now linked from `status.md`.

⛔⛔ **the census must be RE-RUN, not trusted — 2026-08-20.** A fourteenth doc,
`engine/world-geometry-and-spatial-semantics.md` (673 lines, WARRANTED with a MET
trigger since 2026-08-15), was stranded the whole time. Re-measuring across the
whole repository (every `.md`/`.rs`/`.py`/`.sh`/`.toml`/`.ron`, 34,790 files)
found exactly **1 of 267 planning docs referenced by nothing**. Promoted to
**D169**. ⇒ a struck-through list is a receipt for one measurement, not a
standing guarantee — the census is a dozen lines of Python and finding one
live plan pays for it.

⭐ **RE-RUN 2026-08-21, and it paid again: 6 of 267 referenced by nothing, and
one of them was a live ADR.**

```text
docs/adr/0021-authoring-backend-agnostic-world-ir.md   ⇒ ROUTED
  "Accepted; IMPLEMENTED" — it minted `ambition_platformer2d_world` and
  `ambition_platformer2d_ldtk` and the boundary tests that ratchet their
  dependency direction. Neighbours 0019/0020/0022/0023 each had 1-3
  referrers; this had ZERO. Now cited from the LDtk crate's own header,
  which is where a reader asking "why are these two crates" arrives.

docs/brainstorms/{game_modes_and_data_sharing,sprite-enemy-ideas,the_fia_arc}.md
docs/learning/how-to-learn-enough-bevy-…md
docs/patches/llm-doc-cleanup-2026-05-17.md              ⇒ LEFT ALONE
  reference by NATURE, and all five are mtime 2026-07-07/08-01. The rule
  prefers PROMOTE when the mtime is recent because recent means Jon just
  put it there; none of these is recent, and a brainstorm nothing cites is
  not the same defect as a DECISION nothing cites.
```

⇒ **the census's value is the distinction, not the count.** A stranded PLAN or
DECISION is invisible work; a stranded brainstorm is just a brainstorm. Re-run
it by scoping to `docs/**` minus `archive/` — scoping to `docs/planning/**`
alone reports 0 of 99 and misses the ADR entirely, which is exactly how this one
survived the 2026-08-20 pass.

⛔ **do not bulk-archive the routed docs above.** `triage/ambition-test-support.md`
and `triage/ambition-registry-core.md` are residual work re-verified against
HEAD, i.e. live work with no route to it, not spent history.

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
