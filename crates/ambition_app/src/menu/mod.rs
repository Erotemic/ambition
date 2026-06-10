//! The game's menu HOST stack (Stage 20 menu split): the backend-agnostic
//! page model + dispatcher + item-confirm effects, and the two presentation
//! hosts (the flat Bevy-UI grid + the 3D cube). Moved up from the machinery
//! lib because these are app-level host wiring, not reusable machinery.
//!
//! The lib-coupled pieces stay in `ambition_sandbox::menu`: the settings IR
//! (`ir`, read by persistence) the Map tab (`map`, read by presentation), and
//! the backend selector (`backend`).

pub mod dispatch;
pub mod effects;
#[cfg(feature = "bevy_ui_menu")]
pub mod grid_backend;
pub mod kaleidoscope_app;
pub mod model;

#[cfg(all(test, feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
mod parity_tests;
