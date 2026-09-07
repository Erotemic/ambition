//! Game-side menu host stack: backend-agnostic page model, dispatcher, item
//! effects, and the flat-grid / 3D-cube presentation hosts.
//!
//! `ambition_platformer2d::menu` (the `ambition_menu` crate) owns the Map tab; this module keeps
//! backend selector; this crate owns app-level wiring and effects.

/// The two inventory backends' menu-nav systems, ordered against each other.
///
/// Both join `MenuNavConsume`, which orders the frame WRITER before them and says
/// nothing about which of THEM runs first. That mattered once they started
/// consuming: `MenuControlFrame::consume_nav_edges()` clears the nav edges for
/// every later reader in any crate, so "whoever runs first eats the press" was the
/// real arbitration and its order was whatever the scheduler picked.
///
/// Either order is correct — at most one backend is effective in a frame, and each
/// nav is gated on its own backend — so what this fixes is that the order is
/// STATED. Grid first because it is the backend that survives when the cube is
/// compiled out.
///
/// The order lives here, not as a `.after()` inside either backend: the two are
/// registered under DIFFERENT features (`bevy_ui_menu` / `kaleidoscope_menu`), so a
/// direct system-to-system edge would only exist in builds that compile both. A set
/// can be ordered whether or not it has members.
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GridMenuNav;

/// The cube backend's half of [`GridMenuNav`]'s ordering.
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct KaleidoscopeMenuNav;

pub mod dispatch;
pub mod effects;
#[cfg(feature = "bevy_ui_menu")]
pub mod grid_backend;
// Always compiled, despite the name. (repair_wasm §1)
//
// Everything else is the backend-neutral menu host — the cursor, the system-menu navigation,
// item actions, page building — which `dispatch.rs` and the bevy_ui `grid_backend.rs` import
// unconditionally because they genuinely need it. So a build without the cube (the web persona)
// failed to compile the FLAT menu, which has nothing to do with Lunex.
//
// The cube renderer itself is still gated, item by item, inside the module. A
// headless or web build pays for no 3D UI toolkit; it just gets to have a menu.
pub mod kaleidoscope_app;
pub mod model;
pub(crate) mod quality_confirm;
#[cfg(test)]
mod test_support;

#[cfg(all(test, feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
mod parity_tests;
