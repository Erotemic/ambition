---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/systems/ldtk-world-composition.md
  - docs/systems/portals.md
---

# Transition spawn validation

Room/session transitions must prove a safe arrival before replacing the current
world. Arrival validation is a world/construction contract, not a late player
repair system.

## Flow

```text
loading-zone or transition request
    -> resolve stable destination room/zone/anchor
    -> build/validate candidate construction plan
    -> lower destination geometry and spawn records
    -> compute body-specific safe arrival
    -> commit new scope atomically
    -> publish transition/read-model effects
```

The old session/room remains authoritative until the replacement can commit.
Failure reports a deterministic diagnostic and leaves the current world intact.

## Validation inputs

Safe arrival may depend on:

- destination zone/anchor and authored facing/orientation;
- actor body shape/mode and gravity frame;
- static and dynamic collision planned for the new scope;
- one-way/hazard/portal semantics;
- provider policy for fallback candidate generation.

Do not validate against renderer bounds, a partially spawned destination, or a
hard-coded player capsule.

## Invariants

- Every transition target resolves by stable ID.
- Missing/ambiguous destinations fail during provider/world validation where
  possible.
- Candidate ordering and final placement are deterministic.
- Arrival is valid for the actual transitioning body.
- Commit/cleanup is atomic and lifecycle-scoped.
- Symbolic target-zone arrivals and direct anchors use one validation mechanism.
- Headless tests cover failure as well as success.

## Validation

```bash
python scripts/agent_query.py "transition spawn validation arrival"
python scripts/agent_query.py tests "loading zone safe arrival"
./run_tests.sh -k transition
./run_tests.sh -k spawn_validation
./run_tests.sh -k room_spatial_integrity
```
