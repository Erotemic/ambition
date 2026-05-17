---
id: movement-collision
aliases:
  - player sweep
  - kinematic body
  - wall cling teleport
  - ledge snap
  - collision correction
implemented_by:
  - crates/ambition_engine/src/movement.rs
  - crates/ambition_engine/src/kinematic.rs
  - crates/ambition_engine/src/ledge_grab.rs
  - crates/ambition_sandbox/src/player.rs
tested_by:
  - crates/ambition_engine/tests/wall_cling_fuzz.rs
  - crates/ambition_engine/tests/wall_jump_fuzz.rs
  - crates/ambition_sandbox/tests/repro_walls.rs
related_docs:
  - docs/current/risks.md
  - docs/gameplay_trace_recorder.md
related_memory:
  - dev/journals/movement-edge-touch-y-sweep-lessons-2026-05-11.md
  - dev/benchmark-candidates/movement-edge-touch-y-sweep-question-2026-05-11.md
last_verified: 2026-05-17
---

# Movement collision

## Definition

Movement collision is the custom kinematic-controller path for player-like bodies. It owns movement feel, wall/ground semantics, body-shape transitions, collision corrections, and OOB trace diagnostics.

## Core invariants

- Movement feel is the priority; raw collision/debug boxes must remain fun before visuals are final.
- Existing side contact is not a vertical landing. Edge-touching a wall during a y-sweep must not snap the body to the wall top.
- Mid-action mechanics such as dash, blink, wall-cling, ledge hang, swim, and morph/crouch each own their posture while active.
- `PlayerMovementAuthority { player }` is the authoritative player state. Do not reintroduce a god-object runtime shadow copy.
- Collision fixes should include either regression tests, trace coverage, or debug visualization.

## Edit protocol

1. Search dev memory for the symptom/failure class.
2. Identify whether the change belongs in engine primitives or sandbox adapters.
3. Preserve axis ownership and posture ownership when refactoring.
4. Add or update focused tests before broad gameplay changes.
5. For surprising geometry logic, add an `AMBITION_REVIEW:` comment.

## Validation

```bash
cargo test -p ambition_engine wall
cargo test -p ambition_sandbox --test repro_walls
cargo test -p ambition_sandbox --test fuzz_random_walker
cargo run -p ambition_sandbox --bin headless
```

Use the narrowest relevant subset first; run the broader smoke path before handoff if movement semantics changed.
