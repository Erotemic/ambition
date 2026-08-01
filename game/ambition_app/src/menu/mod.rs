//! Game-side menu host stack: backend-agnostic page model, dispatcher, item
//! effects, and the flat-grid / 3D-cube presentation hosts.
//!
//! `ambition_platformer2d::actors::menu` keeps the lib-coupled settings IR, Map tab, and
//! backend selector; this crate owns app-level wiring and effects.

pub mod dispatch;
pub mod effects;
#[cfg(feature = "bevy_ui_menu")]
pub mod grid_backend;
// **Always compiled, despite the name.** (repair_wasm §1)
//
// This module was gated on `kaleidoscope_menu` because of what it is CALLED,
// and the gate was wrong: of its ~1900 lines, nineteen mention `bevy_lunex`.
// Everything else is the backend-neutral menu host — the cursor, the system-menu
// navigation, item actions, page building — which `dispatch.rs` and the bevy_ui
// `grid_backend.rs` import unconditionally because they genuinely need it. So a
// build without the cube (the web persona) failed to compile the FLAT menu, which
// has nothing to do with Lunex.
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
