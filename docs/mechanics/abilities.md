---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/concepts/engine-mental-model.md
  - docs/systems/actors-brains-and-character-content.md
  - docs/adr/0025-character-actions-input-ownership.md
---

# Ability composition

An ability is reusable simulation capability attached to an actor body. A named
character may select, tune, unlock, or visually present an ability, but must not
own a private execution pipeline for it.

## Durable flow

```text
controller intent
    -> semantic action slot
    -> ActorActionScheme
    -> shared slot resolver
    -> ability request / MovePlayback / interaction
    -> body, combat, projectile, or world domain mutates simulation
    -> read model and semantic effects
    -> provider presentation
```

The action scheme answers *what this body's slot currently means*. The ability
implementation answers *whether and how that action executes*. Input devices,
brains, menus, and HUD prompts do not duplicate ability policy.

## What belongs where

- **Provider content:** named move sets, character defaults, unlocks, tuning,
  animation/audio bindings, and encounter restrictions.
- **Character/action domain:** action slots, action schemes, brain/action-set
  requests, gating, and action identity.
- **Movement/combat/projectile/world domains:** authoritative mechanics and
  outcomes.
- **Presentation:** animation, sprites, trails, particles, sound, camera feedback,
  and prompt glyphs.

## One body, one path

Before implementing a move, search for the equivalent behavior on another body
or controller. If it exists, extract or route through the shared seam and delete
the duplicate. Do not create `PlayerX` and `EnemyX` implementations that merely
produce similar output.

Separate orchestrating systems are acceptable when scheduling differs. Separate
authoritative mechanic implementations are not.

## Ability lifecycle

A robust ability can represent:

- request and rejection reason;
- windup, active, recovery, and cancellation windows;
- movement locks or overrides;
- costs, cooldowns, charges, and resource refunds;
- armor, invulnerability, hurtbox, and body-mode changes;
- spawned simulation entities or semantic effects;
- completion/interruption facts for brains, replay, and UI.

Not every ability needs every phase. The vocabulary should compose rather than
forcing each move into a bespoke state machine.

## Validation questions

- Can a human and a brain request the same ability?
- Does the UI prompt use the same resolution result as gameplay?
- Does the authoritative effect exist without animation or audio?
- Is there exactly one emission/execution site for equivalent behavior?
- Does a rotated gravity frame preserve the intended geometry?
- Can the ability be interrupted, reset, restored, and replayed deterministically?
- Can another provider supply different tuning and presentation?

Find current implementation evidence with:

```bash
python scripts/agent_query.py "action scheme ability MovePlayback"
python scripts/agent_query.py tests "action resolution ability"
```
