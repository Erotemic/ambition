# Benchmark candidate: context-heavy Rust module split review

Date: 2026-05-11

## Prompt

You are given a proposed behavior-preserving Rust refactor for a Bevy game crate.
The patch splits three large files into facade modules plus child modules:

- `crates/ambition_gameplay_core/src/body_mode.rs`
- `crates/ambition_gameplay_core/src/mobile_input.rs`
- `crates/ambition_gameplay_core/src/map_menu.rs`

The intended design is:

```text
body_mode.rs
body_mode/mechanics.rs
body_mode/morph_ball.rs
body_mode/tests.rs

mobile_input.rs
mobile_input/bevy_plugin.rs

map_menu.rs
map_menu/model.rs
map_menu/systems.rs
map_menu/input.rs
map_menu/pointer.rs
map_menu/ui.rs
map_menu/tests.rs
```

The author says this is a mechanical split only. They kept each original file as
an entry-point facade, moved implementation clusters into children, and added
`use super::*` in child modules to reduce duplicate imports. The patch also
adds a few facade-level `use` / `pub(super) use` statements so tests and sibling
modules can keep referring to helper functions by their old paths.

Review the refactor before it is handed to a developer to run. Focus on issues
that are likely to be caught by `cargo fmt --all` or `cargo test -p
ambition_gameplay_core --lib ...`, not on subjective architecture preferences.

The changed code includes these representative fragments:

```rust
// crates/ambition_gameplay_core/src/map_menu.rs
mod input;
mod model;
mod pointer;
mod systems;
#[cfg(test)]
mod tests;
mod ui;

pub(super) use ui::short_room_label;
```

```rust
// crates/ambition_gameplay_core/src/map_menu/ui.rs
use super::*;

fn short_room_label(label: &str) -> String {
    label.replace("rooms/", "")
}
```

```rust
// crates/ambition_gameplay_core/src/body_mode/tests.rs
use super::*;

fn body_app(world: ae::World) -> App {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.insert_resource(WorldResource(world));
    app.add_systems(Update, update_body_mode);
    app
}
```

```rust
// crates/ambition_gameplay_core/src/body_mode.rs
use bevy::prelude::*;

mod mechanics;
mod morph_ball;
#[cfg(test)]
mod tests;

pub use mechanics::{BodyMode, BodyModeChanged};
```

Give a concise review. Identify any compile-risk details you would fix before
shipping the overlay, and propose minimal code changes. Avoid broad public API
changes unless they are necessary.

## Expected answer rubric

A strong answer notices that the patch is broad enough that small scope details
are easy to miss, then points out at least these two compile-risk details:

1. The facade tries to `pub(super) use ui::short_room_label`, but the function in
   `ui.rs` is private. Either the helper must be made visible enough for the
   re-export, or the facade should use a private `use` if only the facade/tests
   need it. Prefer the narrower fix.
2. `body_mode/tests.rs` uses `App`, `Time`, and `Update` as unqualified Bevy
   symbols. Those names were in scope in the old parent file, but after moving
   tests into a child module the test file needs its own explicit imports, such
   as `use bevy::prelude::{App, Time, Update};`.

Good answers may also recommend checking every extracted child module for local
imports of framework/prelude types, extension traits, and re-exported helpers.
They should not default to making helpers fully `pub` unless the public API truly
requires it.

## Why this is a useful benchmark

The prompt intentionally frames the task as a normal multi-file mechanical
refactor review rather than naming the failure mode. The candidate has to notice
small Rust module-scope details inside a larger context, which is exactly where
LLMs often over-focus on the high-level architecture and miss compile-time
visibility/import errors.
