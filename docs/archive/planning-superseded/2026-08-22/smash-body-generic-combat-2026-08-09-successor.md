# Smash body-generic combat — successor plan

**State:** OPEN acceptance/product program after the D73 character-authority campaign.

The full 2026-08-09 through 2026-08-13 migration diary is archived at
[`../2026-08-13/smash-body-generic-combat-2026-08-09-pre-successor.md`](../2026-08-13/smash-body-generic-combat-2026-08-09-pre-successor.md).

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
adding parallel fighter exceptions. **D114 is closed outright as of 2026-08-17**:
the actor road now freezes its own integration during that body's hitlag, and the
maintainer ruled that *"hitlag is a combat/body semantic, not something that
should depend on whether a body happens to occupy the primary local-control
road"* — so the feel question this line used to defer was answered by removing the
per-road distinction, not by choosing a beat.
⛔ a future stickiness complaint is answered by hitlag's duration or shape;
restoring a controlled-body/actor asymmetry is forbidden. See
[`maintainer-decisions.md`](../../../planning/maintainer-decisions.md).

## What the expressive-fighter pass measured (2026-08-15)

A pass asking *"can this engine produce a genuinely exciting fighter through
ordinary reusable architecture"* took `smash_george_booul` from eleven moves to
sixteen. **Almost all of the gap was CONTENT UNDERUSE, and that is the finding.**

Already sufficient, adopted by nobody: the `special` / `special_forward` /
`special_up` / `special_down` verb chain (no fighter in the demo had ever bound
a special at all), `WindowTag::Cancelable` with `CancelCondition::OnHit`,
per-volume `on_hit` techniques (`pogo_bounce`), `MoveWindow::motion_scale` as a
committed tail, multiple `Active` windows on one timeline, `MoveEventKind::Sfx`
/ `Vfx` for per-move feedback, and `landing_lag_s` / `autocancel_after_s` /
`smash_charge_mult`. None of it needed engine work; it needed authoring.

⭐ **genuinely missing, and only this:** a move could not displace its owner at a
chosen MOMENT, and could not COMMAND a speed rather than adding to one.
`MoveSpec::start_impulse` fires at the press and is additive, so a recovery
special was strongest when least needed — `vel += -1000` while falling at 900
climbs at 100. `MoveEventKind::Impulse { local, mode: Add | Set }` closes both
halves, and `MoveFrameData::lift_speed` / `lift_at_s` make the result READABLE,
which is what lets a policy layer recognise a recovery by its geometry rather
than by a table of whose special is which.

⚠ **still missing, named rather than half-built:** an airborne move with a
self-impulse has **no per-airtime budget**. George's Up-B is bounded by
arithmetic instead (no cancel window, and the move outlasts its own arc, so
repeated use loses height) — deliberately, because a use counter is rollback
state and the schema was not this pass's to re-baseline. A body wanting a
genuinely once-per-airtime action still cannot say so.

⛔ **and `WindowTag::Invuln` / `Armor` remain declared vocabulary with no
consumer.** Authoring them today parses and does nothing. That is an authoring
trap of exactly the kind the `OnBlock` variant was deliberately not created to
avoid; either implement them or delete them.

## Remaining reusable engine work

### S1 — body-generic grab / hold / throw — ✔✔ **BUILT (2026-08-18)**

⛔ **this section opened "the current combat model still has no general
platform-fighter grab/hold/throw capability" until 2026-08-18, by which point
every bullet under it was satisfied.** Checked one at a time rather than from
memory:

```text
a temporary hold names holder and held       ambition_combat::capture::CapturedBy
constrains control, keeps body identity      body constraint + ControlHold::Relationship
release/throw enters the ordinary path       CaptureThrowRequested -> the same
                                             scaled-knockback road every authored
                                             launcher uses
possession / AI / human see the same         the HUMAN leg was the last one, and it
  held-body semantics                        was broken until `988807b99` — the
                                             player brain never carried
                                             `grab_pressed`
rollback snapshots the relationship          `capture.captured_by` (clone) +
  through the owning domain                  `map.captured_by` (entity remap),
                                             registered by ambition_combat
no Smash-only grabbed-fighter ontology       capture lives in `ambition_combat`,
                                             not in the smash demo
```

⭐ **it follows the mount pattern (ADR 0020) as a SIBLING, not a merge**:
`ControlHold` is the claim registry, and a captive is a body whose control is
suppressed rather than redirected, the same "relationship that redirects or
suppresses `ActorControl` plus a per-tick pose lock" mounts use — a rider
steers its mount (consented, class-matched) where a holder suppresses its
captive (imposed, exits via throw into the launch/damage path, not a dismount).
⛔ do not merge the two into one relationship, and do not give a held fighter
its own ontology when a `ControlGrant` of "none" is what being held means.

▢ **what is genuinely left is the FEEL, which this section already scoped out as
product work**: grab range, break-out and hold duration exist and are authored;
what a hold is WORTH to a CPU's decision is open and costed in queue D166.

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
[`engine/fighter-brain.md`](../../../planning/engine/fighter-brain.md).

### S5 — local-N and match presentation

Use the shared participant/action architecture and the new
[`engine/multiplayer-and-multiview.md`](../../../planning/engine/multiplayer-and-multiview.md)
program. A Smash match should prove multiple local participants/controllers and
HUD/view composition without teaching the engine that a fighter is a special
kind of controlled body.

Arena play will usually prefer one shared view, but the engine must not encode
that as a global one-camera limitation.

### S6 — stage authoring as an engine customer

Keep stages in LDtk. The planned moving-platform stage should consume the same
kinematic-world-object and LDtk authoring capability that Ambition uses; it is a
useful second consumer, not the reason the capability exists. See
[`engine/kinematic-world-objects.md`](../../../planning/engine/kinematic-world-objects.md).

## Product work owned by Smash

The game/provider owns match rules, stocks, timer/sudden-death policy, roster,
stage choice, character-select UX, CPU-fill policy, percent presentation and
results/victory flow. Reusable mechanics discovered while implementing those
belong in the engine only when the abstraction is genuinely game-independent.

The current product acceptance specification lives in
[`demos/super-smash-siblings.md`](../../../planning/demos/super-smash-siblings.md).

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
