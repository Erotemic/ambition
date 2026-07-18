---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/mechanics/abilities.md
  - docs/systems/input-control-and-ui.md
---

# Projectiles and motion inputs

Projectile simulation and motion-command recognition are separate reusable
primitives. A provider may combine them into a named move, but projectile physics
must not parse raw device history and input recognition must not own combat.

## Flow

```text
semantic directional/action history
    -> deterministic command recognizer
    -> action identity/request
    -> action scheme and capability gates
    -> projectile spawn request
    -> projectile domain owns body, lifetime, contacts, and hit facts
    -> presentation consumes read models/effects
```

## Motion-command contract

A recognizer consumes normalized semantic directions in actor/control space,
not keyboard keys or screen coordinates. It should define:

- sampling and quantization;
- bounded history/time window;
- neutral and direction-change treatment;
- mirrored/facing-relative commands;
- precedence when one command is a subsequence of another;
- deterministic tie-breaking and consumption.

The recognizer returns an action identity or evidence. It does not directly
spawn a fireball, spend meter, or choose animation.

## Projectile contract

A projectile is an ECS simulation entity or equivalent authoritative body with:

- stable source/owner/faction identity;
- movement and lifetime policy;
- collision/query shape;
- hit payload and rejection/filter policy;
- surface/portal/field interactions;
- deterministic despawn reason;
- provider-resolved presentation identity.

Damage, knockback, status, and hit acceptance flow through the canonical combat
seam. Visual sprites and trails are not projectile truth.

## Invariants

- Humans, brains, scripts, and replay can request the same projectile action.
- Command recognition is testable as a pure bounded transformation.
- Spawn/resource/cooldown decisions occur once.
- Projectile contact uses shared world semantics rather than a private tile map.
- Entity identity used for persistence/replay is not a raw Bevy allocator handle.
- Headless simulation reaches the same authoritative outcome without assets.

## Validation

```bash
python scripts/agent_query.py "motion input projectile spawn hit"
python scripts/agent_query.py tests "projectile portal collision"
./run_tests.sh -k projectile
```
