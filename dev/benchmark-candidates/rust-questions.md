# Rust benchmark candidates

These are small, concrete Rust maintenance questions distilled from Ambition
refactor mistakes. Each item should be answerable without knowing the eventual
compiler error, but should contain enough surrounding context that a model has
to preserve the intended API rather than only patch the local symptom.

## 2026-05-11: Keep trait bounds and extension-trait imports during child-module splits

Tags: `rust-module-refactor`, `rust-traits`, `extension-trait`, `trait-bounds`, `cargo-command`

### Setup

You are refactoring the movement/collision layer in a Rust game engine. The
existing geometry module defines an extension trait over Bevy's `Aabb2d`:

```rust
pub trait AabbExt {
    fn bottom(self) -> f32;
    fn strict_intersects(self, rhs: Self) -> bool;
    fn sweep_time_of_impact(self, delta: Vec2, rhs: Self) -> Option<f32>;
}
```

The original `movement.rs` lived in one file and imported that extension trait
near the top:

```rust
use crate::geometry::{Aabb, AabbExt};
```

During a refactor, you split movement into private child modules and also add a
richer sweep result:

```rust
pub struct AabbSweepHit {
    pub time_of_impact: f32,
    pub normal1: Vec2,
}

pub trait AabbExt {
    fn bottom(self) -> f32;
    fn strict_intersects(self, rhs: Self) -> bool;
    fn sweep_hit(self, delta: Vec2, rhs: Self) -> Option<AabbSweepHit>;
    fn sweep_time_of_impact(self, delta: Vec2, rhs: Self) -> Option<f32> {
        self.sweep_hit(delta, rhs).map(|hit| hit.time_of_impact)
    }
}
```

The extracted `movement/integration.rs` uses:

```rust
let prev_bottom = player.aabb().bottom();
```

A handoff recommends this command:

```bash
cargo test -p ambition_engine movement geometry world
```

### Question

Before handing off this refactor, what Rust trait/module invariants and command
syntax invariants should you check? What minimal code-shape changes preserve the
old behavior while allowing the richer sweep hit API?

### Expected answer

A default trait method that takes `Self` by value outside the receiver position
must either avoid by-value `Self` or explicitly require sized implementors. This
extension trait is intended for concrete AABB values, so keep the by-value API
and add a method-level bound:

```rust
pub trait AabbExt {
    fn bottom(self) -> f32;
    fn strict_intersects(self, rhs: Self) -> bool;
    fn sweep_hit(self, delta: Vec2, rhs: Self) -> Option<AabbSweepHit>;
    fn sweep_time_of_impact(self, delta: Vec2, rhs: Self) -> Option<f32>
    where
        Self: Sized,
    {
        self.sweep_hit(delta, rhs).map(|hit| hit.time_of_impact)
    }
}
```

Every child module that calls extension-trait methods must import the extension
trait itself; imports in the facade or sibling modules are not inherited. The
extracted movement integration module needs:

```rust
use crate::geometry::AabbExt;
```

Finally, generated validation commands must be valid Cargo syntax. `cargo test`
accepts at most one optional test-name filter before `--`; it does not accept
multiple positional filters. Use one broad command or separate filtered commands:

```bash
cargo test -p ambition_engine
cargo test -p ambition_engine movement
cargo test -p ambition_engine geometry
cargo test -p ambition_engine world
```

For this refactor, the broad package command is the safer handoff check because
it compiles all moved child modules and tests together.

### Why this was easy to miss

All three mistakes are consequences of reasoning at the facade level instead of
at the physical Rust item/module boundary. The default method looked like a thin
compatibility wrapper around the new richer sweep API, but adding a body made the
trait's implicit `Self` sizing assumptions compile-time visible. The `.bottom()`
call looked unchanged from the original file, but extension-trait method lookup
requires the trait import in the module that contains the call. The Cargo command
looked like a natural list of affected areas, but Cargo parses those words as
positional arguments, not independent filters.

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

## 2026-05-09: Don't auto-derive a per-frame edge flag from a held axis

Tags: `game-input`, `edge-vs-held-state`, `bevy-resource`, `multi-source-input`

### Setup

A Bevy game has a per-frame `ControlFrame` resource consumed by simulation
systems:

```rust
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct ControlFrame {
    /// Continuous axis from -1.0 (held up) through 0.0 (centered) to +1.0
    /// (held down). Read by gameplay code that wants the held value (crouch
    /// gating, fast-fall accel, ladder climb).
    pub move_y: f32,
    /// True ONLY on the frame the player just pressed Down. False while held.
    /// Counted by `SandboxRuntime::register_down_tap`; two distinct
    /// `down_pressed` edges within the double-tap window fire the MorphBall
    /// transition.
    pub down_pressed: bool,
    /// (other axes / edges...)
}
```

The desktop input pipeline populates `down_pressed` from a Leafwing edge
query (`actions.just_pressed(MoveDown)`) and `move_y` from the held axis.

You're now writing a *second* source: an `AgentAction` → `ControlFrame`
converter for an RL agent. `AgentAction` looks like:

```rust
#[derive(Default, Debug, Clone, Copy)]
pub struct AgentAction {
    pub move_x: f32,
    pub move_y: f32,
    pub jump: bool,
    pub attack: bool,
    pub dash: bool,
    pub blink: bool,
    // ...
}
```

The agent emits one `AgentAction` per simulation step.

### Question

Sketch the converter from `AgentAction` to `ControlFrame`. Be specific
about how you populate axis fields (`move_y`) versus edge fields
(`down_pressed`, `up_pressed`). What symptoms appear at runtime if you
get the edge fields wrong, and what test would catch the bug pre-merge?

### Expected answer

Continuous axes copy across each frame:

```rust
frame.move_y = action.move_y;
```

Edge fields **must not** be derived from the held axis. The `>0.5`
threshold of a held value is true on every frame the user holds the
input, not just the frame they just pressed it. Either:

- (a) give `AgentAction` matching explicit edge fields
  (`AgentAction { down_pressed: bool, up_pressed: bool, ... }`) that
  default to `false` and that the agent sets only on the frame it wants
  the edge, **or**
- (b) keep a one-frame `Local<f32>` history of the source's own axis
  in the converter and emit the edge from a threshold *crossing* (this
  frame above threshold, last frame below).

If you get this wrong, holding `move_y = 1.0` produces
`down_pressed = true` every frame. `register_down_tap` counts each
frame as a fresh tap, the double-tap-down window fires on frame 2,
`MorphBall` transitions, and the next frame's body-mode driver flips
back. The user sees ~30 Hz body-mode oscillation and a flickering
sprite/camera while merely holding crouch.

A multi-frame regression test pins this:

```rust
#[test]
fn held_down_axis_does_not_fire_repeated_edges() {
    let mut sim = SandboxSim::default();
    for _ in 0..30 {
        sim.set_action(AgentAction { move_y: 1.0, ..default() });
        sim.step();
    }
    assert_eq!(sim.body_mode(), BodyMode::Crouching);
    // Per-frame |delta_y| should also stay under the body-resize step
    // size — the test fails on the buggy converter.
}
```

When more than one source can write `ControlFrame` in the same frame
(e.g. keyboard + on-screen touch joystick), every writer additionally
must gate on its *own* activity before writing — otherwise a stateless
writer that runs each frame will overwrite state another writer just
computed. The same multi-frame held-input test catches both the
"derived edge" failure mode and the "stomped by inactive writer"
failure mode.

### Why this was easy to miss

`down_pressed = move_y > 0.5` is the natural shape for a translator
that wants to feel responsive and has no per-source frame history.
The bug is invisible in single-frame unit tests; only multi-frame
held-input coverage reproduces it. In Ambition this exact bug class
shipped *three times* in three different writers (one RL converter,
two touch paths) before the held-Down-for-N-frames test pattern
became standard.

## 2026-05-09: Don't overlay derived gameplay signals onto an input boundary resource

Tags: `bevy-resource`, `cross-system-signal`, `game-input`, `architecture-seam`

### Setup

A Bevy game frames the simulation as several systems sharing
`Res<ControlFrame>` and `ResMut<SandboxRuntime>`. The schedule is:

```text
populate_control_frame_from_actions  (rebuilds ControlFrame each frame)
sandbox_update                       (mutates ControlFrame in place via ResMut)
body_mode_driver                     (reads Res<ControlFrame> + Res<SandboxRuntime>)
```

`sandbox_update` calls a small helper `input_timer_phase` that wants to
detect a double-tap-down gesture and signal the later
`body_mode_driver` system to enter MorphBall this frame. The simplest
shape an agent reaches for is:

```rust
fn input_timer_phase(controls: &mut ControlFrame, runtime: &mut SandboxRuntime) {
    if runtime.register_down_tap() {
        controls.fast_fall_pressed = true; // signal the body-mode driver
    }
}

fn body_mode_driver(
    controls: Res<ControlFrame>,
    runtime: Res<SandboxRuntime>,
    /* ... */
) {
    if controls.fast_fall_pressed {
        // enter MorphBall
    }
}
```

The in-frame fast-fall path inside `sandbox_update` already reads
`controls` via the same `&mut` borrow, so it sees the flag and
fast-fall works. But the body-mode driver, which runs as a separate
Bevy system later in the same frame, never enters MorphBall.

### Question

Why doesn't the body-mode driver see the flag, and what's the right
shape for a one-frame "edge pending" signal that crosses Bevy systems
without re-coupling the gesture detector to the input boundary?

### Expected answer

Two issues compound:

- `populate_control_frame_from_actions` runs every frame and
  *rebuilds* `ControlFrame` from the input pipeline. Even when
  `sandbox_update` mutates `controls` via `ResMut<ControlFrame>` and
  later systems can technically observe the change in the same frame,
  the very next frame's input rebuild discards anything derived. The
  flag's lifetime is fragile.
- More importantly, `ControlFrame` is the *input boundary* — the
  contract is "this is what the input pipeline says happened this
  frame." Overlaying a derived gameplay signal ("double-tap detected,
  please trigger MorphBall") onto an input field conflates two layers
  and is exactly the kind of seam violation that breaks silently in
  refactors.

The right shape is a separate one-frame "pending edge" field on a
long-lived state resource:

```rust
#[derive(Resource, Default)]
pub struct SandboxRuntime {
    pub double_tap_down_pending: bool,
    // ... other long-lived sim state
}
```

`input_timer_phase` sets it whenever `register_down_tap` returns
`true`. The body-mode driver consumes it via `mem::take` so a stale
signal can't latch across frames:

```rust
let double_tap_down = std::mem::take(&mut runtime.double_tap_down_pending);
if double_tap_down {
    // enter MorphBall
}
```

`SandboxRuntime::reset` clears it defensively (death/respawn must not
leave a stale gesture pending).

A regression test pins the routing — set `controls.fast_fall_pressed
= true` on the resource directly and assert the driver does **not**
enter MorphBall. The test message itself documents the invariant:

```rust
assert_eq!(
    runtime.player.body_mode,
    BodyMode::Standing,
    "the body-mode driver must read runtime.double_tap_down_pending, \
     not controls.fast_fall_pressed (which sandbox_update consumes \
     on a local copy that doesn't reach later systems)"
);
```

### Why this was easy to miss

In Ambition the in-frame consumer (fast-fall integration) ran inside
the same `&mut ControlFrame` scope as the gesture detector and saw
the flag, so the detector "looked like" it was working. The body-mode
driver was a separate Bevy system that visibly didn't fire the
gesture, but only one of two consumers was failing — making the bug
look like a body-mode driver issue instead of a routing/seam issue.
The right test is "set the field directly on the resource and assert
the driver does NOT respond" — the negative assertion is what pins
the seam.

## 2026-05-09: Aligning a deterministic-sim trace dump with its replay loop

Tags: `record-replay`, `deterministic-sim`, `off-by-one`, `ci-fixture`

### Setup

A Bevy/headless game has a deterministic, fixed-timestep simulation
(deterministic RNG, fixed `dt`, all input flows through `ControlFrame`).
You add a `--dump-trace` flag to the headless binary that writes a JSON
file with one `Frame` per step. The dump convention is documented as
**"record state AFTER each step"** and the format is:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DumpedFrame {
    /// The ControlFrame applied during step i+1 (i.e. the inputs that
    /// produced the state recorded below).
    pub controls: ControlFrame,
    /// Player position AFTER step i+1 ran.
    pub player_pos: Vec2,
}

pub struct DumpFile {
    pub frames: Vec<DumpedFrame>,
}
```

You now want a `trace_replay` binary that takes a dump file, re-runs
the same simulation, and asserts that the live post-step position
matches the recorded `player_pos` for every step. The intent is a CI
determinism guard — any drift is a regression.

### Question

Write the replay loop. Be precise about which `frames[i].controls` is
applied on which step, and which `frames[i].player_pos` is compared
to which post-step state. What sub-pixel symptom appears if you get
this wrong, and what acceptance criterion proves the determinism
guard is real?

### Expected answer

The dump records `(controls applied during step i+1, pos after step
i+1)` into `frames[i]`. So in the replay, `frames[i].controls` drives
the i-th step (0-indexed), and the post-step position must equal
`frames[i].player_pos`:

```rust
for i in 0..frames.len() {
    sim.set_controls(frames[i].controls);
    sim.step();
    assert_eq!(sim.player_pos(), frames[i].player_pos);
}
```

**Do not** `frames.iter().skip(1)` and apply `frames[i].controls` on
step `i+1`. That pairs the controls of one frame with the post-state
of a different frame in the comparison — any non-zero player velocity
will introduce a one-frame offset that drifts as the integrator runs,
producing sub-pixel `dx` / `dy` divergence by frame 1 and accumulating
across the trace.

The acceptance criterion that proves the guard is real:

```text
cargo run --bin headless -- 30 --dump-trace /tmp/t/
cargo run --bin trace_replay -- /tmp/t/ambition_trace_*.json
# expected: max_dx == 0.0 && max_dy == 0.0  (NOT "close to zero")
```

Sub-pixel is not "close enough" on a deterministic sim — true
determinism gives bit-exact equality. If the round trip is exact, the
trace replay binary is suitable as a CI fixture: a checked-in trace
file plus a one-line assertion will catch any unintentional change to
sim behavior.

### Why this was easy to miss

Sub-pixel drift looks like float noise — easy to shrug off with "must
be FP roundoff" and write off as acceptable. It isn't. On a
deterministic sim with bit-exact RNG and fixed dt, the integrator is
a pure function of (state, controls); any non-bit-exact divergence
across a re-run is a real bug, almost always in the alignment of
controls and state across the seam. The test that catches it is
"replay a trace you just dumped and demand equality", not "replay and
assert |dx| < 1.0".

The simplest defense is to write the alignment convention down in
plain words next to the format definition (e.g.
`docs/gameplay_trace_recorder.md`) and make the replay loop's first
line read like prose: "frames[i].controls drives step i; the
resulting position must equal frames[i].player_pos."

## 2026-05-09: Preserve "exactly one music source audible" across an outro overlap

Tags: `state-machine-invariant`, `music-director`, `bevy-resource`,
`overlap-window`, `architecture-seam`

### Setup

A Bevy game has a music director with two audio paths sharing one
output:

- A simple base track on `MusicChannel` (the room's lofi loop).
- An adaptive cue (e.g. boss / encounter music) layered across
  several `bevy_kira_audio` channels with crossfade-driven section
  transitions.

The director's mode enum looks like:

```rust
pub enum MusicDirectorMode {
    Idle,
    SimpleTrack,
    AdaptiveIntro,
    AdaptiveLoop,
    AdaptiveOutro,
    AdaptiveFinished,
}

pub struct MusicDirectorState {
    pub mode: MusicDirectorMode,
    pub active_cue_id: Option<String>,
    /* timers, current state/section ids, gain targets, ... */
}
```

When an encounter clears, the cleared-state binding drives the cue
into its **outro** section. Near the end of the outro the director
calls a `resume_simple_music(...)` helper that ramps up the room's
lofi track on `MusicChannel` so the return-to-room transition isn't
a hard cut. Today that helper unconditionally sets
`director.mode = SimpleTrack` and `director.last_simple_track =
Some(target)` after switching `MusicChannel`'s playing track.

A separate per-frame `drive_adaptive_cue_state(...)` decides
whether to (re)start the adaptive cue from its intro. Its current
guard is:

```rust
if director.active_cue_id.as_deref() != Some(cue.id.as_str()) {
    base_music_channel.stop().fade_out(...);
    start_adaptive_state(director, cue, target_state, /* ... */);
    return;
}
// Otherwise: fall through into pending-state /
// crossfade / loop-section bookkeeping, NEVER stops
// base_music_channel.
```

A user reports: starting the encounter, beating it, and dying at
the same time so the player resets. They reset and re-trigger the
encounter while the director is still in the outro tail. Now the
room's lofi *and* the adaptive layers play at the same time.

### Question

What is the invariant the director must preserve, where exactly is
it broken, and what is the smallest fix? Include a free function
that captures the decision so it can be unit-tested without
spinning up Bevy resources.

### Expected answer

The invariant is:

> Simple base track audible ⇔ no adaptive cue identity, no adaptive
> layers audible.
>
> Adaptive cue active ⇔ base music channel stopped/faded out.

The break is two-part:

1. `resume_simple_music` flips `director.mode = SimpleTrack` while
   `director.active_cue_id` is still `Some(cue_id)` and adaptive
   channels are still playing the outro tail. The director enters
   a state the invariant explicitly forbids.
2. `drive_adaptive_cue_state`'s guard only stops the base channel
   when `active_cue_id` differs from the new cue. On encounter
   restart during the overlap window, the cue id matches, so the
   guard skips the stop-and-restart path. Both paths now claim
   the output.

The minimal fix is two changes:

**(a)** `resume_simple_music` takes a `set_mode_to_simple_track:
bool` parameter. Pass `false` from the outro-overlap call site;
pass `true` only from full-shutdown call sites that have already
cleared `active_cue_id` and stopped adaptive layer channels. The
mode then accurately reflects what's audible, not what we wish
were.

**(b)** Extract the restart predicate from
`drive_adaptive_cue_state` into a free function and broaden it:

```rust
pub(super) fn should_restart_adaptive(
    director_active_cue: Option<&str>,
    director_mode: MusicDirectorMode,
    cue_id: &str,
    target_state_is_outro: bool,
) -> bool {
    let same_cue = director_active_cue == Some(cue_id);
    let mode_lost_adaptive = matches!(
        director_mode,
        MusicDirectorMode::SimpleTrack
            | MusicDirectorMode::Idle
            | MusicDirectorMode::AdaptiveFinished,
    );
    let outro_to_active = same_cue
        && director_mode == MusicDirectorMode::AdaptiveOutro
        && !target_state_is_outro;
    !same_cue || mode_lost_adaptive || outro_to_active
}
```

`outro_to_active` is the case the original guard missed:
encounter restart during the outro tail returns the directive to
a non-outro state while the cue id still matches, so the same-cue
fast path would otherwise skip the base-channel stop. The
`mode_lost_adaptive` clause is a defensive belt-and-suspenders
check against any other code path that might leave the director
claiming an active cue while the simple base track is the
audible source.

The free function form lets a unit test cover all six scenarios
(different cue, no prior cue, same-cue steady-state, mode-says-
simple, outro-to-active-restart, outro-continues-into-outro)
without instantiating an `App` with audio channels and asset
servers.

### Why this was easy to miss

The single-channel mental model — "simple track plays through
MusicChannel, adaptive plays through layer channels, they're
different channels so they can't collide" — is backwards. The
director's job is to ensure those channels never produce sound
at the same time, regardless of how they're plumbed. The first
guard was written assuming "different cue id ⇒ source switch",
which is true but covers only one of the three scenarios that
break the audible invariant. The overlap-window restart is the
specific scenario the unit test pins.
