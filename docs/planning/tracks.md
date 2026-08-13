# Tracks — standing backlog and work reservoir

**Role:** preserve worthwhile unresolved work across runs. This file does **not**
own execution order; [`queue-72h-2026-08-08.md`](queue-72h-2026-08-08.md) does.
When the queue needs more work, select from this reservoir, re-measure the claim
against HEAD, add the chosen work to the queue, and continue.

A focused plan owns technical design. This file should normally carry one compact
card and a link rather than a second implementation diary.

`▢` means an unresolved reservoir item. Completed execution narratives do not
stay here; git history and `docs/archive/` preserve them.

## Replenishment order

Use this order unless Jon or the live queue says otherwise:

1. finish the current focused campaign already selected by the queue;
2. direct unresolved items in
   [`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md);
3. architecture work that removes duplicate authority or materially lowers
   change amplification;
4. acceptance-game work that exercises a reusable engine capability;
5. product/features with settled intent but no current campaign; and
6. trigger-based/deferred work only when its trigger is actually present.

Do not infer priority from the age of a card or from whether another document
links to it.

## Active architecture reservoir

- ▢ **Finish D73 authority convergence.** Current ordering authority:
  [`authority-convergance-campaign-2026-08-13.md`](authority-convergance-campaign-2026-08-13.md).
  Close the campaign by its own AC7 criteria; do not reopen deleted archetype,
  mirror, or build-then-patch representations.

- ▢ **Smash body-generic convergence.** Use
  [`smash-body-generic-combat-2026-08-09.md`](smash-body-generic-combat-2026-08-09.md).
  The proving rule is one body/one path: improve the shared engine instead of
  adding Smash-only combat/body exceptions.

- ▢ **Rollback declaration ownership inversion.** After D73, move toward domain
  declarations/capabilities that the rollback/runtime composition consumes
  without the generic runtime importing every gameplay domain merely to register
  its types. The successor boundary is described in the final section of the
  authority-convergence campaign and in [`engine/netcode.md`](engine/netcode.md).

- ▢ **Bevy system-authority decomposition.** Break parameter-ceiling systems
  around semantic phases, named query data/SystemParams and explicit ownership;
  tuple packing is not a decomposition. Use
  [`triage/bevy-system-parameter-architecture.md`](triage/bevy-system-parameter-architecture.md).

- ▢ **Drain the actor monolith by coherent ownership.** Current candidates include
  presentation/authoring/session/menu-affordance seams and other boundaries that
  lower meaningful dependency closure. Do not carve by line count. Use
  [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
  and [`engine/decomposition.md`](engine/decomposition.md).

- ▢ **Public facade / optional capability closure.** Continue making the semantic
  facade hide implementation topology and keep capability selection honest. Use
  [`engine/api-growth-method.md`](engine/api-growth-method.md),
  [`engine/api-1.0-campaign.md`](engine/api-1.0-campaign.md), and the competitive
  roadmap; re-measure old campaign status before acting.

- ▢ **Room transition loading transaction.** Keep transition requests behind
  readiness/preparation/commit semantics, especially under rollback hosts; retain
  the genuinely open preload/performance work. Rewrite the old phase history when
  implementing from [`engine/room-transition-loading.md`](engine/room-transition-loading.md).

- ▢ **Rollback scope provenance correction.** `RoomScopedEntity`/possession made
  the old write-once-provenance assumption false. The current analysis and
  proposed behavioral probe live in
  [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md); this is
  agent-delegated architecture work rather than a blocker waiting on Jon.

- ▢ **Stable identity where strings/numbers still combine distinct lifetimes.**
  Keep `ParticipantId`, session seat, control channel, simulation slot, placement
  identity and display labels semantically distinct. Materialize
  `SessionSeatId`/`ControlChannelId` when topology work next makes that separation
  pay for itself; until then route through `LocalChannelPlan`. See
  [`engine/participant-action-system.md`](engine/participant-action-system.md)
  and [`triage/stable-identifier-centralization.md`](triage/stable-identifier-centralization.md).

- ▢ **Declared-id failures should become authoring diagnostics.** Prefer
  preparation/compiler/schema resolution with useful errors over a permanent
  runtime census or source scanner. See
  [`triage/declared-id-resolution-checks.md`](triage/declared-id-resolution-checks.md).

- ▢ **Cheap real behavioral test support.** Improve reusable harnesses when that
  makes meaningful integration/invariant tests materially cheaper; do not turn
  this into another policy-scanner framework. See
  [`triage/ambition-test-support.md`](triage/ambition-test-support.md) and
  [`engine/headless-verification.md`](engine/headless-verification.md).

## Construction, content and authoring reservoir

- ▢ **Provider-owned authored vocabulary.** Re-measure
  [`proposal-authored-vocabulary-2026-08-04.md`](proposal-authored-vocabulary-2026-08-04.md)
  against the Mary-O LDtk vocabulary that has since landed, then preserve only
  the still-open provider-extension problem.

- ▢ **Remaining content evictions from reusable engine crates.** When a real
  named family is still closed in core, migrate one structurally complete family
  at a time. Historical candidates include the item catalog, content-owned
  presentation tuning such as `deep_dream_strength`, and the Puppy Slug gun as a
  parameterized summon-ally ability + content data. Verify each candidate before
  touching it.

- ▢ **Character/action authoring followups.** Continue provider-extensible action
  schemas, context-aware controls, inventory/special-menu migration, vehicle and
  loading/retry contexts, and authorable binding/cue ownership through
  [`engine/participant-action-system.md`](engine/participant-action-system.md).

- ▢ **Editable component/SVG character authoring.** Continue the code-authored
  sprite workflow toward editable component/paper-doll authoring where it
  improves iteration without changing runtime body identity. See
  [`engine/svg-component-character-migration.md`](engine/svg-component-character-migration.md)
  and [`engine/sprite-renderer.md`](engine/sprite-renderer.md).

- ▢ **Sprite residency and live quality.** The basic quality mechanism exists;
  remaining residency cohorts, packaging and live-Apply work live in
  [`sprite-residency-and-live-quality.md`](sprite-residency-and-live-quality.md).

## Combat, AI and actor-behavior reservoir

- ▢ **Fighter-brain L3 acceptance gate.** The rollout implementation and basic
  fidelity instrumentation exist; what remains is the scenario suite and
  survival/damage-ratio gate, measured against a clearly named ladder. See
  [`engine/fighter-brain.md`](engine/fighter-brain.md).

- ▢ **One victim-side hit/death feedback seam.** Attack/volume authored effect
  identity should reach one victim-side resolution path instead of parallel
  attacker-kind payload branches. Use [`engine/combat-model.md`](engine/combat-model.md)
  and the current Smash campaign before assuming the July wording is still live.

- ▢ **Projectile contact against published body geometry.** The correctness half
  (an intangible/no-hurtbox body is not hit) landed; the remaining product choice
  is whether ordinary projectiles use the victim's published silhouette/parts
  instead of the coarse AABB, which changes shot feel. The active question is in
  [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

- ▢ **Boss animation vocabulary fold.** Converge remaining `BossAnim` / boss-frame
  projection toward shared `CharacterAnim` semantics and retire obsolete target
  mirrors. Do not reopen the already-shared boss body integration path. See
  [`engine/boss-system.md`](engine/boss-system.md) and
  [`engine/boss-design.md`](engine/boss-design.md).

- ▢ **Dialogue continuity in a running world.** Damage/separation can break a
  conversation, capable bodies hold station through ordinary movement ability,
  and interruption gets an outward bark. See
  [`engine/dialogue-continuity.md`](engine/dialogue-continuity.md).

- ▢ **Listener-side dialogue adaptation.** Speaker/listener identity is already
  body-generic/possession-aware; actual listener-side content adaptation remains
  a product capability rather than an identity bug.

## Acceptance-game reservoir

- ▢ **Sanic:** close the remaining single-source acceptance list in
  [`demos/sanic.md`](demos/sanic.md). Do not copy that list back here.

- ▢ **Super Mary-O:** continue from
  [`demos/super-mary-o.md`](demos/super-mary-o.md) and direct maintainer
  observations. Keep block/restart fixes game-owned unless they expose a reusable
  engine defect.

- ▢ **Smash:** use [`demos/super-smash-siblings.md`](demos/super-smash-siblings.md)
  for product acceptance and the D72 campaign for engine convergence.

- ▢ **Hollow-lite / bosses:** exercise authored combat, encounters and high-quality
  multi-phase boss authoring through [`demos/hollow-lite.md`](demos/hollow-lite.md)
  and [`engine/boss-design.md`](engine/boss-design.md).

- ▢ **TwinTrack relativity lab.** Continue the 2D relativity acceptance/research
  path through [`engine/relativity.md`](engine/relativity.md) and
  [`demos/twintrack.md`](demos/twintrack.md): observer-local presentation,
  light-delay mechanics, Doppler/clock pedagogy, dual-observer play and the
  spacetime visualization remain valid candidate directions.

## Product/features deliberately kept alive

- ▢ **Character dialogue from suggestion/bark authoring.** Design is settled and
  implementation is intentionally shelved; do not delete it as stale. See
  [`triage/character-dialogue-from-suggestions.md`](triage/character-dialogue-from-suggestions.md).

- ▢ **Falling-sand extensions.** Sand exists; water/oil and associated Oiler
  mechanics are deferred product work, not rejected ideas. See
  [`engine/falling-sand.md`](engine/falling-sand.md).

- ▢ **Per-route music inside an experience.** The route-keyed frontend/audio
  architecture landed; route-specific music remains a useful capability. See
  [`frontend-audio-is-per-experience.md`](frontend-audio-is-per-experience.md).

- ▢ **Player-facing art/content repairs that are not architecture campaigns.**
  Morph-ball presentation is primarily missing art, shrine + glider presentation
  still needs repair, and the kernel-guide NPC still wants peaceful patrol
  behavior from authored brain policy. Keep these as product work rather than
  inventing engine special cases.

- ▢ **Game cast, bosses and story.** Preserve open product intent in
  [`game/characters.md`](game/characters.md), [`game/bosses.md`](game/bosses.md)
  and [`game/vision.md`](game/vision.md) even when no architecture campaign points
  at it.

## Documentation work that is still genuinely useful

These are documentation migrations because the underlying design is still worth
keeping, not because every old plan needs polishing:

- ▢ rewrite [`engine/room-transition-loading.md`](engine/room-transition-loading.md)
  around the current transaction/readiness model and retain only real open
  preload/performance work;
- ▢ distill [`engine/encounter-orchestration.md`](engine/encounter-orchestration.md)
  from completed E8–E13 execution history to the surviving encounter model;
- ▢ reconcile [`engine/boss-system.md`](engine/boss-system.md) with
  [`engine/boss-design.md`](engine/boss-design.md) so current boss architecture
  has one durable authority; and
- ▢ archive [`engine/shell-vanity-sequence.md`](engine/shell-vanity-sequence.md)
  after its remaining VC5/VC6 product work is either completed or moved to a
  live backlog card.

## Deferred / trigger-based work

These stay discoverable without occupying current execution order:

- **Matchbox/P2P netplay + predicted-A/corrected-B oracle** — start when a real
  game needs online play; [`engine/netcode.md`](engine/netcode.md).
- **Slower Light** — future 3D game; wait for a 3D runtime;
  [`engine/slower-light.md`](engine/slower-light.md).
- **Leafwing clash-scan optimization** — apply only if the dependency/version
  trigger described in
  [`triage/leafwing-clash-scan-patch-2026-07-23.md`](triage/leafwing-clash-scan-patch-2026-07-23.md)
  becomes relevant.
- **Broader stable-id centralization** — do not invent one universal ID framework
  ahead of concrete identity families; use the focused triage document when a
  real migration provides the pressure.

## Standing execution rule

The reservoir exists so the continuation queue does not run out of valuable
work. It is intentionally **not** an execution diary.

Before promoting a card into the queue:

1. inspect HEAD and verify the missing thing is actually still missing;
2. prefer the focused plan that already owns the design;
3. state the deletion/authority/product payoff, not merely a process step; and
4. keep tests proportional to the invariant: behavior tests behavior, structure
   should encode architecture, and migration scans normally retire with the
   migration.
