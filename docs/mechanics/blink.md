---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/systems/blink-and-fastfall.md
  - docs/systems/blink-motion-policy.md
  - docs/concepts/movement-collision.md
---

# Blink and discontinuous movement

Blink is the reference discontinuous-movement mechanic. Its durable lesson is
not a particular distance or cooldown; it is the separation of intent, path
policy, safe placement, and presentation.

## Contract

A blink request supplies actor-local intent. Simulation resolves that intent in
the body's gravity/orientation frame, applies the active blink policy, and either
commits one valid destination or rejects the request with no partial movement.

```text
action request
    -> actor-local direction and requested extent
    -> path/surface policy
    -> deterministic safe-destination search
    -> atomic body relocation
    -> semantic outcome/effects
```

## Invariants

- The source body is one ordinary actor body, not a player-only teleport type.
- Collision and authored blocker semantics decide validity.
- The body never finishes embedded in blocking geometry.
- Failed placement leaves authoritative position unchanged.
- Search order and tie-breaking are deterministic.
- Gravity rotation transforms intent and geometry consistently.
- Camera, trails, particles, and SFX consume the committed outcome; they do not
  determine it.
- Precision aiming may use `InputState::control_dt`; simulation still advances
  through the normal body tick.

## Policy versus mechanism

The reusable mechanism should answer:

- what segment/shape is tested;
- what surfaces block, shorten, or permit transit;
- how candidate destinations are generated;
- how a valid body placement is proven;
- what outcome is reported.

Provider or capability data may choose range, resource cost, recovery, whether a
surface is blink-soft/hard, and visual/audio treatment.

## Validation

Prefer properties over tuned coordinates:

- C4/gravity covariance;
- no penetration after success;
- no state change after rejection;
- monotonic shortening when a blocker approaches;
- equivalent results under replay and headless stepping;
- portals and moving geometry use the same entity/world semantics as other bodies.

Use the current system pages and generated index for concrete symbols:

```bash
python scripts/agent_query.py "blink safe destination collision"
python scripts/agent_query.py tests "blink penetration gravity"
```
