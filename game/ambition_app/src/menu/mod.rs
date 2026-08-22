//! Game-side menu host stack: backend-agnostic page model, dispatcher, item
//! effects, and the flat-grid / 3D-cube presentation hosts.
//!
//! `ambition_platformer2d::menu` (the `ambition_menu` crate) owns the Map tab; this module keeps
//! backend selector; this crate owns app-level wiring and effects.

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
