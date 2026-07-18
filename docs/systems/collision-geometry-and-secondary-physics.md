---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/concepts/movement-collision.md
  - docs/mechanics/body-modes.md
---

# Collision geometry and secondary physics

Primary platformer body movement is deterministic engine simulation. Secondary
physics may animate debris, props, or other non-authoritative presentation/toy
objects, but must not become a second authority for actor outcomes.

## Shared geometry contract

World/lowering produces typed collision and query geometry from provider-authored
world records. Movement, blink, projectiles, portals, hazards, interactions, and
spawn validation consume the same semantic surface vocabulary.

Required properties:

- explicit body/query shapes and stable coordinate frames;
- gravity-relative normals/tangents and contact classification;
- deterministic ordering/tie-breaking for outcome-affecting queries;
- semantic surface kinds rather than room/character names;
- lifecycle-scoped moving/dynamic geometry;
- safe reconstruction across load/reset/snapshot.

## Primary versus secondary physics

Primary simulation owns actor transforms, contacts, hit/hurt geometry, moving
platform attachment, and any object that can change gameplay outcomes.

A secondary physics backend may own visual debris or isolated toy-room state when
its output cannot alter authoritative movement/combat/progression. If gameplay
begins reading it, promote the required state/queries into the canonical
simulation contract rather than coupling core logic to presentation physics.

## Invariants

- One collision representation feeds all authoritative mechanics.
- Body shape transitions are validated before commit.
- Moving-platform/reference-frame behavior is explicit and snapshot-safe.
- Render bounds and sprite anchors never substitute for collision geometry.
- Headless and visible compositions resolve the same contacts.

## Validation

```bash
python scripts/agent_query.py "collision geometry moving platform secondary physics"
python scripts/agent_query.py tests "collision gravity wall cling"
./run_tests.sh -k collision
./run_tests.sh -k moving_platform
```
