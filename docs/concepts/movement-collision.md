---
id: movement-collision
status: current
aliases:
  - wall cling
  - y-sweep
  - ledge grab
  - body modes
  - slash pogo
  - shield parry
  - projectile collision
implemented_by:
  - crates/ambition_engine/src/movement.rs
  - crates/ambition_engine/src/kinematic.rs
  - crates/ambition_engine/src/combat.rs
  - crates/ambition_engine/src/projectile.rs
  - crates/ambition_engine/src/player_state.rs
  - crates/ambition_engine/src/ledge_grab.rs
related_docs:
  - docs/mechanics/expressibility-checklist.md
  - docs/mechanics/body_modes.md
  - docs/mechanics/projectiles_and_motion_inputs.md
related_memory:
  - dev/journals/movement-edge-touch-y-sweep-lessons-2026-05-11.md
last_verified: 2026-05-17
---

# Movement collision

## Definition

Movement collision covers kinematic player motion, swept collision, body modes, ledge behavior, blink safety, melee/pogo hitboxes, projectile collision, shield/parry state, and trace-backed regression tests.

## Core invariants

- Edge-touch contacts must not become far-block vertical teleports.
- One-way platforms block only appropriate downward contacts.
- Body-mode resize must fail safely when the target shape does not fit.
- Slash/pogo/projectile hitboxes should be computed from explicit intents/specs, not presentation sprites.
- Shield/parry state is simulation state; the bubble sprite is presentation.

## Edit protocol

1. Search `dev/` for matching movement/collision traps.
2. Read the focused engine module and tests.
3. Add or update a regression test for the geometry case.
4. Keep presentation effects in sandbox messages/systems.

## Validation

```bash
cargo test -p ambition_engine movement::
cargo test -p ambition_engine combat::
cargo test -p ambition_engine projectile::
cargo test -p ambition_engine --test wall_cling_fuzz
cargo test -p ambition_engine --test body_shape_fits_at
```
