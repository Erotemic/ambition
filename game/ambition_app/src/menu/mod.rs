//! Game-side menu host stack: backend-agnostic page model, dispatcher, item
//! effects, and the flat-grid / 3D-cube presentation hosts.
//!
//! `ambition::actors::menu` keeps the lib-coupled settings IR, Map tab, and
//! backend selector; this crate owns app-level wiring and effects.

pub mod dispatch;
pub mod effects;
#[cfg(feature = "bevy_ui_menu")]
pub mod grid_backend;
pub mod kaleidoscope_app;
pub mod model;
pub(crate) mod quality_confirm;
#[cfg(test)]
mod test_support;

#[cfg(all(test, feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
mod parity_tests;
