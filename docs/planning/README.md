# `docs/planning` — live planning control plane

This directory contains current work and open proposals, not binding decisions or repository history. Git history is the
archive. Before you treat a locally absent commit as lost evidence, read
[repository history and reconstruction](repository-history.md).

A planning document is here because it answers one of four questions:

1. What must happen next? → [`queue.md`](queue.md)
2. Which durable design owns that work? → a focused owner document under
   `engine/`, `game/` or `demos/`
3. Which explicit maintainer ruling constrains it? →
   [`maintainer-decisions.md`](maintainer-decisions.md)
4. Which question needs a maintainer answer? →
   [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md)

[`tracks.md`](tracks.md) holds valuable work that is not selected now.
[`status.md`](status.md) is a short orientation snapshot. Neither is an
execution diary.

## Decision status versus implementation status

Accepted ADRs can be revisited deliberately; an agent-written roadmap is not an explicit maintainer constraint. Treat **implemented**, **proposed**, **measured**, **tentative**, and **maintainer-required** as distinct statuses. The 2026-10-26 architecture reassessment is linked above; it does not silently approve large code migrations. See [ADR 0001](../adr/0001-source-of-truth-docs.md).

## Where the open work is

[`queue.md`](queue.md) is the one live engineering execution ledger. Work that is
not executable yet goes in a focused owner document or in the maintainer decision
ledger, not in a second queue.

## Index

Each planning document is listed once, here or in the sub-index that this page
links to.

### Control plane

- [`queue.md`](queue.md) — live execution order.
- [`status.md`](status.md) — orientation snapshot.
- [`tracks.md`](tracks.md) — standing backlog and trigger-based work.
- [`roadmap.md`](roadmap.md) — Ambition and Engine 1.0 direction.
- [`vision.md`](vision.md) — engine and product vision.
- [`maintainer-decisions.md`](maintainer-decisions.md) — explicit rulings.
- [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) — open
  questions for the maintainer.
- [`decision-principles.md`](decision-principles.md) — how to choose when you
  work autonomously.
- [`repository-history.md`](repository-history.md) — Git epochs and the cold
  store.

### Sub-indexes

- [`consolidation/README.md`](consolidation/README.md) — architecture
  consolidation census, plan, metrics and ledger.
- [`architecture-warts/README.md`](architecture-warts/README.md) — local
  semantic defects to re-measure.
- [`triage/README.md`](triage/README.md) — diagnosed findings that are not
  scheduled.
- [`demos/README.md`](demos/README.md) — secondary games, the Smash inventory
  and moveset reviews.

### Other root documents

- [`engine_rename_campaign.md`](engine_rename_campaign.md) — engine
  restructuring candidates and their triggers.
- [`moveset-inspector.md`](moveset-inspector.md) — combat inspection and the
  moveset observatory.
- [`test-corpus-static-audit-phase2.md`](test-corpus-static-audit-phase2.md) — static test-corpus audit and its follow-up evidence.

### Game (`game/`)

- [`game/vision.md`](game/vision.md) — Ambition game vision.
- [`game/ambition.md`](game/ambition.md) — Ambition as the flagship engine
  customer.
- [`game/open-world-roadmap.md`](game/open-world-roadmap.md) — open-world
  milestones.
- [`game/gate-network.md`](game/gate-network.md) — gate-network design and integration questions.
- [`game/systemic-progression.md`](game/systemic-progression.md) —
  capability-first progression.
- [`game/reactive-characters-and-dialogue.md`](game/reactive-characters-and-dialogue.md)
  — character reactions to world state.
- [`game/inhabited-world-dialogue-pass.md`](game/inhabited-world-dialogue-pass.md) —
  inhabited-world dialogue and presentation follow-up.
- [`game/art-and-ambiance.md`](game/art-and-ambiance.md) — backgrounds, terrain
  skins, foregrounds and motes for each biome; the rules of the art lane.
- [`game/bosses.md`](game/bosses.md) — boss design language and specific bosses.
- [`game/character-generations.md`](game/character-generations.md) — older
  generations of a character as skins (design; open question Q170).
- [`game/noodly-temple.md`](game/noodly-temple.md) — the level of keys and
  doors that ends at the Flying Spaghetti Monster.
- [`game/ability-loadout.md`](game/ability-loadout.md) — the inventory's
  ability variants: equipped into a slot, one variant per slot.
- [`game/robot-combat-feel.md`](game/robot-combat-feel.md) — the robot's
  blade in Ambition: Hollow Knight feel (Jon's ruling), the moves that are
  built, and how the Smash game keeps a fighter's moves.
- [`game/gnu-ton-fighter.md`](game/gnu-ton-fighter.md) — GNU-ton as a
  Smash fighter and a boss that fights back with the same move table; what
  was measured of combat feel, and what is next.
- [`game/gnu-ton-fight-debts.md`](game/gnu-ton-fight-debts.md) — what is
  not elegant in GNU-ton's fight with arms and portals, what each part
  should be, and an order for the refactor.
- [`game/multiplayer.md`](game/multiplayer.md) — Ambition multiplayer product
  intent.

### Engine: architecture

- [`engine/architecture-reassessment-and-composable-multiplayer.md`](engine/architecture-reassessment-and-composable-multiplayer.md) — cross-document review of optional rollback, peer multiplayer, simulation islands, physical objects and crate-boundary tradeoffs (proposed; not queued).
- [`engine/architecture.md`](engine/architecture.md) — the owner page: converged
  shape, the rules workspace policies cite, decomposition doctrine, target
  authorities, standing decisions and the Engine 1.0 program map.
- [`engine/actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md)
  — bounded ownership packets A1–A12 and their state.
- [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
  — carve rules and edge dispositions for the actor kernel.
- [`engine/internal-module-cycle-breaking.md`](engine/internal-module-cycle-breaking.md)
  — source-audit findings, ownership boundaries and candidate cycle cuts.
- [`engine/controlled-character-actor-kernel.md`](engine/controlled-character-actor-kernel.md)
  — target contract for the actor kernel.
- [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md)
  — capability and plugin composition.
- [`engine/system-composition-audit.md`](engine/system-composition-audit.md)
  — which systems each game runs, and the plan to run only what it uses.
- [`engine/performance-and-composition-findings.md`](engine/performance-and-composition-findings.md)
  — CPU cost on weak devices and composition: findings, ranked proposals, decisions for Jon.
- [`engine/public-sdk-1.0.md`](engine/public-sdk-1.0.md) — public SDK scope.
- [`engine/prepared-definition-field-census.md`](engine/prepared-definition-field-census.md)
  — A6 per-field reader census.
- [`engine/pickup-carve-checklist.md`](engine/pickup-carve-checklist.md) — crate
  carve checklist.
- [`engine/source-text-guard-exposure.md`](engine/source-text-guard-exposure.md)
  — source-text guards that can go blind.

### Engine: the three highest-risk seams

- [`engine/checkpoint-restoration-protocol.md`](engine/checkpoint-restoration-protocol.md)
- [`engine/projectile-contact-protocol.md`](engine/projectile-contact-protocol.md)
- [`engine/authored-technique-admission.md`](engine/authored-technique-admission.md)

### Engine: fast iteration and extensions

- [`engine/extension-model.md`](engine/extension-model.md) — the model and its
  tiers.
- [`engine/extension-state-and-execution.md`](engine/extension-state-and-execution.md)
  — state and execution contract.
- [`engine/content-generation-and-reload.md`](engine/content-generation-and-reload.md)
  — generations and development reload.
- [`engine/extension-domain-contracts.md`](engine/extension-domain-contracts.md)
  — domain call contracts.
- [`engine/fast-iteration-implementation.md`](engine/fast-iteration-implementation.md)
  — implementation packets.
- [`engine/fast-iteration-acceptance.md`](engine/fast-iteration-acceptance.md) —
  acceptance fixtures.
- [`engine/extension-iteration-evidence.md`](engine/extension-iteration-evidence.md)
  — evidence and experiments.

### Engine: simulation, construction and lifetime

- [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md)
- [`engine/simulation-tick-rate-compatibility.md`](engine/simulation-tick-rate-compatibility.md)
  — audit and plan: a simulation rate other than 60 Hz (PLANNED).
- [`engine/netcode.md`](engine/netcode.md) — netcode and the rollback host.
- [`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md)
- [`engine/immutable-content-and-transactional-construction.md`](engine/immutable-content-and-transactional-construction.md)
  — shared vocabulary.
- [`engine/instance-lifetime-provenance-and-persistence.md`](engine/instance-lifetime-provenance-and-persistence.md)
- [`engine/room-transition-loading.md`](engine/room-transition-loading.md)
- [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md)
- [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md)
- [`engine/binding-resolution-boundary.md`](engine/binding-resolution-boundary.md)
  — residual binding defects.

### Engine: actors, items and control

- [`engine/composable-actor-resources.md`](engine/composable-actor-resources.md)
- [`engine/character-authoring-package.md`](engine/character-authoring-package.md)
- [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md)
  — occurrence, custody and entitlement.
- [`engine/participant-action-system.md`](engine/participant-action-system.md)
- [`engine/control-authority-and-ai-policy.md`](engine/control-authority-and-ai-policy.md)
- [`engine/capability-progression-and-world-gating.md`](engine/capability-progression-and-world-gating.md)

### Engine: combat, movement and world

- [`engine/combat-model.md`](engine/combat-model.md)
- [`engine/expressive-move-capabilities.md`](engine/expressive-move-capabilities.md)
- [`engine/performer-up-b-the-wire.md`](engine/performer-up-b-the-wire.md) —
  built; feel is open.
- [`engine/unified-movement-kernel.md`](engine/unified-movement-kernel.md)
- [`engine/the-art-says-where-a-body-stands.md`](engine/the-art-says-where-a-body-stands.md)
  The debts of the GNU-ton giant (2026-10-11) and the design that pays
  them: a sheet says each place on its body. Not built.
- [`engine/slopes-feet-ride-surfaces.md`](engine/slopes-feet-ride-surfaces.md)
  A proposal (2026-10-11): a body on the ground keeps a support that
  lasts; its shape says how it sits on it. Not built; one decision open.
- [`engine/portals-one-transit-for-every-mover.md`](engine/portals-one-transit-for-every-mover.md)
  Nothing teleports: props and projectiles straddle and cross as bodies
  do, and a seam lab (a gauge prop, a scenario matrix) measures it.
- [`engine/portals-a-body-in-two-places.md`](engine/portals-a-body-in-two-places.md)
  A body straddling a portal is hit, hurts and collides on both sides, with
  no hacks (Jon, 2026-10-16).
- [`engine/portals-are-moving-frames.md`](engine/portals-are-moving-frames.md)
  A portal is a rigid frame that moves: its host's published motion,
  transit in its own frame, captures that are drawn where they were taken.
  Built 2026-10-15.
- [`engine/collision-and-ccd.md`](engine/collision-and-ccd.md)
- [`engine/kinematic-world-objects.md`](engine/kinematic-world-objects.md)
- [`engine/platformer-navigation-and-reachability.md`](engine/platformer-navigation-and-reachability.md)
- [`engine/world-geometry-and-spatial-semantics.md`](engine/world-geometry-and-spatial-semantics.md)
  — deferred trigger.
- [`engine/reusable-authored-world-composition.md`](engine/reusable-authored-world-composition.md)
  — incubating.
- [`engine/falling-sand.md`](engine/falling-sand.md) — removed; not a work item.
- [`engine/relativity.md`](engine/relativity.md)
- [`engine/frame-awareness.md`](engine/frame-awareness.md) — maintainer
  manifesto.
- [`engine/slower-light.md`](engine/slower-light.md) — deferred 3D direction.

### Engine: AI, perception and dialogue

- [`engine/fighter-brain.md`](engine/fighter-brain.md)
- [`engine/bounded-perception-and-attention.md`](engine/bounded-perception-and-attention.md)
- [`engine/world-facts-observations-and-memory.md`](engine/world-facts-observations-and-memory.md)
- [`engine/agentic-character-runtime.md`](engine/agentic-character-runtime.md)
- [`engine/dialogue-continuity.md`](engine/dialogue-continuity.md)
- [`engine/authored-gameplay-logic-and-orchestration.md`](engine/authored-gameplay-logic-and-orchestration.md)
- [`engine/boss-system.md`](engine/boss-system.md) — actor-local boss behavior.
- [`engine/boss-design.md`](engine/boss-design.md) — boss fight quality.
- [`engine/procedural-dialogue-mumblespeak.md`](engine/procedural-dialogue-mumblespeak.md)
  — speech-like dialogue sounds without recorded lines; deferred.

### Engine: presentation, UI and multiplayer

- [`engine/render-animation-and-vfx.md`](engine/render-animation-and-vfx.md) —
  presentation ownership and body-owned drawables.
- [`engine/sprite-renderer.md`](engine/sprite-renderer.md)
- [`engine/external-art-library-migration.md`](engine/external-art-library-migration.md)
  — shared content libraries and explicit game selection.
- [`engine/character-texel-density.md`](engine/character-texel-density.md) —
  sheet pixels per world unit; the cast rollout is on hold.
- [`engine/runtime-rigged-sprite-animation.md`](engine/runtime-rigged-sprite-animation.md)
  — rigged sprites (on in the shipped game).
- [`engine/mary-o-part-realization.md`](engine/mary-o-part-realization.md) —
  Mary-O drawn entirely from parts: gaps, validation, interpolation, phases.
- [`engine/semantic-part-rendering-and-ragdolls.md`](engine/semantic-part-rendering-and-ragdolls.md)
  — body parts drawn without a mandatory whole-body composite; 2D ragdolls.
- [`engine/svg-component-character-migration.md`](engine/svg-component-character-migration.md)
- [`engine/ui-localization-and-accessibility.md`](engine/ui-localization-and-accessibility.md)
- [`engine/shell-vanity-sequence.md`](engine/shell-vanity-sequence.md) —
  title launcher fade-in (VC5).
- [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md)
- [`engine/local-coop-integration.md`](engine/local-coop-integration.md) —
  adaptive two-player input/body/view integration, diagnostics and acceptance.

### Engine: authoring, tools, build and performance

- [`engine/authoring-and-tools.md`](engine/authoring-and-tools.md)
- [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md)
- [`engine/inspection-diagnostics-and-workbench.md`](engine/inspection-diagnostics-and-workbench.md)
- [`engine/art-inspector-and-character-coverage.md`](engine/art-inspector-and-character-coverage.md)
  — character identity, art families and animation coverage without a game build.
- [`engine/headless-verification.md`](engine/headless-verification.md)
- [`engine/prevalidated-content-artifacts.md`](engine/prevalidated-content-artifacts.md)
  — content validated ahead of time for fast admission; deferred.
- [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md)
- [`engine/runtime-frame-history.md`](engine/runtime-frame-history.md)
- [`engine/project-build-and-distribution.md`](engine/project-build-and-distribution.md)
- [`engine/godot-class-2d-capability.md`](engine/godot-class-2d-capability.md) —
  capability and expressiveness census.

## Mechanical checks

`scripts/check_planning_docs_survive.py` makes sure that the live control-plane
documents exist, are not trivial, and keep their required headings. Its module
docstring and `LIVE_CONTROL_PLANE` table are the reference; do not copy them
here.

For the full list of checks on this tree, run `./run_tests.sh --maintenance` and
read the job names. The lane prints what it does not run; `pytest scripts/tests`
is not part of it. Three classes of check are easy to forget:

- a gate sentence that names a campaign that `queue.md` marks finished;
- a number copied away from its source;
- a link to a heading. A heading rename breaks it, and only
  `scripts/check_planning_anchors_resolve.py` sees that.

## Authority order

Keep observations and decisions separate.

- **What the software does now:** source and executed behavior take precedence
  over a plan. A source inspection is not an executed acceptance test. Record a
  contradiction; do not edit the observation to fit the design.
- **What the software must do:** explicit maintainer rulings constrain design.
  The focused owner document records the current engineering decision and its
  reasons; the queue selects work under that decision. An implementation can
  violate a ruling but does not overrule it. An architectural recommendation is
  not a maintainer ruling.

Each document records which document it supersedes. A crate name, an older
campaign, an author or repeated commentary is not evidence of responsibility or
correctness.

## Direct maintainer observations

There is no permanent maintainer-observation log. When Jon reports something,
triage it immediately:

- reproducible engineering defect → `queue.md`, plus the owner plan if design is
  necessary;
- product or design choice → `awaiting-maintainer-decision.md`;
- explicit ruling → `maintainer-decisions.md`;
- durable product intent → the owner game or system document;
- fixed, superseded or not reproducible → do not keep it as live planning. Git
  history keeps the report.

## Queue contract

`queue.md` contains only executable work.

- `P0/P1/P2/P3` are priority bands, not permanent identifiers.
- A row names the current failure, the owner, the next action and the
  acceptance.
- Deep reasoning goes in the focused plan. The queue links to it.
- When a row closes, keep a short receipt only if another open row depends on
  that fact. Otherwise, remove the row in the same change.
- A finding that is only a human measurement or a maintainer choice does not
  occupy an engineering slot.
- Re-measure a row before you implement it. A queue row is a claim about a
  changing tree.
- When a row converges, rewrite it around its result. Do not append to it.

Keep the queue short enough to read in one sitting.

## Decision contract

`awaiting-maintainer-decision.md` contains only questions that current source,
existing rulings or normal engineering judgment cannot answer. Each entry has a
stable label, the exact question, enough context to choose, the owner plan, and
what changes for each different answer.

When a question is answered, move the ruling into `maintainer-decisions.md` and
delete the question. `maintainer-decisions.md` is compact: decision, date,
confidence, and only the consequence necessary to apply it.

## Focused owner documents

A healthy focused plan reads like an owner contract:

```text
scope
current authority
current topology / measurements when needed
executable work
acceptance
forbidden regressions
```

It does not contain the chronology of failed attempts. When an investigation
converges, rewrite the page around the result.

## Campaigns

A campaign document is a temporary execution scaffold. When a campaign closes:

- move durable design to the owner document;
- move open work to the queue or an inventory;
- delete the campaign file, and fix its inbound links.

## Verification receipts

A status document can carry a dated `Verified against <sha>` line when a pass
has re-read it against the tree. Keep only the most recent one. The receipt dates
the check, not the text. It does not replace running the instrument that owns a
count.

## Planning hygiene

Before you add prose, find out if the fact already has an owner. Prefer:

- one semantic helper to repeated identifier spellings;
- one current measurement and its command to a timeline of old numbers;
- one explicit prohibition to the story of each failed design;
- Git history to an archive section in a live page.
