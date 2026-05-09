# Rust benchmark candidates

These are small, concrete Rust maintenance questions distilled from Ambition
refactor mistakes. Each item should be answerable without knowing the eventual
compiler error, but should contain enough surrounding context that a model has
to preserve the intended API rather than only patch the local symptom.

## 2026-05-09: Preserve facade re-exports when splitting a large Rust module

### Setup

You are refactoring a Rust crate that currently has a large public module file:

```text
crates/ambition_engine/src/movement.rs
```

The crate root exposes the movement API through a parent-level re-export:

```rust
pub mod movement;

pub use movement::{
    blink_destination, blink_destination_to_point, update_player, update_player_control,
    update_player_control_with_tuning, update_player_simulation,
    update_player_simulation_with_tuning, update_player_with_tuning, BlinkEvent, ComboMark,
    FrameEvents, InputState, MovementOp, MovementTuning, Player, AIR_ACCEL, AIR_FRICTION,
    AIR_JUMPS, BLINK_COOLDOWN, BLINK_DISTANCE, BLINK_HOLD_THRESHOLD, COYOTE_TIME, DASH_BUFFER,
    DASH_COOLDOWN, DASH_SPEED, DASH_TIME, DEFAULT_TUNING, DOUBLE_JUMP_SPEED, FAST_FALL_ACCEL,
    FAST_FALL_SPEED, FLIGHT_ACCEL, FLIGHT_DRAG, FLIGHT_HOVER_HZ, FLIGHT_HOVER_SPEED,
    FLIGHT_TERMINAL_SPEED, GRAVITY, GROUND_FRICTION, JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED,
    MAX_RUN_SPEED, POGO_SPEED, PRECISION_BLINK_AIM_SPEED, PRECISION_BLINK_DISTANCE, RUN_ACCEL,
    SLASH_RECOIL, WALL_CLIMB_SPEED, WALL_JUMP_X, WALL_SLIDE_SPEED,
};
```

You split the file into private children like:

```text
crates/ambition_engine/src/movement.rs
crates/ambition_engine/src/movement/events.rs
crates/ambition_engine/src/movement/input.rs
crates/ambition_engine/src/movement/ops.rs
crates/ambition_engine/src/movement/player.rs
crates/ambition_engine/src/movement/tuning.rs
```

The constants are moved to `movement/tuning.rs`, including:

```rust
pub const COYOTE_TIME: f32 = 0.120;
```

### Question

When doing this split, what should you do in the new `movement.rs` facade so the
crate's public API remains stable, and what mechanical check should you run
before handing off the refactor?

### Expected answer

The facade must re-export every item that callers could previously access via
`ambition_engine::movement::*` and every item the crate root re-exports through
`pub use movement::{...}`. In particular, after moving constants into
`movement/tuning.rs`, include `COYOTE_TIME` in the facade's tuning re-export:

```rust
mod tuning;

pub use tuning::{
    MovementTuning, AIR_ACCEL, AIR_FRICTION, AIR_JUMPS, BLINK_COOLDOWN,
    BLINK_DISTANCE, BLINK_HOLD_THRESHOLD, COYOTE_TIME, DASH_BUFFER,
    DASH_COOLDOWN, DASH_SPEED, DASH_TIME, DEFAULT_TUNING, DOUBLE_JUMP_SPEED,
    FAST_FALL_ACCEL, FAST_FALL_SPEED, FLIGHT_ACCEL, FLIGHT_DRAG,
    FLIGHT_HOVER_HZ, FLIGHT_HOVER_SPEED, FLIGHT_TERMINAL_SPEED, GRAVITY,
    GROUND_FRICTION, JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED, MAX_RUN_SPEED,
    POGO_SPEED, PRECISION_BLINK_AIM_SPEED, PRECISION_BLINK_DISTANCE,
    RUN_ACCEL, SLASH_RECOIL, WALL_CLIMB_SPEED, WALL_JUMP_X, WALL_SLIDE_SPEED,
};
```

Before handoff, compare the parent `pub use movement::{...}` list against the
facade exports and run at least:

```bash
cargo check -p ambition_engine
```

A useful automation is a small surface-audit script that parses the crate-root
`pub use movement::{...}` list and verifies each name is either defined in the
facade or publicly re-exported by it.

### Why this was easy to miss

The split can compile mentally because `COYOTE_TIME` still exists and is `pub` in
`movement/tuning.rs`. But `tuning` is a private child module, so a `pub const` in
that child is not visible as `movement::COYOTE_TIME` unless the facade explicitly
re-exports it. The local refactor preserved the definition but accidentally
changed the module's public surface.
