//! Renderers for a [`crate::MenuPageModel`].
//!
//! The same backend-agnostic page model can be drawn by multiple presentations.
//! Today there are two real consumers, which together validate the engine seam:
//!
//! * [`kaleidoscope`] — the bevy_lunex 3D OoT-style cube renderer.
//! * [`bevy_ui`] — a flat, tabbed `bevy_ui` renderer (the second presentation).

pub mod bevy_ui;
pub mod kaleidoscope;
