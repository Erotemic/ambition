//! Renderers for a [`crate::MenuPageModel`].
//!
//! The same backend-agnostic page model can be drawn by multiple presentations.
//! This crate ships the flat, tabbed [`bevy_ui`] renderer; the bevy_lunex 3D
//! OoT-style cube renderer is the optional `ambition_menu_kaleidoscope`
//! extension crate (E1e) — a host installs it to draw the same model as a cube.

pub mod bevy_ui;
