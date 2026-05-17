# Two-clock simulation model

Ambition uses two different notions of time in the playable sandbox:

1. **Control time**: real, unscaled frame time. This drives input gestures such
   as button holds, blink aim cursor movement, jump-release clipping, toggles,
   and other actions that must stay responsive during bullet-time.
2. **Simulation time**: scaled game time. This drives gravity, velocity
   integration, cooldown decay, coyote timers, enemies, moving platforms,
   particles, and other world evolution.

The reason for this split is precision blink. During bullet-time, the world may
be nearly frozen, but the player still needs fine-grained control over the blink
destination. If the blink cursor used simulation time, aiming would become
unusable. If player physics used control time, the player would keep falling at
normal speed while the rest of the room froze.

The Bevy sandbox therefore does this each frame:

```text
read input using real time
process player control with control_dt
compute the desired time scale from the updated player state
advance the moving platform using scaled dt
advance player physics using scaled dt
advance enemies and effects using scaled dt
```

The engine exposes this through separate functions:

```rust
update_player_control(..., control_dt)
update_player_simulation(..., scaled_dt)
```

The older `update_player(...)` wrapper remains for tests and simple callers that
do not need bullet-time.

## Invariant

Anything that represents **intent** should use control time. Anything that
represents **world evolution** should use simulation time.
