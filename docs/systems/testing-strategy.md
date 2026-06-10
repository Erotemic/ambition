# Testing Strategy

Ambition should be testable at several layers.

## 1. Pure engine unit tests

`crates/ambition_engine_core/src/` should contain tests for small deterministic mechanics facts:

- ability dependency warnings
- jump/double-jump gates
- dash/double-dash charge counts
- attack hitbox orientation
- collision primitives
- room generation invariants

These tests should not depend on Bevy, audio, GPU, or a window.

## 2. Headless simulation tests

The engine should grow a helper that can step a world for a fixed input script:

```rust
let trace = run_script(world, player, [
    hold_right(20),
    press_jump(),
    wait(10),
    press_dash(),
]);
```

Then tests can assert that the player reached a region, consumed a charge,
recorded an operation, or avoided a hazard. This is the foundation for
AI-generated-room validation.

## 3. Golden movement traces

For important feel regressions, store small expected traces:

- position/velocity every N frames
- operation sequence
- final resource state

Do not overuse golden traces because tuning changes will invalidate them often.
Use them for invariants, not for every exact number.

## 4. Bevy adapter smoke tests

The Bevy sandbox should stay thin. Most behavior belongs in engine tests. The
Bevy layer can later have smoke tests for resource setup and scene spawning, but
those are secondary until the engine API stabilizes.

## Current test seeds

The first pass adds tests for:

- `AbilitySet::sandbox_all()` compatibility
- dependency warnings
- double jump gating
- double dash charge counts
- attack hitbox orientation

See also: `docs/mechanics/abilities.md` and `docs/mechanics/expressibility-checklist.md`.
