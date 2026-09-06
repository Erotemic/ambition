---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/concepts/movement-collision.md
  - docs/systems/transition-spawn-validation.md
---

# Portals

Portals are reusable spatial transforms over ordinary simulation entities. They
must preserve the one-body model: actors, projectiles, and other eligible bodies
transit through the same portal vocabulary rather than separate player and enemy
implementations.

## Contract

A portal pair/connection is provider-authored world data lowered to typed runtime
records. Transit resolves:

- source/destination identity;
- eligibility and re-entry/cooldown policy;
- position and orientation transform;
- velocity/gravity/frame transform;
- safe destination placement;
- lifecycle/trace/effect outcome.

The result commits atomically. Presentation consumes the transit fact for VFX,
SFX, camera treatment, and interpolation.

## Invariants

- Portal IDs are stable provider/world IDs; Bevy entities are runtime handles.
- Transit uses shared body/projectile/world geometry semantics.
- Transform composition is deterministic and gravity-aware.
- Destination placement cannot embed the body in blocking geometry.
- Re-entry prevention is scoped per transit/body, not global accidental state.
- Headless and visible compositions produce the same authoritative result.
- Reset/room replacement/snapshot restore cannot retain stale connections or
  cooldown markers.

## Validation

Prefer transformation properties:

- through-portal covariance;
- round-trip behavior where topology permits;
- velocity/orientation mapping;
- actor/projectile controller symmetry;
- rejection leaves state unchanged;
- provider validation catches missing/duplicate destinations.

```bash
python scripts/agent_query.py "portal transit transform"
python scripts/agent_query.py tests "portal projectile gravity"
./run_tests.sh -k portal
```
