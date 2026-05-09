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

## 2026-05-09: Move Rust attributes and doc comments with extracted items

Tags: `rust-module-refactor`, `rust-attributes`, `rustdoc`, `serde`, `bevy-resource`

### Setup

You are splitting a large Bevy/Rust file:

```text
crates/ambition_sandbox/src/trace.rs
```

into a facade plus private children:

```text
crates/ambition_sandbox/src/trace.rs
crates/ambition_sandbox/src/trace/model.rs
crates/ambition_sandbox/src/trace/buffer.rs
crates/ambition_sandbox/src/trace/detect.rs
crates/ambition_sandbox/src/trace/dump.rs
crates/ambition_sandbox/src/trace/systems.rs
crates/ambition_sandbox/src/trace/tests.rs
```

The original file contains adjacent item decorations like these:

```rust
/// Lightweight 2D point used in the serialized payload.
#[derive(Serialize, Clone, Copy, Debug, Default)]
pub struct TracePoint { /* ... */ }

/// If the per-frame position delta exceeds the maximum movement we'd
/// expect from the player's velocity (plus a small slack), the recorder
/// treats it as a teleport / collision correction.
const TELEPORT_DETECTION_SLACK_PX: f32 = 16.0;

/// Top-level rolling buffer.
#[derive(Resource, Debug)]
pub struct GameplayTraceBuffer { /* ... */ }

/// Inspect the current player state against the active world and produce
/// the first OOB reason found, if any.
pub fn detect_oob(/* ... */) -> Option<OobReason> { /* ... */ }

#[derive(Serialize, Debug)]
struct DumpPayload<'a> { /* ... */ }
```

You want `model.rs` to own serialized trace data types,
`buffer.rs` to own `GameplayTraceBuffer`, `detect.rs` to own OOB/diff detection,
and `dump.rs` to own JSON/Markdown dump serialization.

### Question

When extracting these items, what must move with each item, and what pre-handoff
static checks would catch mistakes before a teammate runs `cargo fmt` or
`cargo test`?

### Expected answer

Attributes and Rustdoc comments are item-adjacent semantics, not free-floating
notes. Move them with the item they describe:

- `TracePoint` keeps both its doc comment and
  `#[derive(Serialize, Clone, Copy, Debug, Default)]` in `model.rs`.
- `GameplayTraceBuffer` keeps `#[derive(Resource, Debug)]` in `buffer.rs`.
- The `detect_oob` doc comment moves to `detect.rs` immediately above the
  `detect_oob` function.
- The teleport-detection doc comment moves with
  `TELEPORT_DETECTION_SLACK_PX`, or is rewritten as an ordinary `//` comment
  near the code using the constant.
- `DumpPayload<'a>` keeps `#[derive(Serialize, Debug)]` immediately above the
  struct in `dump.rs`.
- Add any imports needed by derives in the destination module, for example
  `use serde::Serialize;` where relying on the facade's glob import would be
  fragile.

Before handoff, run or simulate checks that catch orphaned decorations:

```bash
cargo fmt --all
cargo test -p ambition_sandbox --lib
```

If Rust tooling is unavailable, perform a textual audit:

- no file ends with a `///` doc comment;
- every `#[derive(...)]` is followed by a `struct`, `enum`, or `union`, not a
  doc comment or function;
- every extracted `#[derive(Resource)]` remains on the Bevy resource type;
- every extracted `#[derive(Serialize)]` remains on the type being serialized;
- comments describing functions/constants are in the same destination module as
  the function/constant.

### Why this was easy to miss

A mechanical split by line ranges can leave the item below an attribute behind
or move the item without its decoration. The resulting code still looks locally
plausible because the comment text describes the right concept, but Rust treats
`///` and `#[derive]` as attributes on the *next item*. If the next item is gone,
formatting/parsing fails; if a derive is lost, later trait bounds such as Bevy's
`Resource` or Serde's `Serialize` fail in less obvious places.

## 2026-05-09: Preserve sibling-module helper visibility during facade splits

Tags: `rust-module-refactor`, `rust-visibility`, `procedural-audio`

### Setup

You split a large audio module:

```text
crates/ambition_sandbox/src/audio.rs
```

into:

```text
crates/ambition_sandbox/src/audio.rs          # facade
crates/ambition_sandbox/src/audio/render.rs   # waveform/music rendering helpers
crates/ambition_sandbox/src/audio/runtime.rs  # runtime handles, Kira channels, radio state
crates/ambition_sandbox/src/audio/tests.rs
```

Before the split, a private helper and its call site were in the same file:

```rust
fn render_lofi_theme(spec: &MusicSpec, sample_rate: u32) -> RenderedAudio { /* ... */ }

impl AudioLibrary {
    pub fn from_spec(/* ... */) -> Self {
        add_rendered_audio(audio_sources, render_lofi_theme(&track.arrangement, sample_rate))
    }
}
```

After the split, the helper lives in `audio/render.rs`, while the call site lives
in sibling module `audio/runtime.rs`. The facade has `mod render; mod runtime;`,
and the child modules use `use super::*;`.

### Question

What visibility or facade import change should the split make so the runtime
module can keep using the helper without making it part of the crate's public
API?

### Expected answer

A private `fn` in `audio/render.rs` is only visible inside `render.rs`; sibling
modules such as `runtime.rs` cannot call it. Expose the helper to the parent
`audio` module and siblings without exporting it outside the module subtree:

```rust
// audio/render.rs
#[cfg(feature = "audio")]
pub(super) fn render_lofi_theme(spec: &MusicSpec, sample_rate: u32) -> RenderedAudio {
    /* ... */
}
```

Then import it explicitly in the sibling modules that call it:

```rust
// audio/runtime.rs
#[cfg(feature = "audio")]
use super::render::render_lofi_theme;

// audio/tests.rs
#[cfg(test)]
use super::render::render_lofi_theme;
```

Do not use a plain `pub use` unless external crates or other top-level modules
should rely on this helper as API.

### Why this was easy to miss

The helper still exists after the split, and `pub` on the facade's public API is
not the right goal. The actual invariant is sibling visibility inside a private
module tree. A model that only checks "does the function exist?" misses the Rust
module-boundary rule: private child items are not automatically visible to
sibling modules.

## 2026-05-09: Preserve compile-time asset paths when moving Rust tests into child modules

Tags: `rust-module-refactor`, `include-str`, `asset-paths`, `game-assets`

### Setup

You are splitting a large Bevy game module:

```text
crates/ambition_sandbox/src/audio.rs
```

into a facade plus private children:

```text
crates/ambition_sandbox/src/audio.rs
crates/ambition_sandbox/src/audio/runtime.rs
crates/ambition_sandbox/src/audio/render.rs
crates/ambition_sandbox/src/audio/tests.rs
```

The original inline tests in `audio.rs` loaded a checked-in tune example at
compile time:

```rust
let track: MusicTrackSpec = ron::from_str(include_str!(
    "../assets/ambition/tune_examples/example_drift.ron"
))?;
```

That path was correct when the code lived directly in `src/audio.rs`, because
`../assets` resolved to `crates/ambition_sandbox/assets`.

### Question

When moving this test into `src/audio/tests.rs`, what should you change, and
what general rule should the refactor follow for `include_str!` and
`include_bytes!` calls?

### Expected answer

`include_str!` paths are resolved relative to the file containing the macro, not
relative to the crate root or the old module path. Moving the test one directory
deeper means the path must climb two levels instead of one:

```rust
let track: MusicTrackSpec = ron::from_str(include_str!(
    "../../assets/ambition/tune_examples/example_drift.ron"
))?;
```

A robust split audits every `include_str!`, `include_bytes!`, `#[path = ...]`,
and test fixture path in moved code. For checked-in game assets, either update
the relative path for the new file location or switch to a crate-root anchored
pattern such as:

```rust
include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ambition/tune_examples/example_drift.ron"))
```

The handoff check should include tests, not just `cargo check`, because this
class of error may live under `#[cfg(test)]`:

```bash
cargo test -p ambition_sandbox --lib
```

### Why this was easy to miss

The moved code still looked semantically correct: the asset path text matched the
old file and the asset was still checked in. The invariant that changed was not
module visibility; it was the physical source-file location used by the macro at
compile time.

## 2026-05-09: Keep trait derives aligned with new test assertions after UI hit-test changes

Tags: `rust-tests`, `derive-partialeq`, `ui-hit-testing`, `touch-controls`

### Setup

You are updating touch-control hit tests in a Bevy game. A marker enum identifies
the action button hit by a pointer position:

```rust
#[derive(Component, Clone, Copy, Debug)]
pub enum TouchActionButton {
    Jump,
    Attack,
    Dash,
    Blink,
    Interact,
}

fn touch_action_at_position(pos: Vec2, window_size: Vec2) -> Option<TouchActionButton> {
    /* ... */
}
```

During the same patch, you add a regression test for circular hit testing:

```rust
assert_eq!(touch_action_at_position(square_only, window_size), None);
```

### Question

What should the patch do before handing this off, and how should an agent decide
whether to change the production enum or the test assertion?

### Expected answer

`assert_eq!` compares both sides using `PartialEq`, so
`Option<TouchActionButton>` requires `TouchActionButton: PartialEq`. If the tests
will compare concrete variants or options repeatedly, derive comparison traits on
the small marker enum:

```rust
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchActionButton {
    Jump,
    Attack,
    Dash,
    Blink,
    Interact,
}
```

If the only assertion is absence, an alternative is:

```rust
assert!(touch_action_at_position(square_only, window_size).is_none());
```

For a stable game input API, deriving `PartialEq, Eq` on a copyable marker enum
is usually the better choice because it supports clearer future tests without
changing runtime behavior.

The handoff check should compile tests, not just library code:

```bash
cargo test -p ambition_sandbox --lib
```

### Why this was easy to miss

The refactor changed behavior and tests together. The production function's
return type did not change, but the new assertion introduced a trait requirement
that normal gameplay code did not need. This is a common maintenance trap: test
expressiveness can require additional derives on small domain enums even when the
runtime path compiles.
