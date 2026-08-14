# Smash body-generic combat — successor plan

**State:** OPEN acceptance/product program after the D73 character-authority campaign.

The full 2026-08-09 through 2026-08-13 migration diary is archived at
[`../archive/planning-superseded/2026-08-13/smash-body-generic-combat-2026-08-09-pre-successor.md`](../archive/planning-superseded/2026-08-13/smash-body-generic-combat-2026-08-09-pre-successor.md).

Super Smash Siblings is a serious engine customer and may eventually graduate
from an acceptance game into a first-class game. **Ambition remains the flagship
and primary product driver.** Smash earns engine work by exposing reusable body,
combat, participant, presentation and authoring gaps; it does not earn private
engine paths.

## What is already closed

Do not schedule the old campaign again. Current source already has:

- one body victim vocabulary and one relationship policy;
- body-generic damage, knockback, hitstun, hitlag state and DI;
- charge/hold-release attack authoring;
- landing lag/autocancel and body-generic hard-lock consumption;
- aerial locomotion and jump-squat authoring;
- pose-aware hurt geometry and hitbox tracks;
- shield/parry through the shared victim-resolution path;
- fighter-specific movement identities on the shared movement kernel; and
- the D73 character-definition/construction convergence.

D107 and D108 were closed by the D73 authority-convergence work rather than by
adding parallel fighter exceptions. D114 is no longer an architecture defect:
its remaining question is product feel for hit emphasis when no primary local
seat should own a global camera/time beat; that decision lives in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

## Remaining reusable engine work

### S1 — body-generic grab / hold / throw

The current combat model still has no general platform-fighter grab/hold/throw
capability. Design it through ordinary bodies and control authority:

- a temporary hold relationship names holder and held body;
- holding constrains movement/control without changing body identity;
- release/throw enters the ordinary launch/damage/DI path;
- possession, AI and human control all observe the same held-body semantics;
- rollback snapshots the relationship through the owning domain.

Do not create a Smash-only grabbed-fighter ontology.

### S2 — decide whether richer shield semantics earn implementation

The shared shield/parry model exists. Shield HP, regeneration, shield stun,
break stun and block-specific cancel facts do not need to exist merely because
an old campaign sketch named them. Add them only when actual Smash feel needs
them, and extend the one body shield/damage authority.

### S3 — finish body-scale equipment only if it remains desired

The equipment parameter vocabulary can resolve `BODY_SCALE`, but body size,
collision and presentation do not yet universally consume that fold as one
authoritative size. If the product wants equipment-driven body scaling, make one
resolved size authority feed simulation and presentation rather than adding a
Smash-specific patch.

### S4 — fighter-brain evaluation, not another brain stack

The fighter brain implementation exists. Finish the production scenario runner,
survival/damage measurements and difficulty-ladder calibration in
[`engine/fighter-brain.md`](engine/fighter-brain.md).

### S5 — local-N and match presentation

Use the shared participant/action architecture and the new
[`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md)
program. A Smash match should prove multiple local participants/controllers and
HUD/view composition without teaching the engine that a fighter is a special
kind of controlled body.

Arena play will usually prefer one shared view, but the engine must not encode
that as a global one-camera limitation.

### S6 — stage authoring as an engine customer

Keep stages in LDtk. The planned moving-platform stage should consume the same
kinematic-world-object and LDtk authoring capability that Ambition uses; it is a
useful second consumer, not the reason the capability exists. See
[`engine/kinematic-world-objects.md`](engine/kinematic-world-objects.md).

## Product work owned by Smash

The game/provider owns match rules, stocks, timer/sudden-death policy, roster,
stage choice, character-select UX, CPU-fill policy, percent presentation and
results/victory flow. Reusable mechanics discovered while implementing those
belong in the engine only when the abstraction is genuinely game-independent.

The current product acceptance specification lives in
[`demos/super-smash-siblings.md`](demos/super-smash-siblings.md).

## Exit shape

Smash is a successful engine customer when:

- four ordinary bodies with materially different authored movement/combat kits
  can complete deterministic matches through the shared engine;
- multiple local participants can join and control independent fighters through
  the participant/action seam;
- CPU fighters use the same body reactions and combat laws as human-controlled
  fighters;
- stage geometry, including a moving platform, is authored through supported
  world tooling;
- remaining platform-fighter mechanics are reusable body capabilities rather
  than mode-specific engine branches; and
- the same characters remain ordinary usable Ambition characters outside the
  match ruleset.
